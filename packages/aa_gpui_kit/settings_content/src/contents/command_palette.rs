use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, PartialEq)]
pub struct CommandPaletteSettingsContent {
    /// Whether to use command history ranking for sorting in the command palette.
    ///
    /// Default: true
    pub use_command_history: Option<bool>,
}
