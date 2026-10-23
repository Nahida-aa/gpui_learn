//! Arch Linux 包（PKGBUILD）生成。
//!
//! 这是 `aa-bundle` 的第一个打包目标：把「构建 + 拷贝 + 注册 .desktop」这套
//! 手工流程（此前散在各项目的 justfile / script 里）收敛成生成一份 PKGBUILD，
//! 交给 `makepkg`/`pacman` 处理，从而：
//! - 卸载变成 `pacman -R`，不再需要手写 uninstall 脚本
//! - 身份（包名 / .desktop 名 / 图标）统一来自 `--bundle-meta` 指向的表
//!
//! ## 命名：为什么都叫 `pkgbuild`
//!
//! CLI 子命令 `aa-bundle pkgbuild` 读 Cargo.toml 的
//! `[package.metadata.bundle-pkgbuild]` —— 同名，省一层映射。
//! 早期两者都叫 `arch`，与 `--arch`（CPU 架构：x86_64）撞词：同一个词既指发行版
//! 又指架构。改成 `pkgbuild` 后两边都无歧义。
//!
//! 注意这个段是 **aa-bundle 自己的输入**，不是 Arch 的元数据 ——
//! Arch/pacman 只认 PKGBUILD 与 makepkg 生成的 `.PKGINFO`，从不读 Cargo.toml。
//! 它只是把「推不出来、又不该每次敲命令行」的设置集中声明。
//!
//! **生成的 PKGBUILD 是「本地开发安装」用途**：它直接在源码树里构建（`build()`
//! 里 cd 回源码目录），不带 `source=`，因此**不能用于分发**（AUR 那种需要真正的
//! source + checksum）。分发版是后续的事。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::meta::BundleMeta;

/// crate 的 Cargo.toml 里我们关心的元数据。
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    /// `[[bin]]` 目标名。**只有恰好一个 bin 目标时才有值** —— 多个就必须由项目声明，
    /// 因为 CLI 无从判断该用哪个。
    pub bin_name: Option<String>,
    /// `[[bin]]` 目标数量（0 = 纯 lib crate）
    pub bin_count: usize,
}

/// `license` 可能是字面量，也可能是 `license.workspace = true`（workspace 继承）。
/// `workspace` 字段只是用来让 serde 认出这个形状，值本身不读。
#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrWorkspace {
    Literal(String),
    Inherited { workspace: bool },
}

#[derive(Deserialize)]
struct CrateManifest {
    package: Option<CratePackage>,
    bin: Option<Vec<CrateBin>>,
}

#[derive(Deserialize)]
struct CrateBin {
    name: String,
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
    let package = manifest.package.context(
        "Cargo.toml 没有 [package] 段（传的是 workspace 根？请用 -p 指向具体 crate 目录）",
    )?;

    let license = match package.license {
        Some(StringOrWorkspace::Literal(s)) => s,
        Some(StringOrWorkspace::Inherited { .. }) => find_workspace_root(project_dir)
            .and_then(|root| {
                let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
                let ws: WorkspaceManifest = toml::from_str(&text).ok()?;
                ws.workspace?.package?.license
            })
            .context("license 是 workspace 继承，但没能从 workspace 根解析出值（请在 workspace 根的 [workspace.package] 补 license）")?,
        None => bail!("Cargo.toml 的 [package] 缺少 license（请在 [package] 补 license = \"...\"，或让 workspace 根继承）"),
    };

    let bins: Vec<String> = manifest
        .bin
        .unwrap_or_default()
        .into_iter()
        .map(|b| b.name)
        .collect();

