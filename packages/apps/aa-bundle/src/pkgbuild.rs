//! Arch Linux 包（PKGBUILD）生成。
//!
//! 这是 `aa-bundle` 的第一个打包目标：把「构建 + 拷贝 + 注册 .desktop」这套
//! 手工流程（此前散在各项目的 justfile / script 里）收敛成生成一份 PKGBUILD，
//! 交给 `makepkg`/`pacman` 处理，从而：
//! - 卸载变成 `pacman -R`，不再需要手写 uninstall 脚本
//! - 身份（包名 / .desktop 名 / 图标）统一来自 `--bundle-meta` 指向的表
//!
//! ## 命名：`pkgbuild` vs `arch`
//!
//! CLI 子命令叫 `pkgbuild`（`aa-bundle pkgbuild`），配置段却叫
//! `[package.metadata.bundle-arch]`（默认引用）。这不是笔误，是**两个维度**：
//! - 子命令名 = **产出物**（PKGBUILD），所以将来可并列 `deb` / `rpm`
//! - 配置段名 = **目标发行版**（Arch），所以将来可并列 `bundle-deb`
//!
//! 早期子命令也叫 `arch`，与 `--arch`（CPU 架构：x86_64）撞词，故改掉。
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

/// Arch 打包设置，来自 Cargo.toml 的 `[package.metadata.bundle-arch]`。
///
/// 对齐 cargo-bundle 的做法：**打包元数据声明在 Cargo.toml，不在命令行**。
/// （cargo-bundle 的 `deb_depends` 就是干这个的；这里按同样思路给 Arch 一份。）
/// 全部字段可缺省，缺省值见 [`PkgbuildSettings::default`]。
#[derive(Debug, Clone)]
pub struct PkgbuildSettings {
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub arch: Vec<String>,
    pub url: String,
    pub prefix: String,
    /// .desktop 模板路径（相对 crate 目录）
    pub desktop_template: Option<String>,
    /// 二进制名，缺省取 crate 名
    pub bin_name: Option<String>,
    /// 图标基础名，缺省取 crate 名
    pub icon_name: Option<String>,
}

impl Default for PkgbuildSettings {
    fn default() -> Self {
        Self {
            // 与 cargo-bundle 的 deb_depends 一致：**不猜、不填默认值**。
            // 系统包依赖只有项目自己知道（cargo 只认 -l，不认发行版包名），
            // 没声明就整行不写 Depends/depends（见 cargo-bundle deb_bundle.rs:128-131）。
            // 本机开发安装时依赖早已在机器上，省略无副作用；要分发再声明。
            depends: Vec::new(),
            // 这条不是猜：我们的 PKGBUILD 自己调 cargo build，构建 Rust crate 必然需要
            makedepends: ["cargo", "rust"].iter().map(|s| s.to_string()).collect(),
            arch: vec!["x86_64".to_string()],
            url: String::new(),
            prefix: "/usr".to_string(),
            desktop_template: None,
            bin_name: None,
            icon_name: None,
        }
    }
}

/// 从引用指向的表读 Arch 设置；表不存在时返回默认值（该段是可选的）。
pub fn load_pkgbuild_settings(reference: &str, project_dir: &Path) -> Result<PkgbuildSettings> {
    let mut settings = PkgbuildSettings::default();
    let table = match crate::load_table(reference, project_dir) {
        Ok(t) => t,
        // 该段是可选的：找不到就用默认值。但**引用语法错误**（如方括号不配对）
        // 仍然要报，否则用户以为自己配了其实没生效。
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("方括号不配对") || msg.contains("空 key") || msg.contains("扩展名") {
                return Err(e);
            }
            return Ok(settings);
        }
    };

    let strings = |key: &str| -> Result<Option<Vec<String>>> {
        match table.get(key) {
            None => Ok(None),
            // 数组：["a", "b"]
            Some(serde_json::Value::Array(items)) => {
                let mut out = Vec::new();
                for item in items {
                    match item {
                        serde_json::Value::String(s) => out.push(s.clone()),
                        other => bail!(
                            "Arch 设置 `{key}` 数组的每项应为字符串，实际含 {}",
                            crate::value_type_name(other)
                        ),
                    }
                }
                Ok(Some(out))
            }
            // 也接受空格分隔的单个字符串："a b c"
            Some(serde_json::Value::String(s)) => Ok(Some(
                s.split_whitespace().map(|p| p.to_string()).collect(),
            )),
            Some(other) => bail!(
                "Arch 设置 `{key}` 应为字符串数组或空格分隔字符串，实际是 {}",
                crate::value_type_name(other)
            ),
        }
    };
    let string = |key: &str| -> Result<Option<String>> {
        match table.get(key) {
            None => Ok(None),
            Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
            Some(other) => bail!(
                "Arch 设置 `{key}` 应为字符串，实际是 {}",
                crate::value_type_name(other)
            ),
        }
    };

    if let Some(v) = strings("depends")? {
        settings.depends = v;
    }
    if let Some(v) = strings("makedepends")? {
        settings.makedepends = v;
    }
    if let Some(v) = strings("arch")? {
        settings.arch = v;
    }
    if let Some(v) = string("url")? {
        settings.url = v;
    }
    if let Some(v) = string("prefix")? {
        settings.prefix = v;
    }
    settings.desktop_template = string("desktop_template")?;
    settings.bin_name = string("bin_name")?;
    settings.icon_name = string("icon_name")?;
    Ok(settings)
}

