//! 主题颜色 settings 类型。
//!
//! - [`ThemeColor`] — 单个颜色值（transparent String）
//! - [`ThemeColorsContent`] — 完整颜色集合（UI + Editor + Terminal + VCS + Vim，190+ 字段）

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

// ---------- ThemeColor ----------

/// 单个主题颜色值（transparent String，serde 校验 hex 格式）。
///
/// 对齐 Zed `ThemeColor`。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, MergeFrom)]
#[serde(transparent)]
pub struct ThemeColor(String);

impl JsonSchema for ThemeColor {
    fn schema_name() -> Cow<'static, str> { "Color".into() }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;
        json_schema!({
            "type": "string",
            "pattern": "^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$",
            "format": "color"
        })
    }
}

impl From<String> for ThemeColor {
    fn from(value: String) -> Self { ThemeColor(value) }
}

impl From<&str> for ThemeColor {
    fn from(value: &str) -> Self { ThemeColor(value.to_string()) }
}

impl Deref for ThemeColor {
    type Target = str;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<ThemeColor> for String {
    fn from(value: ThemeColor) -> Self { value.0 }
}

impl Display for ThemeColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

impl TryFrom<&ThemeColor> for gpui::Rgba {
    type Error = anyhow::Error;

    fn try_from(value: &ThemeColor) -> anyhow::Result<Self> {
        let s: &str = &value.0;
        gpui::Rgba::try_from(s)
    }
}

impl ThemeColor {
    pub fn as_str(&self) -> &str { &self.0 }
}

/// 主题颜色完整集合（UI + Editor + Terminal + VCS + Vim）。
///
/// 字段和 serde rename **完全对齐 Zed** `settings_content::ThemeColorsContent`。
/// 所有字段都是 `Option<ThemeColor>`，None 表示使用内建默认。
#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, MergeFrom)]
#[serde(default)]
pub struct ThemeColorsContent {
    // ---- border ----
    /// 通用边框颜色。
    #[serde(rename = "border")]
    pub border: Option<ThemeColor>,
    /// 弱化边框（section divider）。
    #[serde(rename = "border.variant")]
    pub border_variant: Option<ThemeColor>,
    /// 聚焦元素边框（keyboard focused list item）。
    #[serde(rename = "border.focused")]
    pub border_focused: Option<ThemeColor>,
    /// 选中元素边框（active search filter / selected checkbox）。
    #[serde(rename = "border.selected")]
    pub border_selected: Option<ThemeColor>,
    /// 透明占位边框。
    #[serde(rename = "border.transparent")]
    pub border_transparent: Option<ThemeColor>,
    /// 禁用元素边框。
    #[serde(rename = "border.disabled")]
    pub border_disabled: Option<ThemeColor>,

    // ---- background / surface ----
    /// 提升的 surface（context menu / popup / dialog）。
    #[serde(rename = "elevated_surface.background")]
    pub elevated_surface_background: Option<ThemeColor>,
    /// grounded surface（panel / tab）。
    #[serde(rename = "surface.background")]
    pub surface_background: Option<ThemeColor>,
    /// 应用主背景。
    #[serde(rename = "background")]
    pub background: Option<ThemeColor>,

    // ---- element ----
    #[serde(rename = "element.background")]
    pub element_background: Option<ThemeColor>,
    #[serde(rename = "element.hover")]
    pub element_hover: Option<ThemeColor>,
    #[serde(rename = "element.active")]
    pub element_active: Option<ThemeColor>,
    #[serde(rename = "element.selected")]
    pub element_selected: Option<ThemeColor>,
    #[serde(rename = "element.disabled")]
    pub element_disabled: Option<ThemeColor>,
    #[serde(rename = "element.selection_background")]
    pub element_selection_background: Option<ThemeColor>,

    // ---- drop target ----
    #[serde(rename = "drop_target.background")]
    pub drop_target_background: Option<ThemeColor>,
    #[serde(rename = "drop_target.border")]
    pub drop_target_border: Option<ThemeColor>,

    // ---- ghost element ----
    #[serde(rename = "ghost_element.background")]
    pub ghost_element_background: Option<ThemeColor>,
    #[serde(rename = "ghost_element.hover")]
    pub ghost_element_hover: Option<ThemeColor>,
    #[serde(rename = "ghost_element.active")]
    pub ghost_element_active: Option<ThemeColor>,
    #[serde(rename = "ghost_element.selected")]
    pub ghost_element_selected: Option<ThemeColor>,
    #[serde(rename = "ghost_element.disabled")]
    pub ghost_element_disabled: Option<ThemeColor>,

