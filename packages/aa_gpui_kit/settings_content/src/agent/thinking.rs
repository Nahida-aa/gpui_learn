use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingBlockDisplay {
    /// Thinking blocks fully expand during streaming, then auto-collapse
    /// when the model finishes thinking. Users can re-expand after collapse.
    #[default]
    Auto,
    /// Thinking blocks auto-expand with a height constraint during streaming,
    /// then remain in their constrained state when complete. Users can click
    /// to fully expand or collapse.
    Preview,
    /// Thinking blocks are always fully expanded by default (no height constraint).
    AlwaysExpanded,
    /// Thinking blocks are always collapsed by default.
    AlwaysCollapsed,
}