/// PKGBUILD 生成参数。
pub struct PkgbuildOptions<'a> {
    /// 源码目录（含 Cargo.toml）
    pub project_dir: &'a Path,
    /// 频道：决定 bundle 表（`bundle-<channel>`）与图标后缀
    pub channel: &'a str,
    /// 身份来源引用；默认 `Cargo.toml#[package, metadata, bundle-<channel>]`
    pub bundle_meta: &'a str,
    /// Arch 打包设置来源引用；默认 `Cargo.toml#[package, metadata, bundle-arch]`（该段可选）
    pub pkgbuild_meta: &'a str,
    /// 命令行覆盖：二进制名
    pub bin_name: Option<&'a str>,
    /// 命令行覆盖：.desktop 模板路径
    pub desktop_template: Option<&'a Path>,
    /// 命令行覆盖：图标基础名
    pub icon_name: Option<&'a str>,
    /// 命令行覆盖：运行依赖（给了就完全替代 Cargo.toml 里的值）
    pub depends: Option<&'a [String]>,
    /// 命令行覆盖：构建依赖
    pub makedepends: Option<&'a [String]>,
    /// 命令行覆盖：目标架构
    pub arch: Option<&'a [String]>,
    /// 命令行覆盖：url
    pub url: Option<&'a str>,
    /// 命令行覆盖：license
    pub license: Option<&'a str>,
    /// 命令行覆盖：安装前缀
    pub prefix: Option<&'a str>,
}

