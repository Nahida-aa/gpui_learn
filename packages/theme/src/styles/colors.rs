//! UI 语义色（`ThemeColors`），对齐 zed `styles/colors.rs`。
//!
//! 字段与 zed **一一对应**（143 个），因此可以直接吃 zed 主题扩展的
//! `style` 字段；另增补 3 个本仓库自用字段（见结构体末尾）。
//! 所有颜色统一用 `Hsla`（与 gpui 的填充系统无缝衔接，`rgb(..).into()` 可转）。
//!
//! 默认值见 [`crate::default_colors`]（`ThemeColors::{light,dark}` 与
//! 33 个色板）；内置主题（Catppuccin）的取值见 [`crate::builtin`]。
//!
//! 同目录的其他样式集合见 [`system`](super::system) / [`status`](super::status)
//! / [`accents`](super::accents) / [`players`](super::players)
//! / [`syntax`](super::syntax)。

use gpui::{App, Hsla, SharedString};
use refineable::Refineable;

// `all_theme_colors` 里的 `cx.theme()` 来自本包的 ActiveTheme。
use crate::ActiveTheme;

/// UI 语义色（对齐 zed `ThemeColors`，字段名与注释均沿用 zed 原文）。
///
/// `#[derive(Refineable)]` 会生成 [`ThemeColorsRefinement`]——每个字段都是
/// `Option<Hsla>` 的版本；主题 JSON 只填想覆盖的字段，再 `refine` 到默认值上。
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug)]
pub struct ThemeColors {

    /// Border color. Used for most borders, is usually a high contrast color.
    pub border: Hsla,
    /// Border color. Used for deemphasized borders, like a visual divider between two sections
    pub border_variant: Hsla,
    /// Border color. Used for focused elements, like keyboard focused list item.
    pub border_focused: Hsla,
    /// Border color. Used for selected elements, like an active search filter or selected checkbox.
    pub border_selected: Hsla,
    /// Border color. Used for transparent borders. Used for placeholder borders when an element gains a border on state change.
    pub border_transparent: Hsla,
    /// Border color. Used for disabled elements, like a disabled input or button.
    pub border_disabled: Hsla,
    /// Border color. Used for elevated surfaces, like a context menu, popup, or dialog.
    pub elevated_surface_background: Hsla,
    /// Background Color. Used for grounded surfaces like a panel or tab.
    pub surface_background: Hsla,
    /// Background Color. Used for the app background and blank panels or windows.
    pub background: Hsla,
    /// Background Color. Used for the background of an element that should have a different background than the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have the same background as the surface it's on, use `ghost_element_background`.
    pub element_background: Hsla,
    /// Background Color. Used for the hover state of an element that should have a different background than the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    pub element_hover: Hsla,
    /// Background Color. Used for the active state of an element that should have a different background than the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressed.
    pub element_active: Hsla,
    /// Background Color. Used for the selected state of an element that should have a different background than the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    pub element_selected: Hsla,
    /// Background Color. Used for the background of selections in a UI element.
    pub element_selection_background: Hsla,
    /// Background Color. Used for the disabled state of an element that should have a different background than the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    pub element_disabled: Hsla,
    /// Background Color. Used for the area that shows where a dragged element will be dropped.
    pub drop_target_background: Hsla,
    /// Border Color. Used for the border that shows where a dragged element will be dropped.
    pub drop_target_border: Hsla,
    /// Used for the background of a ghost element that should have the same background as the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have a different background than the surface it's on, use `element_background`.
    pub ghost_element_background: Hsla,
    /// Background Color. Used for the hover state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    pub ghost_element_hover: Hsla,
    /// Background Color. Used for the active state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressed.
    pub ghost_element_active: Hsla,
    /// Background Color. Used for the selected state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    pub ghost_element_selected: Hsla,
    /// Background Color. Used for the disabled state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    pub ghost_element_disabled: Hsla,
    /// Text Color. Default text color used for most text.
    pub text: Hsla,
    /// Text Color. Color of muted or deemphasized text. It is a subdued version of the standard text color.
    pub text_muted: Hsla,
    /// Text Color. Color of the placeholder text typically shown in input fields to guide the user to enter valid data.
    pub text_placeholder: Hsla,
    /// Text Color. Color used for text denoting disabled elements. Typically, the color is faded or grayed out to emphasize the disabled state.
    pub text_disabled: Hsla,
    /// Text Color. Color used for emphasis or highlighting certain text, like an active filter or a matched character in a search.
    pub text_accent: Hsla,
    /// Fill Color. Used for the default fill color of an icon.
    pub icon: Hsla,
    /// Fill Color. Used for the muted or deemphasized fill color of an icon.
    ///
    /// This might be used to show an icon in an inactive pane, or to deemphasize a series of icons to give them less visual weight.
    pub icon_muted: Hsla,
    /// Fill Color. Used for the disabled fill color of an icon.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a icon button.
    pub icon_disabled: Hsla,
    /// Fill Color. Used for the placeholder fill color of an icon.
    ///
    /// This might be used to show an icon in an input that disappears when the user enters text.
    pub icon_placeholder: Hsla,
    /// Fill Color. Used for the accent fill color of an icon.
    ///
    /// This might be used to show when a toggleable icon button is selected.
    pub icon_accent: Hsla,
    /// Color used to accent some debugger elements
    /// Is used by breakpoints
    pub debugger_accent: Hsla,

    // ===
    // UI Elements
    // ===
    pub status_bar_background: Hsla,
    pub title_bar_background: Hsla,
    pub title_bar_inactive_background: Hsla,
    pub toolbar_background: Hsla,
    pub tab_bar_background: Hsla,
    pub tab_inactive_background: Hsla,
    pub tab_active_background: Hsla,
    pub search_match_background: Hsla,
    pub search_active_match_background: Hsla,
    pub panel_background: Hsla,
    pub panel_focused_border: Hsla,
    pub panel_indent_guide: Hsla,
    pub panel_indent_guide_hover: Hsla,
    pub panel_indent_guide_active: Hsla,

    /// The color of the overlay surface on top of panel.
    pub panel_overlay_background: Hsla,
    /// The color of the overlay surface on top of panel when hovered over.
    pub panel_overlay_hover: Hsla,

