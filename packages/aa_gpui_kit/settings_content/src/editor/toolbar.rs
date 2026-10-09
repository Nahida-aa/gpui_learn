use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

// Toolbar related settings
#[with_fallible_options]
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Eq)]
pub struct ToolbarContent {
    /// Whether to display breadcrumbs in the editor toolbar.
    ///
    /// Default: true
    pub breadcrumbs: Option<bool>,
    /// Whether to display quick action buttons in the editor toolbar.
    ///
    /// Default: true
    pub quick_actions: Option<bool>,
    /// Whether to show the selections menu in the editor toolbar.
    ///
    /// Default: true
    pub selections_menu: Option<bool>,
    /// Whether to display Agent review buttons in the editor toolbar.
    /// Only applicable while reviewing a file edited by the Agent.
    ///
    /// Default: true
    pub agent_review: Option<bool>,
    /// Whether to display code action buttons in the editor toolbar.
    ///
    /// Default: false
    pub code_actions: Option<bool>,
}
