//! Theme 相关的 settings 内容类型。
//!
//! 对齐 Zed `crates/settings_content/src/theme.rs`。
//!
//! 这里放**从 JSON 反序列化的数据结构**（settings_content 层），
//! 运行时主题类型（`Theme`, `ThemeStyles`, `ActiveTheme` ...）在
//! `aa-gpui-kit-theme` crate（gpui_learn）。
//!
//! ## 子模块
//!
//! 依赖方向（由下至上）：
//! ```text
//! font.rs ←───── highlight_style.rs ───┐
//!                                      ↓
//! theme_color.rs (ThemeColor + Colors) ←── theme_style.rs
//!                                      ↑
//!               status_colors.rs ─────┘
//! ```
//!
//! - [`font`] — FontSize / FontFamilyName / FontWeightContent / BufferLineHeight / FontStyleContent
//! - [`theme_color`] — ThemeColor / ThemeColorsContent（最底层，无内部依赖）
//! - [`theme_selection`] — ThemeName / ThemeSelection / ThemeAppearanceMode / UiDensity + DEFAULT_*_THEME
//! - [`highlight_style`] — HighlightStyleContent（依赖 theme_color + font）
//! - [`status_colors`] — StatusColorsContent（只依赖 ThemeColor）
//! - [`theme_style`] — ThemeStyleContent（聚合顶层入口，依赖上面所有）
//!
//! `ThemeSettingsContent` 聚合所有子模块的类型，作为用户 settings.json 的入口。

pub mod buffer_line_height;
pub mod colors;
pub mod font;
pub mod theme_selection;
pub mod theme_style;
pub mod ui_density;
use crate::merge_from::MergeFrom as MergeFromTrait;
// ---------- 常量 re-export ----------

use collections::HashMap;
use schemars::JsonSchema;
pub use theme_selection::{DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME};

// ---------- 子类型 re-export ----------
pub use buffer_line_height::{BufferLineHeight, BufferLineHeightDiscriminants};
pub use font::{FontFamilyName, FontSize, FontStyleContent, FontWeightContent};

pub use colors::{ThemeColor, ThemeColorsContent};

pub use theme_selection::{
    IconThemeName, IconThemeSelection, IconThemeSelectionDiscriminants, ThemeAppearanceMode,
    ThemeName, ThemeSelection, ThemeSelectionDiscriminants,
};
pub use ui_density::UiDensity;

pub use theme_style::{
    AccentContent, HighlightStyleContent, PlayerColorContent, StatusColorsContent,
    ThemeStyleContent, WindowBackgroundContent,
};

use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

pub use crate::theme::font::{CodeFade, FontFeaturesContent};

// ---------- ThemeSettingsContent ----------

/// Theme 相关的所有 settings 字段。
///
/// 对齐 Zed `settings_content::ThemeSettingsContent` 的核心子集。
/// Zed 还包含 FontFeatures / TextLayout 等，按需添加。
///
/// 所有字段都是 `Option<_>`，None 表示用默认值
/// （由 `theme-settings` crate 的 Default impl 或 gpui_learn ThemeRegistry 提供）。
#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(default)]
pub struct ThemeSettingsContent {
    /// UI 字体大小。
    pub ui_font_size: Option<FontSize>,
    /// UI 字体族。
    pub ui_font_family: Option<FontFamilyName>,
    #[schemars(default = "default_font_fallbacks")]
    #[schemars(extend("uniqueItems" = true))]
    pub ui_font_fallbacks: Option<Vec<FontFamilyName>>,
    /// The OpenType features to enable for text in the UI.
    #[schemars(default = "default_font_features")]
    pub ui_font_features: Option<FontFeaturesContent>,
    /// UI 字体粗细（100-900）。
    #[schemars(default = "default_buffer_font_weight")]
    pub ui_font_weight: Option<FontWeightContent>,

    /// 编辑器字体族。
    pub buffer_font_family: Option<FontFamilyName>,
    /// The font fallbacks to use for rendering in text buffers.
    #[schemars(extend("uniqueItems" = true))]
    pub buffer_font_fallbacks: Option<Vec<FontFamilyName>>,
    /// 编辑器字体大小。
    pub buffer_font_size: Option<FontSize>,
    /// 编辑器字体粗细（100-900）。
    #[schemars(default = "default_buffer_font_weight")]
    pub buffer_font_weight: Option<FontWeightContent>,
    /// 编辑器行高。
    pub buffer_line_height: Option<BufferLineHeight>,
    /// The OpenType features to enable for rendering in text buffers.
    #[schemars(default = "default_font_features")]
    pub buffer_font_features: Option<FontFeaturesContent>,

