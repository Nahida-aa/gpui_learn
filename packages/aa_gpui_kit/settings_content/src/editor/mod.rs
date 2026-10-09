use crate::{
    CursorShape, ShowScrollbar,
    common::DelayMs,
    project::DiagnosticSeverityContent,
};
// 这里是 `pub use`：zed 的 settings_content.rs 用 `pub use editor::*;` 把 editor
// 模块的公开项整批提到 crate 根，settings 再 `pub use ::settings_content::*;`
// 转出去。我们把 editor.rs 拆成目录后，这些名字必须从这一层公开，调用方写
// `settings::ScrollbarDiagnostics` 才能解析到（否则 editor crate 报 E0432）。
pub use crate::editor::{
    code_lens::CodeLens,
    completion::{CompletionDetailAlignment, CompletionMenuItemKind, SnippetSortOrder},
    cursor::CursorAnimationSettingsContent,
    diff::DiffViewStyle,
    drag_and_drop::DragAndDropSelectionContent,
    gutter::{GitGutterWidth, GitGutterWidthDiscriminants},
    jupyter::JupyterContent,
    lsp::{
        DocumentColorsRenderMode, GoToDefinitionFallback, GoToDefinitionScrollStrategy,
        OpenResultsIn,
    },
    minimap::{DisplayIn, MinimapThumbBorder},
    scalars::MinimumContrast,
    scrolling::ScrollBeyondLastLine,
    scrollbar::ScrollbarDiagnostics,
    toolbar::ToolbarContent,
};
pub use scalars::{CenteredPaddingSettings, InactiveOpacity};

