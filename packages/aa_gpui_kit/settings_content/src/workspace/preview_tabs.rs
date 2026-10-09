use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct PreviewTabsSettingsContent {
    /// Whether to show opened editors as preview tabs.
    /// Preview tabs do not stay open, are reused until explicitly set to be kept open opened (via double-click or editing) and show file names in italic.
    ///
    /// Default: true
    pub enabled: Option<bool>,
    /// Whether to open tabs in preview mode when opened from the project panel
    /// with a single click or the `project_panel::Open` action.
    ///
    /// Default: true
    pub enable_preview_from_project_panel: Option<bool>,
    /// Whether to open tabs in preview mode when selected from the file finder.
    ///
    /// Default: false
    pub enable_preview_from_file_finder: Option<bool>,
    /// Whether to open tabs in preview mode when opened from a multibuffer.
    ///
    /// Default: true
    pub enable_preview_from_multibuffer: Option<bool>,
    /// Whether to open tabs in preview mode when code navigation is used to open a multibuffer.
    ///
    /// Default: false
    pub enable_preview_multibuffer_from_code_navigation: Option<bool>,
    /// Whether to open tabs in preview mode when code navigation is used to open a single file.
    ///
    /// Default: true
    pub enable_preview_file_from_code_navigation: Option<bool>,
    /// Whether to keep tabs in preview mode when code navigation is used to navigate away from them.
    /// If `enable_preview_file_from_code_navigation` or `enable_preview_multibuffer_from_code_navigation` is also true, the new tab may replace the existing one.
    ///
    /// Default: false
    pub enable_keep_preview_on_code_navigation: Option<bool>,
}
