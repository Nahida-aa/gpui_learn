//! 自包含 tar.gz 应用包 —— 对齐 zed `script/bundle-linux` 的产物形态。
//!
//! 产出 `<bin>.app/` 目录打成的 tar.gz：
//!
//! ```text
//! aacode.app/
//! ├── bin/aacode                                   # 二进制（本工具不负责构建）
//! ├── lib/libstdc++.so.6 …                         # ldd 依赖，按排除清单过滤后拷入
//! └── share/
//!     ├── applications/dev.aa.AaCode.desktop
//!     └── icons/hicolor/{512x512,1024x1024}/apps/aacode.png
//! ```
//!
//! ## 为什么要自带 `lib/`
//!
//! CI 在 ubuntu 上构建，系统 libstdc++ 常比用户机器新——不打包会在旧发行版上
//! 缺 GLIBCXX 符号。但**绝不打包 glib 家族**：gtk 等宿主库是 dlopen 的，链的
//! 是系统 glib，换成包内旧版会符号不匹配（zed bundle-linux:199-207 同样论述）。
//! 排除清单见 [`EXCLUDED_LIB_STEMS`]。
//!
//! ## 与 pkgbuild 的分工
//!
//! 本模块**只做 staging + 打包**，不跑 `cargo build`——构建期的环境变量
//! （webrtc 缓存、rustflags、lld…）是项目自己的事，CI 里先构建再调本工具。
//! 二进制找不到时明确报错，不静默产出缺文件的包。

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::{
    BundleMeta, icon_size_dir, load_bundle_meta, resolve_desktop_entry, resolve_icon_name,
    write_file,
};

/// 不打包的库（对齐 zed bundle-linux 的 `find_libs` 排除清单）：
/// - glibc / libgcc / libm / pthread / dl / asound：任何 glibc 系统的必备品，
///   打包旧版只会 ABI 冲突
/// - glib 家族及其私有依赖（gobject/gio/gmodule/pcre/mount/blkid/selinux/
///   ffi/resolv/zlib）：被 dlopen 的宿主库依赖，必须链**系统**版本
///
/// 比较用词干（见 [`lib_stem`]），所以 `libz` 命中而 `libzstd` 不命中。
const EXCLUDED_LIB_STEMS: &[&str] = &[
    "libc",
    "libgcc_s",
    "libm",
    "libpthread",
    "libdl",
    "libasound",
    "libglib-2.0",
    "libgobject-2.0",
    "libgio-2.0",
    "libgmodule-2.0",
    "libpcre",
    "libpcre2-8",
    "libmount",
    "libblkid",
    "libselinux",
    "libffi",
    "libresolv",
    "libz",
];

/// soname → 比较用词干：`libstdc++.so.6` → `libstdc++`、`libz.so.1` → `libz`
/// （`libzstd.so.1` → `libzstd`，不在排除清单里，会打包）。
fn lib_stem(soname: &str) -> &str {
    match soname.find(".so") {
        Some(i) => &soname[..i],
        None => soname,
    }
}

/// 解析 `ldd` 输出，返回要拷进 `lib/` 的 .so 路径。
///
/// 只取 `foo.so => /path/foo.so (0x…)` 形式：`linux-vdso.so.1` 没有 `=>`，
/// 跳过（也不需要打包）。
/// `=> not found` 说明二进制有解析不了的 DT_NEEDED，装出去也是死的——
/// 直接报错，不静默跳过。
pub fn libs_to_bundle(ldd_output: &str) -> Result<Vec<PathBuf>> {
    let mut libs = Vec::new();
    for line in ldd_output.lines() {
        let Some((soname, rest)) = line.split_once("=>") else {
            continue;
        };
        let soname = soname.trim();
        let target = rest.split_whitespace().next().unwrap_or("");
        if target == "not" {
            bail!("ldd 报告 `{soname} => not found`：二进制有未解析的依赖，打包出来也无法运行");
        }
        if target.is_empty() {
            continue;
        }
        // 动态 loader（ld-linux*.so）：ELF 的 PT_INTERP 用绝对路径钉死它，
        // lib/ 里的副本运行时不会被用到（除非改写 PT_INTERP）——死重。
        // ldd 这一行的左边可能是裸名也可能是 /lib64/ 全路径，取 basename 判断。
        let base = soname.rsplit('/').next().unwrap_or(soname);
        if base.starts_with("ld-linux") {
            continue;
        }
        if EXCLUDED_LIB_STEMS.contains(&lib_stem(soname)) {
            continue;
        }
        libs.push(PathBuf::from(target));
    }
    Ok(libs)
}

