//! # aa-bundle —— 打包基础设施（库侧）
//!
//! 打包工具 `aa-bundle` 的**可复用**部分，二进制（`src/main.rs`）只是 CLI 壳。
//! 其他工具（如 `aa-gpui-cli`）依赖本库，不重复实现。
//!
//! 当前提供：
//! - **身份解析**：[`BundleMeta`] / [`load_bundle_meta`] —— 从结构化文件
//!   （Cargo.toml 的 `[package.metadata.bundle*]` 或任意 JSON）读出
//!   `identifier` / `name`，字段名对齐 cargo-bundle 的 `BundleSettings`。
//! - **打包**：[`arch`] —— 生成 Arch Linux 的 PKGBUILD（第一个目标）。
//! - 通用基建：[`repo_name_from`]、[`relative_path`]、[`write_file`] /
//!   [`write_bytes`] / [`copy_dir`] / [`set_executable`]。
//!
//! 错误消息里提到 `--bundle-meta`，因为那是消费方 CLI 的约定参数名
//! （本库不定义 CLI 参数，只定义引用语法与解析规则）。

pub mod pkgbuild;

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// bundle 元数据：`--bundle-meta` 指向的结构。
///
/// 字段**对齐 cargo-bundle 的 `BundleSettings`**（cargo-bundle-0.12.0
/// settings.rs:174-213），因此可以直接指向 Cargo.toml 的
/// `[package.metadata.bundle*]` 表 —— 同一份配置将来给 macOS 打包复用。
///
/// | 字段 | 来源 | aa-bundle 的用途 |
/// | --- | --- | --- |
/// | `identifier` | 通用 | .desktop 文件名；缺省推导 `<仓库名>.<bin 名>` |
/// | `name` | 通用 | 桌面显示名、pkgdesc 回退 |
/// | `icon` | 通用 | 图标路径列表（取第一个存在的） |
/// | `version` | 通用 | pkgver；缺省取 Cargo.toml 版本 |
/// | `copyright` | 通用 | PKGBUILD 注释 |
/// | `short_description` / `long_description` | 通用 | pkgdesc |
/// | `resources` | 通用 | 额外拷进 pkgdir 的文件 |
/// | `linux` | Linux 专属 | mime_types / exec_args / use_terminal |
/// | `arch` | **aa-bundle 自有子表** | depends / makedepends / 图标安装路径等 |
/// | `bin` | 多二进制 | 每个 bin 一套配置（预留） |
///
/// cargo-bundle 用 `deny_unknown_fields`，我们**不用** —— 它未来加字段时
/// 老版本 aa-bundle 不该直接报错，未知字段忽略即可。
#[derive(Deserialize, Default, Debug)]
pub struct BundleMeta {
    pub identifier: Option<String>,
    pub name: Option<String>,
    pub icon: Option<Vec<String>>,
    pub version: Option<String>,
    pub copyright: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub resources: Option<Vec<String>>,
    pub linux: Option<LinuxTable>,
    /// aa-bundle 自有的 Arch 打包子表（cargo-bundle 无此概念）
    pub arch: Option<ArchTable>,
    /// aa-bundle 自有：.desktop 模板路径（相对 crate 目录）。
    /// cargo-bundle 是自己生成 .desktop 的，我们用模板——项目自有格式故单列一个键。
    pub desktop_template: Option<String>,
}

/// cargo-bundle 的 `[bundle.linux]` 子表（settings.rs:151-158）
#[derive(Deserialize, Default, Debug)]
pub struct LinuxTable {
    pub mime_types: Option<Vec<String>>,
    pub exec_args: Option<String>,
    pub use_terminal: Option<bool>,
}

