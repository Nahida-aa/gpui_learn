//! 主题选择相关 settings 类型。
//!
//! 对齐 Zed `settings_content::theme` 中的选择部分。

use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

// ---------- 常量 ----------

pub const DEFAULT_LIGHT_THEME: &str = "Catppuccin Latte";
pub const DEFAULT_DARK_THEME: &str = "Catppuccin Macchiato";

// ---------- ThemeName ----------

/// Theme 名称（transparent newtype，包 `Arc<str>`）。
/// #[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, MergeFrom, JsonSchema)]
#[serde(transparent)]
pub struct ThemeName(pub Arc<str>);

impl From<String> for ThemeName {
    fn from(s: String) -> Self { Self(Arc::from(s)) }
}

impl From<&str> for ThemeName {
    fn from(s: &str) -> Self { Self(Arc::from(s)) }
}

// ---------- IconThemeName ----------

/// Icon theme 名称。
#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, MergeFrom, JsonSchema)]
#[serde(transparent)]
pub struct IconThemeName(pub Arc<str>);

impl From<String> for IconThemeName {
    fn from(s: String) -> Self { Self(Arc::from(s)) }
}

impl From<&str> for IconThemeName {
    fn from(s: &str) -> Self { Self(Arc::from(s)) }
}

// ---------- ThemeAppearanceMode ----------

/// 选择主题时用的模式。
///
/// - `Light` / `Dark` — 固定选对应主题
/// - `System` — 跟随系统明暗
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    MergeFrom,
    JsonSchema,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAppearanceMode {
    Light,
    Dark,
    #[default]
    System,
}

// ---------- ThemeSelection ----------

/// 主题选择 — 可以是静态（单个主题）或动态（按明暗切换）。
///
/// JSON 格式：
/// - 静态：`"theme": "One Dark"`
/// - 动态：`"theme": { "mode": "system", "light": "One Light", "dark": "One Dark" }`
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    MergeFrom,
    strum::EnumDiscriminants,
    JsonSchema,
)]
#[strum_discriminants(derive(strum::VariantArray, strum::VariantNames, strum::FromRepr))]
#[serde(untagged)]
pub enum ThemeSelection {
    Static(ThemeName),
    Dynamic {
        #[serde(default)]
        mode: ThemeAppearanceMode,
        light: ThemeName,
        dark: ThemeName,
    },
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self::Dynamic {
            mode: ThemeAppearanceMode::default(),
            light: ThemeName::from(DEFAULT_LIGHT_THEME),
            dark: ThemeName::from(DEFAULT_DARK_THEME),
        }
    }
}

/// 图标主题选择 — 同 ThemeSelection 形状。
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    MergeFrom,
    strum::EnumDiscriminants,
    JsonSchema,
)]
#[strum_discriminants(derive(strum::VariantArray, strum::VariantNames, strum::FromRepr))]
#[serde(untagged)]
pub enum IconThemeSelection {
    Static(IconThemeName),
    Dynamic {
        #[serde(default)]
        mode: ThemeAppearanceMode,
        light: IconThemeName,
        dark: IconThemeName,
    },
}

impl Default for IconThemeSelection {
    fn default() -> Self { Self::Static(IconThemeName::default()) }
}
