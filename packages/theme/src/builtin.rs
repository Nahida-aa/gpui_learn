//! 内置主题家族(对齐 zed `fallback_themes.rs` 的角色)。
//!
//! `ui-gpui Default` 采用 Catppuccin 配色(Mocha 深色 / Latte 浅色)——
//! 与本仓库早期 editor 的硬编码色一致,切换到主题系统后视觉不变。
//!
//! 语法色的 capture 名对齐 tree-sitter 的常见捕获(`keyword`/`string`/
//! `comment`/`function`…),将来接高亮查询时直接复用。

use std::sync::Arc;

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, WindowBackgroundAppearance, hsla};

use crate::state::{Appearance, Theme, ThemeFamily, ThemeStyles};
use crate::styles::{
    AccentColors, PlayerColors, StatusColors, SyntaxTheme, SystemColors, ThemeColors,
};

/// `0xRRGGBB` → `Hsla`。
fn h(hex: u32) -> Hsla {
    gpui::rgb(hex).into()
}

/// 带透明度的 `Hsla`。
fn ha(hex: u32, a: f32) -> Hsla {
    let mut c: Hsla = gpui::rgb(hex).into();
    c.a = a;
    c
}

/// 单状态三色组：(前景, 15% 透明底, 边框)。
///
/// zed 的 `StatusColors` 是扁平结构（`error` / `error_background` /
/// `error_border` 三个独立字段），所以这里返回三元组。
fn status(hex: u32) -> (Hsla, Hsla, Hsla) {
    (h(hex), ha(hex, 0.15), h(hex))
}