    Ok(PackageInfo {
        name: package.name,
        version: package.version,
        description: package.description.unwrap_or_default(),
        license,
        bin_name: (bins.len() == 1).then(|| bins[0].clone()),
        bin_count: bins.len(),
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

/// Arch 打包参数的最终取值：`[bundle.arch]` 子表 > 命令行覆盖 > 默认值。
///
/// 与 cargo-bundle 的 deb_depends 一致：系统包依赖**不猜、不填默认值** ——
/// 只有项目自己知道（cargo 只认 -l，不认发行版包名），没声明就整行不写
/// depends=（见 cargo-bundle deb_bundle.rs:128-131）。
/// makedepends 默认 (cargo rust) 不是猜：我们的 PKGBUILD 自己调 cargo build。
#[derive(Debug, Clone)]
pub struct ArchSettings {
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub arch: Vec<String>,
    pub url: String,
    pub prefix: String,
    /// 图标安装目录（相对 prefix），如 share/icons/hicolor/512x512/apps
    pub icon_dir: String,
}

impl Default for ArchSettings {
    fn default() -> Self {
        Self {
            depends: Vec::new(),
            makedepends: ["cargo", "rust"].iter().map(|s| s.to_string()).collect(),
            arch: vec!["x86_64".to_string()],
            url: String::new(),
            prefix: "/usr".to_string(),
            // 只到 hicolor 为止——尺寸目录由 icon 文件名（@2x）推导后拼上
            icon_dir: "share/icons/hicolor".to_string(),
        }
    }
}

/// 从 `[bundle.arch]` 子表取 Arch 设置（子表不存在 → 默认值）。
///
/// `prebuilt` 为真且子表未显式声明 `makedepends` 时，把默认的 `(cargo rust)`
/// 清空——二进制打包的 PKGBUILD 既不在包内 `cargo build`，也不调 cargo metadata
/// （路径生成期已钉死），构建依赖没有意义。显式声明仍然尊重（优先级见调用方）。
fn arch_settings_from(arch: &Option<crate::meta::ArchTable>, prebuilt: bool) -> ArchSettings {
    let mut s = ArchSettings::default();
    if let Some(t) = arch {
        if let Some(v) = &t.depends {
            s.depends = v.clone();
        }
        if let Some(v) = &t.makedepends {
            s.makedepends = v.clone();
        }
        if let Some(v) = &t.arch {
            s.arch = v.clone();
        }
        if let Some(v) = &t.url {
            s.url = v.clone();
        }
        if let Some(v) = &t.prefix {
            s.prefix = v.clone();
        }
        if let Some(v) = &t.icon_dir {
            s.icon_dir = v.clone();
        }
    }
    if prebuilt && arch.as_ref().and_then(|t| t.makedepends.as_ref()).is_none() {
        s.makedepends = Vec::new();
    }
    s
}

/// PKGBUILD 生成参数。
///
/// 字段分两类：`project_dir` / `bundle_meta` / `bin_name` / `icon_name` /
/// `desktop_template` 是所有格式通用的（CLI 有对应 flag）；其余
/// （`depends` 及以下）是 Arch 特有设置的**库 API 覆盖项**——CLI 不暴露
/// 它们，格式特有设置一律声明在 bundle 元数据的 `[bundle.arch]` 子表里，
/// 免得 `--format tar.gz` 时面对一堆无效 flag。
pub struct PkgbuildOptions<'a> {
    /// 源码目录（含 Cargo.toml）
    pub project_dir: &'a Path,
    /// 身份/打包配置来源引用列表（**必填**，无默认值）：多个按顺序分层，
    /// 后者递归覆盖前者（见 [`crate::meta::load_bundle_meta_layered`]）。
    /// 典型用法：先给共享表，再给 channel 专属表。
    pub bundle_meta: Vec<String>,
    /// 覆盖二进制名
    pub bin_name: Option<&'a str>,
    /// 覆盖 .desktop 模板路径
    pub desktop_template: Option<&'a Path>,
    /// 覆盖图标基础名（不含尺寸目录与扩展）
    pub icon_name: Option<&'a str>,
    /// 覆盖运行依赖（完全替代 `[bundle.arch].depends`）
    pub depends: Option<&'a [String]>,
    /// 覆盖构建依赖
    pub makedepends: Option<&'a [String]>,
    /// 覆盖目标架构
    pub arch: Option<&'a [String]>,
    /// 覆盖 url
    pub url: Option<&'a str>,
    /// 覆盖 license
    pub license: Option<&'a str>,
    /// 覆盖安装前缀
    pub prefix: Option<&'a str>,
    /// 要打包的 cargo 构建 profile（默认 `release`）。用于安装**已编好**的产物：
    /// 如 `release-quick` → PKGBUILD 里 `cargo build --profile release-quick`，
    /// 且取 `target/release-quick/<bin>`。`dev` 映射到 cargo 的 `debug` 目录。
    pub profile: Option<&'a str>,
    /// 二进制打包模式（AUR `-bin` 形态）：`build()` 不在包内编译，只校验生成期
    /// 钉死的二进制路径；`package()` 直接 install 该路径。makedepends 默认清空
    /// （包内不调 cargo）。默认 `false` = 源码打包（`build()` 里 `cargo build`）。
    pub prebuilt: bool,
    /// 直接指定二进制路径（仅 prebuilt 模式有意义）：生成时 canonicalize 成绝对
    /// 路径、校验存在后钉死进 PKGBUILD。默认取 `target/<profile>/<bin>`。
    pub bin_path: Option<&'a Path>,
}

/// bin 名的推导链（对齐 cargo 的自动发现规则）：
/// 1. 覆盖值（最高优先）
/// 2. 有 `[[bin]]` 且唯一 → 该目标名（可能与 package.name 不同）
/// 3. 无 `[[bin]]` → **包名**。这是 cargo 的规则：`src/main.rs` 自动成为以
///    包名命名的二进制。不是猜，是 cargo 文档化的行为。
/// 4. 多个 `[[bin]]` → 歧义，报错要求声明
pub fn resolve_bin_name<'a>(
    info: &'a PackageInfo,
    override_name: Option<&'a str>,
    project_dir: &Path,
) -> Result<&'a str> {
    if let Some(b) = override_name {
        return Ok(b);
    }
    match info.bin_count {
        0 => Ok(info.name.as_str()),
        1 => Ok(info
            .bin_name
            .as_deref()
            .expect("bin_count==1 时必有 bin_name")),
        n => bail!(
            "{} 有 {n} 个 [[bin]] 目标，无法判断该装哪个。\
             请用 --bin-name 指定，或在 bundle 元数据里加 `[bundle.arch]` 后\
             用 --bundle-meta 指向含 bin 配置的表。",
            project_dir.display()
        ),
    }
}

/// pkgbuild 格式的默认输出目录：`<workspace target>/aa-bundle/pkgbuild/<bin>/`。
///
/// 不放在 crate 目录的 `target/` 下——workspace 成员的 target 由 workspace
/// 根决定（见 [`crate::cargo::resolve_target_dir`]），crate 目录下的 target/ cargo
/// 根本不用。末尾按 bin 名分目录：同一 workspace 打多个 crate 不会互相覆盖。
pub fn default_out_dir(project_dir: &Path, bin_name: Option<&str>) -> Result<PathBuf> {
    let info = read_package_info(project_dir)?;
    let bin = resolve_bin_name(&info, bin_name, project_dir)?;
    Ok(crate::cargo::resolve_target_dir(project_dir).join(format!("aa-bundle/pkgbuild/{bin}")))
}

