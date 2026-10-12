//! Arch Linux 包（PKGBUILD）生成。
//!
//! 这是 `aa-bundle` 的第一个打包目标：把「构建 + 拷贝 + 注册 .desktop」这套
//! 手工流程（此前散在各项目的 justfile / script 里）收敛成生成一份 PKGBUILD，
//! 交给 `makepkg`/`pacman` 处理，从而：
//! - 卸载变成 `pacman -R`，不再需要手写 uninstall 脚本
//! - 身份（包名 / .desktop 名 / 图标）统一来自 `--bundle-meta` 指向的表
//!
//! **生成的 PKGBUILD 是「本地开发安装」用途**：它直接在源码树里构建（`build()`
//! 里 cd 回源码目录），不带 `source=`，因此**不能用于分发**（AUR 那种需要真正的
//! source + checksum）。分发版是后续的事。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{IdentityTable, load_identity_table};

/// 图标文件名后缀：stable 无后缀，其余 `-<channel>`，未知 channel 一律 `-dev`。
/// 规则同 Zed `crates/zed/build.rs` 的 `icon_path()`。
pub fn icon_suffix(channel: &str) -> &'static str {
    match channel {
        "stable" => "",
        "preview" => "-preview",
        "nightly" => "-nightly",
        _ => "-dev",
    }
}

/// crate 的 Cargo.toml 里我们关心的元数据。
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
}

/// `license` 可能是字面量，也可能是 `license.workspace = true`（workspace 继承）。
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrWorkspace {
    Literal(String),
    Inherited { workspace: bool },
}

#[derive(Deserialize)]
struct CrateManifest {
    package: Option<CratePackage>,
}

#[derive(Deserialize)]
struct CratePackage {
    name: String,
    version: String,
    description: Option<String>,
    license: Option<StringOrWorkspace>,
}

#[derive(Deserialize)]
struct WorkspaceManifest {
    workspace: Option<WorkspaceTable>,
}

#[derive(Deserialize)]
struct WorkspaceTable {
    package: Option<WorkspacePackage>,
}

#[derive(Deserialize)]
struct WorkspacePackage {
    license: Option<String>,
}

/// 读 crate 的 Cargo.toml；`license.workspace = true` 时上溯到 workspace 根取值。
pub fn read_package_info(project_dir: &Path) -> Result<PackageInfo> {
    let manifest_path = project_dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("读取 {} 失败", manifest_path.display()))?;
    let manifest: CrateManifest =
        toml::from_str(&text).with_context(|| format!("解析 {} 失败", manifest_path.display()))?;
    let package = manifest
        .package
        .context("Cargo.toml 没有 [package] 段（传的是 workspace 根？请用 -p 指向具体 crate 目录）")?;

    let license = match package.license {
        Some(StringOrWorkspace::Literal(s)) => s,
        Some(StringOrWorkspace::Inherited { .. }) => find_workspace_root(project_dir)
            .and_then(|root| {
                let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
                let ws: WorkspaceManifest = toml::from_str(&text).ok()?;
                ws.workspace?.package?.license
            })
            .context("license 是 workspace 继承，但没能从 workspace 根解析出值（请显式传 --license）")?,
        None => bail!("Cargo.toml 的 [package] 缺少 license，请显式传 --license"),
    };

    Ok(PackageInfo {
        name: package.name,
        version: package.version,
        description: package.description.unwrap_or_default(),
        license,
    })
}

/// 向上找含 `[workspace]` 的 Cargo.toml 所在目录。
fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        let candidate = dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            if toml::from_str::<WorkspaceManifest>(&text)
                .ok()
                .and_then(|m| m.workspace)
                .is_some()
            {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// PKGBUILD 生成参数。
pub struct ArchOptions<'a> {
    /// 源码目录（含 Cargo.toml）
    pub project_dir: &'a Path,
    /// 频道：决定 bundle 表（`bundle-<channel>`）与图标后缀
    pub channel: &'a str,
    /// 身份来源引用；默认 `Cargo.toml#[package, metadata, bundle-<channel>]`
    pub bundle_meta: &'a str,
    /// 二进制名（默认取 crate 名）
    pub bin_name: Option<&'a str>,
    /// 工作区场景下的 `-p <package>`（默认不加）
    pub cargo_package: Option<&'a str>,
    /// .desktop 模板路径（模板里的 `$APP_NAME` 等会被替换）；不传则不产出 desktop
    pub desktop_template: Option<&'a Path>,
    /// 图标基础名（不含后缀/扩展），默认取 crate 名
    pub icon_name: Option<&'a str>,
    pub depends: &'a [String],
    pub makedepends: &'a [String],
    pub arch: &'a [String],
    pub url: Option<&'a str>,
    /// 覆盖 license（Cargo.toml 读不到时用）
    pub license: Option<&'a str>,
    /// 产物安装前缀（默认 `/usr`）
    pub prefix: &'a str,
}

