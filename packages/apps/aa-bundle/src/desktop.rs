//! `.desktop` 条目渲染：各格式共用的桌面集成基建。
//!
//! 内容生成对齐 cargo-bundle 的 `linux/desktop.rs`（字段映射、输出顺序、
//! 几处刻意的省略），类别表照抄其 `category.rs` 的 macOS → freedesktop 映射。

use std::path::Path;

use anyhow::{Context, Result};

use crate::identity::{derive_identifier, repo_name_from};
use crate::meta::BundleMeta;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_category_parsing_tolerates_case_spaces_and_prefix() {
        // 大小写 / 空格 / 连字符不敏感；public.app-category. 前缀剥离
        assert_eq!(
            AppCategory::parse("Developer Tool").map(|c| c.0),
            Some("Development;")
        );
        assert_eq!(
            AppCategory::parse("developer-tool").map(|c| c.0),
            Some("Development;")
        );
        assert_eq!(
            AppCategory::parse("public.app-category.utility").map(|c| c.0),
            Some("Utility;")
        );
        // 认不出 → None
        assert!(
            AppCategory::parse("不是类别").is_none(),
            "认不出的类别应返回 None"
        );
    }

    #[test]
    fn desktop_rendering_substitutes_only_known_vars() {
        let out = render_template(
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
}