    // ---- text ----
    #[serde(rename = "text")]
    pub text: Option<ThemeColor>,
    #[serde(rename = "text.muted")]
    pub text_muted: Option<ThemeColor>,
    #[serde(rename = "text.placeholder")]
    pub text_placeholder: Option<ThemeColor>,
    #[serde(rename = "text.disabled")]
    pub text_disabled: Option<ThemeColor>,
    #[serde(rename = "text.accent")]
    pub text_accent: Option<ThemeColor>,

    // ---- icon ----
    #[serde(rename = "icon")]
    pub icon: Option<ThemeColor>,
    #[serde(rename = "icon.muted")]
    pub icon_muted: Option<ThemeColor>,
    #[serde(rename = "icon.disabled")]
    pub icon_disabled: Option<ThemeColor>,
    #[serde(rename = "icon.placeholder")]
    pub icon_placeholder: Option<ThemeColor>,
    #[serde(rename = "icon.accent")]
    pub icon_accent: Option<ThemeColor>,

    // ---- debugger ----
    #[serde(rename = "debugger.accent")]
    pub debugger_accent: Option<ThemeColor>,

    // ---- status / title / toolbar / tab / search ----
    #[serde(rename = "status_bar.background")]
    pub status_bar_background: Option<ThemeColor>,
    #[serde(rename = "title_bar.background")]
    pub title_bar_background: Option<ThemeColor>,
    #[serde(rename = "title_bar.inactive_background")]
    pub title_bar_inactive_background: Option<ThemeColor>,
    #[serde(rename = "toolbar.background")]
    pub toolbar_background: Option<ThemeColor>,
    #[serde(rename = "tab_bar.background")]
    pub tab_bar_background: Option<ThemeColor>,
    #[serde(rename = "tab.inactive_background")]
    pub tab_inactive_background: Option<ThemeColor>,
    #[serde(rename = "tab.active_background")]
    pub tab_active_background: Option<ThemeColor>,
    #[serde(rename = "search.match_background")]
    pub search_match_background: Option<ThemeColor>,
    #[serde(rename = "search.active_match_background")]
    pub search_active_match_background: Option<ThemeColor>,

    // ---- panel ----
    #[serde(rename = "panel.background")]
    pub panel_background: Option<ThemeColor>,
    #[serde(rename = "panel.focused_border")]
    pub panel_focused_border: Option<ThemeColor>,
    #[serde(rename = "panel.indent_guide")]
    pub panel_indent_guide: Option<ThemeColor>,
    #[serde(rename = "panel.indent_guide_hover")]
    pub panel_indent_guide_hover: Option<ThemeColor>,
    #[serde(rename = "panel.indent_guide_active")]
    pub panel_indent_guide_active: Option<ThemeColor>,
    #[serde(rename = "panel.overlay_background")]
    pub panel_overlay_background: Option<ThemeColor>,
    #[serde(rename = "panel.overlay_hover")]
    pub panel_overlay_hover: Option<ThemeColor>,

    // ---- pane ----
    #[serde(rename = "pane.focused_border")]
    pub pane_focused_border: Option<ThemeColor>,
    #[serde(rename = "pane_group.border")]
    pub pane_group_border: Option<ThemeColor>,

    // ---- scrollbar ----
    /// 已废弃：`scrollbar_thumb.background`。
    #[serde(rename = "scrollbar_thumb.background", skip_serializing)]
    pub deprecated_scrollbar_thumb_background: Option<ThemeColor>,
    #[serde(rename = "scrollbar.thumb.background")]
    pub scrollbar_thumb_background: Option<ThemeColor>,
    #[serde(rename = "scrollbar.thumb.hover_background")]
    pub scrollbar_thumb_hover_background: Option<ThemeColor>,
    #[serde(rename = "scrollbar.thumb.active_background")]
    pub scrollbar_thumb_active_background: Option<ThemeColor>,
    #[serde(rename = "scrollbar.thumb.border")]
    pub scrollbar_thumb_border: Option<ThemeColor>,
    #[serde(rename = "scrollbar.track.background")]
    pub scrollbar_track_background: Option<ThemeColor>,
    #[serde(rename = "scrollbar.track.border")]
    pub scrollbar_track_border: Option<ThemeColor>,

