use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, PartialEq)]
pub struct TabBarSettingsContent {
    /// Whether or not to show the tab bar in the editor.
    ///
    /// Default: true
    pub show: Option<bool>,
    /// Whether or not to show the navigation history buttons in the tab bar.
    ///
    /// Default: true
    pub show_nav_history_buttons: Option<bool>,
    /// Whether or not to show the tab bar buttons.
    ///
    /// Default: true
    pub show_tab_bar_buttons: Option<bool>,
    /// Whether or not to show pinned tabs in a separate row.
    /// When enabled, pinned tabs appear in a top row and unpinned tabs in a bottom row.
    ///
    /// Default: false
    pub show_pinned_tabs_in_separate_row: Option<bool>,
}

#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, PartialEq, Eq)]
pub struct StatusBarSettingsContent {
    /// Whether to show the status bar.
    ///
    /// Default: true
    #[serde(rename = "experimental.show")]
    pub show: Option<bool>,
    /// Whether to show the name of the active file in the status bar.
    ///
    /// Default: false
    pub show_active_file: Option<bool>,
    /// Whether to display the active language button in the status bar.
    ///
    /// Default: true
    pub active_language_button: Option<bool>,
    /// Whether to show the cursor position button in the status bar.
    ///
    /// Default: true
    pub cursor_position_button: Option<bool>,
    /// Whether to show active line endings button in the status bar.
    ///
    /// Default: false
    pub line_endings_button: Option<bool>,
    /// Whether to show the active encoding button in the status bar.
    ///
    /// Default: non_utf8
    pub active_encoding_button: Option<EncodingDisplayOptions>,
    /// Whether to show an indicator with a countdown while timed multi-stroke input is pending.
    /// Hovering the indicator pauses the timeout.
    /// Its binding preview popover is disabled when the which-key popup is enabled.
    ///
    /// Default: true
    pub pending_keystrokes_indicator: Option<bool>,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantNames,
    strum::VariantArray,
)]
#[serde(rename_all = "snake_case")]
pub enum EncodingDisplayOptions {
    Enabled,
    Disabled,
    #[default]
    NonUtf8,
}
impl EncodingDisplayOptions {
    pub fn should_show(&self, is_utf8: bool, has_bom: bool) -> bool {
        match self {
            Self::Disabled => false,
            Self::Enabled => true,
            Self::NonUtf8 => {
                let is_standard_utf8 = is_utf8 && !has_bom;
                !is_standard_utf8
            }
        }
    }
}