/// `[bundle.arch]` 子表 —— Arch 打包设置。
///
/// 这是 cargo-bundle 没有的概念（它不支持 Arch），由 aa-bundle 自己定义。
/// 之所以放在 bundle 表内部而不是独立段：让「身份 + 打包配置」单一来源，
/// 与 cargo-bundle 的 `linux` / `osx` 子表同构。
#[derive(Deserialize, Default, Debug)]
pub struct ArchTable {
    /// 运行依赖 → PKGBUILD 的 `depends=`
    pub depends: Option<Vec<String>>,
    /// 构建依赖 → `makedepends=`
    pub makedepends: Option<Vec<String>>,
    /// 目标架构 → `arch=()`，缺省 `x86_64`
    pub arch: Option<Vec<String>>,
    pub url: Option<String>,
    /// 安装前缀，缺省 `/usr`
    pub prefix: Option<String>,
    /// 图标安装路径（相对 prefix），缺省 `share/icons/hicolor`
    /// （尺寸子目录由 aa-bundle 按 `@2x` 推导，不写在这里）
    pub icon_dir: Option<String>,
}

/// 解析打包配置来源引用并读出 bundle 元数据。
///
/// 语法：`<文件>` 或 `<文件>#[k1, k2, ...]`。
/// - 文件部分相对 `project_dir` 解析；类型按扩展名分派（.toml / .json）。
/// - 省略 `#[...]` 时取文件根作为表；给了路径则逐层下钻。
/// - 表内 `identifier` / `name` 均可缺省（返回 `None`，由调用方推导）；
///   但**引用本身必须解析成功**——文件缺失、路径不存在、中途或目标不是表、
///   字段存在却不是字符串，都硬报错。不静默回落：路径写错、文件被移动这类
///   漂移必须立刻暴露，而不是悄悄用推导值生成一个错的包名。
pub fn load_bundle_meta(reference: &str, project_dir: &Path) -> Result<BundleMeta> {
    let table = load_table(reference, project_dir)?;
    // 整表反序列化成 BundleSettings 同构的结构。与 cargo-bundle 不同，
    // 这里**不用** deny_unknown_fields：它将来加字段时老版本不该报错。
    let value = serde_json::to_value(&table).context("表转 JSON 值失败")?;
    let identity: BundleMeta = serde_json::from_value(value).with_context(|| {
        // serde 报的是 "invalid type: integer `42`, expected a string" 这种，
        // 补上引用原文与字段名才定位得了。
        let keys = table
            .keys()
            .map(|k| format!("`{k}`"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "解析 bundle 元数据 {reference} 失败：有字段类型不符（identifier / name / icon 等应为\
             字符串或字符串数组）。该表现有键：{keys}"
        )
    })?;
    Ok(identity)
}

/// 按引用读出一张表（任意键值），是 [`load_bundle_meta`] 的通用版。
///
/// 打包配置（如 Arch 的 depends / 模板路径）也是「结构化文件里的一张表」，
/// 走同一套引用语法与同样的硬错误策略，消费方自行取键。
pub fn load_table(
    reference: &str,
    project_dir: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    // 拆「文件」与「结构路径」。分隔符用 `#` 而非 `:`——后者会与 Windows 盘符
    // （`C:\...`）碰撞；路径用数组形式而非点分——JSON key 合法含点，且
    // identifier 本身（如 `dev.aa.AaCode`）就含点，点分在这里会歧义。
    let (file_part, path_part) = match reference.split_once('#') {
        Some((file, path)) => {
            let path = path.trim();
            // 路径规范写法是 `[k1, k2, ...]`：剥掉首尾方括号（省略方括号也容忍）；
            // 只写半边说明是笔误，直接报错而不是猜。
            let inner = match (path.starts_with('['), path.ends_with(']')) {
                (true, true) => &path[1..path.len() - 1],
                (false, false) => path,
                _ => bail!(
                    "--bundle-meta 的结构路径方括号不配对：`{reference}`\
                     （应为 `文件#[k1, k2, ...]`）"
                ),
            };
            (file.trim(), Some(inner))
        }
        None => (reference.trim(), None),
    };
    anyhow::ensure!(
        !file_part.is_empty(),
        "--bundle-meta 引用的文件名为空：`{reference}`"
    );

    let file_path = project_dir.join(file_part);
    let text = std::fs::read_to_string(&file_path)
        .with_context(|| format!("读取 --bundle-meta 指向的文件失败：{}", file_path.display()))?;

    // 按扩展名把文件解析成统一的 JSON 值树（TOML 也转成 JSON 值，路径遍历只有一套逻辑）
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut value: serde_json::Value = match ext.as_str() {
        "toml" => {
            let toml_value: toml::Value = toml::from_str(&text)
                .with_context(|| format!("解析 {} 失败（TOML）", file_path.display()))?;
            serde_json::to_value(toml_value).context("TOML 值转 JSON 值失败")?
        }
        "json" => serde_json::from_str(&text)
            .with_context(|| format!("解析 {} 失败（JSON）", file_path.display()))?,
        other => bail!(
            "--bundle-meta 不支持的文件类型 `.{other}`（仅 .toml / .json）：{}",
            file_path.display()
        ),
    };

    // 逐层下钻结构路径
    let mut walked: Vec<&str> = Vec::new();
    if let Some(path) = path_part {
        for key in path.split(',') {
            let key = key.trim();
            anyhow::ensure!(
                !key.is_empty(),
                "--bundle-meta 的结构路径含空 key：`{reference}`"
            );
            let object = value.as_object().with_context(|| {
                format!(
                    "--bundle-meta `{reference}`：下钻 key `{key}` 时所在层级不是表/对象（实际是 {}）",
                    value_type_name(&value)
                )
            })?;
            walked.push(key);
            value = object.get(key).cloned().with_context(|| {
                let parent = if walked.len() > 1 {
                    walked[..walked.len() - 1].join(" > ")
                } else {
                    "文件根".to_string()
                };
                format!("--bundle-meta `{reference}`：{parent} 下找不到 key `{key}`")
            })?;
        }
    }

    Ok(value
        .as_object()
        .with_context(|| {
            format!(
                "--bundle-meta `{reference}`：目标不是表/对象（实际是 {}）",
                value_type_name(&value)
            )
        })?
        .clone())
}

/// 把一个字符串规范成「合法标识段」(`[A-Za-z][A-Za-z0-9_]*`)：
/// - 仅保留 ASCII 字母/数字/下划线，其余（含 `-`、空格、中文、点等）替换为 `_`；
/// - 整体转小写（反向域名惯例全小写）；
/// - **剔除开头所有非字母**（数字/下划线不能作段首，而不是给它们补 `_` ——
///   补 `_` 后仍是非法段）。如 `_09_a11y` → `a11y`、`3repo` → `repo`；
/// - 若剔除后不含任何字母（纯符号/中文名），兜底为单段 `x`，保证段非空且合法。
///
/// 仓库目录名可能含 `-`/大写，包名/二进制名应以同样规则约束，
/// 故它们都过这一道。Android applicationId 与桌面 identifier 都用它分段。
pub fn pkg_segment(s: &str) -> String {
    let mut seg: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while seg
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        seg.remove(0);
    }
    if seg.is_empty() {
        seg.push('x');
    }
    seg
}

