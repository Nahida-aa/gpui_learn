//! 完整主题文件的反序列化目标 + 相关辅助类型。
//!
//! - `ThemeStyleContent` 是主题 JSON 的顶层结构体（`flatten` colors + status + syntax + accents + players）
//! - `WindowBackgroundContent` 窗口背景外观
//! - `AccentContent` accent 颜色条目
//! - `PlayerColorContent` 多人光标颜色

use collections::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use settings_macros::{MergeFrom, with_fallible_options};

use super::colors::{ThemeColor, ThemeColorsContent};
use super::font::{FontStyleContent, FontWeightContent};

/// 完整主题文件的反序列化目标。
///
/// 用于从主题 JSON 文件一次性解析出所有颜色 + 状态 + syntax + accent + players。
/// `colors` 和 `status` 用 `#[serde(flatten)]` 内联，使 JSON 结构扁平（所有 `border` / `editor.*` / `conflict` 等都在顶层）。
#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, MergeFrom, JsonSchema)]
#[serde(default)]
pub struct ThemeStyleContent {
    #[serde(rename = "background.appearance")]
    pub window_background_appearance: Option<WindowBackgroundContent>,

    #[serde(default)]
    pub accents: Vec<AccentContent>,

    #[serde(flatten, default)]
    pub colors: ThemeColorsContent,

    #[serde(flatten, default)]
    pub status: StatusColorsContent,

    #[serde(default)]
    pub players: Vec<PlayerColorContent>,

    /// 语法高亮。
    #[serde(default)]
    pub syntax: IndexMap<String, HighlightStyleContent>,
}

#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, MergeFrom)]
#[serde(default)]
pub struct StatusColorsContent {
    #[serde(rename = "conflict")]
    pub conflict: Option<ThemeColor>,
    #[serde(rename = "conflict.background")]
    pub conflict_background: Option<ThemeColor>,
    #[serde(rename = "conflict.border")]
    pub conflict_border: Option<ThemeColor>,

    #[serde(rename = "created")]
    pub created: Option<ThemeColor>,
    #[serde(rename = "created.background")]
    pub created_background: Option<ThemeColor>,
    #[serde(rename = "created.border")]
    pub created_border: Option<ThemeColor>,

    #[serde(rename = "deleted")]
    pub deleted: Option<ThemeColor>,
    #[serde(rename = "deleted.background")]
    pub deleted_background: Option<ThemeColor>,
    #[serde(rename = "deleted.border")]
    pub deleted_border: Option<ThemeColor>,

    #[serde(rename = "error")]
    pub error: Option<ThemeColor>,
    #[serde(rename = "error.background")]
    pub error_background: Option<ThemeColor>,
    #[serde(rename = "error.border")]
    pub error_border: Option<ThemeColor>,

    #[serde(rename = "hidden")]
    pub hidden: Option<ThemeColor>,
    #[serde(rename = "hidden.background")]
    pub hidden_background: Option<ThemeColor>,
    #[serde(rename = "hidden.border")]
    pub hidden_border: Option<ThemeColor>,

    #[serde(rename = "hint")]
    pub hint: Option<ThemeColor>,
    #[serde(rename = "hint.background")]
    pub hint_background: Option<ThemeColor>,
    #[serde(rename = "hint.border")]
    pub hint_border: Option<ThemeColor>,

    #[serde(rename = "ignored")]
    pub ignored: Option<ThemeColor>,
    #[serde(rename = "ignored.background")]
    pub ignored_background: Option<ThemeColor>,
    #[serde(rename = "ignored.border")]
    pub ignored_border: Option<ThemeColor>,

    #[serde(rename = "info")]
    pub info: Option<ThemeColor>,
    #[serde(rename = "info.background")]
    pub info_background: Option<ThemeColor>,
    #[serde(rename = "info.border")]
    pub info_border: Option<ThemeColor>,

    #[serde(rename = "modified")]
    pub modified: Option<ThemeColor>,
    #[serde(rename = "modified.background")]
    pub modified_background: Option<ThemeColor>,
    #[serde(rename = "modified.border")]
    pub modified_border: Option<ThemeColor>,

    #[serde(rename = "predictive")]
    pub predictive: Option<ThemeColor>,
    #[serde(rename = "predictive.background")]
    pub predictive_background: Option<ThemeColor>,
    #[serde(rename = "predictive.border")]
    pub predictive_border: Option<ThemeColor>,

    #[serde(rename = "renamed")]
    pub renamed: Option<ThemeColor>,
    #[serde(rename = "renamed.background")]
    pub renamed_background: Option<ThemeColor>,
    #[serde(rename = "renamed.border")]
    pub renamed_border: Option<ThemeColor>,

    #[serde(rename = "success")]
    pub success: Option<ThemeColor>,
    #[serde(rename = "success.background")]
    pub success_background: Option<ThemeColor>,
    #[serde(rename = "success.border")]
    pub success_border: Option<ThemeColor>,

    #[serde(rename = "unreachable")]
    pub unreachable: Option<ThemeColor>,
    #[serde(rename = "unreachable.background")]
    pub unreachable_background: Option<ThemeColor>,
    #[serde(rename = "unreachable.border")]
    pub unreachable_border: Option<ThemeColor>,

    #[serde(rename = "warning")]
    pub warning: Option<ThemeColor>,
    #[serde(rename = "warning.background")]
    pub warning_background: Option<ThemeColor>,
    #[serde(rename = "warning.border")]
    pub warning_border: Option<ThemeColor>,
}

/// 语法高亮样式（颜色 + 可选字体样式/粗细）。
///
/// `#[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::fallible_options::deserialize")]`
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, MergeFrom)]
#[serde(default)]
pub struct HighlightStyleContent {
    pub color: Option<ThemeColor>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "treat_error_as_none"
    )]
    pub background_color: Option<ThemeColor>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "treat_error_as_none"
    )]
    pub font_style: Option<FontStyleContent>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "treat_error_as_none"
    )]
    pub font_weight: Option<FontWeightContent>,
}

impl HighlightStyleContent {
    pub fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.background_color.is_none()
            && self.font_style.is_none()
            && self.font_weight.is_none()
    }
}
fn treat_error_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}
/// The background appearance of the window.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(rename_all = "snake_case")]
pub enum WindowBackgroundContent {
    Opaque,
    Transparent,
    Blurred,
}

/// accent 颜色条目（可空）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct AccentContent(pub Option<ThemeColor>);

/// 多人光标颜色。
#[with_fallible_options]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct PlayerColorContent {
    pub cursor: Option<ThemeColor>,
    pub background: Option<ThemeColor>,
    pub selection: Option<ThemeColor>,
}