    // ---- minimap ----
    #[serde(rename = "minimap.thumb.background")]
    pub minimap_thumb_background: Option<ThemeColor>,
    #[serde(rename = "minimap.thumb.hover_background")]
    pub minimap_thumb_hover_background: Option<ThemeColor>,
    #[serde(rename = "minimap.thumb.active_background")]
    pub minimap_thumb_active_background: Option<ThemeColor>,
    #[serde(rename = "minimap.thumb.border")]
    pub minimap_thumb_border: Option<ThemeColor>,

    // ---- editor ----
    #[serde(rename = "editor.foreground")]
    pub editor_foreground: Option<ThemeColor>,
    #[serde(rename = "editor.code_lens.foreground")]
    pub editor_code_lens_foreground: Option<ThemeColor>,
    #[serde(rename = "editor.background")]
    pub editor_background: Option<ThemeColor>,
    #[serde(rename = "editor.gutter.background")]
    pub editor_gutter_background: Option<ThemeColor>,
    #[serde(rename = "editor.subheader.background")]
    pub editor_subheader_background: Option<ThemeColor>,
    #[serde(rename = "editor.active_line.background")]
    pub editor_active_line_background: Option<ThemeColor>,
    #[serde(rename = "editor.highlighted_line.background")]
    pub editor_highlighted_line_background: Option<ThemeColor>,
    #[serde(rename = "editor.debugger_active_line.background")]
    pub editor_debugger_active_line_background: Option<ThemeColor>,
    #[serde(rename = "editor.line_number")]
    pub editor_line_number: Option<ThemeColor>,
    #[serde(rename = "editor.active_line_number")]
    pub editor_active_line_number: Option<ThemeColor>,
    #[serde(rename = "editor.hover_line_number")]
    pub editor_hover_line_number: Option<ThemeColor>,
    #[serde(rename = "editor.invisible")]
    pub editor_invisible: Option<ThemeColor>,
    #[serde(rename = "editor.wrap_guide")]
    pub editor_wrap_guide: Option<ThemeColor>,
    #[serde(rename = "editor.active_wrap_guide")]
    pub editor_active_wrap_guide: Option<ThemeColor>,
    #[serde(rename = "editor.indent_guide")]
    pub editor_indent_guide: Option<ThemeColor>,
    #[serde(rename = "editor.indent_guide_active")]
    pub editor_indent_guide_active: Option<ThemeColor>,
    #[serde(rename = "editor.document_highlight.read_background")]
    pub editor_document_highlight_read_background: Option<ThemeColor>,
    #[serde(rename = "editor.document_highlight.write_background")]
    pub editor_document_highlight_write_background: Option<ThemeColor>,
    #[serde(rename = "editor.document_highlight.bracket_background")]
    pub editor_document_highlight_bracket_background: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.added.background")]
    pub editor_diff_hunk_added_background: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.added.hollow_background")]
    pub editor_diff_hunk_added_hollow_background: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.added.hollow_border")]
    pub editor_diff_hunk_added_hollow_border: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.deleted.background")]
    pub editor_diff_hunk_deleted_background: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.deleted.hollow_background")]
    pub editor_diff_hunk_deleted_hollow_background: Option<ThemeColor>,
    #[serde(rename = "editor.diff_hunk.deleted.hollow_border")]
    pub editor_diff_hunk_deleted_hollow_border: Option<ThemeColor>,