/// 推导默认 identifier：`<仓库名>.<bin 名>`，两段都过 [`pkg_segment`] 规范化。
///
/// 第二段用 **bin 名**而非 package 名 —— `[[bin]].name` 完全可以与
/// `package.name` 不同，用包名就是假设用户从不设 bin 名。bin 名的取值规则：
/// - 有 `[[bin]]` 且唯一 → 该目标名
/// - 无 `[[bin]]` → 包名（cargo 的自动发现规则，不是猜）
/// - 多个 `[[bin]]` → 歧义，调用方应要求显式声明（返回 `None`）
///
/// `bin_name` 传 `None` 表示「多 bin，无法推断」。仓库名拿不到（不在 git 仓库内）
/// 同样返回 `None`，由调用方报错。
pub fn derive_identifier(repo_name: &str, bin_name: Option<&str>) -> Option<String> {
    let bin = bin_name?;
    Some(format!("{}.{}", pkg_segment(repo_name), pkg_segment(bin)))
}

/// JSON 值的中文类型名，用于错误消息。
pub fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "布尔值",
        serde_json::Value::Number(_) => "数字",
        serde_json::Value::String(_) => "字符串",
        serde_json::Value::Array(_) => "数组",
        serde_json::Value::Object(_) => "表/对象",
    }
}