/// 生成 PKGBUILD 文本，并渲染 .desktop。
///
/// 返回 `(PKGBUILD 文本, Option<(desktop 文件名, desktop 内容)>)`。
pub fn generate_pkgbuild(opts: &PkgbuildOptions) -> Result<(String, Option<(String, String)>)> {
    let info = read_package_info(opts.project_dir)?;
    let identity: IdentityTable = load_identity_table(opts.bundle_meta, opts.project_dir)?;
    let mut settings = load_pkgbuild_settings(opts.pkgbuild_meta, opts.project_dir)?;
    // 命令行覆盖优先于 Cargo.toml
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

    // 各名字的推导链：命令行 > Cargo.toml 的 arch 设置 > crate 名
    let bin = opts
        .bin_name
        .or(settings.bin_name.as_deref())
        .unwrap_or(&info.name);
    let icon = opts
        .icon_name
        .or(settings.icon_name.as_deref())
        .unwrap_or(&info.name);
    let suffix = icon_suffix(opts.channel);
    let license = opts.license.unwrap_or(&info.license);
    let prefix = settings.prefix.trim_end_matches('/');
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

    // cargo 包名直接从 crate 的 Cargo.toml 取 —— 已经指了目录，不需要再让人传一遍
    let cargo_pkg_flag = format!(" -p {}", info.name);

    // 渲染 .desktop（模板路径：命令行 > Cargo.toml 的 arch 设置 > 不产出）
    let template_path = match opts.desktop_template {
        Some(p) => Some(p.to_path_buf()),
        None => settings
            .desktop_template
            .as_ref()
            .map(|rel| opts.project_dir.join(rel)),
    };
    let desktop_rel = match &template_path {
        Some(template_path) => {
            let template = std::fs::read_to_string(template_path)
                .with_context(|| format!("读取 .desktop 模板 {} 失败", template_path.display()))?;
            let rendered = render_desktop(
                &template,
                &[
                    ("APP_NAME", identity.name.as_deref().unwrap_or(&info.name)),
                    ("APP_ID", &desktop_id),
                    ("APP_CLI", &format!("{prefix}/bin/{bin}")),
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

    let desktop_install = match &desktop_rel {
        Some((name, _)) => format!(
            "  install -Dm644 \"$startdir/{name}\" \"$pkgdir{prefix}/share/applications/{name}\"\n"
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
{depends_line}{makedepends_line}
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
        depends_line = depends_line,
        makedepends_line = makedepends_line,
        src = src.display(),
        bin = bin,
        suffix = suffix,
        icon = icon,
        prefix = prefix,
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

        let opts = PkgbuildOptions {
            project_dir: &dir,
            channel: "stable",
            bundle_meta: "Cargo.toml#[package, metadata, bundle-stable]",
            pkgbuild_meta: "Cargo.toml#[package, metadata, bundle-arch]",
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();

        // 身份来自 bundle 表：pkgname 取 crate 名、desktop 名取 identifier
        assert!(pkgbuild.contains("pkgname=aacode"));
        assert!(pkgbuild.contains("pkgver=1.23.2"));
        assert!(pkgbuild.contains("license=(Apache-2.0)"));
        // cargo 包名从 Cargo.toml 推导，不需要再传
        assert!(pkgbuild.contains("cargo build --release -p aacode"));
        assert!(
            pkgbuild.contains("hicolor/512x512/apps/aacode.png"),
            "图标应装到 hicolor 512x512：{pkgbuild}"
        );
        assert!(!pkgbuild.contains("url="), "未给 url 时不该输出空字段");
        // 未声明 depends → 整行不写（对齐 cargo-bundle 的 deb 行为）
        // 注意按行判断：makedepends= 里也含子串 "depends="
        assert!(
            !pkgbuild.lines().any(|l| l.starts_with("depends=")),
            "没声明依赖时不该写 depends= 行：{pkgbuild}"
        );
        assert!(
            pkgbuild
                .lines()
                .any(|l| l == "makedepends=(cargo rust)"),
            "构建 Rust crate 必然需要 cargo/rust，这不算猜：{pkgbuild}"
        );
        // 没配 desktop_template → 不产出 desktop
        assert!(desktop.is_none());
        // 源码路径绝对化
        let src = dir.canonicalize().unwrap();
        assert!(
            pkgbuild.contains(&format!("_srcdir=\"{}\"", src.display())),
            "_srcdir 应为绝对路径：{pkgbuild}"
        );
    }

    #[test]
    fn pkgbuild_settings_come_from_cargo_toml_not_flags() {
        let dir = test_dir("arch_settings");
        // 该段是可选的：不声明也能生成（用默认值）
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

[package.metadata.bundle-arch]
depends = ["gcc-libs", "glibc", "gtk3"]
desktop_template = "resources/aacode.desktop.in"
bin_name = "aacode"
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
            channel: "stable",
            bundle_meta: "Cargo.toml#[package, metadata, bundle-stable]",
            pkgbuild_meta: "Cargo.toml#[package, metadata, bundle-arch]",
            bin_name: None,
            desktop_template: None,
            icon_name: None,
            depends: None,
            makedepends: None,
            arch: None,
            url: None,
            license: None,
            prefix: None,
        };
        let (pkgbuild, desktop) = generate_pkgbuild(&opts).unwrap();

        assert!(
            pkgbuild.contains("depends=(gcc-libs glibc gtk3)"),
            "依赖应来自 Cargo.toml 而非命令行：{pkgbuild}"
        );
        // 默认 makedepends（Cargo.toml 未声明）
        assert!(pkgbuild.contains("makedepends=(cargo rust)"));
        // desktop_template 生效：产出 desktop 且渲染过
        let (name, content) = desktop.expect("配了 desktop_template 就应产出 desktop");
        assert_eq!(name, "dev.aa.AaCode.desktop");
        assert!(content.contains("Name=AaCode"));
        assert!(content.contains("Exec=/usr/bin/aacode %F"));
    }

    #[test]
    fn missing_arch_section_falls_back_to_defaults() {
        let dir = test_dir("arch_defaults");
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

        let settings = load_pkgbuild_settings(
            "Cargo.toml#[package, metadata, bundle-arch]",
            &dir,
        )
        .unwrap();
        assert_eq!(settings.makedepends, vec!["cargo", "rust"]);
        assert_eq!(settings.prefix, "/usr");
        assert!(settings.desktop_template.is_none());
        // 与 cargo-bundle 一致：不声明就不给依赖（而不是塞一份猜的基线）
        assert!(settings.depends.is_empty(), "默认依赖应为空：{:?}", settings.depends);
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
