use std::num;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use super::display::CurrentLineHighlight;

/// Minimap related settings
#[with_fallible_options]
#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq)]
pub struct MinimapContent {
    /// When to show the minimap in the editor.
    ///
    /// Default: never
    pub show: Option<ShowMinimap>,

    /// Where to show the minimap in the editor.
    ///
    /// Default: [`DisplayIn::ActiveEditor`]
    pub display_in: Option<DisplayIn>,

    /// When to show the minimap thumb.
    ///
    /// Default: always
    pub thumb: Option<MinimapThumb>,

    /// Defines the border style for the minimap's scrollbar thumb.
    ///
    /// Default: left_open
    pub thumb_border: Option<MinimapThumbBorder>,

    /// How to highlight the current line in the minimap.
    ///
    /// Default: inherits editor line highlights setting
    pub current_line_highlight: Option<CurrentLineHighlight>,

    /// Maximum number of columns to display in the minimap.
    ///
    /// Default: 80
    pub max_width_columns: Option<num::NonZeroU32>,
}

/// When to show the minimap in the editor.
///
/// Default: never
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ShowMinimap {
    /// Follow the visibility of the scrollbar.
    Auto,
    /// Always show the minimap.
    Always,
    /// Never show the minimap.
    #[default]
    Never,
}

/// Where to show the minimap in the editor.
///
/// Default: all_editors
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DisplayIn {
    /// Show on all open editors.
    AllEditors,
    /// Show the minimap on the active editor only.
    #[default]
    ActiveEditor,
}

/// When to show the minimap thumb.
///
/// Default: always
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum MinimapThumb {
    /// Show the minimap thumb only when the mouse is hovering over the minimap.
    Hover,
    /// Always show the minimap thumb.
    #[default]
    Always,
}

/// Defines the border style for the minimap's scrollbar thumb.
///
/// Default: left_open
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum MinimapThumbBorder {
    /// Displays a border on all sides of the thumb.
    Full,
    /// Displays a border on all sides except the left side of the thumb.
    #[default]
    LeftOpen,
    /// Displays a border on all sides except the right side of the thumb.
    RightOpen,
    /// Displays a border only on the left side of the thumb.
    LeftOnly,
    /// Displays the thumb without any border.
    None,
}