/// 推导仓库名（用于默认包名 / identifier 的命名空间前缀）。
///
/// 优先用 `git rev-parse --show-toplevel` 拿仓库根目录名；若 git 不可用，
/// 则向上回溯目录树找第一个含 `.git` 的祖先，取其目录名。**两者都拿不到
/// （不在 git 仓库内）返回 None**——调用方据此报错，不兜底假名。
pub fn repo_name_from(project_dir: &Path) -> Option<String> {
    // 1) git 可用时直接用仓库根目录名（最可靠）。
    if let Some(name) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            Path::new(s.trim())
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
    {
        return Some(name);
    }

    // 2) git 不可用：向上找含 .git 的祖先目录，取其目录名。
    let mut dir = project_dir.canonicalize().ok()?;
    loop {
        if dir.join(".git").exists() {
            return dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .filter(|s| !s.is_empty());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// 计算 `from` 到 `to` 的相对路径（用 `../` 形式），用于在生成文件里引用。
pub fn relative_path(from: &Path, to: &Path) -> Option<String> {
    use std::path::Component;
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    // 找到公共前缀长度
    let mut common = 0;
    while common < from.len() && common < to.len() && from[common] == to[common] {
        common += 1;
    }
    let mut rel = Vec::new();
    // 从 from 退到公共前缀
    for _ in common..from.len() {
        rel.push(Component::Normal(std::ffi::OsStr::new("..")));
    }
    // 再从公共前缀走到 to
    for &c in &to[common..] {
        rel.push(c);
    }
    let mut s = String::new();
    for (i, c) in rel.iter().enumerate() {
        if i > 0 {
            s.push('/');
        }
        s.push_str(c.as_os_str().to_str()?);
    }
    Some(s)
}

pub fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {}", parent.display()))?;
    }

    std::fs::write(path, content).with_context(|| format!("写文件 {}", path.display()))?;
    Ok(())
}

pub fn write_bytes(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("写文件 {}", path.display()))?;
    Ok(())
}