fn syntax_mocha() -> SyntaxTheme {
    SyntaxTheme::new([
        (
            "comment".into(),
            HighlightStyle {
                color: Some(h(0x6c7086)),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
        (
            "keyword".into(),
            HighlightStyle {
                color: Some(h(0xcba6f7)),
                ..Default::default()
            },
        ),
        (
            "string".into(),
            HighlightStyle {
                color: Some(h(0xa6e3a1)),
                ..Default::default()
            },
        ),
        (
            "number".into(),
            HighlightStyle {
                color: Some(h(0xf9e2af)),
                ..Default::default()
            },
        ),
        (
            "function".into(),
            HighlightStyle {
                color: Some(h(0x89b4fa)),
                ..Default::default()
            },
        ),
        (
            "type".into(),
            HighlightStyle {
                color: Some(h(0x89dceb)),
                ..Default::default()
            },
        ),
        (
            "constant".into(),
            HighlightStyle {
                color: Some(h(0xfab387)),
                ..Default::default()
            },
        ),
        (
            "variable".into(),
            HighlightStyle {
                color: Some(h(0xcdd6f4)),
                ..Default::default()
            },
        ),
        (
            "operator".into(),
            HighlightStyle {
                color: Some(h(0x94e2d5)),
                ..Default::default()
            },
        ),
        (
            "punctuation".into(),
            HighlightStyle {
                color: Some(h(0x9399b2)),
                ..Default::default()
            },
        ),
        (
            "tag".into(),
            HighlightStyle {
                color: Some(h(0xf38ba8)),
                ..Default::default()
            },
        ),
        (
            "attribute".into(),
            HighlightStyle {
                color: Some(h(0xfab387)),
                ..Default::default()
            },
        ),
        (
            "title".into(),
            HighlightStyle {
                color: Some(h(0x89b4fa)),
                font_weight: Some(FontWeight::MEDIUM),
                ..Default::default()
            },
        ),
        (
            "link".into(),
            HighlightStyle {
                color: Some(h(0x89b4fa)),
                ..Default::default()
            },
        ),
    ])
}

fn syntax_latte() -> SyntaxTheme {
    SyntaxTheme::new([
        (
            "comment".into(),
            HighlightStyle {
                color: Some(h(0x9ca0b0)),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
        (
            "keyword".into(),
            HighlightStyle {
                color: Some(h(0x8839ef)),
                ..Default::default()
            },
        ),
        (
            "string".into(),
            HighlightStyle {
                color: Some(h(0x40a02b)),
                ..Default::default()
            },
        ),
        (
            "number".into(),
            HighlightStyle {
                color: Some(h(0xfe640b)),
                ..Default::default()
            },
        ),
        (
            "function".into(),
            HighlightStyle {
                color: Some(h(0x1e66f5)),
                ..Default::default()
            },
        ),
        (
            "type".into(),
            HighlightStyle {
                color: Some(h(0x179299)),
                ..Default::default()
            },
        ),
        (
            "constant".into(),
            HighlightStyle {
                color: Some(h(0xfe640b)),
                ..Default::default()
            },
        ),
        (
            "variable".into(),
            HighlightStyle {
                color: Some(h(0x4c4f69)),
                ..Default::default()
            },
        ),
        (
            "operator".into(),
            HighlightStyle {
                color: Some(h(0x04a5e5)),
                ..Default::default()
            },
        ),
        (
            "punctuation".into(),
            HighlightStyle {
                color: Some(h(0x6c6f85)),
                ..Default::default()
            },
        ),
        (
            "tag".into(),
            HighlightStyle {
                color: Some(h(0xd20f39)),
                ..Default::default()
            },
        ),
        (
            "attribute".into(),
            HighlightStyle {
                color: Some(h(0xfe640b)),
                ..Default::default()
            },
        ),
        (
            "title".into(),
            HighlightStyle {
                color: Some(h(0x1e66f5)),
                font_weight: Some(FontWeight::MEDIUM),
                ..Default::default()
            },
        ),
        (
            "link".into(),
            HighlightStyle {
                color: Some(h(0x1e66f5)),
                ..Default::default()
            },
        ),
    ])
}

pub fn status_colors_mocha() -> StatusColors {
    // 以前景为基底派生 background(15% 透明) / border(同前景),对齐 zed 的派生规则。

    let mut colors = StatusColors::dark();
    let (fg, bg, border) = status(0xf38ba8);
    colors.conflict = fg;
    colors.conflict_background = bg;
    colors.conflict_border = border;
    let (fg, bg, border) = status(0xa6e3a1);
    colors.created = fg;
    colors.created_background = bg;
    colors.created_border = border;
    let (fg, bg, border) = status(0xf38ba8);
    colors.deleted = fg;
    colors.deleted_background = bg;
    colors.deleted_border = border;
    let (fg, bg, border) = status(0xf38ba8);
    colors.error = fg;
    colors.error_background = bg;
    colors.error_border = border;
    let (fg, bg, border) = status(0x6c7086);
    colors.hidden = fg;
    colors.hidden_background = bg;
    colors.hidden_border = border;
    let (fg, bg, border) = status(0x6c7086);
    colors.ignored = fg;
    colors.ignored_background = bg;
    colors.ignored_border = border;
    let (fg, bg, border) = status(0x89dceb);
    colors.info = fg;
    colors.info_background = bg;
    colors.info_border = border;
    let (fg, bg, border) = status(0xfab387);
    colors.modified = fg;
    colors.modified_background = bg;
    colors.modified_border = border;
    let (fg, bg, border) = status(0x89b4fa);
    colors.renamed = fg;
    colors.renamed_background = bg;
    colors.renamed_border = border;
    let (fg, bg, border) = status(0xa6e3a1);
    colors.success = fg;
    colors.success_background = bg;
    colors.success_border = border;
    let (fg, bg, border) = status(0xf9e2af);
    colors.warning = fg;
    colors.warning_background = bg;
    colors.warning_border = border;
    colors
}

pub fn status_colors_latte() -> StatusColors {
    // 同 `status_colors_mocha`。

    let mut colors = StatusColors::light();
    let (fg, bg, border) = status(0xd20f39);
    colors.conflict = fg;
    colors.conflict_background = bg;
    colors.conflict_border = border;
    let (fg, bg, border) = status(0x40a02b);
    colors.created = fg;
    colors.created_background = bg;
    colors.created_border = border;
    let (fg, bg, border) = status(0xd20f39);
    colors.deleted = fg;
    colors.deleted_background = bg;
    colors.deleted_border = border;
    let (fg, bg, border) = status(0xd20f39);
    colors.error = fg;
    colors.error_background = bg;
    colors.error_border = border;
    let (fg, bg, border) = status(0x9ca0b0);
    colors.hidden = fg;
    colors.hidden_background = bg;
    colors.hidden_border = border;
    let (fg, bg, border) = status(0x9ca0b0);
    colors.ignored = fg;
    colors.ignored_background = bg;
    colors.ignored_border = border;
    let (fg, bg, border) = status(0x179299);
    colors.info = fg;
    colors.info_background = bg;
    colors.info_border = border;
    let (fg, bg, border) = status(0xdf8e1d);
    colors.modified = fg;
    colors.modified_background = bg;
    colors.modified_border = border;
    let (fg, bg, border) = status(0x1e66f5);
    colors.renamed = fg;
    colors.renamed_background = bg;
    colors.renamed_border = border;
    let (fg, bg, border) = status(0x40a02b);
    colors.success = fg;
    colors.success_background = bg;
    colors.success_border = border;
    let (fg, bg, border) = status(0xdf8e1d);
    colors.warning = fg;
    colors.warning_background = bg;
    colors.warning_border = border;
    colors
}

pub fn theme_colors_mocha() -> ThemeColors {
    // 以 zed 默认值为基底（`default_colors.rs` 的灰阶配色）：下面只覆盖
    // Catppuccin 有明确取值的字段，其余（终端 ANSI、minimap、panel 等
    // 我们暂无消费方的 92 个色）继承默认，不必逐字段重复一遍。
    let mut colors = ThemeColors::dark();
    {
        // border
        colors.border = h(0x45475a);
        colors.border_variant = h(0x313244);
        colors.border_focused = h(0x89b4fa);
        colors.border_selected = h(0x89b4fa);
        colors.border_disabled = h(0x313244);
        colors.border_transparent = hsla(0., 0., 0., 0.);
        // background
        colors.background = h(0x1e1e2e);
        colors.surface_background = h(0x181825);
        colors.elevated_surface_background = h(0x11111b);
        colors.element_background = h(0x313244);
        colors.element_hover = h(0x45475a);
        colors.element_active = h(0x585b70);
        colors.element_selected = h(0x45475a);
        colors.element_disabled = h(0x313244);
        colors.ghost_element_background = hsla(0., 0., 0., 0.);
        colors.ghost_element_hover = h(0x313244);
        colors.ghost_element_active = h(0x45475a);
        colors.ghost_element_selected = h(0x45475a);
        colors.ghost_element_disabled = hsla(0., 0., 0., 0.);
        // text
        colors.text = h(0xcdd6f4);
        colors.text_muted = h(0xa6adc8);
        colors.text_placeholder = h(0x6c7086);
        colors.text_disabled = h(0x7f849c);
        colors.text_accent = h(0x89b4fa);
        // icon
        colors.icon = h(0xcdd6f4);
        colors.icon_muted = h(0xa6adc8);
        colors.icon_disabled = h(0x585b70);
        colors.icon_accent = h(0x89b4fa);
        // editor
        colors.editor_foreground = h(0xcdd6f4);
        colors.editor_background = h(0x1e1e2e);
        colors.editor_gutter_background = h(0x1e1e2e);
        colors.editor_active_line_background = h(0x313244);
        colors.editor_line_number = h(0x6c7086);
        colors.editor_active_line_number = h(0xcdd6f4);
        colors.editor_wrap_guide = h(0x313244);
        colors.editor_indent_guide = h(0x313244);
        colors.editor_indent_guide_active = h(0x45475a);
        colors.editor_invisible = h(0x6c7086);
        colors.selection_background = h(0x45475a);
        colors.editor_cursor = h(0xf5e0dc);
        colors.editor_document_highlight_read_background = ha(0x89b4fa, 0.15);
        colors.editor_document_highlight_write_background = ha(0x89b4fa, 0.25);
        // 面板 / 杂项
        colors.panel_background = h(0x181825);
        colors.pane_focused_border = h(0x89b4fa);
        colors.pane_group_border = h(0x45475a);
        colors.search_match_background = h(0x45475a);
        colors.search_active_match_background = h(0x585b70);
        colors.scrollbar_thumb_background = h(0x585b70);
        colors.scrollbar_thumb_hover_background = h(0x6c7086);
        colors.scrollbar_thumb_active_background = h(0x89b4fa);
        colors.scrollbar_thumb_border = hsla(0., 0., 0., 0.);
        colors.scrollbar_track_background = h(0x181825);
        colors.scrollbar_track_border = hsla(0., 0., 0., 0.);
        colors.link = h(0x89b4fa);

        colors
    }
}

pub fn theme_colors_latte() -> ThemeColors {
    // 同 `theme_colors_mocha`：zed 默认值为基底 + Catppuccin Latte 覆盖。
    let mut colors = ThemeColors::light();
    {
        colors.border = h(0xccd0da);
        colors.border_variant = h(0xbcc0cc);
        colors.border_focused = h(0x1e66f5);
        colors.border_selected = h(0x1e66f5);
        colors.border_disabled = h(0xccd0da);
        colors.border_transparent = hsla(0., 0., 0., 0.);
        colors.background = h(0xeff1f5);
        colors.surface_background = h(0xe6e9ef);
        colors.elevated_surface_background = h(0xffffff);
        colors.element_background = h(0xccd0da);
        colors.element_hover = h(0xbcc0cc);
        colors.element_active = h(0xacb0be);
        colors.element_selected = h(0xbcc0cc);
        colors.element_disabled = h(0xccd0da);
        colors.ghost_element_background = hsla(0., 0., 0., 0.);
        colors.ghost_element_hover = h(0xccd0da);
        colors.ghost_element_active = h(0xbcc0cc);
        colors.ghost_element_selected = h(0xbcc0cc);
        colors.ghost_element_disabled = hsla(0., 0., 0., 0.);
        colors.text = h(0x4c4f69);
        colors.text_muted = h(0x6c6f85);
        colors.text_placeholder = h(0x9ca0b0);
        colors.text_disabled = h(0x8c8fa1);
        colors.text_accent = h(0x1e66f5);
        colors.icon = h(0x4c4f69);
        colors.icon_muted = h(0x6c6f85);
        colors.icon_disabled = h(0xacb0be);
        colors.icon_accent = h(0x1e66f5);
        colors.editor_foreground = h(0x4c4f69);
        colors.editor_background = h(0xeff1f5);
        colors.editor_gutter_background = h(0xeff1f5);
        colors.editor_active_line_background = h(0xccd0da);
        colors.editor_line_number = h(0x9ca0b0);
        colors.editor_active_line_number = h(0x4c4f69);
        colors.editor_wrap_guide = h(0xccd0da);
        colors.editor_indent_guide = h(0xccd0da);
        colors.editor_indent_guide_active = h(0xbcc0cc);
        colors.editor_invisible = h(0x9ca0b0);
        colors.selection_background = h(0xbcc0cc);
        colors.editor_cursor = h(0xdc8a78);
        colors.editor_document_highlight_read_background = ha(0x1e66f5, 0.12);
        colors.editor_document_highlight_write_background = ha(0x1e66f5, 0.2);
        colors.panel_background = h(0xe6e9ef);
        colors.pane_focused_border = h(0x1e66f5);
        colors.pane_group_border = h(0xccd0da);
        colors.search_match_background = h(0xccd0da);
        colors.search_active_match_background = h(0xbcc0cc);
        colors.scrollbar_thumb_background = h(0xacb0be);
        colors.scrollbar_thumb_hover_background = h(0x9ca0b0);
        colors.scrollbar_thumb_active_background = h(0x1e66f5);
        colors.scrollbar_thumb_border = hsla(0., 0., 0., 0.);
        colors.scrollbar_track_background = h(0xe6e9ef);
        colors.scrollbar_track_border = hsla(0., 0., 0., 0.);
        colors.link = h(0x1e66f5);

        colors
    }
}

/// Catppuccin Mocha(深色)。
fn catppuccin_mocha() -> Theme {
    Theme {
        id: "ui-gpui-default-dark".into(),
        name: "ui-gpui Dark".into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors::dark(),
            players: PlayerColors::dark(),
            syntax: Arc::new(syntax_mocha()),
            colors: theme_colors_mocha(),
            status: status_colors_mocha(),
        },
    }
}

/// Catppuccin Latte(浅色)。
fn catppuccin_latte() -> Theme {
    Theme {
        id: "ui-gpui-default-light".into(),
        name: "ui-gpui Light".into(),
        appearance: Appearance::Light,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors::light(),
            players: PlayerColors::light(),
            syntax: Arc::new(syntax_latte()),
            colors: theme_colors_latte(),
            status: status_colors_latte(),
        },
    }
}

/// 内置主题注册表(对齐 zed `ThemeRegistry` 的精简版):
/// 内置主题 + 运行时 `insert`。查找按 id / 名称。
#[derive(Clone, Default)]
pub struct ThemeRegistry {
    themes: Vec<Theme>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个主题。
    pub fn insert(&mut self, theme: Theme) {
        self.themes.push(theme);
    }

    /// 注册整个主题家族（来自主题扩展 JSON）。
    ///
    /// id 已存在的主题跳过：同一家族多个文件（如 catppuccin 的
    /// =mauve/-no-italics）重名时先注册的胜出。
    pub fn load_theme_family(&mut self, family: ThemeFamily) {
        for theme in family.themes {
            if self.get(&theme.id).is_none() {
                self.insert(theme);
            }
        }
    }

    /// 按 id 或名称查找。
    pub fn get(&self, id_or_name: &str) -> Option<&Theme> {
        self.themes
            .iter()
            .find(|t| t.id == id_or_name || t.name == id_or_name)
    }

    /// 全部主题。
    pub fn themes(&self) -> &[Theme] {
        &self.themes
    }

    /// 内置主题集。
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.insert(catppuccin_mocha());
        registry.insert(catppuccin_latte());
        registry
    }
}

/// 内置主题家族:"ui-gpui Default"(Catppuccin Mocha / Latte 成对)。
pub fn default_theme_family() -> ThemeFamily {
    ThemeFamily {
        id: "ui-gpui-default".into(),
        name: "ui-gpui Default".into(),
        author: String::new(),
        themes: vec![catppuccin_mocha(), catppuccin_latte()],
    }
}
