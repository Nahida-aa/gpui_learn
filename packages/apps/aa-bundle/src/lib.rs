//! # aa-bundle —— 打包基础设施（库侧）
//!
//! 打包工具 `aa-bundle` 的**可复用**部分，二进制（`src/main.rs`）只是 CLI 壳。
//! 其他工具（如 `aa-gpui-cli`）依赖本库，不重复实现。
//!
//! 当前提供：
//! - **身份解析**：[`BundleMeta`] / [`load_bundle_meta`] —— 从结构化文件
//!   （Cargo.toml 的 `[package.metadata.bundle*]` 或任意 JSON）读出
//!   `identifier` / `name`，字段名对齐 cargo-bundle 的 `BundleSettings`。
//! - **打包格式**（CLI 的 `--format` 枚举，一个模块一种）：
//!   - [`pkgbuild`] —— Arch Linux 的 PKGBUILD（makepkg 的输入）
//!   - [`targz`] —— 自包含 tar.gz 应用包（zed.app 布局，跨发行版）
//! - **格式共用的桌面/图标基建**：[`resolve_desktop_entry`]、
//!   [`icon_size_dir`]、[`resolve_icon_name`]。
//! - 通用基建：[`repo_name_from`]、[`relative_path`]、[`write_file`] /
//!   [`write_bytes`]、[`copy_dir`] / [`set_executable`]。
//!
//! 格式特有设置（Arch 的 depends / 安装前缀等）一律放 bundle 元数据的
//! 子表（如 `[bundle.arch]`），**不做成 CLI flag**——flag 只留所有格式
//! 通用的（`-p` / `--bundle-meta` / `--bin-name` / `--icon-name` /
//! `--desktop-template` / `--out`），否则加新格式时用户会看到一堆对当前
//! 格式无效的参数。
//!
//! 错误消息里提到 `--bundle-meta`，因为那是消费方 CLI 的约定参数名
//! （本库不定义 CLI 参数，只定义引用语法与解析规则）。

pub mod pkgbuild;
pub mod targz;

use std::path::{Path, PathBuf};

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
    pub short_description: Option<String>, // cargo.description, pkgdesc
    pub long_description: Option<String>,
    /// freedesktop Categories 的来源（cargo-bundle 的 `category`，macOS 体系值如
    /// `"Developer Tool"`）。渲染时经 [`AppCategory::parse`] 映射。
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

/// 应用类别：cargo-bundle 的 `category` 字段。
///
/// **值用的是 macOS `LSApplicationCategoryType` 体系**（如 `"Developer Tool"`），
/// 与 freedesktop Categories 是两套不同的枚举 —— cargo-bundle 自己做了映射
/// （category.rs:131-174）。这里照抄那张表，免得项目为两个平台各写一份。
///
/// 解析时容错（category.rs:57-80 的简化版）：大小写不敏感、忽略空格与连字符、
/// 剥离 `public.app-category.` 前缀。认不出时返回 `None`，调用方告警并回退 `Utility;`。
#[derive(Debug, Clone)]
pub struct AppCategory(pub &'static str);

impl AppCategory {
    /// 归一化后查表。认不出返回 `None`。
    pub fn parse(input: &str) -> Option<Self> {
        let mut s = input.to_ascii_lowercase();
        if let Some(rest) = s.strip_prefix("public.app-category.") {
            s = rest.to_string();
        }
        s = s.replace([' ', '-'], "");
        CATEGORIES
            .iter()
            .find(|(key, _)| *key == s.as_str())
            .map(|(_, v)| Self(v))
    }

    /// 映射到 freedesktop Categories（末尾带分号）
    pub fn gnome_desktop_categories(&self) -> &'static str {
        self.0
    }
}