/// 递归复制目录（用于把源目录的 assets/ 之类带进生成物）。
pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("创建目录 {}", to.display()))?;
    for entry in std::fs::read_dir(from).with_context(|| format!("读目录 {}", from.display()))? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).with_context(|| format!("复制 {} 失败", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 建一个本测试独有的临时目录（pid + tag 防相撞）。
    fn identity_test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aa-bundle-identity-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        dir
    }

    #[test]
    fn pkg_segment_normalizes_names() {
        // 大写 + 连字符 → 小写 + 下划线
        assert_eq!(pkg_segment("My-Repo2"), "my_repo2");
        // 以数字开头 → 剔除数字，不能前缀补 _
        assert_eq!(pkg_segment("3repo"), "repo");
        // 中文等非 ASCII → 下划线；剔光后兜底为单个合法段
        assert_eq!(pkg_segment("我的仓库"), "x");
        // 合法名不变
        assert_eq!(pkg_segment("uniform_list_07"), "uniform_list_07");
        assert_eq!(pkg_segment("_09_a11y"), "a11y");
        assert_eq!(pkg_segment("AaCode"), "aacode");
    }

    #[test]
    fn derive_identifier_uses_repo_and_bin() {
        // 第二段是 bin 名，不是 package 名
        assert_eq!(
            derive_identifier("gpui_learn", Some("AaCode")).as_deref(),
            Some("gpui_learn.aacode")
        );
        // 多 bin 无法推断 → None（调用方应要求显式声明）
        assert_eq!(derive_identifier("gpui_learn", None), None);
        // 仓库名同样要规范化
        assert_eq!(
            derive_identifier("My-Repo", Some("app_01")).as_deref(),
            Some("my_repo.app_01")
        );
    }

    #[test]
    fn identity_reads_json_root() {
        let dir = identity_test_dir("json_root");
        std::fs::write(
            dir.join("id.json"),
            r#"{"identifier": "aa.hello_android", "name": "Hello Android"}"#,
        )
        .unwrap();
        // 文件根即 bundle 元数据
        let table = load_bundle_meta("id.json", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("aa.hello_android"));
        assert_eq!(table.name.as_deref(), Some("Hello Android"));
    }

    #[test]
    fn identity_fields_are_optional() {
        let dir = identity_test_dir("optional");
        std::fs::write(dir.join("id.json"), r#"{"name": "Only Name"}"#).unwrap();
        let table = load_bundle_meta("id.json", &dir).unwrap();
        assert_eq!(table.identifier, None, "缺 identifier → None，由调用方推导");
        assert_eq!(table.name.as_deref(), Some("Only Name"));
    }

    #[test]
    fn identity_reads_toml_deep_path() {
        let dir = identity_test_dir("toml_deep");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "ug_01_hello_android"

[package.metadata.bundle]
identifier = "aa.hello_android"
name = "Hello Android"
icon = ["app-icon.png"]
"#,
        )
        .unwrap();
        // 数组路径下钻 + key 两侧空白容忍；表内额外 key（icon）忽略
        let table = load_bundle_meta("Cargo.toml#[package, metadata, bundle]", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("aa.hello_android"));
        assert_eq!(table.name.as_deref(), Some("Hello Android"));
    }

    #[test]
    fn identity_array_path_supports_dotted_keys() {
        // JSON key 合法含点，且 identifier 本身含点——所以路径必须是数组形式
        let dir = identity_test_dir("dotted_key");
        std::fs::write(
            dir.join("id.json"),
            r#"{"a.b": {"identifier": "dev.aa.AaCode", "name": "AaCode"}}"#,
        )
        .unwrap();
        let table = load_bundle_meta("id.json#[a.b]", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("dev.aa.AaCode"));
        assert_eq!(table.name.as_deref(), Some("AaCode"));
    }

    #[test]
    fn identity_errors_are_hard_not_silent() {
        let dir = identity_test_dir("errors");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\n\n[package.metadata.bundle]\nidentifier = 42\nname = \"Ok\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("arr.json"), r#"["not", "an", "object"]"#).unwrap();

        // 文件缺失
        let err = load_bundle_meta("nope.json", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("nope.json"), "应指出文件：{err}");

        // 路径不存在
        let err = load_bundle_meta("Cargo.toml#[package, metadata, missing]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing"), "应指出缺的 key：{err}");

        // 字段类型不对（identifier 是数字）
        let err = load_bundle_meta("Cargo.toml#[package, metadata, bundle]", &dir)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("identifier") && err.contains("字符串"),
            "应指出字段与期望类型：{err}"
        );

        // 下钻中途不是表
        let err = load_bundle_meta("arr.json#[0]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("不是表/对象"), "应指出层级类型：{err}");

        // 目标本身不是表
        let err = load_bundle_meta("arr.json", &dir).unwrap_err().to_string();
        assert!(err.contains("目标不是表/对象"), "应指出目标类型：{err}");

        // 不支持的扩展名
        std::fs::write(dir.join("id.yaml"), "identifier: x\n").unwrap();
        let err = load_bundle_meta("id.yaml", &dir).unwrap_err().to_string();
        assert!(err.contains("yaml"), "应指出扩展名：{err}");

        // 结构路径含空 key
        let err = load_bundle_meta("Cargo.toml#[package, , name]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("空 key"), "应指出空 key：{err}");
    }
}
