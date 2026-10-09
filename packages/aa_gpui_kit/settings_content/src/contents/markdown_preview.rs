use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::{FontFamilyName, FontSize, ThemeSelection, common::PixelSetting};

/// The settings for the markdown preview.
#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, Default, PartialEq)]
pub struct MarkdownPreviewSettingsContent {
    /// The name of a font to use for rendering in the markdown preview.
    /// Falls back to the UI font if unset.
    pub font_family: Option<FontFamilyName>,
    /// The name of a font to use for code (code blocks and inline code) in the
    /// markdown preview. Falls back to the buffer font if unset.
    pub code_font_family: Option<FontFamilyName>,
    /// The font size to use for rendering in the markdown preview.
    /// Falls back to the UI font size if unset.
    pub font_size: Option<FontSize>,
    /// The theme to use for the markdown preview.
    /// Falls back to the main editor theme if unset.
    pub theme: Option<ThemeSelection>,
    /// Whether to automatically open Markdown files in the preview.
    ///
    /// Default: false
    pub open_markdown_files_in_preview: Option<bool>,
    /// Whether to limit the width of the rendered markdown content. When
    /// enabled, content is constrained to `max_width` and centered
    /// horizontally within the preview pane, for optimal readability.
    ///
    /// Default: true
    pub limit_content_width: Option<bool>,
    /// The maximum width, in pixels, of the rendered markdown content when
    /// `limit_content_width` is enabled.
    ///
    /// Default: 800
    pub max_width: Option<PixelSetting>,
}
