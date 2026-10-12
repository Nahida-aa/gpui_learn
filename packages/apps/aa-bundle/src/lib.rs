//! # aa-bundle —— 打包基础设施（库侧）
//!
//! 打包工具 `aa-bundle` 的**可复用**部分，二进制（`src/main.rs`）只是 CLI 壳。
//! 其他工具（如 `aa-gpui-cli`）依赖本库，不重复实现。
//!
//! 当前提供：
//! - **身份解析**：[`IdentityTable`] / [`load_identity_table`] —— 从结构化文件
//!   （Cargo.toml 的 `[package.metadata.bundle*]` 或任意 JSON）读出
//!   `identifier` / `name`，字段名对齐 cargo-bundle 的 `BundleSettings`。
//! - **打包**：[`arch`] —— 生成 Arch Linux 的 PKGBUILD（第一个目标）。
//! - 通用基建：[`repo_name_from`]、[`relative_path`]、[`write_file`] /
//!   [`write_bytes`] / [`copy_dir`] / [`set_executable`]。
//!
//! 错误消息里提到 `--bundle-meta`，因为那是消费方 CLI 的约定参数名
//! （本库不定义 CLI 参数，只定义引用语法与解析规则）。

pub mod arch;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// 身份表：`--bundle-meta` 指向的结构（两个字段均可缺省，缺的由调用方推导）。
///
/// 字段名**对齐 cargo-bundle 的 `BundleSettings`**（`identifier` / `name`），
/// 因此可以直接指向 Cargo.toml 的 `[package.metadata.bundle*]` 表，
/// 或任何根级含这两个 key 的 JSON 文件。
#[derive(Deserialize, Default, Debug)]
pub struct IdentityTable {
    pub identifier: Option<String>,
    pub name: Option<String>,
}

/// 解析身份来源引用并读出身份表。
///
/// 语法：`<文件>` 或 `<文件>#[k1, k2, ...]`。
/// - 文件部分相对 `project_dir` 解析；类型按扩展名分派（.toml / .json）。
/// - 省略 `#[...]` 时取文件根作为表；给了路径则逐层下钻。
/// - 表内 `identifier` / `name` 均可缺省（返回 `None`，由调用方推导）；
///   但**引用本身必须解析成功**——文件缺失、路径不存在、中途或目标不是表、
///   字段存在却不是字符串，都硬报错。不静默回落：路径写错、文件被移动这类
///   漂移必须立刻暴露，而不是悄悄用推导值生成一个错的包名。
pub fn load_identity_table(reference: &str, project_dir: &Path) -> Result<IdentityTable> {
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

    let table = value.as_object().with_context(|| {
        format!(
            "--bundle-meta `{reference}`：目标不是表/对象（实际是 {}），无法读取 identifier / name",
            value_type_name(&value)
        )
    })?;

    // 字段存在但类型不对 → 硬错误；缺省 → None，由调用方推导。
    // 只认字符串：把数字/布尔静默转成字符串会掩盖配置错误。
    let string_field = |key: &str| -> Result<Option<String>> {
        match table.get(key) {
            None => Ok(None),
            Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
            Some(other) => bail!(
                "--bundle-meta `{reference}`：字段 `{key}` 应为字符串，实际是 {}",
                value_type_name(other)
            ),
        }
    };
    Ok(IdentityTable {
        identifier: string_field("identifier")?,
        name: string_field("name")?,
    })
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
    fn identity_reads_json_root() {
        let dir = identity_test_dir("json_root");
        std::fs::write(
            dir.join("id.json"),
            r#"{"identifier": "aa.hello_android", "name": "Hello Android"}"#,
        )
        .unwrap();
        // 文件根即身份表
        let table = load_identity_table("id.json", &dir).unwrap();
        assert_eq!(table.identifier.as_deref(), Some("aa.hello_android"));
        assert_eq!(table.name.as_deref(), Some("Hello Android"));
    }

    #[test]
    fn identity_fields_are_optional() {
        let dir = identity_test_dir("optional");
        std::fs::write(dir.join("id.json"), r#"{"name": "Only Name"}"#).unwrap();
        let table = load_identity_table("id.json", &dir).unwrap();
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
        let table = load_identity_table("Cargo.toml#[package, metadata, bundle]", &dir).unwrap();
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
        let table = load_identity_table("id.json#[a.b]", &dir).unwrap();
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
        let err = load_identity_table("nope.json", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("nope.json"), "应指出文件：{err}");

        // 路径不存在
        let err = load_identity_table("Cargo.toml#[package, metadata, missing]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing"), "应指出缺的 key：{err}");

        // 字段类型不对（identifier 是数字）
        let err = load_identity_table("Cargo.toml#[package, metadata, bundle]", &dir)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("identifier") && err.contains("字符串"),
            "应指出字段与期望类型：{err}"
        );

        // 下钻中途不是表
        let err = load_identity_table("arr.json#[0]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("不是表/对象"), "应指出层级类型：{err}");

        // 目标本身不是表
        let err = load_identity_table("arr.json", &dir).unwrap_err().to_string();
        assert!(err.contains("目标不是表/对象"), "应指出目标类型：{err}");

        // 不支持的扩展名
        std::fs::write(dir.join("id.yaml"), "identifier: x\n").unwrap();
        let err = load_identity_table("id.yaml", &dir).unwrap_err().to_string();
        assert!(err.contains("yaml"), "应指出扩展名：{err}");

        // 结构路径含空 key
        let err = load_identity_table("Cargo.toml#[package, , name]", &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("空 key"), "应指出空 key：{err}");
    }
}