/// `(归一化键, freedesktop Categories)`。键 = 小写、无空格无连字符。
/// 值取自 cargo-bundle category.rs:131-174 的 gnome_desktop_categories()。
const CATEGORIES: &[(&str, &str)] = &[
    ("business", "Office;"),
    ("developertool", "Development;"),
    ("education", "Education;"),
    ("entertainment", "Network;"),
    ("finance", "Office;Finance;"),
    ("game", "Game;"),
    ("actiongame", "Game;ActionGame;"),
    ("adventuregame", "Game;AdventureGame;"),
    ("arcadegame", "Game;ArcadeGame;"),
    ("boardgame", "Game;BoardGame;"),
    ("cardgame", "Game;CardGame;"),
    ("casinogame", "Game;"),
    ("dicegame", "Game;"),
    ("educationalgame", "Game;Education;"),
    ("familygame", "Game;"),
    ("kidsgame", "Game;KidsGame;"),
    ("musicgame", "Game;"),
    ("puzzlegame", "Game;LogicGame;"),
    ("racinggame", "Game;"),
    ("roleplayinggame", "Game;RolePlaying;"),
    ("simulationgame", "Game;Simulation;"),
    ("sportsgame", "Game;SportsGame;"),
    ("strategygame", "Game;StrategyGame;"),
    ("triviagame", "Game;"),
    ("wordgame", "Game;"),
    ("graphicsanddesign", "Graphics;"),
    ("healthcareandfitness", "Science;"),
    ("lifestyle", "Education;"),
    ("medical", "Science;MedicalSoftware;"),
    ("music", "AudioVideo;Audio;Music;"),
    ("news", "Network;News;"),
    ("photography", "Graphics;Photography;"),
    ("productivity", "Office;"),
    ("reference", "Education;"),
    ("socialnetworking", "Network;"),
    ("sports", "Education;Sports;"),
    ("travel", "Education;"),
    ("utility", "Utility;"),
    ("video", "AudioVideo;Video;"),
    ("weather", "Science;"),
];

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

/// 生成 FreeDesktop `.desktop` 条目内容。
///
/// 对齐 cargo-bundle 的 `render_desktop_entry`（linux/desktop.rs:38-144）：
/// 字段映射、输出顺序、以及几处刻意的省略都照它来 ——
/// - 不写 `Version=`（那是 spec 版本，不是 app 版本）
/// - 不写 `Encoding=`（已废弃）
/// - `MimeType` 为空则整行省略（裸 `MimeType=` 过不了某些校验器）
/// - `category` 未设时由调用方告警并回退 `Utility;`
///
/// `icon` 是**安装名**（hicolor `apps/<名字>.png`），与 `bin` 分开传：
/// `--icon-name` 可覆盖它，覆盖后 `Icon=` 必须跟着变，否则桌面环境
/// 按 bin 名找不到装好的图标。
///
/// `fallback_comment` 是 `Comment=` 的回退值（Cargo.toml 的
/// `[package].description`，由调用方读入）：`short_description` 缺省时用它，
/// 两者都空则整行省略（与 `MimeType` 同理）。
pub fn render_desktop_entry(
    meta: &BundleMeta,
    bin: &str,
    icon: &str,
    desktop_id: &str,
    fallback_comment: &str,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let app_name = meta.name.as_deref().unwrap_or(bin);
    let comment = meta
        .short_description
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_comment);
    // category 认不出时回退 Utility;（调用方已告警）
    let categories = meta
        .category
        .as_deref()
        .and_then(AppCategory::parse)
        .map(|c| c.gnome_desktop_categories().to_string())
        .unwrap_or_else(|| "Utility;".to_string());

    // spec: https://specifications.freedesktop.org/desktop-entry-spec/latest/
    let _ = writeln!(out, "[Desktop Entry]");
    let _ = writeln!(out, "Type=Application");
    let _ = writeln!(out, "Name={app_name}");
    if !comment.is_empty() {
        let _ = writeln!(out, "Comment={comment}");
    }
    let _ = writeln!(out, "Exec={bin}{}", meta.exec_suffix());
    let _ = writeln!(out, "Icon={icon}");
    let _ = writeln!(out, "Terminal={}", meta.terminal());
    let _ = writeln!(out, "Categories={categories}");
    // 与桌面身份一致：launcher 用它把窗口归到该条目下
    let _ = writeln!(out, "StartupWMClass={desktop_id}");

    let mime = meta.mime_types();
    if !mime.is_empty() {
        let _ = write!(out, "MimeType=");
        for m in mime {
            let _ = write!(out, "{m};");
        }
        let _ = writeln!(out);
    }

    // Desktop Actions：`Actions=` 列表 + 每组一个 `[Desktop Action <id>]`
    let actions = meta.desktop_actions();
    if !actions.is_empty() {
        let _ = write!(out, "Actions=");
        for (id, _) in actions {
            let _ = write!(out, "{id};");
        }
        let _ = writeln!(out);
        for (id, action) in actions {
            let _ = writeln!(out);
            let _ = writeln!(out, "[Desktop Action {id}]");
            let _ = writeln!(out, "Name={}", action.name);
            let exec = action.exec.as_deref().unwrap_or(bin);
            let _ = writeln!(out, "Exec={exec}");
            if let Some(icon) = &action.icon {
                let _ = writeln!(out, "Icon={icon}");
            }
        }
    }

    out
}