    // ---- terminal ----
    #[serde(rename = "terminal.background")]
    pub terminal_background: Option<ThemeColor>,
    #[serde(rename = "terminal.foreground")]
    pub terminal_foreground: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.background")]
    pub terminal_ansi_background: Option<ThemeColor>,
    #[serde(rename = "terminal.bright_foreground")]
    pub terminal_bright_foreground: Option<ThemeColor>,
    #[serde(rename = "terminal.dim_foreground")]
    pub terminal_dim_foreground: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.black")]
    pub terminal_ansi_black: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_black")]
    pub terminal_ansi_bright_black: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_black")]
    pub terminal_ansi_dim_black: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.red")]
    pub terminal_ansi_red: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_red")]
    pub terminal_ansi_bright_red: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_red")]
    pub terminal_ansi_dim_red: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.green")]
    pub terminal_ansi_green: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_green")]
    pub terminal_ansi_bright_green: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_green")]
    pub terminal_ansi_dim_green: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.yellow")]
    pub terminal_ansi_yellow: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_yellow")]
    pub terminal_ansi_bright_yellow: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_yellow")]
    pub terminal_ansi_dim_yellow: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.blue")]
    pub terminal_ansi_blue: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_blue")]
    pub terminal_ansi_bright_blue: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_blue")]
    pub terminal_ansi_dim_blue: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.magenta")]
    pub terminal_ansi_magenta: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_magenta")]
    pub terminal_ansi_bright_magenta: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_magenta")]
    pub terminal_ansi_dim_magenta: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.cyan")]
    pub terminal_ansi_cyan: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_cyan")]
    pub terminal_ansi_bright_cyan: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_cyan")]
    pub terminal_ansi_dim_cyan: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.white")]
    pub terminal_ansi_white: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.bright_white")]
    pub terminal_ansi_bright_white: Option<ThemeColor>,
    #[serde(rename = "terminal.ansi.dim_white")]
    pub terminal_ansi_dim_white: Option<ThemeColor>,

    // ---- link / vcs ----
    #[serde(rename = "link_text.hover")]
    pub link_text_hover: Option<ThemeColor>,
    #[serde(rename = "version_control.added")]
    pub version_control_added: Option<ThemeColor>,
    #[serde(rename = "version_control.deleted")]
    pub version_control_deleted: Option<ThemeColor>,
    #[serde(rename = "version_control.modified")]
    pub version_control_modified: Option<ThemeColor>,
    #[serde(rename = "version_control.renamed")]
    pub version_control_renamed: Option<ThemeColor>,
    #[serde(rename = "version_control.conflict")]
    pub version_control_conflict: Option<ThemeColor>,
    #[serde(rename = "version_control.ignored")]
    pub version_control_ignored: Option<ThemeColor>,
    #[serde(rename = "version_control.word_added")]
    pub version_control_word_added: Option<ThemeColor>,
    #[serde(rename = "version_control.word_deleted")]
    pub version_control_word_deleted: Option<ThemeColor>,
    #[serde(rename = "version_control.conflict_marker.ours")]
    pub version_control_conflict_marker_ours: Option<ThemeColor>,
    #[serde(rename = "version_control.conflict_marker.theirs")]
    pub version_control_conflict_marker_theirs: Option<ThemeColor>,
    /// 已废弃：用 conflict_marker.ours。
    #[deprecated]
    pub version_control_conflict_ours_background: Option<ThemeColor>,
    /// 已废弃：用 conflict_marker.theirs。
    #[deprecated]
    pub version_control_conflict_theirs_background: Option<ThemeColor>,

    // ---- vim ----
    #[serde(rename = "vim.normal.background")]
    pub vim_normal_background: Option<ThemeColor>,
    #[serde(rename = "vim.insert.background")]
    pub vim_insert_background: Option<ThemeColor>,
    #[serde(rename = "vim.replace.background")]
    pub vim_replace_background: Option<ThemeColor>,
    #[serde(rename = "vim.visual.background")]
    pub vim_visual_background: Option<ThemeColor>,
    #[serde(rename = "vim.visual_line.background")]
    pub vim_visual_line_background: Option<ThemeColor>,
    #[serde(rename = "vim.visual_block.background")]
    pub vim_visual_block_background: Option<ThemeColor>,
    #[serde(rename = "vim.yank.background")]
    pub vim_yank_background: Option<ThemeColor>,
    #[serde(rename = "vim.helix_jump_label.foreground")]
    pub vim_helix_jump_label_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.helix_normal.background")]
    pub vim_helix_normal_background: Option<ThemeColor>,
    #[serde(rename = "vim.helix_select.background")]
    pub vim_helix_select_background: Option<ThemeColor>,
    #[serde(rename = "vim.normal.foreground")]
    pub vim_normal_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.insert.foreground")]
    pub vim_insert_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.replace.foreground")]
    pub vim_replace_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.visual.foreground")]
    pub vim_visual_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.visual_line.foreground")]
    pub vim_visual_line_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.visual_block.foreground")]
    pub vim_visual_block_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.helix_normal.foreground")]
    pub vim_helix_normal_foreground: Option<ThemeColor>,
    #[serde(rename = "vim.helix_select.foreground")]
    pub vim_helix_select_foreground: Option<ThemeColor>,
}