    pub pane_focused_border: Hsla,
    pub pane_group_border: Hsla,
    /// The color of the scrollbar thumb.
    pub scrollbar_thumb_background: Hsla,
    /// The color of the scrollbar thumb when hovered over.
    pub scrollbar_thumb_hover_background: Hsla,
    /// The color of the scrollbar thumb whilst being actively dragged.
    pub scrollbar_thumb_active_background: Hsla,
    /// The border color of the scrollbar thumb.
    pub scrollbar_thumb_border: Hsla,
    /// The background color of the scrollbar track.
    pub scrollbar_track_background: Hsla,
    /// The border color of the scrollbar track.
    pub scrollbar_track_border: Hsla,
    /// The color of the minimap thumb.
    pub minimap_thumb_background: Hsla,
    /// The color of the minimap thumb when hovered over.
    pub minimap_thumb_hover_background: Hsla,
    /// The color of the minimap thumb whilst being actively dragged.
    pub minimap_thumb_active_background: Hsla,
    /// The border color of the minimap thumb.
    pub minimap_thumb_border: Hsla,

    /// Background color for Vim Normal mode indicator.
    pub vim_normal_background: Hsla,
    /// Background color for Vim Insert mode indicator.
    pub vim_insert_background: Hsla,
    /// Background color for Vim Replace mode indicator.
    pub vim_replace_background: Hsla,
    /// Background color for Vim Visual mode indicator.
    pub vim_visual_background: Hsla,
    /// Background color for Vim Visual Line mode indicator.
    pub vim_visual_line_background: Hsla,
    /// Background color for Vim Visual Block mode indicator.
    pub vim_visual_block_background: Hsla,
    /// Background color for Vim yank highlight.
    pub vim_yank_background: Hsla,
    /// Foreground color for Helix jump labels.
    pub vim_helix_jump_label_foreground: Hsla,
    /// Background color for Vim Helix Normal mode indicator.
    pub vim_helix_normal_background: Hsla,
    /// Background color for Vim Helix Select mode indicator.
    pub vim_helix_select_background: Hsla,
    /// Foreground color for Vim Normal mode indicator.
    pub vim_normal_foreground: Hsla,
    /// Foreground color for Vim Insert mode indicator.
    pub vim_insert_foreground: Hsla,
    /// Foreground color for Vim Replace mode indicator.
    pub vim_replace_foreground: Hsla,
    /// Foreground color for Vim Visual mode indicator.
    pub vim_visual_foreground: Hsla,
    /// Foreground color for Vim Visual Line mode indicator.
    pub vim_visual_line_foreground: Hsla,
    /// Foreground color for Vim Visual Block mode indicator.
    pub vim_visual_block_foreground: Hsla,
    /// Foreground color for Vim Helix Normal mode indicator.
    pub vim_helix_normal_foreground: Hsla,
    /// Foreground color for Vim Helix Select mode indicator.
    pub vim_helix_select_foreground: Hsla,

    // ===
    // Editor
    // ===
    pub editor_foreground: Hsla,
    /// Text color used for CodeLens items in the editor.
    ///
    /// Falls back to `text_muted` when not explicitly set.
    pub editor_code_lens_foreground: Option<Hsla>,
    pub editor_background: Hsla,
    pub editor_gutter_background: Hsla,
    pub editor_subheader_background: Hsla,
    pub editor_active_line_background: Hsla,
    pub editor_highlighted_line_background: Hsla,
    /// Line color of the line a debugger is currently stopped at
    pub editor_debugger_active_line_background: Hsla,
    /// Text Color. Used for the text of the line number in the editor gutter.
    pub editor_line_number: Hsla,
    /// Text Color. Used for the text of the line number in the editor gutter when the line is highlighted.
    pub editor_active_line_number: Hsla,
    /// Text Color. Used for the text of the line number in the editor gutter when the line is hovered over.
    pub editor_hover_line_number: Hsla,
    /// Text Color. Used to mark invisible characters in the editor.
    ///
    /// Example: spaces, tabs, carriage returns, etc.
    pub editor_invisible: Hsla,
    pub editor_wrap_guide: Hsla,
    pub editor_active_wrap_guide: Hsla,
    pub editor_indent_guide: Hsla,
    pub editor_indent_guide_active: Hsla,
    /// Read-access of a symbol, like reading a variable.
    ///
    /// A document highlight is a range inside a text document which deserves
    /// special attention. Usually a document highlight is visualized by changing
    /// the background color of its range.
    pub editor_document_highlight_read_background: Hsla,
    /// Read-access of a symbol, like reading a variable.
    ///
    /// A document highlight is a range inside a text document which deserves
    /// special attention. Usually a document highlight is visualized by changing
    /// the background color of its range.
    pub editor_document_highlight_write_background: Hsla,
    /// Highlighted brackets background color.
    ///
    /// Matching brackets in the cursor scope are highlighted with this background color.
    pub editor_document_highlight_bracket_background: Hsla,
    /// Filled background color for added diff hunk row highlights in the editor.
    pub editor_diff_hunk_added_background: Hsla,
    /// Hollow background color for added diff hunk row highlights in the editor.
    pub editor_diff_hunk_added_hollow_background: Hsla,
    /// Hollow border color for added diff hunk row highlights in the editor.
    pub editor_diff_hunk_added_hollow_border: Hsla,
    /// Filled background color for deleted diff hunk row highlights in the editor.
    pub editor_diff_hunk_deleted_background: Hsla,
    /// Hollow background color for deleted diff hunk row highlights in the editor.
    pub editor_diff_hunk_deleted_hollow_background: Hsla,
    /// Hollow border color for deleted diff hunk row highlights in the editor.
    pub editor_diff_hunk_deleted_hollow_border: Hsla,

