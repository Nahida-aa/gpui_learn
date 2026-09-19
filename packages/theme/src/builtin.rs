//! 内置主题家族(对齐 zed `fallback_themes.rs` 的角色)。
//!
//! `ui-gpui Default` 采用 Catppuccin 配色(Mocha 深色 / Latte 浅色)——
//! 与本仓库早期 editor 的硬编码色一致,切换到主题系统后视觉不变。
//!
//! 语法色的 capture 名对齐 tree-sitter 的常见捕获(`keyword`/`string`/
//! `comment`/`function`…),将来接高亮查询时直接复用。

use std::sync::Arc;

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, hsla};

use crate::state::{Appearance, Theme, ThemeFamily, ThemeStyles};
use crate::styles::{
    AccentColors, PlayerColors, StatusColor, StatusColors, SyntaxTheme, SystemColors, ThemeColors,
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

/// 单状态三色组:base / 15% 透明底 / 边框即 base。
fn status(hex: u32) -> StatusColor {
    StatusColor {
        base: h(hex),
        background: ha(hex, 0.15),
        border: h(hex),
    }
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

pub(crate) fn status_colors_mocha() -> StatusColors {
    StatusColors {
        conflict: status(0xf38ba8),
        created: status(0xa6e3a1),
        deleted: status(0xf38ba8),
        error: status(0xf38ba8),
        hidden: status(0x6c7086),
        ignored: status(0x6c7086),
        info: status(0x89dceb),
        modified: status(0xfab387),
        renamed: status(0x89b4fa),
        success: status(0xa6e3a1),
        warning: status(0xf9e2af),
    }
}

pub(crate) fn status_colors_latte() -> StatusColors {
    StatusColors {
        conflict: status(0xd20f39),
        created: status(0x40a02b),
        deleted: status(0xd20f39),
        error: status(0xd20f39),
        hidden: status(0x9ca0b0),
        ignored: status(0x9ca0b0),
        info: status(0x179299),
        modified: status(0xdf8e1d),
        renamed: status(0x1e66f5),
        success: status(0x40a02b),
        warning: status(0xdf8e1d),
    }
}

pub(crate) fn theme_colors_mocha() -> ThemeColors {
    ThemeColors {
        // border
        border: h(0x45475a),
        border_variant: h(0x313244),
        border_focused: h(0x89b4fa),
        border_selected: h(0x89b4fa),
        border_disabled: h(0x313244),
        border_transparent: hsla(0., 0., 0., 0.),
        // background
        background: h(0x1e1e2e),
        surface_background: h(0x181825),
        elevated_surface_background: h(0x11111b),
        element_background: h(0x313244),
        element_hover: h(0x45475a),
        element_active: h(0x585b70),
        element_selected: h(0x45475a),
        element_disabled: h(0x313244),
        ghost_element_background: hsla(0., 0., 0., 0.),
        ghost_element_hover: h(0x313244),
        ghost_element_active: h(0x45475a),
        ghost_element_selected: h(0x45475a),
        ghost_element_disabled: hsla(0., 0., 0., 0.),
        // text
        text: h(0xcdd6f4),
        text_muted: h(0xa6adc8),
        text_placeholder: h(0x6c7086),
        text_disabled: h(0x7f849c),
        text_accent: h(0x89b4fa),
        // icon
        icon: h(0xcdd6f4),
        icon_muted: h(0xa6adc8),
        icon_disabled: h(0x585b70),
        icon_accent: h(0x89b4fa),
        // editor
        editor_foreground: h(0xcdd6f4),
        editor_background: h(0x1e1e2e),
        editor_gutter_background: h(0x1e1e2e),
        editor_active_line_background: h(0x313244),
        editor_line_number: h(0x6c7086),
        editor_active_line_number: h(0xcdd6f4),
        editor_wrap_guide: h(0x313244),
        editor_indent_guide: h(0x313244),
        editor_indent_guide_active: h(0x45475a),
        editor_invisible: h(0x6c7086),
        selection_background: h(0x45475a),
        editor_cursor: h(0xf5e0dc),
        editor_document_highlight_read_background: ha(0x89b4fa, 0.15),
        editor_document_highlight_write_background: ha(0x89b4fa, 0.25),
        // 面板 / 杂项
        panel_background: h(0x181825),
        pane_focused_border: h(0x89b4fa),
        pane_group_border: h(0x45475a),
        search_match_background: h(0x45475a),
        search_active_match_background: h(0x585b70),
        scrollbar_thumb_background: h(0x585b70),
        scrollbar_thumb_hover_background: h(0x6c7086),
        scrollbar_thumb_active_background: h(0x89b4fa),
        scrollbar_thumb_border: hsla(0., 0., 0., 0.),
        scrollbar_track_background: h(0x181825),
        scrollbar_track_border: hsla(0., 0., 0., 0.),
        link: h(0x89b4fa),
    }
}

pub(crate) fn theme_colors_latte() -> ThemeColors {
    ThemeColors {
        border: h(0xccd0da),
        border_variant: h(0xbcc0cc),
        border_focused: h(0x1e66f5),
        border_selected: h(0x1e66f5),
        border_disabled: h(0xccd0da),
        border_transparent: hsla(0., 0., 0., 0.),
        background: h(0xeff1f5),
        surface_background: h(0xe6e9ef),
        elevated_surface_background: h(0xffffff),
        element_background: h(0xccd0da),
        element_hover: h(0xbcc0cc),
        element_active: h(0xacb0be),
        element_selected: h(0xbcc0cc),
        element_disabled: h(0xccd0da),
        ghost_element_background: hsla(0., 0., 0., 0.),
        ghost_element_hover: h(0xccd0da),
        ghost_element_active: h(0xbcc0cc),
        ghost_element_selected: h(0xbcc0cc),
        ghost_element_disabled: hsla(0., 0., 0., 0.),
        text: h(0x4c4f69),
        text_muted: h(0x6c6f85),
        text_placeholder: h(0x9ca0b0),
        text_disabled: h(0x8c8fa1),
        text_accent: h(0x1e66f5),
        icon: h(0x4c4f69),
        icon_muted: h(0x6c6f85),
        icon_disabled: h(0xacb0be),
        icon_accent: h(0x1e66f5),
        editor_foreground: h(0x4c4f69),
        editor_background: h(0xeff1f5),
        editor_gutter_background: h(0xeff1f5),
        editor_active_line_background: h(0xccd0da),
        editor_line_number: h(0x9ca0b0),
        editor_active_line_number: h(0x4c4f69),
        editor_wrap_guide: h(0xccd0da),
        editor_indent_guide: h(0xccd0da),
        editor_indent_guide_active: h(0xbcc0cc),
        editor_invisible: h(0x9ca0b0),
        selection_background: h(0xbcc0cc),
        editor_cursor: h(0xdc8a78),
        editor_document_highlight_read_background: ha(0x1e66f5, 0.12),
        editor_document_highlight_write_background: ha(0x1e66f5, 0.2),
        panel_background: h(0xe6e9ef),
        pane_focused_border: h(0x1e66f5),
        pane_group_border: h(0xccd0da),
        search_match_background: h(0xccd0da),
        search_active_match_background: h(0xbcc0cc),
        scrollbar_thumb_background: h(0xacb0be),
        scrollbar_thumb_hover_background: h(0x9ca0b0),
        scrollbar_thumb_active_background: h(0x1e66f5),
        scrollbar_thumb_border: hsla(0., 0., 0., 0.),
        scrollbar_track_background: h(0xe6e9ef),
        scrollbar_track_border: hsla(0., 0., 0., 0.),
        link: h(0x1e66f5),
    }
}

/// Catppuccin Mocha(深色)。
fn catppuccin_mocha() -> Theme {
    Theme {
        id: "ui-gpui-default-dark".into(),
        name: "ui-gpui Dark".into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            system: SystemColors::default(),
            accents: AccentColors::default(),
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
            system: SystemColors::default(),
            accents: AccentColors::default(),
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
        name: "ui-gpui Default".into(),
        themes: vec![catppuccin_mocha(), catppuccin_latte()],
    }
}
