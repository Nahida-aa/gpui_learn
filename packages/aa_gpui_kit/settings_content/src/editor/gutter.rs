use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

/// Gutter related settings
#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq)]
pub struct GutterContent {
    /// Whether to show line numbers in the gutter.
    ///
    /// Default: true
    pub line_numbers: Option<bool>,
    /// Minimum number of characters to reserve space for in the gutter.
    ///
    /// Default: 4
    pub min_line_number_digits: Option<usize>,
    /// Whether to show runnable buttons in the gutter.
    ///
    /// Default: true
    pub runnables: Option<bool>,
    /// Whether to show breakpoints in the gutter.
    ///
    /// Default: true
    pub breakpoints: Option<bool>,
    /// Whether to show bookmarks in the gutter.
    ///
    /// Default: true
    pub bookmarks: Option<bool>,
    /// Whether to show fold buttons in the gutter.
    ///
    /// Default: true
    pub folds: Option<bool>,
    /// The width of the git diff hunk indicators in the gutter.
    /// Use "default" to scale with the buffer font size, or {"custom": <pixels>} for a fixed width.
    ///
    /// Default: "default"
    pub git_gutter_width: Option<GitGutterWidth>,
}

/// Controls the width of the git diff hunk indicators in the gutter.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    strum::EnumDiscriminants,
)]
#[strum_discriminants(derive(strum::VariantArray, strum::VariantNames, strum::FromRepr))]
#[serde(rename_all = "snake_case")]
pub enum GitGutterWidth {
    /// Width scales automatically with the buffer font size.
    #[default]
    Default,
    /// A fixed pixel width for the git diff indicators.
    Custom(crate::PixelSetting),
}