pub struct TargzOptions<'a> {
    /// 源码目录（含 Cargo.toml 与 resources/）
    pub project_dir: &'a Path,
    /// 身份/打包配置来源引用；**必填**，无默认值
    pub bundle_meta: &'a str,
    /// 覆盖二进制名（默认取 Cargo.toml 的 [[bin]] 推导链）
    pub bin_name: Option<&'a str>,
    /// 覆盖图标基础名（默认取二进制名）
    pub icon_name: Option<&'a str>,
    /// 覆盖 .desktop 模板路径
    pub desktop_template: Option<&'a Path>,
    /// 覆盖输出文件路径，默认
    /// `<project>/target/aa-bundle/targz/<bin>-linux-<arch>.tar.gz`
    pub out: Option<&'a Path>,
    /// 直接指定二进制路径，跳过 cargo 推导。测试用；CI 把产物放在自定义
    /// 位置时也可用。
    pub bin_path: Option<&'a Path>,
}

/// 生成 tar.gz，返回产物路径。
pub fn generate(opts: &TargzOptions) -> Result<PathBuf> {
    let info = crate::pkgbuild::read_package_info(opts.project_dir)?;
    let meta: BundleMeta = load_bundle_meta(opts.bundle_meta, opts.project_dir)?;
    let bin = crate::pkgbuild::resolve_bin_name(&info, opts.bin_name, opts.project_dir)?;
    let icon = resolve_icon_name(opts.icon_name, bin);

    // 二进制：覆盖路径 > cargo metadata 问出的 target_directory/release/<bin>。
    // 本工具不构建——构建环境（webrtc/rustflags/lld）是项目自己的事。
    let bin_path = match opts.bin_path {
        Some(p) => p.to_path_buf(),
        None => resolve_binary_path(opts.project_dir, bin),
    };
    anyhow::ensure!(
        bin_path.exists(),
        "找不到二进制 {}：先 `cargo build --release -p {}`（aa-bundle 只打包，不构建）",
        bin_path.display(),
        info.name
    );

    let app_name = format!("{bin}.app");
    let out = match opts.out {
        Some(p) => p.to_path_buf(),
        None => opts.project_dir.join(format!(
            "target/aa-bundle/targz/{bin}-linux-{arch}.tar.gz",
            arch = std::env::consts::ARCH
        )),
    };

    // staging 放系统临时目录：tar 完就删，不污染 target/；中途失败也清
    // （下面的闭包无论成败都走到 remove_dir_all）。
    let staging = std::env::temp_dir().join(format!(
        "aa-bundle-targz-{}-{app_name}",
        std::process::id()
    ));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).ok();
    }
    let result = (|| {
        stage_app(opts, &meta, bin, &icon, &bin_path, &staging, &app_name)?;
        pack_targz(&staging, &app_name, &out)
    })();
    std::fs::remove_dir_all(&staging).ok();
    result?;

    println!("已生成 {}", out.display());
    println!(
        "解包即用：tar -xzf {} -C <目标目录>，再把 {app_name}/bin/{bin} 软链进 PATH",
        out.display()
    );
    Ok(out)
}

/// 组装 `<bin>.app/` 目录树。
fn stage_app(
    opts: &TargzOptions,
    meta: &BundleMeta,
    bin: &str,
    icon: &str,
    bin_path: &Path,
    staging: &Path,
    app_name: &str,
) -> Result<()> {
    let app_dir = staging.join(app_name);

    // 1. 二进制 → bin/
    copy_file(bin_path, &app_dir.join("bin").join(bin))?;

    // 2. ldd 依赖 → lib/。静态二进制（ldd 失败）没有可打包的依赖，跳过。
    let ldd = Command::new("ldd")
        .arg(bin_path)
        .output()
        .with_context(|| format!("对 {} 运行 ldd 失败", bin_path.display()))?;
    let libs = if ldd.status.success() {
        libs_to_bundle(&String::from_utf8_lossy(&ldd.stdout))?
    } else {
        println!("提示：ldd 对 {bin} 失败（静态链接？），跳过 lib/ 打包。");
        Vec::new()
    };
    for lib in &libs {
        anyhow::ensure!(
            lib.exists(),
            "ldd 报告的依赖 {} 不存在",
            lib.display()
        );
        copy_file(lib, &app_dir.join("lib").join(lib.file_name().context("依赖路径没有文件名")?))?;
    }

    // 3. 图标 → share/icons/hicolor/<size>/apps/<icon>.png
    for icon_path in meta.icon.iter().flatten() {
        let abs = opts.project_dir.join(icon_path);
        anyhow::ensure!(
            abs.exists(),
            "bundle 元数据 {} 的 icon 指向的文件不存在：{}",
            opts.bundle_meta,
            abs.display()
        );
        let size = icon_size_dir(icon_path);
        copy_file(
            &abs,
            &app_dir.join(format!("share/icons/hicolor/{size}/apps/{icon}.png")),
        )?;
    }
    if meta.icon.as_ref().is_none_or(|v| v.is_empty()) {
        println!("提示：bundle 元数据里没有 icon，tar.gz 将不带应用图标。");
    }

    // 4. .desktop → share/applications/<id>.desktop。
    //    Exec 用裸 bin 名：tar.gz 可重定位，解包位置由安装脚本决定（zed 同款）。
    let (name, content) = resolve_desktop_entry(
        meta,
        bin,
        icon,
        bin,
        opts.project_dir,
        opts.bundle_meta,
        opts.desktop_template,
    )?;
    write_file(&app_dir.join("share/applications").join(&name), &content)?;

    Ok(())
}