    /// The name of a font to use for agent responses in the agent panel. Falls back to the UI font if unset.
    pub agent_ui_font_family: Option<FontFamilyName>,
    /// The font size for agent responses in the agent panel. Falls back to the UI font size if unset.
    pub agent_ui_font_size: Option<FontSize>,
    /// The name of a font to use for user messages in the agent panel. Falls back to the buffer font if unset.
    pub agent_buffer_font_family: Option<FontFamilyName>,
    /// The font size for user messages in the agent panel.
    pub agent_buffer_font_size: Option<FontSize>,
    pub git_commit_buffer_font_size: Option<FontSize>,

    /// 主题选择。
    pub theme: Option<ThemeSelection>,
    /// 图标主题选择。
    pub icon_theme: Option<IconThemeSelection>,

    /// UI 密度（experimental）。
    #[serde(rename = "unstable.ui_density")]
    pub ui_density: Option<UiDensity>,

    #[schemars(range(min = 0.0, max = 0.9))]
    pub unnecessary_code_fade: Option<CodeFade>,

    /// EXPERIMENTAL: Overrides for the current theme.
    ///
    /// These values will override the ones on the current theme specified in `theme`.
    #[serde(rename = "experimental.theme_overrides")]
    pub experimental_theme_overrides: Option<ThemeStyleContent>,

    /// Overrides per theme
    ///
    /// These values will override the ones on the specified theme
    #[serde(default)]
    pub theme_overrides: HashMap<String, ThemeStyleContent>,
}

fn default_font_features() -> Option<FontFeaturesContent> { Some(FontFeaturesContent::default()) }

fn default_font_fallbacks() -> Option<Vec<FontFamilyName>> { Some(Vec::new()) }

fn default_buffer_font_weight() -> Option<FontWeightContent> { Some(FontWeightContent::NORMAL) }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_theme_selection_static() {
        let s: ThemeSelection = serde_json::from_value(json!("One Dark")).unwrap();
        assert!(matches!(s, ThemeSelection::Static(_)));
    }

    #[test]
    fn test_theme_selection_dynamic() {
        let s: ThemeSelection = serde_json::from_value(json!({
            "mode": "system",
            "light": "One Light",
            "dark": "One Dark"
        }))
        .unwrap();
        assert!(matches!(s, ThemeSelection::Dynamic { .. }));
    }

    #[test]
    fn test_theme_settings_default() {
        let s: ThemeSettingsContent = serde_json::from_value(json!({})).unwrap();
        assert!(s.theme.is_none());
        assert!(s.buffer_font_size.is_none());
    }

    #[test]
    fn test_theme_color_hex() {
        let s: ThemeColor = serde_json::from_value(json!("#ff5733")).unwrap();
        assert_eq!(s.as_str(), "#ff5733");
    }

    #[test]
    fn test_theme_colors_content_minimal() {
        let s: ThemeColorsContent = serde_json::from_value(json!({
            "background": "#1e1e1e",
            "editor.background": "#2d2d2d",
            "editor.foreground": "#d4d4d4"
        }))
        .unwrap();
        assert_eq!(s.background.as_deref(), Some("#1e1e1e"));
        assert_eq!(s.editor_background.as_deref(), Some("#2d2d2d"));
        assert_eq!(s.editor_foreground.as_deref(), Some("#d4d4d4"));
    }

    #[test]
    fn test_theme_style_content_flatten() {
        let s: ThemeStyleContent = serde_json::from_value(json!({
            "background": "#1e1e1e",
            "error": "#f44747",
            "error.background": "rgba(244,71,71,0.15)",
            "syntax": {
                "keyword": { "color": "#569cd6" },
                "string": { "color": "#ce9178" }
            }
        }))
        .unwrap();
        // colors 部分（flatten 进来）
        assert_eq!(s.colors.background.as_deref(), Some("#1e1e1e"));
        // status 部分（flatten 进来）
        assert_eq!(s.status.error.as_deref(), Some("#f44747"));
        assert_eq!(
            s.status.error_background.as_deref(),
            Some("rgba(244,71,71,0.15)")
        );
        // syntax
        assert_eq!(s.syntax.len(), 2);
        assert_eq!(
            s.syntax.get("keyword").unwrap().color.as_deref(),
            Some("#569cd6")
        );
    }
}
