//! bundle 元数据：`--bundle-meta` 指向的结构与解析规则。
//!
//! 字段**对齐 cargo-bundle 的 `BundleSettings`**（cargo-bundle-0.12.0
//! settings.rs:174-213），因此可以直接指向 Cargo.toml 的
//! `[package.metadata.bundle*]` 表 —— 同一份配置将来给 macOS 打包复用。

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// bundle 元数据：`--bundle-meta` 指向的结构。
///
/// | 字段 | 来源 | aa-bundle 的用途 |
/// | --- | --- | --- |
/// | `identifier` | 通用 | .desktop 文件名；缺省推导 `<仓库名>.<bin 名>` |
/// | `name` | 通用 | 桌面显示名、pkgdesc 回退 |
/// | `icon` | 通用 | 图标路径列表（取第一个存在的） |
/// | `version` | 通用 | pkgver；缺省 Cargo.toml 版本 |
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
    pub short_description: Option<String>, // cargo.description, pkgdesc
    pub long_description: Option<String>,
    /// freedesktop Categories 的来源（cargo-bundle 的 `category`，macOS 体系值如
    /// `"Developer Tool"`）。渲染时经 [`crate::desktop::AppCategory::parse`] 映射。
    pub category: Option<String>,
    pub resources: Option<Vec<String>>,
    pub linux: Option<LinuxTable>,
    /// aa-bundle 自有的 Arch 打包子表（cargo-bundle 无此概念）
    pub arch: Option<ArchTable>,
    /// aa-bundle 自有：.desktop **模板**路径（相对 crate 目录），可选。
    /// 不填则由 aa-bundle 按 cargo-bundle 的方式自己生成 .desktop；
    /// 填了则用模板渲染（`$APP_NAME` / `$APP_ID` / `$APP_CLI` / `$APP_ICON` 等）。
    pub desktop_template: Option<String>,
}

impl BundleMeta {
    /// `linux.exec_args` → Exec 的后缀（带前导空格）；未设则为空
    pub fn exec_suffix(&self) -> String {
        match self.linux.as_ref().and_then(|l| l.exec_args.as_deref()) {
            Some(a) if !a.is_empty() => format!(" {a}"),
            _ => String::new(),
        }
    }

    pub fn terminal(&self) -> bool {
        self.linux
            .as_ref()
            .and_then(|l| l.use_terminal)
            .unwrap_or(false)
    }

    pub fn mime_types(&self) -> &[String] {
        self.linux
            .as_ref()
            .and_then(|l| l.mime_types.as_deref())
            .unwrap_or(&[])
    }

    pub fn desktop_actions(&self) -> &std::collections::BTreeMap<String, DesktopAction> {
        static EMPTY: std::collections::BTreeMap<String, DesktopAction> =
            std::collections::BTreeMap::new();
        self.linux
            .as_ref()
            .and_then(|l| l.desktop_actions.as_ref())
            .unwrap_or(&EMPTY)
    }
}

/// cargo-bundle 的 `[bundle.linux]` 子表（settings.rs:151-158）
#[derive(Deserialize, Default, Debug)]
pub struct LinuxTable {
    pub mime_types: Option<Vec<String>>,
    pub exec_args: Option<String>,
    pub use_terminal: Option<bool>,
    pub startup_wm_class: Option<String>,
    /// `Actions=` + `[Desktop Action <id>]` 组
    pub desktop_actions: Option<std::collections::BTreeMap<String, DesktopAction>>,
}