/// 生成 PKGBUILD 文本，并渲染 .desktop。
///
/// 返回 `(PKGBUILD 文本, Option<(desktop 文件名, desktop 内容)>)`。
pub fn generate_pkgbuild(opts: &ArchOptions) -> Result<(String, Option<(String, String)>)> {
    let info = read_package_info(opts.project_dir)?;
    let identity: IdentityTable = load_identity_table(opts.bundle_meta, opts.project_dir)?;

    let bin = opts.bin_name.unwrap_or(&info.name);
    let icon = opts.icon_name.unwrap_or(&info.name);
    let suffix = icon_suffix(opts.channel);
    let license = opts.license.unwrap_or(&info.license);
    let pkgdesc = if info.description.is_empty() {
        identity.name.clone().unwrap_or_else(|| info.name.clone())
    } else {
        info.description.clone()
    };
    // .desktop 名用 identifier（如 dev.aa.AaCode-Nightly.desktop），与桌面身份一致
    let desktop_id = identity.identifier.clone().with_context(|| {
        format!(
            "身份表 {} 里没有 identifier，无法命名 .desktop（请在该表补 identifier，或改用别的 --bundle-meta）",
            opts.bundle_meta
        )
    })?;

    // 源码目录绝对化：PKGBUILD 会在自己的目录里执行，必须按绝对路径回到源码树
    let src = opts
        .project_dir
        .canonicalize()
        .with_context(|| format!("无法解析源码目录 {}", opts.project_dir.display()))?;

    let cargo_pkg_flag = match opts.cargo_package {
        Some(p) => format!(" -p {p}"),
        None => String::new(),
    };

    // 渲染 .desktop
    let desktop_rel = match opts.desktop_template {
        Some(template_path) => {
            let template = std::fs::read_to_string(template_path).with_context(|| {
                format!("读取 .desktop 模板 {} 失败", template_path.display())
            })?;
            let rendered = render_desktop(
                &template,
                &[
                    ("APP_NAME", identity.name.as_deref().unwrap_or(&info.name)),
                    ("APP_ID", &desktop_id),
                    ("APP_CLI", &format!("{}/bin/{}", opts.prefix.trim_end_matches('/'), bin)),
                    ("APP_ICON", icon),
                    ("APP_ARGS", "%F"),
                    ("DO_STARTUP_NOTIFY", "true"),
                ],
            );
            Some((format!("{desktop_id}.desktop"), rendered))
        }
        None => None,
    };

    // 没给 url 就整行不输出，避免生成 `url=""` 这种空字段
    let url_line = match opts.url {
        Some(u) if !u.is_empty() => format!("url=\"{u}\"\n"),
        _ => String::new(),
    };
    let depends = opts.depends.join(" ");
    let makedepends = opts.makedepends.join(" ");
    let arch = opts.arch.join(" ");

    let desktop_install = match &desktop_rel {
        Some((name, _)) => format!(
            "  install -Dm644 \"$startdir/{name}\" \"$pkgdir{prefix}/share/applications/{name}\"\n",
            prefix = opts.prefix.trim_end_matches('/')
        ),
        None => String::new(),
    };

    let pkgbuild = format!(
        r#"# 由 aa-bundle 生成（本地开发安装用途，非分发版）—— 手写修改会在下次生成时被覆盖。
# 身份来源：{bundle_meta}
pkgname={name}
pkgver={version}
pkgrel=1
pkgdesc="{pkgdesc}"
arch=({arch})
{url_line}license=({license})
depends=({depends})
makedepends=({makedepends})
# aacode 二进制体积大（strip 后仍 300MB+），开 LTO 链接会非常慢；与项目构建配置一致关掉。
options=(!lto)

# 源码树路径。本 PKGBUILD 不带 source=，直接在源码目录里构建以复用 target/ 的
# 增量缓存 —— 代价是不能用于 AUR/分发，只适合本机安装。
_srcdir="{src}"

build() {{
  cd "$_srcdir"
  cargo build --release{cargo_pkg_flag}
}}

package() {{
  install -Dm755 "$_srcdir/target/release/{bin}" "$pkgdir{prefix}/bin/{bin}"

  # 图标：512 + 1024 两个尺寸（Arch 默认 options 含 strip，故这里不再手工 strip）
  install -Dm644 "$_srcdir/resources/app-icon{suffix}.png" \
    "$pkgdir{prefix}/share/icons/hicolor/512x512/apps/{icon}.png"
  install -Dm644 "$_srcdir/resources/app-icon{suffix}@2x.png" \
    "$pkgdir{prefix}/share/icons/hicolor/1024x1024/apps/{icon}.png"
{desktop_install}}}
"#,
        name = info.name,
        version = info.version,
        pkgdesc = pkgdesc,
        arch = arch,

        license = license,
        depends = depends,
        makedepends = makedepends,
        src = src.display(),
        bin = bin,
        suffix = suffix,
        icon = icon,
        prefix = opts.prefix.trim_end_matches('/'),
        bundle_meta = opts.bundle_meta,
        cargo_pkg_flag = cargo_pkg_flag,
        desktop_install = desktop_install,
    );

    Ok((pkgbuild, desktop_rel))
}