/// 生成 PKGBUILD 文本，并渲染 .desktop。
///
/// 返回 `(PKGBUILD 文本, Option<(desktop 文件名, desktop 内容)>)`。
pub fn generate_pkgbuild(opts: &PkgbuildOptions) -> Result<(String, Option<(String, String)>)> {
    let info = read_package_info(opts.project_dir)?;
    let identity: BundleMeta =
        crate::meta::load_bundle_meta_layered(&opts.bundle_meta, opts.project_dir)?;
    // 错误消息 / PKGBUILD 注释里展示的「配置来源」：多引用按顺序用 + 连起来
    let meta_ref = opts.bundle_meta.join(" + ");
    let mut settings = arch_settings_from(&identity.arch, opts.prebuilt);
    // 命令行覆盖优先于 Cargo.toml 的 [bundle.arch]
    if let Some(v) = opts.depends {
        settings.depends = v.to_vec();
    }
    if let Some(v) = opts.makedepends {
        settings.makedepends = v.to_vec();
    }
    if let Some(v) = opts.arch {
        settings.arch = v.to_vec();
    }
    if let Some(v) = opts.url {
        settings.url = v.to_string();
    }
    if let Some(v) = opts.prefix {
        settings.prefix = v.to_string();
    }

    // —— bin 名（对齐 cargo 的自动发现规则，见 resolve_bin_name）——
    let bin: &str = resolve_bin_name(&info, opts.bin_name, opts.project_dir)?;

    // 图标安装名：覆盖 > bin 名。无数据源可推导，回退时不静默（见 resolve_icon_name）。
    let icon = crate::desktop::resolve_icon_name(opts.icon_name, bin);
    let license = opts.license.unwrap_or(&info.license);
    let prefix = settings.prefix.trim_end_matches('/');
    // pkgver：bundle 元数据的 version > Cargo.toml 的 version
    let version = identity
        .version
        .clone()
        .unwrap_or_else(|| info.version.clone());
    // pkgdesc 回退链：short_description > Cargo.toml description > name > crate 名
    // （与 cargo-bundle 的 short_description() 同思路）
    let pkgdesc = identity
        .short_description
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if !info.description.is_empty() {
                info.description.clone()
            } else {
                identity.name.clone().unwrap_or_else(|| info.name.clone())
            }
        });
    // 源码目录绝对化：PKGBUILD 会在自己的目录里执行，必须按绝对路径回到源码树
    let src = opts
        .project_dir
        .canonicalize()
        .with_context(|| format!("无法解析源码目录 {}", opts.project_dir.display()))?;

    // cargo 包名直接从 crate 的 Cargo.toml 取 —— 已经指了目录，不需要再让人传一遍
    let cargo_pkg_flag = format!(" -p {}", info.name);

    // 要打包的 profile：默认 release。`dev` 在 cargo 里落 target/debug，其余同名。
    // release 用简写 `--release`（保持与旧输出一致），其余走 `--profile <name>`。
    // （profile_dir / profile_flag 与 tar.gz 共用，见 lib.rs。）
    let profile = opts.profile.unwrap_or("release");
    let profile_dir = crate::cargo::profile_dir(profile);
    let profile_flag = crate::cargo::profile_flag(profile);

    // prebuilt 模式：生成期就把二进制定位并钉死成绝对路径。canonicalize 一体
    // 解决「相对→绝对」与存在性；`--bin-path` 覆盖优先于 target/<profile>/<bin>。
    // 在进程内校验（而不是留给 makepkg）是为了早失败：缺产物时立刻告诉用户
    // 先 cargo build，而不是等 makepkg 跑到 build() 才报错。
    let pinned_bin = if opts.prebuilt {
        let path = match opts.bin_path {
            Some(p) => p.canonicalize().with_context(|| {
                format!(
                    "--bin-path 指向的二进制不存在或无法解析：{}",
                    p.display()
                )
            })?,
            None => {
                let path = crate::cargo::resolve_binary_path(opts.project_dir, bin, profile);
                anyhow::ensure!(
                    path.exists(),
                    "找不到二进制 {}：先 `cargo build {} -p {}`（prebuilt 模式不构建，只安装已编好的产物）",
                    path.display(),
                    profile_flag,
                    info.name
                );
                path.canonicalize().unwrap_or(path)
            }
        };
        Some(path)
    } else {
        None
    };

    // .desktop（文件名 + 内容）：模板 > aa-bundle 自动生成，见 resolve_desktop_entry。
    // `$APP_CLI` 用绝对安装路径——PKGBUILD 装完二进制就在 <prefix>/bin/<bin>。
    // Comment 回退值取 Cargo.toml 的 [package].description。
    let desktop_rel = crate::desktop::resolve_desktop_entry(
        &identity,
        bin,
        &icon,
        &format!("{prefix}/bin/{bin}"),
        opts.project_dir,
        &meta_ref,
        opts.desktop_template,
        &info.description,
    )?;

    // 没给 url 就整行不输出，避免生成 `url=""` 这种空字段
    let url_line = match opts.url {
        Some(u) if !u.is_empty() => format!("url=\"{u}\"\n"),
        _ => String::new(),
    };
    let depends = settings.depends.join(" ");
    let makedepends = settings.makedepends.join(" ");
    let arch = settings.arch.join(" ");
    // 与 cargo-bundle 一致：空则不写该行（而不是写个空的 depends=()）
    let depends_line = if settings.depends.is_empty() {
        String::new()
    } else {
        format!("depends=({depends})\n")
    };
    let makedepends_line = if settings.makedepends.is_empty() {
        String::new()
    } else {
        format!("makedepends=({makedepends})\n")
    };

    let desktop_install = {
        let (name, _) = &desktop_rel;
        format!(
            "  install -Dm644 \"$startdir/{name}\" \"$pkgdir{prefix}/share/applications/{name}\"\n"
        )
    };

    // 图标安装行：路径来自 `[bundle].icon`（cargo-bundle 的字段），安装目录来自
    // `[bundle.arch].icon_dir`。**不再硬编码 resources/app-icon.png** —— 那正是
    // 「CLI 假设项目怎么命名」的老问题。没配 icon 就不装图标。
    let mut icon_install = String::new();
    let icon_dir_full = format!("{prefix}/{}", settings.icon_dir.trim_end_matches('/'));
    for icon_path in identity.icon.iter().flatten() {
        let abs = opts.project_dir.join(icon_path);
        anyhow::ensure!(
            abs.exists(),
            "bundle 元数据 {} 的 icon 指向的文件不存在：{}",
            meta_ref,
            abs.display()
        );
        // 尺寸目录按文件名约定判断：`@2x` 结尾 → 1024x1024，否则 512x512。
        // （cargo-bundle 的 icon 列表也是这个约定；不按文件大小猜——那不可靠。）
        let size_dir = crate::desktop::icon_size_dir(icon_path);
        icon_install.push_str(&format!(
            "  install -Dm644 \"$_srcdir/{icon_path}\" \"$pkgdir{icon_dir_full}/{size_dir}/apps/{icon}.png\"\n"
        ));
    }
    if icon_install.is_empty() {
        println!("提示：bundle 元数据里没有 icon，将不安装应用图标。");
    }

    // 模板按打包模式分叉的三段：头部注释+_srcdir、build() 主体、package() 的
    // 二进制安装段。其余（包元信息、图标、.desktop）两模式共用。
    // 分叉理由：prebuilt 的二进制路径在**生成期**钉死（pinned_bin），makepkg
    // 侧零子进程、不读 makepkg 注入的环境；源码模式维持「现场 cargo build +
    // 问 cargo metadata」的现状。
    let (src_note, build_body, bin_install) = match &pinned_bin {
        Some(pinned) => (
            format!(
                r#"# 二进制打包（prebuilt）：二进制不在包内编译。aa-bundle 生成时已把
# {pinned} 钉死为安装源；重新构建后必须重新运行 aa-bundle --prebuilt
# 生成本文件，否则装的是旧产物。图标 / .desktop 源文件仍在源码树里
# （bundle 元数据的 icon 字段），按 $_srcdir 引用。
_srcdir="{src}"
"#,
                pinned = pinned.display(),
                src = src.display(),
            ),
            format!(
                r#"  # prebuilt 模式：不编译。二进制由包外构建提供
  # （cargo build {profile_flag} -p {cargo_pkg}），此处只校验生成期
  # 钉死的路径仍在——cargo clean / 移动 target 后会失效，重新构建后
  # 需重新运行 aa-bundle --prebuilt。
  # 环境仍按源码模式卸掉：本函数虽不编译，保持环境一致以防将来加步骤。
  unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS DEBUG_RUSTFLAGS
  if [ ! -x "{pinned}" ]; then
    echo "aa-bundle(prebuilt): 找不到可执行文件 {pinned}" >&2
    echo "  请先在包外构建：cargo build {profile_flag} -p {cargo_pkg}" >&2
    return 1
  fi
"#,
                pinned = pinned.display(),
                profile_flag = profile_flag,
                cargo_pkg = info.name,
            ),
            format!(
                r#"  # prebuilt：直接安装生成期钉死的二进制。路径生成时已定，makepkg
  # 运行环境的 CARGO_TARGET_DIR 一概不影响。
  install -Dm755 "{pinned}" "$pkgdir{prefix}/bin/{bin}"
"#,
                pinned = pinned.display(),
                prefix = prefix,
                bin = bin,
            ),
        ),
        None => (
            format!(
                r#"# 源码树路径。本 PKGBUILD 不带 source=，直接在源码目录里构建以复用 target/ 的
# 增量缓存 —— 代价是不能用于 AUR/分发，只适合本机安装。图标源文件也在源码树里
# （bundle 元数据的 icon 字段），同样按 $_srcdir 引用。
_srcdir="{src}"
"#,
                src = src.display(),
            ),
            format!(
                r#"  cd "$_srcdir"
  # 卸掉 makepkg 从 /etc/makepkg.conf 注入的编译器环境变量。rustc 不读它们，
  # 但 cc-rs 会读，且**追加在命令行末尾**——覆盖 crate 为个别文件设的 -O0。
  # 实例：aws-lc-sys 的 jitterentropy-base.c 禁用优化（源码 #error 强制），
  # 被 CFLAGS 的 -O2 覆盖后必挂。用户 shell 里这些变量本来就不存在，卸掉
  # 正好让包内构建与直接 `cargo build` 一致。
  # DEBUG_RUSTFLAGS 同族：OPTIONS=(debug) 时 makepkg 的 buildenv/rust.sh
  # 会往 RUSTFLAGS 追加 `-C debuginfo=2`（不卸就把调试符号编进产物，
  # rustc 内存与二进制体积双双暴涨）。
  unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS DEBUG_RUSTFLAGS
  cargo build {profile_flag}{cargo_pkg_flag}
"#,
                profile_flag = profile_flag,
                cargo_pkg_flag = cargo_pkg_flag,
            ),
            // 二进制路径不能硬编码「crate 目录下的 target/release」——cargo 的
            // target_directory 由 **workspace 根**决定（多 crate workspace 下产物不在
            // crate 目录），且 CARGO_TARGET_DIR 可改。直接问 cargo；--no-deps 只读
            // manifest，不解析依赖、不联网。也顺带尊重 makepkg 运行环境的
            // CARGO_TARGET_DIR。
            format!(
                r#"  _target_dir=$(cd "$_srcdir" && cargo metadata --no-deps --format-version 1 \
    | sed -n 's/.*"target_directory": *"\([^"]*\)".*/\1/p')
  if [ -z "$_target_dir" ]; then
    echo "aa-bundle: 无法从 cargo metadata 解析 target_directory" >&2
    return 1
  fi
  install -Dm755 "$_target_dir/{profile_dir}/{bin}" "$pkgdir{prefix}/bin/{bin}"
"#,
                profile_dir = profile_dir,
                bin = bin,
                prefix = prefix,
            ),
        ),
    };

    let pkgbuild = format!(
        r#"# 由 aa-bundle 生成（本地开发安装用途，非分发版）—— 手写修改会在下次生成时被覆盖。
# 配置来源：{bundle_meta}
pkgname={name}
pkgver={version}
pkgrel=1
pkgdesc="{pkgdesc}"
arch=({arch})
{url_line}license=({license})
{depends_line}{makedepends_line}
# 二进制体积大（strip 后仍 300MB+），开 LTO 链接会非常慢；与项目构建配置一致关掉。
options=(!lto)
{src_note}
build() {{
{build_body}}}

package() {{
{bin_install}
  # 图标路径来自 bundle 元数据的 icon 字段，安装目录来自 [bundle.arch].icon_dir
{icon_install}{desktop_install}}}
"#,
        name = info.name,
        version = version,
        pkgdesc = pkgdesc,
        arch = arch,
        license = license,
        depends_line = depends_line,
        makedepends_line = makedepends_line,
        src_note = src_note,
        build_body = build_body,
        bin_install = bin_install,
        icon_install = icon_install,
        bundle_meta = meta_ref,
        desktop_install = desktop_install,
    );

    Ok((pkgbuild, Some(desktop_rel)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 建一个本测试独有的临时目录（pid + tag 防相撞）。
    fn test_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("aa-bundle-pkgbuild-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        dir
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

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();

        // 身份来自 bundle 表：pkgname 取 crate 名、desktop 名取 identifier
        assert!(pkgbuild.contains("pkgname=aacode"));
        assert!(pkgbuild.contains("pkgver=1.23.2"));
        assert!(pkgbuild.contains("license=(Apache-2.0)"));
        // cargo 包名从 Cargo.toml 推导，不需要再传
        assert!(pkgbuild.contains("cargo build --release -p aacode"));
        // 二进制路径必须向 cargo 问 target_directory（workspace 根的 target/），
        // 不能硬编码 crate 目录——多 crate workspace 下那样必挂
        assert!(
            pkgbuild.contains("cargo metadata --no-deps --format-version 1"),
            "package() 应向 cargo 询问 target_directory：{pkgbuild}"
        );
        assert!(
            !pkgbuild.contains("$_srcdir/target/release/"),
            "不该硬编码产物在 crate 目录的 target/ 下：{pkgbuild}"
        );
        // build() 必须卸掉 makepkg 注入的编译器环境变量：cc-rs 把它们追加在
        // C 编译命令行末尾，会覆盖 aws-lc-sys 等 crate 为个别文件设的 -O0
        // （jitterentropy-base.c 禁优化，被 CFLAGS 的 -O2 覆盖后 #error 挂死）
        assert!(
            pkgbuild.contains("unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS DEBUG_RUSTFLAGS"),
            "build() 应卸掉 makepkg 的编译器环境变量（含 DEBUG_RUSTFLAGS——\
             OPTIONS=(debug) 时 rust.sh 会往 RUSTFLAGS 追加 -C debuginfo=2）：{pkgbuild}"
        );
        assert!(
            !pkgbuild.contains("install -Dm644") || !pkgbuild.contains("hicolor"),
            "没配 icon 时不该装图标：{pkgbuild}"
        );
        assert!(!pkgbuild.contains("url="), "未给 url 时不该输出空字段");
        // 未声明 depends → 整行不写（对齐 cargo-bundle 的 deb 行为）
        // 注意按行判断：makedepends= 里也含子串 "depends="
        assert!(
            !pkgbuild.lines().any(|l| l.starts_with("depends=")),
            "没声明依赖时不该写 depends= 行：{pkgbuild}"
        );
        assert!(
            pkgbuild.lines().any(|l| l == "makedepends=(cargo rust)"),
            "构建 Rust crate 必然需要 cargo/rust，这不算猜：{pkgbuild}"
        );
        // 没配 desktop_template → 由 aa-bundle 自动生成（不再是不产出）
        let (dname, dcontent) = desktop.expect("无模板时应自动生成 .desktop");
        assert_eq!(dname, "dev.aa.AaCode.desktop");
        assert!(dcontent.contains("Name=AaCode"), "{dcontent}");
        // 源码路径绝对化
        let src = dir.canonicalize().unwrap();
        assert!(
            pkgbuild.contains(&format!("_srcdir=\"{}\"", src.display())),
            "_srcdir 应为绝对路径：{pkgbuild}"
        );
    }

    /// 测试共用的 Options 构造：默认源码模式。prebuilt / bin_path 见
    /// [`opts_with`]。
    fn opts<'a>(dir: &'a Path, profile: Option<&'a str>) -> PkgbuildOptions<'a> {
        opts_with(dir, profile, false, None)
    }

    fn opts_with<'a>(
        dir: &'a Path,
        profile: Option<&'a str>,
        prebuilt: bool,
        bin_path: Option<&'a Path>,
    ) -> PkgbuildOptions<'a> {
        PkgbuildOptions {
            project_dir: dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile,
            prebuilt,
            bin_path,
        }
    }

    /// `--profile` 决定 PKGBUILD 的构建命令与产物目录：非 release 走
    /// `--profile <name>` + `target/<name>/<bin>`；`dev` 映射到 cargo 的 `debug`。
    #[test]
    fn pkgbuild_profile_flag_controls_build_and_artifact_dir() {
        let dir = test_dir("profile");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        let (pkgbuild, _) = generate_pkgbuild(&opts(&dir, Some("release-quick"))).unwrap();
        assert!(
            pkgbuild.contains("cargo build --profile release-quick -p aacode"),
            "{pkgbuild}"
        );
        assert!(
            pkgbuild.contains("$_target_dir/release-quick/aacode"),
            "{pkgbuild}"
        );

        let (pkgbuild, _) = generate_pkgbuild(&opts(&dir, Some("dev"))).unwrap();
        assert!(
            pkgbuild.contains("cargo build --profile dev -p aacode"),
            "{pkgbuild}"
        );
        assert!(pkgbuild.contains("$_target_dir/debug/aacode"), "{pkgbuild}");
    }

    #[test]
    fn pkgbuild_settings_come_from_bundle_arch_subtable() {
        let dir = test_dir("arch_subtable");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
short_description = "A coding agent built with Rust and GPUI"
desktop_template = "resources/aacode.desktop.in"

# Arch 打包参数作为 bundle 表的子表（方案 B：与 cargo-bundle 的 linux/osx 子表同构）
[package.metadata.bundle-stable.arch]
depends = ["gcc-libs", "glibc", "gtk3"]
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::write(
            dir.join("resources/aacode.desktop.in"),
            "Name=$APP_NAME\nExec=$APP_CLI %F\n",
        )
        .unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();

        assert!(
            pkgbuild.contains("depends=(gcc-libs glibc gtk3)"),
            "依赖应来自 [bundle.arch] 子表而非命令行：{pkgbuild}"
        );
        // 默认 makedepends（子表未声明）
        assert!(pkgbuild.contains("makedepends=(cargo rust)"));
        // short_description 优先于 Cargo.toml 的 description
        assert!(
            pkgbuild.contains("pkgdesc=\"A coding agent built with Rust and GPUI\""),
            "pkgdesc 应取 short_description：{pkgbuild}"
        );
        // desktop_template 生效：产出 desktop 且渲染过
        let (name, content) = desktop.expect("配了 desktop_template 就应产出 desktop");
        assert_eq!(name, "dev.aa.AaCode.desktop");
        assert!(content.contains("Name=AaCode"));
        assert!(content.contains("Exec=/usr/bin/aacode %F"));
    }

    #[test]
    fn bin_name_comes_from_bin_target_not_crate_name() {
        // [[bin]].name 与 package.name 不同：要用真实的 bin 目标，不能拿 crate 名顶替
        let dir = test_dir("bin_differs");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "some_crate"
version = "0.1.0"
license = "Apache-2.0"

[[bin]]
name = "the_real_binary"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let (pkgbuild, _) = generate_pkgbuild(&opts).unwrap();
        assert!(
            pkgbuild.contains("$pkgdir/usr/bin/the_real_binary"),
            "应装真实的 [[bin]] 名，而不是 crate 名：{pkgbuild}"
        );
        // 但 cargo -p 用的是 package.name
        assert!(pkgbuild.contains("cargo build --release -p some_crate"));
    }

    #[test]
    fn default_out_dir_lives_in_workspace_target() {
        let dir = test_dir("default_out");
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
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();

        // 独立 crate（无 workspace）：cargo metadata 问出的 target 就是 <dir>/target，
        // 默认输出在其下的 aa-bundle/pkgbuild/<bin>/——不能猜在 crate 目录乱放，
        // 也不能不给 bin 名分目录（同一 workspace 打多个 crate 会互相覆盖）。
        let out = default_out_dir(&dir, None).unwrap();
        assert_eq!(
            out,
            dir.join("target").join("aa-bundle/pkgbuild/aacode"),
            "默认输出应为 <workspace target>/aa-bundle/pkgbuild/<bin>：{}",
            out.display()
        );
    }

    #[test]
    fn multiple_bins_must_be_declared() {
        let dir = test_dir("multi_bin");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[[bin]]
name = "aacode-cli"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let err = generate_pkgbuild(&opts).unwrap_err().to_string();
        assert!(
            err.contains("2 个 [[bin]]"),
            "多个 bin 时不应猜，应要求声明：{err}"
        );
    }

    #[test]
    fn missing_arch_subtable_falls_back_to_defaults() {
        let dir = test_dir("arch_defaults");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();

        // 没有 [bundle.arch] 子表 → 走默认值
        let identity =
            crate::meta::load_bundle_meta("Cargo.toml#[package, metadata, bundle-stable]", &dir).unwrap();
        let settings = arch_settings_from(&identity.arch, false);
        assert_eq!(settings.makedepends, vec!["cargo", "rust"]);
        assert_eq!(settings.prefix, "/usr");
        // 与 cargo-bundle 一致：不声明就不给依赖（而不是塞一份猜的基线）
        assert!(
            settings.depends.is_empty(),
            "默认依赖应为空：{:?}",
            settings.depends
        );
        // 未知字段被忽略（不像 cargo-bundle 那样 deny_unknown_fields）
        assert!(identity.desktop_template.is_none());
        // prebuilt 且未显式声明 → 清空：包内不 cargo build 也不 cargo metadata，
        // (cargo rust) 没有意义
        let prebuilt_settings = arch_settings_from(&identity.arch, true);
        assert!(
            prebuilt_settings.makedepends.is_empty(),
            "prebuilt 模式默认 makedepends 应为空：{:?}",
            prebuilt_settings.makedepends
        );
        // depends / prefix 等其余默认值不随模式变
        assert!(prebuilt_settings.depends.is_empty());
        assert_eq!(prebuilt_settings.prefix, "/usr");
    }

    #[test]
    fn icon_list_drives_installation() {
        let dir = test_dir("icon_list");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
icon = ["resources/app-icon.png", "resources/app-icon@2x.png"]

[package.metadata.bundle-stable.arch]
icon_dir = "share/icons/hicolor"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::write(dir.join("resources/app-icon.png"), "small").unwrap();
        std::fs::write(dir.join("resources/app-icon@2x.png"), "x".repeat(2048)).unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let (pkgbuild, _) = generate_pkgbuild(&opts).unwrap();
        // 两个图标都装，路径来自 icon 列表而非硬编码；目录来自 [bundle.arch].icon_dir
        assert!(
            pkgbuild.contains("share/icons/hicolor/512x512/apps/aacode.png"),
            "512 图标应按 icon 列表安装：{pkgbuild}"
        );
        assert!(
            pkgbuild.contains("share/icons/hicolor/1024x1024/apps/aacode.png"),
            "1024 图标应按 icon 列表安装：{pkgbuild}"
        );
        assert!(
            !pkgbuild.contains("$startdir_icon"),
            "图标引用应走 $_srcdir（曾因定义/引用变量名不一致导致空路径）：{pkgbuild}"
        );
        assert!(
            pkgbuild.contains("$_srcdir/resources/app-icon.png"),
            "图标路径应来自 icon 列表：{pkgbuild}"
        );
    }

    #[test]
    fn bin_name_falls_back_to_package_name_when_no_bin_section() {
        // cargo 的规则：没有 [[bin]] 时 src/main.rs 自动成为以包名命名的二进制。
        // aacode 正是这种（无 [[bin]] 段），此前误报"没有 [[bin]] 目标"。
        let dir = test_dir("no_bin_section");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
description = "A coding agent built with Rust and GPUI"
license = "Apache-2.0"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();

        let opts = PkgbuildOptions {
            project_dir: &dir,
            bundle_meta: vec!["Cargo.toml#[package, metadata, bundle-stable]".to_string()],
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
            profile: None,
            prebuilt: false,
            bin_path: None,
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();
        assert!(
            pkgbuild.contains("$pkgdir/usr/bin/aacode"),
            "无 [[bin]] 时应回退包名：{pkgbuild}"
        );
        assert!(pkgbuild.contains("cargo build --release -p aacode"));
        // 无 desktop_template → 自动生成 .desktop
        let (name, content) = desktop.expect("应自动生成 .desktop");
        assert_eq!(name, "dev.aa.AaCode.desktop");
        assert!(content.contains("Name=AaCode"), "{content}");
        assert!(content.contains("Exec=aacode"), "{content}");
        assert!(content.contains("Icon=aacode"), "{content}");
        assert!(content.contains("Terminal=false"), "{content}");
        // Comment 回退链：short_description 缺省 → Cargo.toml 的 [package].description
        assert!(
            content.contains("Comment=A coding agent built with Rust and GPUI"),
            "Comment 应回退到 package.description：{content}"
        );
        // 没配 category → 回退 Utility;
        assert!(content.contains("Categories=Utility;"), "{content}");
        assert!(
            content.contains("StartupWMClass=dev.aa.AaCode"),
            "{content}"
        );
    }

    #[test]
    fn desktop_entry_uses_cargo_bundle_category_mapping() {
        // cargo-bundle 的 category 用 macOS LSApplicationCategoryType 体系的值
        let meta = crate::meta::BundleMeta {
            name: Some("AaCode".to_string()),
            short_description: Some("A coding agent".to_string()),
            category: Some("Developer Tool".to_string()),
            linux: Some(crate::meta::LinuxTable {
                mime_types: Some(vec!["text/plain".to_string()]),
                exec_args: Some("%F".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        // 图标安装名与 bin 名不同时，Icon= 必须用安装名（曾写死 bin 名的 bug）
        let out = crate::desktop::render_desktop_entry(&meta, "aacode", "my_icon", "dev.aa.AaCode", "");
        // "Developer Tool" → Development;（cargo-bundle category.rs:134）
        assert!(out.contains("Categories=Development;"), "{out}");
        assert!(out.contains("Comment=A coding agent"), "{out}");
        assert!(out.contains("Exec=aacode %F"), "{out}");
        assert!(out.contains("Icon=my_icon"), "{out}");
        assert!(out.contains("MimeType=text/plain;"), "{out}");
        // 刻意不写的键
        assert!(
            !out.contains("Version="),
            "Version 是 spec 版本不是 app 版本"
        );
        assert!(!out.contains("Encoding="), "Encoding 已废弃");
    }

    #[test]
    fn desktop_rendering_substitutes_only_known_vars() {
        let out = crate::desktop::render_template(
            "Name=$APP_NAME\nExec=$APP_CLI %F\nIcon=$APP_ICON\nX=$UNKNOWN",
            &[
                ("APP_NAME", "AaCode"),
                ("APP_CLI", "/usr/bin/aacode"),
                ("APP_ICON", "aacode"),
            ],
        );
        assert!(out.contains("Name=AaCode"));
        assert!(out.contains("Exec=/usr/bin/aacode %F"));
        assert!(out.contains("Icon=aacode"));
        assert!(out.contains("X=$UNKNOWN"), "未提供的变量应原样保留");
    }

    /// prebuilt 模式测试共用的 fixture：临时目录 + 假 crate + 假产物
    /// `target/release-quick/aacode`（targz 测试同款：写个非 ELF 文本即可，
    /// 生成期只查存在性）。
    fn prebuilt_fixture(tag: &str) -> (PathBuf, PathBuf) {
        let dir = test_dir(tag);
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();
        // resolve_target_dir 走 cargo metadata：临时目录里也能问出 target_directory
        // （= <临时目录>/target），所以假产物放这里就能被默认路径定位到。
        let bin = dir.join("target/release-quick/aacode");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "not a real elf").unwrap();
        (dir, bin)
    }

    #[test]
    fn prebuilt_mode_installs_without_building() {
        let (dir, bin) = prebuilt_fixture("prebuilt_basic");
        let (pkgbuild, _) =
            generate_pkgbuild(&opts_with(&dir, Some("release-quick"), true, None)).unwrap();

        // build() 不编译：命令形态的 cargo build 不出现（echo 提示里的字符串不算）
        assert!(
            !pkgbuild.contains("\n  cargo build"),
            "prebuilt 的 build() 不该现场编译：{pkgbuild}"
        );
        // build() 只做存在性校验
        assert!(
            pkgbuild.contains("[ ! -x"),
            "prebuilt 的 build() 应校验钉死的二进制：{pkgbuild}"
        );
        // package() 直接 install 生成期钉死的绝对路径，不调 cargo metadata
        let pinned = bin.canonicalize().unwrap();
        assert!(
            pkgbuild.contains(&format!(
                "install -Dm755 \"{}\"",
                pinned.display()
            )),
            "prebuilt 应直接 install 钉死路径：{pkgbuild}"
        );
        assert!(
            !pkgbuild.contains("cargo metadata"),
            "prebuilt 的 package() 不该调 cargo metadata：{pkgbuild}"
        );
        // 包内不调 cargo → makedepends 整行不写
        assert!(
            !pkgbuild.contains("makedepends="),
            "prebuilt 默认 makedepends 应为空：{pkgbuild}"
        );
        // 头部注释写明「重编后需重新生成」
        assert!(
            pkgbuild.contains("重新构建后必须重新运行 aa-bundle --prebuilt"),
            "{pkgbuild}"
        );
        // 两模式共用的 unset 行（含 DEBUG_RUSTFLAGS）
        assert!(pkgbuild.contains("unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS DEBUG_RUSTFLAGS"));
    }

    #[test]
    fn prebuilt_mode_requires_existing_binary() {
        let dir = test_dir("prebuilt_missing");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"
"#,
        )
        .unwrap();
        // 不建 target/release-quick/aacode → 生成期就报错（不留给 makepkg）
        let err = generate_pkgbuild(&opts_with(&dir, Some("release-quick"), true, None))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("cargo build --profile release-quick -p aacode"),
            "错误消息应告诉用户先构建什么：{err}"
        );
        assert!(
            err.contains("prebuilt 模式不构建"),
            "错误消息应说明 prebuilt 语义：{err}"
        );
    }

    #[test]
    fn prebuilt_bin_path_override_is_pinned() {
        let (dir, _default_bin) = prebuilt_fixture("prebuilt_override");
        // 覆盖路径指向别处的二进制（canonicalize 后钉死）
        let elsewhere = dir.join("elsewhere/aacode");
        std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
        std::fs::write(&elsewhere, "not a real elf either").unwrap();
        let (pkgbuild, _) =
            generate_pkgbuild(&opts_with(&dir, Some("release-quick"), true, Some(&elsewhere)))
                .unwrap();
        let pinned = elsewhere.canonicalize().unwrap();
        assert!(
            pkgbuild.contains(&format!("install -Dm755 \"{}\"", pinned.display())),
            "--bin-path 覆盖应被钉死进 PKGBUILD：{pkgbuild}"
        );
        // 覆盖路径不存在 → 生成期报错
        let missing = dir.join("elsewhere/missing");
        let err = generate_pkgbuild(&opts_with(&dir, Some("release-quick"), true, Some(&missing)))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("--bin-path"),
            "应指出是 --bin-path 的问题：{err}"
        );
    }

    #[test]
    fn prebuilt_makedepends_explicit_still_respected() {
        let dir = test_dir("prebuilt_makedepends");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"
version = "1.23.2"
license = "Apache-2.0"

[[bin]]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"

[package.metadata.bundle-stable.arch]
makedepends = ["foo", "bar"]
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("target/release-quick")).unwrap();
        std::fs::write(dir.join("target/release-quick/aacode"), "not a real elf").unwrap();
        let (pkgbuild, _) =
            generate_pkgbuild(&opts_with(&dir, Some("release-quick"), true, None)).unwrap();
        assert!(
            pkgbuild.contains("makedepends=(foo bar)"),
            "显式声明的 makedepends 在 prebuilt 下仍应保留：{pkgbuild}"
        );
    }
}