/// `tar -czf <out> -C <staging> <app_name>`。
fn pack_targz(staging: &Path, app_name: &str, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录 {} 失败", parent.display()))?;
    }
    let status = Command::new("tar")
        .args([
            "-czf",
            &out.to_string_lossy(),
            "-C",
            &staging.to_string_lossy(),
            app_name,
        ])
        .status()
        .context("运行 tar 失败")?;
    if !status.success() {
        bail!("tar 打包失败（退出码 {:?}）", status.code());
    }
    Ok(())
}

/// 二进制路径：`cargo metadata --no-deps` 问出 target_directory（workspace 根，
/// 尊重 CARGO_TARGET_DIR），拼 `release/<bin>`。metadata 拿不到时退回 crate
/// 目录的 target/，由调用方的 exists() 检查兜底。
fn resolve_binary_path(project_dir: &Path, bin: &str) -> PathBuf {
    let target_dir = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("target_directory")?.as_str().map(PathBuf::from))
        .unwrap_or_else(|| project_dir.join("target"));
    target_dir.join("release").join(bin)
}

/// 拷文件并自动建父目录。权限随源文件（`fs::copy` 保留 mode），二进制因此
/// 保持可执行位。
fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    std::fs::copy(src, dst)
        .with_context(|| format!("复制 {} → {} 失败", src.display(), dst.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实的 ldd 输出形状：vdso 行没有 `=>`；loader 行**有** `=>`
    /// （ldd 版本差异，两种都见过），要排除。
    const SAMPLE_LDD: &str = "\
        linux-vdso.so.1 (0x00007ffd12345000)
        libstdc++.so.6 => /usr/lib/libstdc++.so.6 (0x00007f1234560000)
        libc.so.6 => /usr/lib/libc.so.6 (0x00007f12345000)
        libzstd.so.1 => /usr/lib/libzstd.so.1 (0x00007f12344000)
        libz.so.1 => /usr/lib/libz.so.1 (0x00007f12343000)
        libgcc_s.so.1 => /usr/lib/libgcc_s.so.1 (0x00007f12342000)
        /lib64/ld-linux-x86-64.so.2 => /usr/lib64/ld-linux-x86-64.so.2 (0x00007f12346000)
    ";

    #[test]
    fn ldd_parsing_keeps_needed_libs_and_applies_exclusions() {
        let libs = libs_to_bundle(SAMPLE_LDD).unwrap();
        let names: Vec<&str> = libs.iter().filter_map(|p| p.to_str()).collect();
        // 要打包的
        assert!(
            names.contains(&"/usr/lib/libstdc++.so.6"),
            "libstdc++ 要打包（CI 的比用户机器新）：{names:?}"
        );
        assert!(names.contains(&"/usr/lib/libzstd.so.1"), "zstd 要打包：{names:?}");
        // 排除的
        assert!(!names.iter().any(|n| n.contains("libc.so")), "glibc 排除：{names:?}");
        assert!(!names.iter().any(|n| n.contains("libz.so")), "zlib 排除：{names:?}");
        assert!(
            !names.iter().any(|n| n.contains("libgcc_s")),
            "libgcc_s 排除：{names:?}"
        );
        // 没有 => 的行（vdso）与动态 loader 都不该出现
        assert!(
            !names.iter().any(|n| n.contains("vdso") || n.contains("ld-linux")),
            "vdso 与 loader 行应跳过/排除：{names:?}"
        );
    }

    #[test]
    fn ldd_not_found_is_an_error() {
        let err = libs_to_bundle("libfoo.so => not found\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn lib_stem_truncates_at_so() {
        assert_eq!(lib_stem("libstdc++.so.6"), "libstdc++");
        assert_eq!(lib_stem("libz.so.1"), "libz");
        assert_eq!(lib_stem("libzstd.so.1"), "libzstd");
        assert_eq!(lib_stem("libc.so.6"), "libc");
        assert_eq!(lib_stem("weird-name"), "weird-name");
    }

    /// 端到端：staging 布局 + tar.gz 内容。ldd 对非 ELF 假二进制会失败 →
    /// 跳过 lib/（静态链接路径），其余布局照常。
    #[test]
    fn targz_stages_app_layout() {
        let dir = std::env::temp_dir().join(format!(
            "aa-bundle-targz-test-{}-layout",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
category = "Developer Tool"
icon = ["resources/app-icon@2x.png", "resources/app-icon.png"]
"#,
        )
        .unwrap();
        std::fs::write(dir.join("resources/app-icon.png"), "small").unwrap();
        std::fs::write(dir.join("resources/app-icon@2x.png"), "x".repeat(2048)).unwrap();

        let fake_bin = dir.join("fake-aacode");
        std::fs::write(&fake_bin, "not a real elf").unwrap();

        let opts = TargzOptions {
            project_dir: &dir,
            bundle_meta: "Cargo.toml#[package, metadata, bundle-stable]",
            bin_name: None,
            icon_name: None,
            desktop_template: None,
            out: None,
            bin_path: Some(&fake_bin),
        };
        let out = generate(&opts).unwrap();

        // 默认输出名带 arch
        let file_name = out.file_name().unwrap().to_str().unwrap().to_string();
        assert!(
            file_name.starts_with("aacode-linux-") && file_name.ends_with(".tar.gz"),
            "默认输出名应为 <bin>-linux-<arch>.tar.gz：{file_name}"
        );

        // 列出 tar.gz 内容验证布局
        let listing = Command::new("tar")
            .args(["-tzf", out.to_str().unwrap()])
            .output()
            .unwrap();
        let entries = String::from_utf8_lossy(&listing.stdout);
        assert!(entries.contains("aacode.app/bin/aacode"), "二进制进 bin/：{entries}");
        assert!(
            entries.contains("aacode.app/share/applications/dev.aa.AaCode.desktop"),
            ".desktop 进 share/applications/：{entries}"
        );
        assert!(
            entries.contains("aacode.app/share/icons/hicolor/512x512/apps/aacode.png"),
            "512 图标：{entries}"
        );
        assert!(
            entries.contains("aacode.app/share/icons/hicolor/1024x1024/apps/aacode.png"),
            "1024 图标：{entries}"
        );
        assert!(!entries.contains("lib/"), "ldd 失败时不该有 lib/ 条目：{entries}");

        // 解出来看 .desktop 内容（Exec 裸 bin 名——可重定位）
        let extract = dir.join("extract");
        std::fs::create_dir_all(&extract).unwrap();
        Command::new("tar")
            .args(["-xzf", out.to_str().unwrap(), "-C", extract.to_str().unwrap()])
            .status()
            .unwrap();
        let desktop = std::fs::read_to_string(
            extract.join("aacode.app/share/applications/dev.aa.AaCode.desktop"),
        )
        .unwrap();
        assert!(desktop.contains("Name=AaCode"), "{desktop}");
        assert!(desktop.contains("Exec=aacode"), "tar.gz 的 Exec 用裸 bin 名：{desktop}");
        assert!(desktop.contains("Icon=aacode"), "{desktop}");
        assert!(desktop.contains("Categories=Development;"), "{desktop}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_binary_is_a_clear_error() {
        let dir = std::env::temp_dir().join(format!(
            "aa-bundle-targz-test-{}-nobin",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        let opts = TargzOptions {
            project_dir: &dir,
            bundle_meta: "Cargo.toml#[package, metadata, bundle-stable]",
            bin_name: None,
            icon_name: None,
            desktop_template: None,
            out: None,
            bin_path: Some(&dir.join("does-not-exist")),
        };
        let err = generate(&opts).unwrap_err().to_string();
        assert!(err.contains("cargo build --release"), "应提示先构建：{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