/// `[bundle.linux.desktop_actions.<id>]`（cargo-bundle settings.rs:107-118）
#[derive(Deserialize, Default, Debug)]
pub struct DesktopAction {
    pub name: String,
    pub exec: Option<String>,
    pub icon: Option<String>,
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
///
/// 单引用版；多引用分层见 [`load_bundle_meta_layered`]。
pub fn load_bundle_meta(reference: &str, project_dir: &Path) -> Result<BundleMeta> {
    load_bundle_meta_layered(&[reference], project_dir)
}

/// 分层解析：多个引用按给出顺序合并，**后者递归覆盖前者**（对象按 key 递归
/// 合并；数组与标量整体替换）。
///
/// 典型用法是「共享表打底 + channel 表覆盖」：
///
/// ```text
/// --bundle-meta "Cargo.toml#[package, metadata, bundle-shared]" \
/// --bundle-meta "Cargo.toml#[package, metadata, bundle-stable]"
/// ```
///
/// 这样 category / linux 设置等各 channel 相同的部分只写一份，stable /
/// nightly 的表只留自己的差异（identifier / name / icon …）。
/// 每个引用独立解析（各自的硬错误策略不变），全部成功后才合并、
/// 整体反序列化——类型错误报的是合并后的表。
pub fn load_bundle_meta_layered<S: AsRef<str>>(
    references: &[S],
    project_dir: &Path,
) -> Result<BundleMeta> {
    let Some((first, rest)) = references.split_first() else {
        bail!("bundle 元数据引用列表为空：至少给一个 --bundle-meta");
    };
    let mut merged = serde_json::to_value(load_table(first.as_ref(), project_dir)?)
        .context("表转 JSON 值失败")?;
    for r in rest {
        let overlay = serde_json::to_value(load_table(r.as_ref(), project_dir)?)
            .context("表转 JSON 值失败")?;
        merge_json(&mut merged, overlay);
    }
    // 整表反序列化成 BundleSettings 同构的结构。与 cargo-bundle 不同，
    // 这里**不用** deny_unknown_fields：它将来加字段时老版本不该报错。
    let keys = merged
        .as_object()
        .map(|m| {
            m.keys()
                .map(|k| format!("`{k}`"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let refs = references
        .iter()
        .map(|r| r.as_ref())
        .collect::<Vec<_>>()
        .join(" + ");
    let identity: BundleMeta = serde_json::from_value(merged).with_context(|| {
        // serde 报的是 "invalid type: integer `42`, expected a string" 这种，
        // 补上引用原文与字段名才定位得了。
        format!(
            "解析 bundle 元数据 {refs} 失败：有字段类型不符（identifier / name / icon 等应为\
             字符串或字符串数组）。合并后的表现有键：{keys}"
        )
    })?;
    Ok(identity)
}

/// 递归合并两个 JSON 值：对象按 key 递归；其余（标量、数组）整体替换。
///
/// 分层配置的语义——后者只覆盖自己声明的 key，没声明的部分保留前者的值；
/// 数组不拼接（`depends` 覆盖 `depends`，不是追加）。
fn merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    use serde_json::Value;
    match (base, overlay) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(bv) => merge_json(bv, v),
                    None => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, o) => *b = o,
    }
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
                "--bundle-meta 引用的结构路径含空 key：`{reference}`"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 建一个本测试独有的临时目录（pid + tag 防相撞）。
    fn test_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("aa-bundle-meta-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        dir
    }

    #[test]
    fn identity_reads_json_root() {
        let dir = test_dir("json_root");
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
        let dir = test_dir("optional");
        std::fs::write(dir.join("id.json"), r#"{"name": "Only Name"}"#).unwrap();
        let table = load_bundle_meta("id.json", &dir).unwrap();
        assert_eq!(table.identifier, None, "缺 identifier → None，由调用方推导");
        assert_eq!(table.name.as_deref(), Some("Only Name"));
    }

    #[test]
    fn linux_subtable_is_nested_not_flat() {
        let dir = test_dir("linux_nested");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"

[package.metadata.bundle-stable]
identifier = "dev.aa.AaCode"
name = "AaCode"

[package.metadata.bundle-stable.linux]
mime_types = ["text/plain", "x-scheme-handler/aacode"]
"#,
        )
        .unwrap();
        let meta = load_bundle_meta("Cargo.toml#[package, metadata, bundle-stable]", &dir).unwrap();
        // cargo-bundle 的 BundleSettings 是 linux: Option<LinuxSettings>（嵌套，
        // settings.rs:191），mime_types 在 LinuxSettings 里。平铺的
        // linux_mime_types 不在 schema 里——cargo-bundle 有 deny_unknown_fields
        // 会直接报错，我们静默忽略（所以平铺写法等于没配）。这个测试锁住嵌套形态。
        assert_eq!(
            meta.mime_types(),
            ["text/plain", "x-scheme-handler/aacode"],
            "mime_types 应在 [....linux] 嵌套子表里"
        );
    }

    #[test]
    fn layered_meta_merges_shared_then_specific() {
        let dir = test_dir("layered");
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "aacode"

[package.metadata.bundle-shared]
category = "Developer Tool"
short_description = "shared desc"

[package.metadata.bundle-shared.linux]
mime_types = ["text/plain"]

[package.metadata.bundle-stable]
name = "AaCode"
short_description = "stable desc"

[package.metadata.bundle-stable.linux]
use_terminal = true
"#,
        )
        .unwrap();

        let refs = vec![
            "Cargo.toml#[package, metadata, bundle-shared]".to_string(),
            "Cargo.toml#[package, metadata, bundle-stable]".to_string(),
        ];
        let meta = load_bundle_meta_layered(&refs, &dir).unwrap();
        // specific 覆盖 shared 的同名键
        assert_eq!(meta.name.as_deref(), Some("AaCode"));
        assert_eq!(
            meta.short_description.as_deref(),
            Some("stable desc"),
            "后者应覆盖前者"
        );
        // shared 独有的键保留
        assert_eq!(meta.category.as_deref(), Some("Developer Tool"));
        // 子表递归合并：shared 的 mime_types 与 specific 的 use_terminal 都在
        let linux = meta.linux.expect("linux 子表应合并保留");
        assert_eq!(
            linux.mime_types.as_deref(),
            Some(&["text/plain".to_string()][..]),
            "shared 子表里的键应保留"
        );
        assert_eq!(linux.use_terminal, Some(true), "specific 子表里的键应合入");
    }

    #[test]
    fn layered_meta_requires_at_least_one_reference() {
        let dir = test_dir("layered_empty");
        let refs: Vec<String> = Vec::new();
        let err = load_bundle_meta_layered(&refs, &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("至少"), "空列表应报错：{err}");
    }

    #[test]
    fn identity_reads_toml_deep_path() {
        let dir = test_dir("toml_deep");
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
        let dir = test_dir("dotted_key");
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
        let dir = test_dir("errors");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\n\n[package.metadata.bundle]\nidentifier = 42\nname = \"Ok\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("arr.json"), r#"["not", "an", "object"]"#).unwrap();

        // 文件缺失
        let err = load_bundle_meta("nope.json", &dir).unwrap_err().to_string();
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
        assert!(err.contains("空 key"), "空 key 应报错：{err}");
    }
}