    // ===
    // Terminal
    // ===
    /// Terminal layout background color.
    pub terminal_background: Hsla,
    /// Terminal foreground color.
    pub terminal_foreground: Hsla,
    /// Bright terminal foreground color.
    pub terminal_bright_foreground: Hsla,
    /// Dim terminal foreground color.
    pub terminal_dim_foreground: Hsla,
    /// Terminal ANSI background color.
    pub terminal_ansi_background: Hsla,
    /// Black ANSI terminal color.
    pub terminal_ansi_black: Hsla,
    /// Bright black ANSI terminal color.
    pub terminal_ansi_bright_black: Hsla,
    /// Dim black ANSI terminal color.
    pub terminal_ansi_dim_black: Hsla,
    /// Red ANSI terminal color.
    pub terminal_ansi_red: Hsla,
    /// Bright red ANSI terminal color.
    pub terminal_ansi_bright_red: Hsla,
    /// Dim red ANSI terminal color.
    pub terminal_ansi_dim_red: Hsla,
    /// Green ANSI terminal color.
    pub terminal_ansi_green: Hsla,
    /// Bright green ANSI terminal color.
    pub terminal_ansi_bright_green: Hsla,
    /// Dim green ANSI terminal color.
    pub terminal_ansi_dim_green: Hsla,
    /// Yellow ANSI terminal color.
    pub terminal_ansi_yellow: Hsla,
    /// Bright yellow ANSI terminal color.
    pub terminal_ansi_bright_yellow: Hsla,
    /// Dim yellow ANSI terminal color.
    pub terminal_ansi_dim_yellow: Hsla,
    /// Blue ANSI terminal color.
    pub terminal_ansi_blue: Hsla,
    /// Bright blue ANSI terminal color.
    pub terminal_ansi_bright_blue: Hsla,
    /// Dim blue ANSI terminal color.
    pub terminal_ansi_dim_blue: Hsla,
    /// Magenta ANSI terminal color.
    pub terminal_ansi_magenta: Hsla,
    /// Bright magenta ANSI terminal color.
    pub terminal_ansi_bright_magenta: Hsla,
    /// Dim magenta ANSI terminal color.
    pub terminal_ansi_dim_magenta: Hsla,
    /// Cyan ANSI terminal color.
    pub terminal_ansi_cyan: Hsla,
    /// Bright cyan ANSI terminal color.
    pub terminal_ansi_bright_cyan: Hsla,
    /// Dim cyan ANSI terminal color.
    pub terminal_ansi_dim_cyan: Hsla,
    /// White ANSI terminal color.
    pub terminal_ansi_white: Hsla,
    /// Bright white ANSI terminal color.
    pub terminal_ansi_bright_white: Hsla,
    /// Dim white ANSI terminal color.
    pub terminal_ansi_dim_white: Hsla,

    /// Represents a link text hover color.
    pub link_text_hover: Hsla,

    /// Represents an added entry or hunk in vcs, like git.
    pub version_control_added: Hsla,
    /// Represents a deleted entry in version control systems.
    pub version_control_deleted: Hsla,
    /// Represents a modified entry in version control systems.
    pub version_control_modified: Hsla,
    /// Represents a renamed entry in version control systems.
    pub version_control_renamed: Hsla,
    /// Represents a conflicting entry in version control systems.
    pub version_control_conflict: Hsla,
    /// Represents an ignored entry in version control systems.
    pub version_control_ignored: Hsla,
    /// Represents an added word in a word diff.
    pub version_control_word_added: Hsla,
    /// Represents a deleted word in a word diff.
    pub version_control_word_deleted: Hsla,
    /// Represents the "ours" region of a merge conflict.
    pub version_control_conflict_marker_ours: Hsla,
    /// Represents the "theirs" region of a merge conflict.
    pub version_control_conflict_marker_theirs: Hsla,

    // ---- 以下是我们相对 zed 增补的字段 ----

    /// 选区背景色。zed 的 v0.2.0 主题扩展里没有对应 key，由加载器派生于
    /// `element_selected`。
    pub selection_background: Hsla,
    /// 编辑器光标色。同样由加载器派生于 `editor_foreground`。
    pub editor_cursor: Hsla,
    /// 链接色。zed 用 `text_accent` 表达，我们保留这个更直白的名字。
    pub link: Hsla,
}

/// 主题色字段的枚举（对齐 zed `theme::ThemeColorField`）。
///
/// 用途：把「结构体的字段」变成「可以遍历的值」。有了它才能:
/// - 批量取出当前主题全部颜色（[`all_theme_colors`]，主题预览/色板用）；
/// - 按名字单点取色（[`ThemeColors::color`]，设置里覆盖单个色用）。
///
/// `EnumIter` 让 `ThemeColorField::iter()` 可用，`AsRefStr` +
/// `serialize_all = "snake_case"` 让 `field.as_ref()` 直接得到与
/// `ThemeColors` 字段名一致的 snake_case 字符串。
#[derive(strum::EnumIter, Debug, Clone, Copy, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum ThemeColorField {
    Border,
    BorderVariant,
    BorderFocused,
    BorderSelected,
    BorderTransparent,
    BorderDisabled,
    ElevatedSurfaceBackground,
    SurfaceBackground,
    Background,
    ElementBackground,
    ElementHover,
    ElementActive,
    ElementSelected,
    ElementSelectionBackground,
    ElementDisabled,
    DropTargetBackground,
    DropTargetBorder,
    GhostElementBackground,
    GhostElementHover,
    GhostElementActive,
    GhostElementSelected,
    GhostElementDisabled,
    Text,
    TextMuted,
    TextPlaceholder,
    TextDisabled,
    TextAccent,
    Icon,
    IconMuted,
    IconDisabled,
    IconPlaceholder,
    IconAccent,
    DebuggerAccent,
    StatusBarBackground,
    TitleBarBackground,
    TitleBarInactiveBackground,
    ToolbarBackground,
    TabBarBackground,
    TabInactiveBackground,
    TabActiveBackground,
    SearchMatchBackground,
    SearchActiveMatchBackground,
    PanelBackground,
    PanelFocusedBorder,
    PanelIndentGuide,
    PanelIndentGuideHover,
    PanelIndentGuideActive,
    PanelOverlayBackground,
    PanelOverlayHover,
    PaneFocusedBorder,
    PaneGroupBorder,
    ScrollbarThumbBackground,
    ScrollbarThumbHoverBackground,
    ScrollbarThumbActiveBackground,
    ScrollbarThumbBorder,
    ScrollbarTrackBackground,
    ScrollbarTrackBorder,
    MinimapThumbBackground,
    MinimapThumbHoverBackground,
    MinimapThumbActiveBackground,
    MinimapThumbBorder,
    VimNormalBackground,
    VimInsertBackground,
    VimReplaceBackground,
    VimVisualBackground,
    VimVisualLineBackground,
    VimVisualBlockBackground,
    VimYankBackground,
    VimHelixJumpLabelForeground,
    VimHelixNormalBackground,
    VimHelixSelectBackground,
    VimNormalForeground,
    VimInsertForeground,
    VimReplaceForeground,
    VimVisualForeground,
    VimVisualLineForeground,
    VimVisualBlockForeground,
    VimHelixNormalForeground,
    VimHelixSelectForeground,
    EditorForeground,
    EditorCodeLensForeground,
    EditorBackground,
    EditorGutterBackground,
    EditorSubheaderBackground,
    EditorActiveLineBackground,
    EditorHighlightedLineBackground,
    EditorDebuggerActiveLineBackground,
    EditorLineNumber,
    EditorActiveLineNumber,
    EditorHoverLineNumber,
    EditorInvisible,
    EditorWrapGuide,
    EditorActiveWrapGuide,
    EditorIndentGuide,
    EditorIndentGuideActive,
    EditorDocumentHighlightReadBackground,
    EditorDocumentHighlightWriteBackground,
    EditorDocumentHighlightBracketBackground,
    EditorDiffHunkAddedBackground,
    EditorDiffHunkAddedHollowBackground,
    EditorDiffHunkAddedHollowBorder,
    EditorDiffHunkDeletedBackground,
    EditorDiffHunkDeletedHollowBackground,
    EditorDiffHunkDeletedHollowBorder,
    TerminalBackground,
    TerminalForeground,
    TerminalBrightForeground,
    TerminalDimForeground,
    TerminalAnsiBackground,
    TerminalAnsiBlack,
    TerminalAnsiBrightBlack,
    TerminalAnsiDimBlack,
    TerminalAnsiRed,
    TerminalAnsiBrightRed,
    TerminalAnsiDimRed,
    TerminalAnsiGreen,
    TerminalAnsiBrightGreen,
    TerminalAnsiDimGreen,
    TerminalAnsiYellow,
    TerminalAnsiBrightYellow,
    TerminalAnsiDimYellow,
    TerminalAnsiBlue,
    TerminalAnsiBrightBlue,
    TerminalAnsiDimBlue,
    TerminalAnsiMagenta,
    TerminalAnsiBrightMagenta,
    TerminalAnsiDimMagenta,
    TerminalAnsiCyan,
    TerminalAnsiBrightCyan,
    TerminalAnsiDimCyan,
    TerminalAnsiWhite,
    TerminalAnsiBrightWhite,
    TerminalAnsiDimWhite,
    LinkTextHover,
    VersionControlAdded,
    VersionControlDeleted,
    VersionControlModified,
    VersionControlRenamed,
    VersionControlConflict,
    VersionControlIgnored,
    VersionControlWordAdded,
    VersionControlWordDeleted,
    VersionControlConflictMarkerOurs,
    VersionControlConflictMarkerTheirs,
    SelectionBackground,
    EditorCursor,
    Link,
}

