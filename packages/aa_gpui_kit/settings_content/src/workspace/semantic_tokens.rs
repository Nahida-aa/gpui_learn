use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// Controls how semantic tokens from language servers are used for syntax highlighting.
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
    /// In `full` mode, tree-sitter is disabled in favor of LSP semantic tokens.
    pub fn use_tree_sitter(&self) -> bool { self != &Self::Full }
}

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
    /// Returns true if LSP folding ranges should be requested from language servers.
    pub fn enabled(&self) -> bool { self != &Self::Off }
}

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
    /// Use the language server's `textDocument/documentSymbol` LSP response for outlines and
    /// breadcrumbs. When enabled, tree-sitter is not used for document symbols.
    #[serde(alias = "language_server")]
    On,
}

impl DocumentSymbols {
    /// Returns true if LSP document symbols should be used instead of tree-sitter.
    pub fn lsp_enabled(&self) -> bool { self == &Self::On }
}
