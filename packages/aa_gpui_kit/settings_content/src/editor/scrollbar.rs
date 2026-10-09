use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::ShowScrollbar;

/// Scrollbar related settings
#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Default)]
pub struct ScrollbarContent {
    /// When to show the scrollbar in the editor.
    ///
    /// Default: auto
    pub show: Option<ShowScrollbar>,
    /// Whether to show git diff indicators in the scrollbar.
    ///
    /// Default: true
    pub git_diff: Option<bool>,
    /// Whether to show buffer search result indicators in the scrollbar.
    ///
    /// Default: true
    pub search_results: Option<bool>,
    /// Whether to show selected text occurrences in the scrollbar.
    ///
    /// Default: true
    pub selected_text: Option<bool>,
    /// Whether to show selected symbol occurrences in the scrollbar.
    ///
    /// Default: true
    pub selected_symbol: Option<bool>,
    /// Which diagnostic indicators to show in the scrollbar:
    ///
    /// Default: all
    pub diagnostics: Option<ScrollbarDiagnostics>,
    /// Whether to show cursor positions in the scrollbar.
    ///
    /// Default: true
    pub cursors: Option<bool>,
    /// Forcefully enable or disable the scrollbar for each axis
    pub axes: Option<ScrollbarAxesContent>,
}

/// Which diagnostic indicators to show in the scrollbar.
///
/// Default: all
#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "lowercase")]
pub enum ScrollbarDiagnostics {
    /// Show all diagnostic levels: hint, information, warnings, error.
    All,
    /// Show only the following diagnostic levels: information, warning, error.
    Information,
    /// Show only the following diagnostic levels: warning, error.
    Warning,
    /// Show only the following diagnostic level: error.
    Error,
    /// Do not show diagnostics.
    None,
}

/// Forcefully enable or disable the scrollbar for each axis
#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Default)]
pub struct ScrollbarAxesContent {
    /// When false, forcefully disables the horizontal scrollbar. Otherwise, obey other settings.
    ///
    /// Default: true
    pub horizontal: Option<bool>,

    /// When false, forcefully disables the vertical scrollbar. Otherwise, obey other settings.
    ///
    /// Default: true
    pub vertical: Option<bool>,
}