impl ThemeColors {
    /// 按字段取色。
    ///
    /// 与 zed 一致：`Option<Hsla>` 的字段在 `None` 时回落到 `text_muted`。
    pub fn color(&self, field: ThemeColorField) -> Hsla {
        match field {
            ThemeColorField::Border => self.border,
            ThemeColorField::BorderVariant => self.border_variant,
            ThemeColorField::BorderFocused => self.border_focused,
            ThemeColorField::BorderSelected => self.border_selected,
            ThemeColorField::BorderTransparent => self.border_transparent,
            ThemeColorField::BorderDisabled => self.border_disabled,
            ThemeColorField::ElevatedSurfaceBackground => self.elevated_surface_background,
            ThemeColorField::SurfaceBackground => self.surface_background,
            ThemeColorField::Background => self.background,
            ThemeColorField::ElementBackground => self.element_background,
            ThemeColorField::ElementHover => self.element_hover,
            ThemeColorField::ElementActive => self.element_active,
            ThemeColorField::ElementSelected => self.element_selected,
            ThemeColorField::ElementSelectionBackground => self.element_selection_background,
            ThemeColorField::ElementDisabled => self.element_disabled,
            ThemeColorField::DropTargetBackground => self.drop_target_background,
            ThemeColorField::DropTargetBorder => self.drop_target_border,
            ThemeColorField::GhostElementBackground => self.ghost_element_background,
            ThemeColorField::GhostElementHover => self.ghost_element_hover,
            ThemeColorField::GhostElementActive => self.ghost_element_active,
            ThemeColorField::GhostElementSelected => self.ghost_element_selected,
            ThemeColorField::GhostElementDisabled => self.ghost_element_disabled,
            ThemeColorField::Text => self.text,
            ThemeColorField::TextMuted => self.text_muted,
            ThemeColorField::TextPlaceholder => self.text_placeholder,
            ThemeColorField::TextDisabled => self.text_disabled,
            ThemeColorField::TextAccent => self.text_accent,
            ThemeColorField::Icon => self.icon,
            ThemeColorField::IconMuted => self.icon_muted,
            ThemeColorField::IconDisabled => self.icon_disabled,
            ThemeColorField::IconPlaceholder => self.icon_placeholder,
            ThemeColorField::IconAccent => self.icon_accent,
            ThemeColorField::DebuggerAccent => self.debugger_accent,
            ThemeColorField::StatusBarBackground => self.status_bar_background,
            ThemeColorField::TitleBarBackground => self.title_bar_background,
            ThemeColorField::TitleBarInactiveBackground => self.title_bar_inactive_background,
            ThemeColorField::ToolbarBackground => self.toolbar_background,
            ThemeColorField::TabBarBackground => self.tab_bar_background,
            ThemeColorField::TabInactiveBackground => self.tab_inactive_background,
            ThemeColorField::TabActiveBackground => self.tab_active_background,
            ThemeColorField::SearchMatchBackground => self.search_match_background,
            ThemeColorField::SearchActiveMatchBackground => self.search_active_match_background,
            ThemeColorField::PanelBackground => self.panel_background,
            ThemeColorField::PanelFocusedBorder => self.panel_focused_border,
            ThemeColorField::PanelIndentGuide => self.panel_indent_guide,
            ThemeColorField::PanelIndentGuideHover => self.panel_indent_guide_hover,
            ThemeColorField::PanelIndentGuideActive => self.panel_indent_guide_active,
            ThemeColorField::PanelOverlayBackground => self.panel_overlay_background,
            ThemeColorField::PanelOverlayHover => self.panel_overlay_hover,
            ThemeColorField::PaneFocusedBorder => self.pane_focused_border,
            ThemeColorField::PaneGroupBorder => self.pane_group_border,
            ThemeColorField::ScrollbarThumbBackground => self.scrollbar_thumb_background,
            ThemeColorField::ScrollbarThumbHoverBackground => self.scrollbar_thumb_hover_background,
            ThemeColorField::ScrollbarThumbActiveBackground => self.scrollbar_thumb_active_background,
            ThemeColorField::ScrollbarThumbBorder => self.scrollbar_thumb_border,
            ThemeColorField::ScrollbarTrackBackground => self.scrollbar_track_background,
            ThemeColorField::ScrollbarTrackBorder => self.scrollbar_track_border,
            ThemeColorField::MinimapThumbBackground => self.minimap_thumb_background,
            ThemeColorField::MinimapThumbHoverBackground => self.minimap_thumb_hover_background,
            ThemeColorField::MinimapThumbActiveBackground => self.minimap_thumb_active_background,
            ThemeColorField::MinimapThumbBorder => self.minimap_thumb_border,
            ThemeColorField::VimNormalBackground => self.vim_normal_background,
            ThemeColorField::VimInsertBackground => self.vim_insert_background,
            ThemeColorField::VimReplaceBackground => self.vim_replace_background,
            ThemeColorField::VimVisualBackground => self.vim_visual_background,
            ThemeColorField::VimVisualLineBackground => self.vim_visual_line_background,
            ThemeColorField::VimVisualBlockBackground => self.vim_visual_block_background,
            ThemeColorField::VimYankBackground => self.vim_yank_background,
            ThemeColorField::VimHelixJumpLabelForeground => self.vim_helix_jump_label_foreground,
            ThemeColorField::VimHelixNormalBackground => self.vim_helix_normal_background,
            ThemeColorField::VimHelixSelectBackground => self.vim_helix_select_background,
            ThemeColorField::VimNormalForeground => self.vim_normal_foreground,
            ThemeColorField::VimInsertForeground => self.vim_insert_foreground,
            ThemeColorField::VimReplaceForeground => self.vim_replace_foreground,
            ThemeColorField::VimVisualForeground => self.vim_visual_foreground,
            ThemeColorField::VimVisualLineForeground => self.vim_visual_line_foreground,
            ThemeColorField::VimVisualBlockForeground => self.vim_visual_block_foreground,
            ThemeColorField::VimHelixNormalForeground => self.vim_helix_normal_foreground,
            ThemeColorField::VimHelixSelectForeground => self.vim_helix_select_foreground,
            ThemeColorField::EditorForeground => self.editor_foreground,
            ThemeColorField::EditorCodeLensForeground => self.editor_code_lens_foreground.unwrap_or(self.text_muted),
            ThemeColorField::EditorBackground => self.editor_background,
            ThemeColorField::EditorGutterBackground => self.editor_gutter_background,
            ThemeColorField::EditorSubheaderBackground => self.editor_subheader_background,
            ThemeColorField::EditorActiveLineBackground => self.editor_active_line_background,
            ThemeColorField::EditorHighlightedLineBackground => self.editor_highlighted_line_background,
            ThemeColorField::EditorDebuggerActiveLineBackground => self.editor_debugger_active_line_background,
            ThemeColorField::EditorLineNumber => self.editor_line_number,
            ThemeColorField::EditorActiveLineNumber => self.editor_active_line_number,
            ThemeColorField::EditorHoverLineNumber => self.editor_hover_line_number,
            ThemeColorField::EditorInvisible => self.editor_invisible,
            ThemeColorField::EditorWrapGuide => self.editor_wrap_guide,
            ThemeColorField::EditorActiveWrapGuide => self.editor_active_wrap_guide,
            ThemeColorField::EditorIndentGuide => self.editor_indent_guide,
            ThemeColorField::EditorIndentGuideActive => self.editor_indent_guide_active,
            ThemeColorField::EditorDocumentHighlightReadBackground => self.editor_document_highlight_read_background,
            ThemeColorField::EditorDocumentHighlightWriteBackground => self.editor_document_highlight_write_background,
            ThemeColorField::EditorDocumentHighlightBracketBackground => self.editor_document_highlight_bracket_background,
            ThemeColorField::EditorDiffHunkAddedBackground => self.editor_diff_hunk_added_background,
            ThemeColorField::EditorDiffHunkAddedHollowBackground => self.editor_diff_hunk_added_hollow_background,
            ThemeColorField::EditorDiffHunkAddedHollowBorder => self.editor_diff_hunk_added_hollow_border,
            ThemeColorField::EditorDiffHunkDeletedBackground => self.editor_diff_hunk_deleted_background,
            ThemeColorField::EditorDiffHunkDeletedHollowBackground => self.editor_diff_hunk_deleted_hollow_background,
            ThemeColorField::EditorDiffHunkDeletedHollowBorder => self.editor_diff_hunk_deleted_hollow_border,
            ThemeColorField::TerminalBackground => self.terminal_background,
            ThemeColorField::TerminalForeground => self.terminal_foreground,
            ThemeColorField::TerminalBrightForeground => self.terminal_bright_foreground,
            ThemeColorField::TerminalDimForeground => self.terminal_dim_foreground,
            ThemeColorField::TerminalAnsiBackground => self.terminal_ansi_background,
            ThemeColorField::TerminalAnsiBlack => self.terminal_ansi_black,
            ThemeColorField::TerminalAnsiBrightBlack => self.terminal_ansi_bright_black,
            ThemeColorField::TerminalAnsiDimBlack => self.terminal_ansi_dim_black,
            ThemeColorField::TerminalAnsiRed => self.terminal_ansi_red,
            ThemeColorField::TerminalAnsiBrightRed => self.terminal_ansi_bright_red,
            ThemeColorField::TerminalAnsiDimRed => self.terminal_ansi_dim_red,
            ThemeColorField::TerminalAnsiGreen => self.terminal_ansi_green,
            ThemeColorField::TerminalAnsiBrightGreen => self.terminal_ansi_bright_green,
            ThemeColorField::TerminalAnsiDimGreen => self.terminal_ansi_dim_green,
            ThemeColorField::TerminalAnsiYellow => self.terminal_ansi_yellow,
            ThemeColorField::TerminalAnsiBrightYellow => self.terminal_ansi_bright_yellow,
            ThemeColorField::TerminalAnsiDimYellow => self.terminal_ansi_dim_yellow,
            ThemeColorField::TerminalAnsiBlue => self.terminal_ansi_blue,
            ThemeColorField::TerminalAnsiBrightBlue => self.terminal_ansi_bright_blue,
            ThemeColorField::TerminalAnsiDimBlue => self.terminal_ansi_dim_blue,
            ThemeColorField::TerminalAnsiMagenta => self.terminal_ansi_magenta,
            ThemeColorField::TerminalAnsiBrightMagenta => self.terminal_ansi_bright_magenta,
            ThemeColorField::TerminalAnsiDimMagenta => self.terminal_ansi_dim_magenta,
            ThemeColorField::TerminalAnsiCyan => self.terminal_ansi_cyan,
            ThemeColorField::TerminalAnsiBrightCyan => self.terminal_ansi_bright_cyan,
            ThemeColorField::TerminalAnsiDimCyan => self.terminal_ansi_dim_cyan,
            ThemeColorField::TerminalAnsiWhite => self.terminal_ansi_white,
            ThemeColorField::TerminalAnsiBrightWhite => self.terminal_ansi_bright_white,
            ThemeColorField::TerminalAnsiDimWhite => self.terminal_ansi_dim_white,
            ThemeColorField::LinkTextHover => self.link_text_hover,
            ThemeColorField::VersionControlAdded => self.version_control_added,
            ThemeColorField::VersionControlDeleted => self.version_control_deleted,
            ThemeColorField::VersionControlModified => self.version_control_modified,
            ThemeColorField::VersionControlRenamed => self.version_control_renamed,
            ThemeColorField::VersionControlConflict => self.version_control_conflict,
            ThemeColorField::VersionControlIgnored => self.version_control_ignored,
            ThemeColorField::VersionControlWordAdded => self.version_control_word_added,
            ThemeColorField::VersionControlWordDeleted => self.version_control_word_deleted,
            ThemeColorField::VersionControlConflictMarkerOurs => self.version_control_conflict_marker_ours,
            ThemeColorField::VersionControlConflictMarkerTheirs => self.version_control_conflict_marker_theirs,
            ThemeColorField::SelectionBackground => self.selection_background,
            ThemeColorField::EditorCursor => self.editor_cursor,
            ThemeColorField::Link => self.link,
        }
    }