/// 类 envsubst 的 `$VAR` 替换（只处理传入的变量名，其他 `$xxx` 原样保留）。
pub fn render_desktop(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${key}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 建一个本测试独有的临时目录（pid + tag 防相撞）。
    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aa-bundle-arch-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        dir
    }

    #[test]
    fn icon_suffix_follows_channel() {
        assert_eq!(icon_suffix("stable"), "");
        assert_eq!(icon_suffix("preview"), "-preview");
        assert_eq!(icon_suffix("nightly"), "-nightly");
        assert_eq!(icon_suffix("dev"), "-dev");
        assert_eq!(icon_suffix("whatever"), "-dev");
    }

    #[test]
    fn pkgbuild_uses_identity_from_bundle_meta() {
        let dir = test_dir("pkgbuild");
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

        let opts = ArchOptions {
            project_dir: &dir,
            channel: "stable",
            bundle_meta: "Cargo.toml#[package, metadata, bundle-stable]",
            bin_name: None,
            cargo_package: Some("aacode"),
            desktop_template: None,
            icon_name: None,
            depends: &["gcc-libs".to_string(), "glibc".to_string()],
            makedepends: &["cargo".to_string()],
            arch: &["x86_64".to_string()],
            url: None,
            license: None,
            prefix: "/usr",
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();

        // 身份来自 bundle 表：pkgname 取 crate 名、desktop 名取 identifier
        assert!(pkgbuild.contains("pkgname=aacode"));
        assert!(pkgbuild.contains("pkgver=1.23.2"));
        assert!(pkgbuild.contains("license=(Apache-2.0)"));
        assert!(pkgbuild.contains("depends=(gcc-libs glibc)"));
        assert!(pkgbuild.contains("cargo build --release -p aacode"));
        assert!(
            pkgbuild.contains("hicolor/512x512/apps/aacode.png"),
            "图标应装到 hicolor 512x512：{pkgbuild}"
        );
        assert!(!pkgbuild.contains("url="), "未给 url 时不该输出空字段");
        // 没传模板 → 不产出 desktop
        assert!(desktop.is_none());
        // 源码路径绝对化（canonicalize 后应含该临时目录）
        let src = dir.canonicalize().unwrap();
        assert!(
            pkgbuild.contains(&format!("_srcdir=\"{}\"", src.display())),
            "_srcdir 应为绝对路径：{pkgbuild}"
        );
    }

    #[test]
    fn desktop_rendering_substitutes_only_known_vars() {
        let out = render_desktop(
            "Name=$APP_NAME\nExec=$APP_CLI %F\nIcon=$APP_ICON\nX=$UNKNOWN",
            &[("APP_NAME", "AaCode"), ("APP_CLI", "/usr/bin/aacode"), ("APP_ICON", "aacode")],
        );
        assert!(out.contains("Name=AaCode"));
        assert!(out.contains("Exec=/usr/bin/aacode %F"));
        assert!(out.contains("Icon=aacode"));
        assert!(out.contains("X=$UNKNOWN"), "未提供的变量应原样保留");
    }
}
