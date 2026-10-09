use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// How to display diffs in the editor.
///
/// Default: unified
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::Display,
    strum::EnumIter,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DiffViewStyle {
    /// Show diffs in a single unified view.
    Unified,
    /// Show diffs in a split view.
    #[default]
    Split,
}