    /// 遍历全部 (字段, 颜色)。
    pub fn iter(&self) -> impl Iterator<Item = (ThemeColorField, Hsla)> + '_ {
        use strum::IntoEnumIterator;

        ThemeColorField::iter().map(move |field| (field, self.color(field)))
    }

    /// 收集成 `Vec`（批量渲染色板时常用）。
    pub fn to_vec(&self) -> Vec<(ThemeColorField, Hsla)> {
        self.iter().collect()
    }
}

/// 当前主题的全部颜色：`(颜色, 字段名)`。
///
/// 对齐 zed `theme::all_theme_colors`。zed 里唯一的消费方是
/// `workspace/src/theme_preview.rs`（把主题色铺成色板预览）。
pub fn all_theme_colors(cx: &mut App) -> Vec<(Hsla, SharedString)> {
    let theme = cx.theme();
    theme
        .colors()
        .iter()
        .map(|(field, color)| (color, SharedString::from(field.as_ref().to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// 主题色字段总数。改 `ThemeColors` 的字段时这个数必须同步 ——
    /// 忘了同步 `ThemeColorField` 就会在这里挂掉（枚举少一个变体，
    /// `color()` 的 match 也会编译不过，双重保险）。
    const THEME_COLOR_FIELD_COUNT: usize = 147;

    #[test]
    fn field_enum_covers_every_color() {
        assert_eq!(
            ThemeColorField::iter().count(),
            THEME_COLOR_FIELD_COUNT,
            "ThemeColorField 的变体数与 ThemeColors 的字段数不一致");
    }

    #[test]
    fn field_names_match_struct_fields() {
        // `AsRefStr` + snake_case 必须能还原出结构体里的字段名，
        // 否则按名字取色 / 主题 JSON 覆盖会对不上。
        // 注意不能收 `&str`：`as_ref()` 借用闭包里的临时值，收引用会悬垂。
        let names: Vec<String> = ThemeColorField::iter()
            .map(|f| f.as_ref().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "border"), "缺少字段 border");
        assert!(names.iter().any(|n| n == "border_variant"), "缺少字段 border_variant");
        assert!(names.iter().any(|n| n == "border_focused"), "缺少字段 border_focused");
        assert!(names.iter().any(|n| n == "border_selected"), "缺少字段 border_selected");
        assert!(names.iter().any(|n| n == "border_transparent"), "缺少字段 border_transparent");
        assert!(names.iter().any(|n| n == "border_disabled"), "缺少字段 border_disabled");
        assert!(names.iter().any(|n| n == "elevated_surface_background"), "缺少字段 elevated_surface_background");
        assert!(names.iter().any(|n| n == "surface_background"), "缺少字段 surface_background");
        assert!(names.iter().any(|n| n == "background"), "缺少字段 background");
        assert!(names.iter().any(|n| n == "element_background"), "缺少字段 element_background");
        assert!(names.iter().any(|n| n == "element_hover"), "缺少字段 element_hover");
        assert!(names.iter().any(|n| n == "element_active"), "缺少字段 element_active");
        assert!(names.iter().any(|n| n == "element_selected"), "缺少字段 element_selected");
        assert!(names.iter().any(|n| n == "element_selection_background"), "缺少字段 element_selection_background");
        assert!(names.iter().any(|n| n == "element_disabled"), "缺少字段 element_disabled");
        assert!(names.iter().any(|n| n == "drop_target_background"), "缺少字段 drop_target_background");
        assert!(names.iter().any(|n| n == "drop_target_border"), "缺少字段 drop_target_border");
        assert!(names.iter().any(|n| n == "ghost_element_background"), "缺少字段 ghost_element_background");
        assert!(names.iter().any(|n| n == "ghost_element_hover"), "缺少字段 ghost_element_hover");
        assert!(names.iter().any(|n| n == "ghost_element_active"), "缺少字段 ghost_element_active");
        assert!(names.iter().any(|n| n == "ghost_element_selected"), "缺少字段 ghost_element_selected");
        assert!(names.iter().any(|n| n == "ghost_element_disabled"), "缺少字段 ghost_element_disabled");
        assert!(names.iter().any(|n| n == "text"), "缺少字段 text");
        assert!(names.iter().any(|n| n == "text_muted"), "缺少字段 text_muted");
        assert!(names.iter().any(|n| n == "text_placeholder"), "缺少字段 text_placeholder");
        assert!(names.iter().any(|n| n == "text_disabled"), "缺少字段 text_disabled");
        assert!(names.iter().any(|n| n == "text_accent"), "缺少字段 text_accent");
        assert!(names.iter().any(|n| n == "icon"), "缺少字段 icon");
        assert!(names.iter().any(|n| n == "icon_muted"), "缺少字段 icon_muted");
        assert!(names.iter().any(|n| n == "icon_disabled"), "缺少字段 icon_disabled");
        assert!(names.iter().any(|n| n == "icon_placeholder"), "缺少字段 icon_placeholder");
        assert!(names.iter().any(|n| n == "icon_accent"), "缺少字段 icon_accent");
        assert!(names.iter().any(|n| n == "debugger_accent"), "缺少字段 debugger_accent");
        assert!(names.iter().any(|n| n == "status_bar_background"), "缺少字段 status_bar_background");
        assert!(names.iter().any(|n| n == "title_bar_background"), "缺少字段 title_bar_background");
        assert!(names.iter().any(|n| n == "title_bar_inactive_background"), "缺少字段 title_bar_inactive_background");
        assert!(names.iter().any(|n| n == "toolbar_background"), "缺少字段 toolbar_background");
        assert!(names.iter().any(|n| n == "tab_bar_background"), "缺少字段 tab_bar_background");
        assert!(names.iter().any(|n| n == "tab_inactive_background"), "缺少字段 tab_inactive_background");
        assert!(names.iter().any(|n| n == "tab_active_background"), "缺少字段 tab_active_background");
        assert!(names.iter().any(|n| n == "search_match_background"), "缺少字段 search_match_background");
        assert!(names.iter().any(|n| n == "search_active_match_background"), "缺少字段 search_active_match_background");
        assert!(names.iter().any(|n| n == "panel_background"), "缺少字段 panel_background");
        assert!(names.iter().any(|n| n == "panel_focused_border"), "缺少字段 panel_focused_border");
        assert!(names.iter().any(|n| n == "panel_indent_guide"), "缺少字段 panel_indent_guide");
        assert!(names.iter().any(|n| n == "panel_indent_guide_hover"), "缺少字段 panel_indent_guide_hover");
        assert!(names.iter().any(|n| n == "panel_indent_guide_active"), "缺少字段 panel_indent_guide_active");
        assert!(names.iter().any(|n| n == "panel_overlay_background"), "缺少字段 panel_overlay_background");
        assert!(names.iter().any(|n| n == "panel_overlay_hover"), "缺少字段 panel_overlay_hover");
        assert!(names.iter().any(|n| n == "pane_focused_border"), "缺少字段 pane_focused_border");
        assert!(names.iter().any(|n| n == "pane_group_border"), "缺少字段 pane_group_border");
        assert!(names.iter().any(|n| n == "scrollbar_thumb_background"), "缺少字段 scrollbar_thumb_background");
        assert!(names.iter().any(|n| n == "scrollbar_thumb_hover_background"), "缺少字段 scrollbar_thumb_hover_background");
        assert!(names.iter().any(|n| n == "scrollbar_thumb_active_background"), "缺少字段 scrollbar_thumb_active_background");
        assert!(names.iter().any(|n| n == "scrollbar_thumb_border"), "缺少字段 scrollbar_thumb_border");
        assert!(names.iter().any(|n| n == "scrollbar_track_background"), "缺少字段 scrollbar_track_background");
        assert!(names.iter().any(|n| n == "scrollbar_track_border"), "缺少字段 scrollbar_track_border");
        assert!(names.iter().any(|n| n == "minimap_thumb_background"), "缺少字段 minimap_thumb_background");
        assert!(names.iter().any(|n| n == "minimap_thumb_hover_background"), "缺少字段 minimap_thumb_hover_background");
        assert!(names.iter().any(|n| n == "minimap_thumb_active_background"), "缺少字段 minimap_thumb_active_background");
        assert!(names.iter().any(|n| n == "minimap_thumb_border"), "缺少字段 minimap_thumb_border");
        assert!(names.iter().any(|n| n == "vim_normal_background"), "缺少字段 vim_normal_background");
        assert!(names.iter().any(|n| n == "vim_insert_background"), "缺少字段 vim_insert_background");
        assert!(names.iter().any(|n| n == "vim_replace_background"), "缺少字段 vim_replace_background");
        assert!(names.iter().any(|n| n == "vim_visual_background"), "缺少字段 vim_visual_background");
        assert!(names.iter().any(|n| n == "vim_visual_line_background"), "缺少字段 vim_visual_line_background");
        assert!(names.iter().any(|n| n == "vim_visual_block_background"), "缺少字段 vim_visual_block_background");
        assert!(names.iter().any(|n| n == "vim_yank_background"), "缺少字段 vim_yank_background");
        assert!(names.iter().any(|n| n == "vim_helix_jump_label_foreground"), "缺少字段 vim_helix_jump_label_foreground");
        assert!(names.iter().any(|n| n == "vim_helix_normal_background"), "缺少字段 vim_helix_normal_background");
        assert!(names.iter().any(|n| n == "vim_helix_select_background"), "缺少字段 vim_helix_select_background");
        assert!(names.iter().any(|n| n == "vim_normal_foreground"), "缺少字段 vim_normal_foreground");
        assert!(names.iter().any(|n| n == "vim_insert_foreground"), "缺少字段 vim_insert_foreground");
        assert!(names.iter().any(|n| n == "vim_replace_foreground"), "缺少字段 vim_replace_foreground");
        assert!(names.iter().any(|n| n == "vim_visual_foreground"), "缺少字段 vim_visual_foreground");
        assert!(names.iter().any(|n| n == "vim_visual_line_foreground"), "缺少字段 vim_visual_line_foreground");
        assert!(names.iter().any(|n| n == "vim_visual_block_foreground"), "缺少字段 vim_visual_block_foreground");
        assert!(names.iter().any(|n| n == "vim_helix_normal_foreground"), "缺少字段 vim_helix_normal_foreground");
        assert!(names.iter().any(|n| n == "vim_helix_select_foreground"), "缺少字段 vim_helix_select_foreground");
        assert!(names.iter().any(|n| n == "editor_foreground"), "缺少字段 editor_foreground");
        assert!(names.iter().any(|n| n == "editor_code_lens_foreground"), "缺少字段 editor_code_lens_foreground");
        assert!(names.iter().any(|n| n == "editor_background"), "缺少字段 editor_background");
        assert!(names.iter().any(|n| n == "editor_gutter_background"), "缺少字段 editor_gutter_background");
        assert!(names.iter().any(|n| n == "editor_subheader_background"), "缺少字段 editor_subheader_background");
        assert!(names.iter().any(|n| n == "editor_active_line_background"), "缺少字段 editor_active_line_background");
        assert!(names.iter().any(|n| n == "editor_highlighted_line_background"), "缺少字段 editor_highlighted_line_background");
        assert!(names.iter().any(|n| n == "editor_debugger_active_line_background"), "缺少字段 editor_debugger_active_line_background");
        assert!(names.iter().any(|n| n == "editor_line_number"), "缺少字段 editor_line_number");
        assert!(names.iter().any(|n| n == "editor_active_line_number"), "缺少字段 editor_active_line_number");
        assert!(names.iter().any(|n| n == "editor_hover_line_number"), "缺少字段 editor_hover_line_number");
        assert!(names.iter().any(|n| n == "editor_invisible"), "缺少字段 editor_invisible");
        assert!(names.iter().any(|n| n == "editor_wrap_guide"), "缺少字段 editor_wrap_guide");
        assert!(names.iter().any(|n| n == "editor_active_wrap_guide"), "缺少字段 editor_active_wrap_guide");
        assert!(names.iter().any(|n| n == "editor_indent_guide"), "缺少字段 editor_indent_guide");
        assert!(names.iter().any(|n| n == "editor_indent_guide_active"), "缺少字段 editor_indent_guide_active");
        assert!(names.iter().any(|n| n == "editor_document_highlight_read_background"), "缺少字段 editor_document_highlight_read_background");
        assert!(names.iter().any(|n| n == "editor_document_highlight_write_background"), "缺少字段 editor_document_highlight_write_background");
        assert!(names.iter().any(|n| n == "editor_document_highlight_bracket_background"), "缺少字段 editor_document_highlight_bracket_background");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_added_background"), "缺少字段 editor_diff_hunk_added_background");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_added_hollow_background"), "缺少字段 editor_diff_hunk_added_hollow_background");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_added_hollow_border"), "缺少字段 editor_diff_hunk_added_hollow_border");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_deleted_background"), "缺少字段 editor_diff_hunk_deleted_background");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_deleted_hollow_background"), "缺少字段 editor_diff_hunk_deleted_hollow_background");
        assert!(names.iter().any(|n| n == "editor_diff_hunk_deleted_hollow_border"), "缺少字段 editor_diff_hunk_deleted_hollow_border");
        assert!(names.iter().any(|n| n == "terminal_background"), "缺少字段 terminal_background");
        assert!(names.iter().any(|n| n == "terminal_foreground"), "缺少字段 terminal_foreground");
        assert!(names.iter().any(|n| n == "terminal_bright_foreground"), "缺少字段 terminal_bright_foreground");
        assert!(names.iter().any(|n| n == "terminal_dim_foreground"), "缺少字段 terminal_dim_foreground");
        assert!(names.iter().any(|n| n == "terminal_ansi_background"), "缺少字段 terminal_ansi_background");
        assert!(names.iter().any(|n| n == "terminal_ansi_black"), "缺少字段 terminal_ansi_black");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_black"), "缺少字段 terminal_ansi_bright_black");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_black"), "缺少字段 terminal_ansi_dim_black");
        assert!(names.iter().any(|n| n == "terminal_ansi_red"), "缺少字段 terminal_ansi_red");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_red"), "缺少字段 terminal_ansi_bright_red");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_red"), "缺少字段 terminal_ansi_dim_red");
        assert!(names.iter().any(|n| n == "terminal_ansi_green"), "缺少字段 terminal_ansi_green");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_green"), "缺少字段 terminal_ansi_bright_green");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_green"), "缺少字段 terminal_ansi_dim_green");
        assert!(names.iter().any(|n| n == "terminal_ansi_yellow"), "缺少字段 terminal_ansi_yellow");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_yellow"), "缺少字段 terminal_ansi_bright_yellow");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_yellow"), "缺少字段 terminal_ansi_dim_yellow");
        assert!(names.iter().any(|n| n == "terminal_ansi_blue"), "缺少字段 terminal_ansi_blue");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_blue"), "缺少字段 terminal_ansi_bright_blue");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_blue"), "缺少字段 terminal_ansi_dim_blue");
        assert!(names.iter().any(|n| n == "terminal_ansi_magenta"), "缺少字段 terminal_ansi_magenta");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_magenta"), "缺少字段 terminal_ansi_bright_magenta");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_magenta"), "缺少字段 terminal_ansi_dim_magenta");
        assert!(names.iter().any(|n| n == "terminal_ansi_cyan"), "缺少字段 terminal_ansi_cyan");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_cyan"), "缺少字段 terminal_ansi_bright_cyan");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_cyan"), "缺少字段 terminal_ansi_dim_cyan");
        assert!(names.iter().any(|n| n == "terminal_ansi_white"), "缺少字段 terminal_ansi_white");
        assert!(names.iter().any(|n| n == "terminal_ansi_bright_white"), "缺少字段 terminal_ansi_bright_white");
        assert!(names.iter().any(|n| n == "terminal_ansi_dim_white"), "缺少字段 terminal_ansi_dim_white");
        assert!(names.iter().any(|n| n == "link_text_hover"), "缺少字段 link_text_hover");
        assert!(names.iter().any(|n| n == "version_control_added"), "缺少字段 version_control_added");
        assert!(names.iter().any(|n| n == "version_control_deleted"), "缺少字段 version_control_deleted");
        assert!(names.iter().any(|n| n == "version_control_modified"), "缺少字段 version_control_modified");
        assert!(names.iter().any(|n| n == "version_control_renamed"), "缺少字段 version_control_renamed");
        assert!(names.iter().any(|n| n == "version_control_conflict"), "缺少字段 version_control_conflict");
        assert!(names.iter().any(|n| n == "version_control_ignored"), "缺少字段 version_control_ignored");
        assert!(names.iter().any(|n| n == "version_control_word_added"), "缺少字段 version_control_word_added");
        assert!(names.iter().any(|n| n == "version_control_word_deleted"), "缺少字段 version_control_word_deleted");
        assert!(names.iter().any(|n| n == "version_control_conflict_marker_ours"), "缺少字段 version_control_conflict_marker_ours");
        assert!(names.iter().any(|n| n == "version_control_conflict_marker_theirs"), "缺少字段 version_control_conflict_marker_theirs");
        assert!(names.iter().any(|n| n == "selection_background"), "缺少字段 selection_background");
        assert!(names.iter().any(|n| n == "editor_cursor"), "缺少字段 editor_cursor");
        assert!(names.iter().any(|n| n == "link"), "缺少字段 link");
    }
}