// Re-export commonly used types from editor submodules.
pub use display::{
    CurrentLineHighlight, DoubleClickInMultibuffer, MultiCursorModifier, RelativeLineNumbers,
    SeedQuerySetting,
};
pub use gutter::GutterContent;
pub use minimap::{MinimapContent, MinimapThumb, ShowMinimap};
use schemars::JsonSchema;
pub use scrollbar::{ScrollbarAxesContent, ScrollbarContent};
pub use search::SearchSettingsContent;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
pub use sticky_scroll::StickyScrollContent;
mod code_lens;
mod completion;
pub mod cursor;
mod diff;
mod display;
mod drag_and_drop;
mod gutter;
mod jupyter;
mod lsp;
mod minimap;
pub mod scalars;
mod scrollbar;
mod scrolling;
mod search;
mod sticky_scroll;
mod toolbar;
#[with_fallible_options]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct EditorSettingsContent {
    /// Whether the cursor blinks in the editor.
    ///
    /// Default: true
    pub cursor_blink: Option<bool>,
    /// Cursor shape for the default editor.
    /// Can be "bar", "block", "underline", or "hollow".
    ///
    /// Default: bar
    pub cursor_shape: Option<CursorShape>,
    /// Cursor movement animation settings.
    pub cursor_animation: Option<CursorAnimationSettingsContent>,
    /// Determines how snippets are sorted relative to other completion items.
    ///
    /// Default: inline
    pub snippet_sort_order: Option<SnippetSortOrder>,
    /// How to highlight the current line in the editor.
    ///
    /// Default: all
    pub current_line_highlight: Option<CurrentLineHighlight>,
    /// Whether to highlight all occurrences of the selected text in an editor.
    ///
    /// Default: true
    pub selection_highlight: Option<bool>,
    /// Whether the text selection should have rounded corners.
    ///
    /// Default: true
    pub rounded_selection: Option<bool>,
    /// The debounce delay before querying highlights from the language
    /// server based on the current cursor location.
    ///
    /// Default: 75
    pub lsp_highlight_debounce: Option<DelayMs>,
    /// Whether to show the informational hover box when moving the mouse
    /// over symbols in the editor.
    ///
    /// Default: true
    pub hover_popover_enabled: Option<bool>,
    /// Time to wait in milliseconds before showing the informational hover box.
    /// This delay also applies to auto signature help when `auto_signature_help` is enabled.
    ///
    /// Default: 300
    pub hover_popover_delay: Option<DelayMs>,
    /// Whether the hover popover sticks when the mouse moves toward it,
    /// allowing interaction with its contents before it disappears.
    ///
    /// Default: true
    pub hover_popover_sticky: Option<bool>,
    /// Time to wait in milliseconds before hiding the hover popover
    /// after the mouse moves away from the hover target.
    /// Only applies when `hover_popover_sticky` is enabled.
    ///
    /// Default: 300
    pub hover_popover_hiding_delay: Option<DelayMs>,
    /// Toolbar related settings
    pub toolbar: Option<ToolbarContent>,
    /// Scrollbar related settings
    pub scrollbar: Option<ScrollbarContent>,
    /// Minimap related settings
    pub minimap: Option<MinimapContent>,
    /// Gutter related settings
    pub gutter: Option<GutterContent>,
    /// Whether the editor will scroll beyond the last line.
    ///
    /// Default: one_page
    pub scroll_beyond_last_line: Option<ScrollBeyondLastLine>,
    /// The number of lines to keep above/below the cursor when auto-scrolling.
    ///
    /// Default: 3.
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub vertical_scroll_margin: Option<f32>,
    /// Whether to scroll when clicking near the edge of the visible text area.
    ///
    /// Default: false
    pub autoscroll_on_clicks: Option<bool>,
    /// The number of characters to keep on either side when scrolling with the mouse.
    ///
    /// Default: 5.
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub horizontal_scroll_margin: Option<f32>,
    /// Scroll sensitivity multiplier. This multiplier is applied
    /// to both the horizontal and vertical delta values while scrolling.
    ///
    /// Default: 1.0
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub scroll_sensitivity: Option<f32>,
    /// Whether to zoom the editor font size with the mouse wheel
    /// while holding the primary modifier key (Cmd on macOS, Ctrl on other platforms).
    ///
    /// Default: false
    pub mouse_wheel_zoom: Option<bool>,
    /// Scroll sensitivity multiplier for fast scrolling. This multiplier is applied
    /// to both the horizontal and vertical delta values while scrolling. Fast scrolling
    /// happens when a user holds the alt or option key while scrolling.
    ///
    /// Default: 4.0
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub fast_scroll_sensitivity: Option<f32>,
    /// Settings for sticking scopes to the top of the editor.
    ///
    /// Default: sticky scroll is disabled
    pub sticky_scroll: Option<StickyScrollContent>,
    /// Whether the line numbers on editors gutter are relative or not.
    /// When "enabled" shows relative number of buffer lines, when "wrapped" shows
    /// relative number of display lines.
    ///
    /// Default: "disabled"
    pub relative_line_numbers: Option<RelativeLineNumbers>,
    /// When to populate a new search's query based on the text under the cursor.
    ///
    /// Default: always
    pub seed_search_query_from_cursor: Option<SeedQuerySetting>,
    pub use_smartcase_search: Option<bool>,
    /// Determines the modifier to be used to add multiple cursors with the mouse. The open hover link mouse gestures will adapt such that it do not conflict with the multicursor modifier.
    ///
    /// Default: alt
    pub multi_cursor_modifier: Option<MultiCursorModifier>,
    /// Hide the values of variables in `private` files, as defined by the
    /// private_files setting. This only changes the visual representation,
    /// the values are still present in the file and can be selected / copied / pasted
    ///
    /// Default: false
    pub redact_private_values: Option<bool>,

    /// How many lines to expand the multibuffer excerpts by default
    ///
    /// Default: 3
    pub expand_excerpt_lines: Option<u32>,

    /// How many lines of context to provide in multibuffer excerpts by default
    ///
    /// Default: 2
    pub excerpt_context_lines: Option<u32>,

    /// Whether to enable middle-click paste on Linux
    ///
    /// Default: true
    pub middle_click_paste: Option<bool>,

    /// What to do when multibuffer is double clicked in some of its excerpts
    /// (parts of singleton buffers).
    ///
    /// Default: select
    pub double_click_in_multibuffer: Option<DoubleClickInMultibuffer>,
    /// Whether the editor search results will loop
    ///
    /// Default: true
    pub search_wrap: Option<bool>,

    /// Defaults to use when opening a new buffer and project search items.
    ///
    /// Default: nothing is enabled
    pub search: Option<SearchSettingsContent>,

    /// Whether to automatically show a signature help pop-up or not.
    ///
    /// Default: false
    pub auto_signature_help: Option<bool>,

    /// Whether to automatically detect the language of an untitled buffer from its contents.
    /// Languages explicitly selected from the language selector are not changed.
    ///
    /// Default: true
    pub language_detection: Option<bool>,

    /// Whether to show the signature help pop-up after completions or bracket pairs inserted.
    ///
    /// Default: false
    pub show_signature_help_after_edits: Option<bool>,
    /// The minimum APCA perceptual contrast to maintain when
    /// rendering text over highlight backgrounds in the editor.
    ///
    /// Values range from 0 to 106. Set to 0 to disable adjustments.
    /// Default: 45
    #[schemars(range(min = 0, max = 106))]
    pub minimum_contrast_for_highlights: Option<MinimumContrast>,

    /// Whether to follow-up empty go to definition responses from the language server or not.
    /// `FindAllReferences` allows to look up references of the same symbol instead.
    /// `None` disables the fallback.
    ///
    /// Default: FindAllReferences
    pub go_to_definition_fallback: Option<GoToDefinitionFallback>,

    /// Where to show LSP results that can contain multiple locations
    /// (Go to Definition, Go to Implementation, Find All References). A single
    /// result always opens directly. Individual actions can override this with
    /// their `open_results_in` argument.
    ///
    /// Default: multi_buffer
    pub lsp_results_location: Option<OpenResultsIn>,

    /// How to scroll the target into view when navigating to a definition or reference
    /// (e.g. Go to Definition, Go to Type Definition, Find All References).
    ///
    /// Default: center
    pub go_to_definition_scroll_strategy: Option<GoToDefinitionScrollStrategy>,

    /// Jupyter REPL settings.
    pub jupyter: Option<JupyterContent>,

    /// Which level to use to filter out diagnostics displayed in the editor.
    ///
    /// Affects the editor rendering only, and does not interrupt
    /// the functionality of diagnostics fetching and project diagnostics editor.
    /// Which files containing diagnostic errors/warnings to mark in the tabs.
    /// Diagnostics are only shown when file icons are also active.
    ///
    /// Shows all diagnostics if not specified.
    ///
    /// Default: warning
    pub diagnostics_max_severity: Option<DiagnosticSeverityContent>,

    /// Whether to show code action button at start of buffer line.
    ///
    /// Default: true
    pub inline_code_actions: Option<bool>,

    /// Drag and drop related settings
    pub drag_and_drop_selection: Option<DragAndDropSelectionContent>,

    /// Whether and how to display code lenses from language servers.
    ///
    /// Default: "off"
    pub code_lens: Option<CodeLens>,

    /// How to render LSP `textDocument/documentColor` colors in the editor.
    ///
    /// Default: [`DocumentColorsRenderMode::Inlay`]
    pub lsp_document_colors: Option<DocumentColorsRenderMode>,
    /// Whether to query and display LSP `textDocument/documentLink` links in the editor.
    ///
    /// Default: true
    pub lsp_document_links: Option<bool>,
    /// When to show the scrollbar in the completion menu.
    /// This setting can take four values:
    ///
    /// 1. Show the scrollbar if there's important information or
    ///    follow the system's configured behavior
    ///   "auto"
    /// 2. Match the system's configured behavior:
    ///    "system"
    /// 3. Always show the scrollbar:
    ///    "always"
    /// 4. Never show the scrollbar:
    ///    "never" (default)
    pub completion_menu_scrollbar: Option<ShowScrollbar>,

    /// Whether to align detail text in code completions context menus left or right.
    ///
    /// Default: left
    pub completion_detail_alignment: Option<CompletionDetailAlignment>,

    /// How to display the LSP item kind (function, method, variable, etc.)
    /// of each entry in the completions menu.
    ///
    /// - "off": do not display item kinds (default).
    /// - "symbol": display a single-letter badge, colorized based on the
    ///   active syntax theme.
    ///
    /// Default: off
    pub completion_menu_item_kind: Option<CompletionMenuItemKind>,

    /// How to display diffs in the editor.
    ///
    /// Default: split
    pub diff_view_style: Option<DiffViewStyle>,

    /// The minimum width (in em-widths) at which the split diff view is used.
    /// When the editor is narrower than this, the diff view automatically
    /// switches to unified mode and switches back when the editor is wide
    /// enough. Set to 0 to disable automatic switching.
    ///
    /// Default: 100
    pub minimum_split_diff_width: Option<f32>,
}
