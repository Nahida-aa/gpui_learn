use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// How to render LSP `textDocument/documentColor` colors in the editor.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
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
pub enum DocumentColorsRenderMode {
    /// Do not query and render document colors.
    None,
    /// Render document colors as inlay hints near the color text.
    #[default]
    Inlay,
    /// Draw a border around the color text.
    Border,
    /// Draw a background behind the color text.
    Background,
}

/// What to do when go to definition yields no results.
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
pub enum GoToDefinitionFallback {
    /// Disables the fallback.
    None,
    /// Looks up references of the same symbol instead.
    #[default]
    FindAllReferences,
}

/// Where to show LSP results that can contain multiple locations.
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
pub enum OpenResultsIn {
    /// Open the results in a multibuffer.
    #[default]
    MultiBuffer,
    /// Open the results in a filterable picker.
    Picker,
}

/// How to scroll the target into view when navigating to a definition or reference.
///
/// Default: center
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
pub enum GoToDefinitionScrollStrategy {
    /// Vertically center the target in the viewport.
    #[default]
    Center,
    /// Scroll the minimum amount needed to make the target visible.
    Minimum,
    /// Scroll so the target appears near the top of the viewport.
    Top,
    /// Preserve the cursor's vertical position within the viewport, falling
    /// back to centering when the cursor is offscreen.
    Preserve,
}
