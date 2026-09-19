//! 图标主题 JSON 的 serde 结构,对齐 zed `crates/theme/src/icon_theme_schema.rs`。
//!
//! 一个大致的文件形如(zed 图标主题扩展的 `themes/*.json`):
//!
//! ```json
//! {
//!   "name": "My Icons",
//!   "author": "...",
//!   "themes": [
//!     {
//!       "name": "My Icons Dark",
//!       "appearance": "dark",
//!       "directory_icons": { "collapsed": "...", "expanded": "..." },
//!       "named_directory_icons": { "src": { ... } },
//!       "chevron_icons": { "collapsed": "...", "expanded": "..." },
//!       "file_stems": { "Dockerfile": "docker" },
//!       "file_suffixes": { "rs": "rust" },
//!       "file_icons": { "rust": { "path": "icons/rust.svg" } }
//!     }
//!   ]
//! }
//! ```
//!
//! 与「配色主题」的 content 结构(`theme-settings` 包的 `content`)的分工:
//! 那个描述**配色**主题,这个描述**图标**主题。两者共用 [`AppearanceContent`]。
//!
//! 与 zed 的差异:同 [`schema`](crate::schema),未引入 `schemars`
//! (JSON Schema 生成),故 `JsonSchema` derive 从略。

use std::collections::HashMap;

use gpui::SharedString;
use serde::Deserialize;

use crate::schema::AppearanceContent;

/// 一个图标主题家族文件(= 一个图标扩展的 `themes/*.json`)。
#[derive(Debug, Clone, Deserialize)]
pub struct IconThemeFamilyContent {
    pub name: String,
    pub author: String,
    pub themes: Vec<IconThemeContent>,
}

/// 家族内的单个图标主题。
#[derive(Debug, Clone, Deserialize)]
pub struct IconThemeContent {
    /// 展示名,同时作为图标主题 id。
    pub name: String,
    pub appearance: AppearanceContent,
    #[serde(default)]
    pub directory_icons: DirectoryIconsContent,
    /// 按目录名指定的图标(如 `src` / `node_modules` 用不同图标)。
    #[serde(default)]
    pub named_directory_icons: HashMap<String, DirectoryIconsContent>,
    #[serde(default)]
    pub chevron_icons: ChevronIconsContent,
    /// 完整文件名 → 图标 key(`Dockerfile` → `docker`)。
    #[serde(default)]
    pub file_stems: HashMap<String, String>,
    /// 扩展名 → 图标 key(`rs` → `rust`)。
    #[serde(default)]
    pub file_suffixes: HashMap<String, String>,
    /// 图标 key → 图标文件路径。
    #[serde(default)]
    pub file_icons: HashMap<String, IconDefinitionContent>,
}

/// 目录图标(展开/折叠两态)。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DirectoryIconsContent {
    pub collapsed: Option<SharedString>,
    pub expanded: Option<SharedString>,
}

/// 折叠箭头图标(展开/折叠两态)。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChevronIconsContent {
    pub collapsed: Option<SharedString>,
    pub expanded: Option<SharedString>,
}

/// 一个图标定义(目前只有文件路径)。
#[derive(Debug, Clone, Deserialize)]
pub struct IconDefinitionContent {
    pub path: SharedString,
}