/// 图标在 hicolor 主题里的尺寸子目录。
///
/// 按 cargo-bundle / zed 的**文件名约定**：`@2x` 结尾 → 1024x1024，
/// 否则 512x512。不读图片真实尺寸——那要引入图片解码，且 cargo-bundle
/// 同样只认文件名约定。
pub fn icon_size_dir(icon_path: &str) -> &'static str {
    if icon_path.ends_with("@2x.png") || icon_path.ends_with("@2x") {
        "1024x1024"
    } else {
        "512x512"
    }
}

/// 图标安装名：命令行覆盖值 > 二进制名。
///
/// 图标文件名的 basename 是 `app-icon` 这类通用名，推不出应用名，所以
/// 回退 bin 名时**不静默**——打印提示，让使用者有机会发现并用
/// `--icon-name` 纠正。
pub fn resolve_icon_name(override_name: Option<&str>, bin: &str) -> String {
    match override_name {
        Some(s) => s.to_string(),
        None => {
            println!(
                "提示：未用 --icon-name 指定图标名，默认取二进制名 `{bin}`。\
                 若你的图标文件不叫这个名字，请用 --icon-name 指定。"
            );
            bin.to_string()
        }
    }
}

/// `.desktop` 文件名用的 identifier：`meta.identifier` > 推导
/// `<仓库名>.<bin 名>`（两段蛇形规范化）。推导不出（不在 git 仓库内）
/// 报错，不兜底假名。注意第二段用 bin 名而非 package 名——`[[bin]].name`
/// 可以与 `package.name` 不同。
pub fn resolve_desktop_id(
    meta: &BundleMeta,
    project_dir: &Path,
    bin: &str,
    meta_ref: &str,
) -> Result<String> {
    if let Some(id) = &meta.identifier {
        return Ok(id.clone());
    }
    let repo = repo_name_from(project_dir);
    repo.as_deref()
        .and_then(|r| derive_identifier(r, Some(bin)))
        .with_context(|| {
            format!(
                "bundle 元数据 {meta_ref} 里没有 identifier，且无法推导默认值：当前目录不在 git 仓库内\
                 （拿不到 .git，推导不出仓库名）。请在 bundle 元数据里补 identifier，\
                 或在 git 仓库中运行"
            )
        })
}

/// 类 envsubst 的 `$VAR` 替换（只处理传入的变量名，其他 `$xxx` 原样保留）。
pub fn render_template(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${key}"), value);
    }
    out
}

