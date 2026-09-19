//! theme/content：zed 主题扩展 JSON（schema v0.2.0）的 serde 结构。
//!
//! 源文件形如（catppuccin/zed 等主题扩展）：
//!
//! ```json
//! {
//!   "name": "Catppuccin",
//!   "author": "...",
//!   "themes": [
//!     {
//!       "name": "Catppuccin Macchiato",
//!       "appearance": "dark",
//!       "style": {
//!         "accents": ["#8839ef", ...],
//!         "border": "#363a4f",
//!         "border.variant": "#313148",
//!         "editor.background": "#24273a",
//!         "text": "#cad3f5",
//!         "syntax": { "keyword": {...}, ... },
//!         "vim.*", "terminal.*", "players", ...
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! `style` 是**扁平 dotted-key**（`组.字段`）加个别嵌套对象（`syntax`）。
//! 我们只关心 `syntax` 与已知的 UI 语义色 key，其余（`vim.*` / `terminal.*` /
//! `players` / `background.appearance`…）一律进 [`StyleContent::colors`]
//! 的散集，由 [`loaders`] 里的映射表挑拣，未识别的直接忽略。

use std::collections::BTreeMap;

use serde::Deserialize;

/// 一个主题家族文件（= 一个主题扩展的 `themes/*.json`）。
#[derive(Debug, Deserialize)]
pub struct ThemeFamilyContent {
    pub name: String,
    #[allow(dead_code)]
    pub author: String,
    pub themes: Vec<ThemeContent>,
}

/// 家族内的单个主题。
#[derive(Debug, Deserialize)]
pub struct ThemeContent {
    /// 展示名（如 `Catppuccin Macchiato`），同时作为主题 id。
    pub name: String,
    pub appearance: AppearanceContent,
    pub style: StyleContent,
}

/// 明暗形态（对齐 zed `theme::AppearanceContent`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceContent {
    Light,
    Dark,
}

/// 扁平 `style` 映射。`syntax` 是唯一要单独抽出来的嵌套对象，
/// 其余全部收集进 [`StyleContent::colors`] 让加载器逐个挑拣。
#[derive(Debug, Default, Deserialize)]
pub struct StyleContent {
    #[serde(default)]
    pub syntax: BTreeMap<String, SyntaxContent>,
    /// 非 `syntax` 的其余扁平 key（含 `accents` / `background.appearance` 和
    /// 大量用不到的 `vim.*` / `terminal.*` 等）。
    #[serde(flatten)]
    pub colors: BTreeMap<String, serde_json::Value>,
}

/// 单个语法捕获（`style.syntax` 的值）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyntaxContent {
    pub color: Option<String>,
    pub font_style: Option<String>,
    /// zed 方案里 `font_weight` 既可能是 `"bold"` / `"normal"` 字符串，
    /// 也可能是数字权重（如 `700`），统一用 `Value` 交给加载器判断。
    #[serde(default)]
    pub font_weight: Option<serde_json::Value>,
}
