use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::DelayMs;

/// Whether to allow drag and drop text selection in buffer.
#[with_fallible_options]
#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Eq)]
pub struct DragAndDropSelectionContent {
    /// When true, enables drag and drop text selection in buffer.
    ///
    /// Default: true
    pub enabled: Option<bool>,

    /// The delay in milliseconds that must elapse before drag and drop is allowed. Otherwise, a new text selection is created.
    ///
    /// Default: 300
    pub delay: Option<DelayMs>,
}
