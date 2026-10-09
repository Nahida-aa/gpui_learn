use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

/// Sticky scroll related settings
#[with_fallible_options]
#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq)]
pub struct StickyScrollContent {
    /// Whether sticky scroll is enabled.
    ///
    /// Default: false
    pub enabled: Option<bool>,
}