/// `.desktop` 条目（文件名 + 内容），各格式共用。
///
/// 内容来源优先级：
/// 1. `template_override`（命令行 `--desktop-template`，临时覆盖）
/// 2. `meta.desktop_template`（项目自有模板，相对 crate 目录）
/// 3. **aa-bundle 自己生成**（[`render_desktop_entry`]，对齐 cargo-bundle）——
///    项目零配置也能得到合规的 .desktop
///
/// `app_cli` 是模板里 `$APP_CLI` 的值，按格式的安装形态给：PKGBUILD 用绝对
/// 安装路径（`/usr/bin/<bin>`），tar.gz 用裸 bin 名（可重定位，解包位置由
/// 安装脚本决定，zed 同款）。自动生成路径的 `Exec=` 恒为裸 bin 名（靠 PATH）。
///
/// `fallback_comment` 是 `Comment=` 的回退值（Cargo.toml 的
/// `[package].description`）：`short_description` 缺省时用它，见
/// [`render_desktop_entry`]。
///
/// category 缺失或认不出时告警并回退 `Utility;`（cargo-bundle
/// desktop.rs:161-170 同款）。
// 参数多是「一个字段一个来源」的代价（身份/路径/模板/回退值各自独立），
// 拆参数结构体只会把调用方搞累；同名 &str 传串的风险由单测的行为断言兜底。
#[allow(clippy::too_many_arguments)]
pub fn resolve_desktop_entry(
    meta: &BundleMeta,
    bin: &str,
    icon: &str,
    app_cli: &str,
    project_dir: &Path,
    meta_ref: &str,
    template_override: Option<&Path>,
    fallback_comment: &str,
) -> Result<(String, String)> {
    let desktop_id = resolve_desktop_id(meta, project_dir, bin, meta_ref)?;
    let app_name = meta.name.as_deref().unwrap_or(bin);

    let template_path = match template_override {
        Some(p) => Some(p.to_path_buf()),
        None => meta
            .desktop_template
            .as_ref()
            .map(|rel| project_dir.join(rel)),
    };
    if let Some(template_path) = &template_path {
        let template = std::fs::read_to_string(template_path)
            .with_context(|| format!("读取 .desktop 模板 {} 失败", template_path.display()))?;
        let rendered = render_template(
            &template,
            &[
                ("APP_NAME", app_name),
                ("APP_ID", &desktop_id),
                ("APP_CLI", app_cli),
                ("APP_ICON", icon),
                ("APP_ARGS", "%F"),
                ("DO_STARTUP_NOTIFY", "true"),
            ],
        );
        return Ok((format!("{desktop_id}.desktop"), rendered));
    }

    // 自己生成。category 缺失或认不出时告警并回退 Utility;
    match meta.category.as_deref().map(AppCategory::parse) {
        Some(None) => println!(
            "提示：bundle 元数据的 category = {:?} 无法识别，.desktop 的 Categories \
             回退为 Utility;。可用值如 \"Developer Tool\" / \"Utility\" / \"Graphics\"。",
            meta.category.as_deref().unwrap_or("")
        ),
        None => println!(
            "提示：bundle 元数据里没有 category，.desktop 的 Categories 回退为 Utility;。\
             要更准确请设 category = \"Developer Tool\" 等。"
        ),
        Some(Some(_)) => {}
    }
    Ok((
        format!("{desktop_id}.desktop"),
        render_desktop_entry(meta, bin, icon, &desktop_id, fallback_comment),
    ))
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

/// cargo 的 `target_directory`：`cargo metadata --no-deps` 问出。
///
/// **workspace 成员的 target 由 workspace 根决定**，不在 crate 目录下——
/// 把产物/输出猜在 `<crate>/target/` 是多 crate workspace 下的经典错误。
/// 这里问 cargo 而不是拼路径，顺带尊重 `CARGO_TARGET_DIR`；metadata 拿不到
/// （cargo 不在 PATH 等）时退回 `<project_dir>/target`，由调用方的存在性
/// 检查兜底。`--no-deps` 只读 manifest，不解析依赖、不联网。
pub fn resolve_target_dir(project_dir: &Path) -> PathBuf {
    std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("target_directory")?.as_str().map(PathBuf::from))
        .unwrap_or_else(|| project_dir.join("target"))
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
        let dir =
            std::env::temp_dir().join(format!("aa-bundle-identity-{}-{}", std::process::id(), tag));
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
    fn linux_subtable_is_nested_not_flat() {
        let dir = identity_test_dir("linux_nested");
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
        let dir = identity_test_dir("layered");
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
        let dir = identity_test_dir("layered_empty");
        let refs: Vec<String> = Vec::new();
        let err = load_bundle_meta_layered(&refs, &dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("至少"), "空列表应报错：{err}");
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
        assert!(err.contains("空 key"), "应指出空 key：{err}");
    }
}
