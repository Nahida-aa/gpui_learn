use collections::IndexSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

use crate::merge_from;
use anyhow::Context as _;

/// A non-negative size in pixels.
///
/// Valid range: 0.0 and up
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    PartialOrd,
    derive_more::FromStr,
    derive_more::Deref,
    derive_more::From,
)]
#[serde(transparent)]
pub struct PixelSetting(
    #[serde(serialize_with = "crate::serialize_f32_with_two_decimal_places")] pub f32,
);

impl std::fmt::Display for PixelSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rounded = (self.0 * 100.0).round() / 100.0;
        write!(f, "{rounded}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseStatus {
    /// Settings were parsed successfully
    Success,
    /// Settings file was not changed, so no parsing was performed
    Unchanged,
    /// Settings failed to parse
    Failed { error: String },
}

#[derive(
    Copy,
    Clone,
    Default,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    MergeFrom,
    JsonSchema,
)]
#[serde(transparent)]
pub struct DelayMs(pub u64);

impl From<u64> for DelayMs {
    fn from(n: u64) -> Self { Self(n) }
}

impl std::fmt::Display for DelayMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}ms", self.0) }
}

impl std::str::FromStr for DelayMs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim()
            .strip_suffix("ms")
            .unwrap_or(s.trim())
            .parse::<u64>()
            .map(DelayMs)
            .with_context(|| format!("failed to parse delay duration: {s}"))
    }
}

// An ExtendingVec in the settings can only accumulate new values.
//
// This is useful for things like private files where you only want
// to allow new values to be added.
//
// Consider using a HashMap<String, bool> instead of this type
// (like auto_install_extensions) so that user settings files can both add
// and remove values from the set.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExtendingVec<T>(pub Vec<T>);

impl<T> Into<Vec<T>> for ExtendingVec<T> {
    fn into(self) -> Vec<T> { self.0 }
}
impl<T> From<Vec<T>> for ExtendingVec<T> {
    fn from(vec: Vec<T>) -> Self { ExtendingVec(vec) }
}

impl<T: Clone> merge_from::MergeFrom for ExtendingVec<T> {
    fn merge_from(&mut self, other: &Self) { self.0.extend_from_slice(other.0.as_slice()); }
}

// A SplicingVec in the settings replaces the value it merges over, except that
// a `...` entry expands to that previous value.
//
// This lets a settings file add to a list without restating what it inherits,
// while omitting `...` still replaces the list outright. Unlike ExtendingVec,
// entries can be dropped by leaving `...` out and listing what to keep.
//
// Entries collapse to their first occurrence, so naming a value that `...`
// already covers keeps it at the position it was written in rather than
// repeating it.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SplicingVec(pub Vec<String>);

impl SplicingVec {
    pub const REST: &str = "...";
}

impl From<Vec<String>> for SplicingVec {
    fn from(vec: Vec<String>) -> Self { SplicingVec(vec) }
}

impl merge_from::MergeFrom for SplicingVec {
    fn merge_from(&mut self, other: &Self) {
        let inherited = std::mem::take(&mut self.0);
        self.0 = other
            .0
            .iter()
            .flat_map(|entry| {
                if entry == Self::REST {
                    inherited.clone()
                } else {
                    vec![entry.clone()]
                }
            })
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();
    }
}

// An ExtendingSet in the settings can only accumulate new values, and ignores
// values that are already present, so merging the same source more than once
// (e.g. re-importing VS Code settings) is idempotent.
//
// Insertion order is preserved, so it round-trips through the user's settings
// file without reordering their entries.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExtendingSet<T: std::hash::Hash + Eq>(pub IndexSet<T>);

impl<T: std::hash::Hash + Eq> From<Vec<T>> for ExtendingSet<T> {
    fn from(vec: Vec<T>) -> Self { ExtendingSet(vec.into_iter().collect()) }
}

impl<T: Clone + std::hash::Hash + Eq> merge_from::MergeFrom for ExtendingSet<T> {
    fn merge_from(&mut self, other: &Self) { self.0.extend(other.0.iter().cloned()); }
}

// A SaturatingBool in the settings can only ever be set to true,
// later attempts to set it to false will be ignored.
//
// Used by `disable_ai`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SaturatingBool(pub bool);

impl From<bool> for SaturatingBool {
    fn from(value: bool) -> Self { SaturatingBool(value) }
}

impl From<SaturatingBool> for bool {
    fn from(value: SaturatingBool) -> bool { value.0 }
}

impl merge_from::MergeFrom for SaturatingBool {
    fn merge_from(&mut self, other: &Self) { self.0 |= other.0 }
}

// ---------- Language server capability toggles ----------

/// Controls how semantic tokens from language servers are used for syntax highlighting.
///
/// 对齐 Zed `crates/settings_content/src/workspace.rs:1120`。
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
    strum::EnumMessage,
)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTokens {
    /// Do not request semantic tokens from language servers.
    #[default]
    Off,
    /// Use LSP semantic tokens together with tree-sitter highlighting.
    Combined,
    /// Use LSP semantic tokens exclusively, replacing tree-sitter highlighting.
    Full,
}

impl SemanticTokens {
    /// Returns true if semantic tokens should be requested from language servers.
    pub fn enabled(&self) -> bool { self != &Self::Off }

    /// Returns true if tree-sitter syntax highlighting should be used.
    pub fn use_tree_sitter(&self) -> bool { self != &Self::Full }
}

/// Controls whether folding ranges from language servers are used instead of
/// tree-sitter and indent-based folding.
///
/// 对齐 Zed `crates/settings_content/src/workspace.rs:1158`。
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFoldingRanges {
    /// Do not request folding ranges from language servers; use tree-sitter and indent-based folding.
    #[default]
    Off,
    /// Use LSP folding wherever possible, falling back to tree-sitter and indent-based folding when no results were returned by the server.
    On,
}

impl DocumentFoldingRanges {
    pub fn enabled(&self) -> bool { self != &Self::Off }
}

/// Controls the source of document symbols used for outlines and breadcrumbs.
///
/// 对齐 Zed `crates/settings_content/src/workspace.rs:1188`。
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSymbols {
    /// Use tree-sitter queries to compute document symbols for outlines and breadcrumbs (default).
    #[default]
    #[serde(alias = "tree_sitter")]
    Off,
    /// Use the language server's `textDocument/documentSymbol` LSP response for outlines and breadcrumbs.
    #[serde(alias = "language_server")]
    On,
}

impl DocumentSymbols {
    pub fn lsp_enabled(&self) -> bool { self == &Self::On }
}
