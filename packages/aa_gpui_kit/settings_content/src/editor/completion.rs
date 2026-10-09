use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
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
pub enum CompletionDetailAlignment {
    #[default]
    Left,
    Right,
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
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
pub enum CompletionMenuItemKind {
    #[default]
    Off,
    Symbol,
}

/// Determines how snippets are sorted relative to other completion items.
///
/// Default: inline
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum SnippetSortOrder {
    /// Place snippets at the top of the completion list
    Top,
    /// Sort snippets normally using the default comparison logic
    #[default]
    Inline,
    /// Place snippets at the bottom of the completion list
    Bottom,
    /// Do not show snippets in the completion list
    None,
}
