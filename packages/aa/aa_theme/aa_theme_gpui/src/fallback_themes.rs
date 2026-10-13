//! 回退主题与「缺失字段补齐」规则，对齐 zed `crates/theme/src/fallback_themes.rs`。
//!
//! ## 用途
//!
//! 主题 JSON 可能载入失败、或目标主题不存在，此时需要一个**兜底主题**
//! 保证界面不空。本模块提供 [`ctp_default_dark`]——它是主题体系的最低
//! 保证，任何情况下都能取到。
//!
//! ## 本模块还放什么
//!
//! CTP 配色（Macchiato / Latte 的 `ThemeColors` / `StatusColors` /
//! `SyntaxTheme`，以及 [`ctp_default_dark`] 的组装）也在这里——它们只服务
//! 兜底路径，与 `default_colors`（色阶 + 基底）职责不同。
//!
//! ## 与 zed 的差异
//!
//! | zed | 我们 |
//! |---|---|
//! | `zed_default_themes()` 注册一个灰阶兜底主题 | [`default_theme_family`] 只注册深色 Macchiato |
//! | `ThemeFamily.scales` 每家族一份 | 也有该字段（`ThemeFamily` 结构对齐），但填的是全局色阶的聚合 |
//!
//! ## 两个补齐规则
//!
//! [`apply_status_color_defaults`] 与 [`apply_theme_color_defaults`]
//! 作用于 `Refineable` 生成的 refinement 类型：主题 JSON 只写部分字段时，
//! 未写的那些按语义**派生**（而不是硬编码一个值）。

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, hsla};

use crate::styles::{PlayerColors, StatusColors, SyntaxTheme, ThemeColors};
use crate::{StatusColorsRefinement, ThemeColorsRefinement};

/// 兜底主题的展示名(对齐 zed `DEFAULT_DARK_THEME` 的角色:找不到就退到它)。
///
/// 用 **Macchiato** 而非 Mocha:基底 `ThemeColors::dark()` 就是 Macchiato，
/// 兜底与基底同风味,免得「找不到主题」时突然换一套观感。
pub const DEFAULT_DARK_THEME: &str = "Catppuccin Macchiato";

/// 兜底主题家族。对应 zed 的 `zed_default_themes()`（`fallback_themes.rs:15`）。
///
/// zed 原版只注册一个灰阶兜底主题；我们只注册深色 Macchiato 一个
/// （[`ctp_default_dark`]）。
///
/// **已知取舍**：浅色系统外观下若资产 JSON 没加载（`LoadThemes::JustBase`、
/// 资产缺失），会退到这个深色主题，UI 变深。接受这个取舍的理由：
/// 生产路径 `LoadThemes::All` 总会装上 `assets/themes/**/*.json`（含 Aa Latte），
/// 兜底家族只在「一个主题都装载不到」时兜底，此时「有得用」比「明暗正确」优先。
///
/// 浅色基底仍在 `ThemeColors::light()`（= Catppuccin Latte 配色，见
/// [`theme_colors_latte`]），只是兜底家族不再单独装一个浅色 Theme。
pub fn default_theme_family() -> crate::ThemeFamily {
    crate::ThemeFamily {
        id: "ui-gpui-fallback".into(),
        name: "ui-gpui Fallback".into(),
        author: "".into(),
        themes: vec![ctp_default_dark()],
        // 与 zed `zed_default_themes()` 同款：家族自带一份色阶。我们的色阶取值
        // 走模块级静态函数（blue()/neutral()…），这里填的是同一份数据的聚合
        // 形式，供消费方按 zed 的形状读。
        scales: crate::default_colors::default_color_scales(),
    }
}

/// 若某状态只给了前景色、没给背景色，则用前景的 25% 透明度版本补上。
///
/// 规则来自 zed：主题作者通常只写 `"error": "#ff0000"`，
/// 背景该由主题系统派生，而不是要求作者再写一遍。
pub fn apply_status_color_defaults(status: &mut StatusColorsRefinement) {
    for (foreground, background) in [
        (&status.conflict, &mut status.conflict_background),
        (&status.created, &mut status.created_background),
        (&status.deleted, &mut status.deleted_background),
        (&status.error, &mut status.error_background),
        (&status.hidden, &mut status.hidden_background),
        (&status.hint, &mut status.hint_background),
        (&status.ignored, &mut status.ignored_background),
        (&status.info, &mut status.info_background),
        (&status.modified, &mut status.modified_background),
        (&status.predictive, &mut status.predictive_background),
        (&status.renamed, &mut status.renamed_background),
        (&status.success, &mut status.success_background),
        (&status.unreachable, &mut status.unreachable_background),
        (&status.warning, &mut status.warning_background),
    ] {
        if background.is_none()
            && let Some(foreground) = foreground
        {
            *background = Some(foreground.opacity(0.25));
        }
    }
}

/// 用协作者配色补齐 `element_selection_background`。
///
/// 选区背景应当“看得出是谁在选”，所以默认取本地玩家的选区色；
/// 若它不透明则压到 25%，避免盖住文字。
pub fn apply_theme_color_defaults(
    theme_colors: &mut ThemeColorsRefinement,
    player_colors: &PlayerColors,
) {
    if theme_colors.element_selection_background.is_none() {
        let mut selection = player_colors.local().selection;
        if selection.a == 1.0 {
            selection.a = 0.25;
        }
        theme_colors.element_selection_background = Some(selection);
    }
}

/// 一次性套用两条补齐规则（主题装载的「派生补齐」阶段）。
pub fn apply_defaults(
    colors: &mut ThemeColorsRefinement,
    status: &mut StatusColorsRefinement,
    players: &PlayerColors,
) {
    apply_status_color_defaults(status);
    apply_theme_color_defaults(colors, players);
}

/// 便捷：非 refinement 版本的状态色补齐——直接改 `StatusColors` 里
/// 仍是全透明的 `*_background` 字段（值为 `Hsla::default()` 时视为未设置）。
pub fn fill_status_backgrounds(status: &mut crate::StatusColors) {
    let transparent = Hsla::default();
    let pairs = [
        (status.conflict, &mut status.conflict_background),
        (status.created, &mut status.created_background),
        (status.deleted, &mut status.deleted_background),
        (status.error, &mut status.error_background),
        (status.hidden, &mut status.hidden_background),
        (status.hint, &mut status.hint_background),
        (status.ignored, &mut status.ignored_background),
        (status.info, &mut status.info_background),
        (status.modified, &mut status.modified_background),
        (status.predictive, &mut status.predictive_background),
        (status.renamed, &mut status.renamed_background),
        (status.success, &mut status.success_background),
        (status.unreachable, &mut status.unreachable_background),
        (status.warning, &mut status.warning_background),
    ];
    for (foreground, background) in pairs {
        if *background == transparent {
            *background = foreground.opacity(0.25);
        }
    }
}

// ── Catppuccin Macchiato ──────────────────────────────────────────────────────
// Macchiato = Mocha 的次深 variant (base #24273A vs Mocha #1E1E2E)。
// 色值来源: https://github.com/catppuccin/palette (macchiato)
/// `0xRRGGBB` → `Hsla`。
pub(crate) fn h(hex: u32) -> Hsla {
    gpui::rgb(hex).into()
}

/// 带透明度的 `Hsla`。
pub(crate) fn ha(hex: u32, a: f32) -> Hsla {
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
pub(crate) fn syntax_macchiato() -> SyntaxTheme {
    SyntaxTheme::new([
        (
            "comment".into(),
            HighlightStyle {
                color: Some(h(0x6e738d)),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
        (
            "keyword".into(),
            HighlightStyle {
                color: Some(h(0xc6a0f6)),
                ..Default::default()
            },
        ),
        (
            "string".into(),
            HighlightStyle {
                color: Some(h(0xa6da95)),
                ..Default::default()
            },
        ),
        (
            "number".into(),
            HighlightStyle {
                color: Some(h(0xf5deb3)),
                ..Default::default()
            },
        ),
        (
            "function".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                ..Default::default()
            },
        ),
        (
            "type".into(),
            HighlightStyle {
                color: Some(h(0x91d7e3)),
                ..Default::default()
            },
        ),
        (
            "constant".into(),
            HighlightStyle {
                color: Some(h(0xf5a97f)),
                ..Default::default()
            },
        ),
        (
            "variable".into(),
            HighlightStyle {
                color: Some(h(0xcad3f5)),
                ..Default::default()
            },
        ),
        (
            "operator".into(),
            HighlightStyle {
                color: Some(h(0x8bd5ca)),
                ..Default::default()
            },
        ),
        (
            "punctuation".into(),
            HighlightStyle {
                color: Some(h(0x9198b0)),
                ..Default::default()
            },
        ),
        (
            "tag".into(),
            HighlightStyle {
                color: Some(h(0xed8796)),
                ..Default::default()
            },
        ),
        (
            "attribute".into(),
            HighlightStyle {
                color: Some(h(0xf5a97f)),
                ..Default::default()
            },
        ),
        (
            "title".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                font_weight: Some(FontWeight::MEDIUM),
                ..Default::default()
            },
        ),
        (
            "link".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                ..Default::default()
            },
        ),
    ])
}

pub fn status_colors_macchiato() -> StatusColors {
    let mut colors = StatusColors::dark();
    let (fg, bg, border) = status(0xed8796); // red
    colors.conflict = fg;
    colors.conflict_background = bg;
    colors.conflict_border = border;
    let (fg, bg, border) = status(0xa6da95); // green
    colors.created = fg;
    colors.created_background = bg;
    colors.created_border = border;
    let (fg, bg, border) = status(0xed8796);
    colors.deleted = fg;
    colors.deleted_background = bg;
    colors.deleted_border = border;
    let (fg, bg, border) = status(0xed8796);
    colors.error = fg;
    colors.error_background = bg;
    colors.error_border = border;
    let (fg, bg, border) = status(0x6e738d); // overlay0 (muted)
    colors.hidden = fg;
    colors.hidden_background = bg;
    colors.hidden_border = border;
    let (fg, bg, border) = status(0x6e738d);
    colors.ignored = fg;
    colors.ignored_background = bg;
    colors.ignored_border = border;
    let (fg, bg, border) = status(0x91d7e3); // sky
    colors.info = fg;
    colors.info_background = bg;
    colors.info_border = border;
    let (fg, bg, border) = status(0xf5a97f); // peach
    colors.modified = fg;
    colors.modified_background = bg;
    colors.modified_border = border;
    let (fg, bg, border) = status(0x8aadf4); // blue
    colors.renamed = fg;
    colors.renamed_background = bg;
    colors.renamed_border = border;
    let (fg, bg, border) = status(0xa6da95); // green
    colors.success = fg;
    colors.success_background = bg;
    colors.success_border = border;
    let (fg, bg, border) = status(0xf5deb3); // yellow
    colors.warning = fg;
    colors.warning_background = bg;
    colors.warning_border = border;
    // — Zed JSON 里 macchiato 额外给了 hint / predictive / unreachable（JSON 给了完整三色，不派生）—
    colors.hint = h(0x5b6078);
    colors.hint_background = h(0x1e2030);
    colors.hint_border = h(0x5b6078);
    colors.predictive = h(0x6e738d);
    colors.predictive_background = h(0x1e2030);
    colors.predictive_border = h(0xb7bdf8);
    colors.unreachable = h(0xed8796);
    colors.unreachable_background = ha(0xed8796, 0.12);
    colors.unreachable_border = h(0xed8796);
    colors
}

/// Catppuccin Macchiato 的 `ThemeColors`（= 深色基底 [`ThemeColors::dark`]）。
///
/// **结构**：`ThemeColors::zed_dark()` 起步，再覆盖 CTP 调色板有的字段。
/// 因此本函数只写 CTP 定义了的角色；**CTP 没定义的角色自动留在 zed 默认
/// 配色上**——这是有意的分层，不是漏配：
///
/// - CTP 是固定调色板（26 个语义色），没有 zed 那种 12 级色阶；
/// - zed 对 vim 模式、diff hunk、code lens、jump label 等角色都有默认值，
///   CTP JSON 则不提供，故这些字段**来自 zed 默认配色**（`ThemeColors::zed_dark()`）。
///
/// 未覆盖字段共 35 个，与 `theme_colors_latte` 的 33 个大致同批
/// （vim_* 全套、editor_diff_hunk_*、editor_code_lens_foreground、
/// editor_highlighted_line_background、editor_hover_line_number、
/// element_selection_background、version_control_word_*、
/// vim_helix_jump_label_foreground、drop_target_border、
/// minimap_thumb_border、panel_overlay_hover），外加
/// scrollbar_thumb_active_background / scrollbar_thumb_border。
///
/// 若日后 CTP 或自定义主题要覆盖这些角色，直接在本函数体内补 `colors.x = …`。
pub fn theme_colors_macchiato() -> ThemeColors {
    let mut colors = ThemeColors::zed_dark();
    {
        colors.border = h(0x363a4f); // surface0
        colors.border_variant = h(0x313148); // surface0-darker
        colors.border_focused = h(0xb7bdf8); // lavender
        colors.border_selected = h(0xc6a0f6); // mauve
        colors.border_disabled = h(0x6e738d); // overlay0
        colors.border_transparent = hsla(0., 0., 0., 0.);
        colors.background = h(0x2c2f46); // base — Zed JSON "background"
        colors.surface_background = h(0x1e2030); // mantle
        colors.elevated_surface_background = h(0x1e2030); // mantle (同 surface)
        colors.element_background = h(0x181926); // crust
        colors.element_hover = ha(0x363a4f, 1.0); // surface0
        colors.element_active = ha(0x5b6078, 0.30); // surface2 @30%
        colors.element_selected = ha(0x363a4f, 0.30); // surface0 @30%
        colors.element_disabled = h(0x6e738d);
        colors.ghost_element_background = hsla(0., 0., 0., 0.);
        colors.ghost_element_hover = ha(0x494d64, 0.30); // surface1 @30%
        colors.ghost_element_active = ha(0x5b6078, 0.60); // surface2 @60%
        colors.ghost_element_selected = ha(0x737995, 0.40); // overlay2 @40%
        colors.ghost_element_disabled = h(0x6e738d);
        colors.text = h(0xcad3f5); // text
        colors.text_muted = h(0xb8c0e0); // subtext0
        colors.text_placeholder = h(0x5b6078); // surface2
        colors.text_disabled = h(0x6e738d); // overlay0
        colors.text_accent = h(0xc6a0f6); // mauve
        colors.icon = h(0xcad3f5);
        colors.icon_muted = h(0x8087a2); // overlay2
        colors.icon_disabled = h(0x6e738d);
        colors.icon_placeholder = h(0x5b6078);
        colors.icon_accent = h(0xc6a0f6);

        // — Zed JSON 显式设置的状态/标题栏/Tab/toolbar（之前缺失！）——
        colors.status_bar_background = h(0x181926); // crust
        colors.title_bar_background = h(0x181926); // crust
        colors.title_bar_inactive_background = h(0x1e1f30); // mantle-darker
        colors.toolbar_background = h(0x24273a); // base (CTP base #24273a)
        colors.tab_bar_background = h(0x181926); // crust
        colors.tab_inactive_background = h(0x12121c); // base-darker
        colors.tab_active_background = h(0x24273a); // base
        colors.search_match_background = ha(0x8bd5ca, 0.30); // teal @30%
        colors.search_active_match_background = ha(0xed8796, 0.30); // red @30%
        colors.panel_background = h(0x1e2030); // mantle
        colors.panel_focused_border = h(0xcad3f5); // text
        colors.panel_indent_guide = ha(0x363a4f, 0.60); // surface0 @60%
        colors.panel_indent_guide_active = h(0x5b6078); // surface2
        colors.panel_indent_guide_hover = h(0xc6a0f6); // mauve
        colors.panel_overlay_background = h(0x1e2030); // mantle
        colors.pane_focused_border = h(0xcad3f5); // text
        colors.pane_group_border = h(0x363a4f); // surface0
        colors.scrollbar_thumb_background = ha(0x5b6078, 0.50); // surface2 @50%
        colors.scrollbar_thumb_hover_background = h(0x6e738d); // overlay0
        colors.scrollbar_track_background = h(0x181926); // crust
        colors.scrollbar_track_border = ha(0xcad3f5, 0.07); // text @7%

        colors.editor_foreground = h(0xcad3f5);
        colors.editor_background = h(0x24273a); // base
        colors.editor_gutter_background = h(0x24273a);
        colors.editor_subheader_background = h(0x1e2030); // mantle
        colors.editor_active_line_background = ha(0xcad3f5, 0.07); // text @7%
        colors.editor_line_number = h(0x8087a2); // overlay2
        colors.editor_active_line_number = h(0xc6a0f6); // mauve
        colors.editor_wrap_guide = h(0x5b6078); // surface2
        colors.editor_indent_guide = ha(0x363a4f, 0.60); // surface0 @60%
        colors.editor_indent_guide_active = h(0x5b6078); // surface2
        colors.editor_invisible = ha(0x939ab7, 0.40); // overlay1 @40%
        colors.editor_debugger_active_line_background = ha(0xf5a97f, 0.07); // peach @7%
        colors.selection_background = ha(0x939ab7, 0.25); // overlay1 @25%
        colors.editor_cursor = h(0xf4dbd6); // rosewater
        colors.editor_document_highlight_read_background = ha(0xa5adcb, 0.16); // subtext1 @16%
        colors.editor_document_highlight_write_background = ha(0xa5adcb, 0.16); // subtext1 @16%
        colors.editor_document_highlight_bracket_background = ha(0xc6a0f6, 0.09); // mauve @9%

        colors.terminal_background = h(0x24273a);
        colors.terminal_foreground = h(0xcad3f5);
        colors.terminal_dim_foreground = h(0x8087a2);
        colors.terminal_bright_foreground = h(0xcad3f5);
        colors.terminal_ansi_background = h(0x24273a);
        colors.terminal_ansi_black = h(0x494d64);
        colors.terminal_ansi_white = h(0xa5adcb);
        colors.terminal_ansi_red = h(0xed8796);
        colors.terminal_ansi_green = h(0xa6da95);
        colors.terminal_ansi_yellow = h(0xeed49f);
        colors.terminal_ansi_blue = h(0x8aadf4);
        colors.terminal_ansi_magenta = h(0xf5bde6);
        colors.terminal_ansi_cyan = h(0x8bd5ca);

        colors.link = h(0x91d7e3); // teal-light

        // — Zed JSON 的 debugger / version_control / minimap / terminal ansi（之前缺失！）—
        colors.debugger_accent = h(0xed8796);
        colors.version_control_added = h(0xa6da95);
        colors.version_control_deleted = h(0xed8796);
        colors.version_control_modified = h(0xeed49f);
        colors.version_control_renamed = h(0x7dc4e4);
        colors.version_control_conflict = h(0xf5a97f);
        colors.version_control_ignored = h(0x6e738d);
        colors.version_control_conflict_marker_ours = ha(0xa6da95, 0.20);
        colors.version_control_conflict_marker_theirs = ha(0x8aadf4, 0.20);
        colors.drop_target_background = ha(0x363a4f, 0.40);
        colors.editor_active_wrap_guide = h(0x5b6078);
        colors.link_text_hover = h(0x91d7e3);
        colors.minimap_thumb_background = ha(0xc6a0f6, 0.20);
        colors.minimap_thumb_hover_background = ha(0xc6a0f6, 0.40);
        colors.minimap_thumb_active_background = ha(0xc6a0f6, 0.60);
        colors.terminal_ansi_bright_black = h(0x5b6078);
        colors.terminal_ansi_bright_white = h(0xb8c0e0);
        colors.terminal_ansi_bright_red = h(0xec7486);
        colors.terminal_ansi_bright_green = h(0x8ccf7f);
        colors.terminal_ansi_bright_yellow = h(0xe1c682);
        colors.terminal_ansi_bright_blue = h(0x78a1f6);
        colors.terminal_ansi_bright_magenta = h(0xf2a9dd);
        colors.terminal_ansi_bright_cyan = h(0x63cbc0);
        colors.terminal_ansi_dim_black = h(0x494d64);
        colors.terminal_ansi_dim_white = h(0xa5adcb);
        colors.terminal_ansi_dim_red = h(0xed8796);
        colors.terminal_ansi_dim_green = h(0xa6da95);
        colors.terminal_ansi_dim_yellow = h(0xeed49f);
        colors.terminal_ansi_dim_blue = h(0x8aadf4);
        colors.terminal_ansi_dim_magenta = h(0xf5bde6);
        colors.terminal_ansi_dim_cyan = h(0x8bd5ca);

        colors
    }
}

/// Catppuccin Macchiato 兜底主题(= 深色基底 [`ThemeColors::dark`])。
///
/// 直接复用 [`default_colors`](crate::default_colors) 的内置主题构造，
/// 不再绕道注册表——兜底要的就是「一定能拿到」。
pub fn ctp_default_dark() -> crate::Theme {
    // 所有值取自 Zed Catppuccin 扩展 JSON:
    // /home/aa/repos/ide_ls/learn_ls/zed/themes/catppuccin-mauve.json → "Catppuccin Macchiato" 段
    //
    // 这就是 `catppuccin_macchiato()`（= 深色基底 ThemeColors::dark()）的组装；
    // 之所以在本文件再写一遍而不是直接调那个构造函数，是因为 `catppuccin_*`
    // 家族构造函数不随基底走——这里是「兜底一定要拿到」的独立保证。
    use crate::state::{Appearance, Theme, ThemeStyles};
    use crate::styles::{AccentColors, PlayerColor, PlayerColors, SystemColors};
    use gpui::WindowBackgroundAppearance;
    use gpui::hsla;
    use std::sync::Arc;

    // Zed JSON "accents" — 7 个,我们扩到 9 个(AccentColors 固定 13 槽位, 多余留 transparent)
    let accents = AccentColors(Arc::from([
        h(0xc6a0f6), // mauve
        h(0xb7bdf8), // lavender
        h(0x7dc4e4), // sky
        h(0xa6da95), // green
        h(0xeed49f), // yellow
        h(0xf5a97f), // peach
        h(0xed8796), // red
        h(0xf5bde6), // pink
        h(0x8bd5ca), // teal
        hsla(0., 0., 0., 0.),
        hsla(0., 0., 0., 0.),
        hsla(0., 0., 0., 0.),
        hsla(0., 0., 0., 0.),
    ]));

    // Zed JSON "players" — 8 个,第一个是本地玩家(cursor=rosewater)
    let players = PlayerColors(vec![
        PlayerColor {
            cursor: h(0xf4dbd6),
            selection: ha(0x939ab7, 0.25),
            background: h(0xf4dbd6),
        },
        PlayerColor {
            cursor: h(0xc6a0f6),
            selection: ha(0xc6a0f6, 0.25),
            background: h(0xc6a0f6),
        },
        PlayerColor {
            cursor: h(0xb7bdf8),
            selection: ha(0xb7bdf8, 0.25),
            background: h(0xb7bdf8),
        },
        PlayerColor {
            cursor: h(0x7dc4e4),
            selection: ha(0x7dc4e4, 0.25),
            background: h(0x7dc4e4),
        },
        PlayerColor {
            cursor: h(0xa6da95),
            selection: ha(0xa6da95, 0.25),
            background: h(0xa6da95),
        },
        PlayerColor {
            cursor: h(0xeed49f),
            selection: ha(0xeed49f, 0.25),
            background: h(0xeed49f),
        },
        PlayerColor {
            cursor: h(0xf5a97f),
            selection: ha(0xf5a97f, 0.25),
            background: h(0xf5a97f),
        },
        PlayerColor {
            cursor: h(0xed8796),
            selection: ha(0xed8796, 0.25),
            background: h(0xed8796),
        },
    ]);

    Theme {
        id: "ctp-macchiato".into(),
        name: DEFAULT_DARK_THEME.into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents,
            players,
            syntax: Arc::new(syntax_macchiato()),
            colors: theme_colors_macchiato(),
            status: status_colors_macchiato(),
        },
    }
}

pub fn status_colors_latte() -> StatusColors {
    // 同 `status_colors_macchiato`（浅色版用 Latte 的 red）。

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
    // — Zed JSON 里 latte 额外给了 hint / predictive / unreachable（JSON 给了完整三色，不派生）—
    colors.hint = h(0xacb0be);
    colors.hint_background = h(0xe6e9ef);
    colors.hint_border = h(0xacb0be);
    colors.predictive = h(0x9ca0b0);
    colors.predictive_background = h(0xe6e9ef);
    colors.predictive_border = h(0x7287fd);
    colors.unreachable = h(0xd20f39);
    colors.unreachable_background = ha(0xd20f39, 0.12);
    colors.unreachable_border = h(0xd20f39);
    colors
}

/// Catppuccin Latte 的 `ThemeColors`（= 浅色基底 [`ThemeColors::light`]）。
///
/// **结构**同 [`theme_colors_macchiato`]：`ThemeColors::zed_light()` 起步，
/// 只覆盖 CTP 定义了的角色；未覆盖的 33 个字段**来自 zed 默认配色**
/// （vim_* 全套、editor_diff_hunk_*、editor_code_lens_foreground、
/// editor_highlighted_line_background、editor_hover_line_number、
/// element_selection_background、version_control_word_*、
/// vim_helix_jump_label_foreground、drop_target_border、
/// minimap_thumb_border、panel_overlay_hover）。
pub fn theme_colors_latte() -> ThemeColors {
    // 同 `theme_colors_macchiato`：zed 默认值为基底 + Catppuccin Latte 覆盖。
    let mut colors = ThemeColors::zed_light();
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
        // — Zed JSON 里 latte 还显式设置了这些（之前缺失，落回 zed 浅色默认值）—
        colors.icon_placeholder = h(0xacb0be);
        colors.status_bar_background = h(0xdce0e8);
        colors.title_bar_background = h(0xdce0e8);
        colors.title_bar_inactive_background = h(0xe6e9ee);
        colors.toolbar_background = h(0xeff1f5);
        colors.tab_bar_background = h(0xdce0e8);
        colors.tab_inactive_background = h(0xd2d7e2);
        colors.tab_active_background = h(0xeff1f5);
        colors.panel_focused_border = h(0x4c4f69);
        colors.panel_indent_guide = ha(0xccd0da, 0.60);
        colors.panel_indent_guide_active = h(0xacb0be);
        colors.panel_indent_guide_hover = h(0x8839ef);
        colors.panel_overlay_background = h(0xe6e9ef);
        colors.drop_target_background = ha(0xccd0da, 0.40);
        colors.minimap_thumb_background = ha(0x8839ef, 0.20);
        colors.minimap_thumb_hover_background = ha(0x8839ef, 0.40);
        colors.minimap_thumb_active_background = ha(0x8839ef, 0.60);
        colors.editor_subheader_background = h(0xe6e9ef);
        colors.editor_active_wrap_guide = h(0xacb0be);
        colors.editor_document_highlight_bracket_background = ha(0x8839ef, 0.09);
        colors.editor_debugger_active_line_background = ha(0xfe640b, 0.07);
        colors.link_text_hover = h(0x04a5e5);
        colors.debugger_accent = h(0xd20f39);
        colors.version_control_added = h(0x40a02b);
        colors.version_control_deleted = h(0xd20f39);
        colors.version_control_modified = h(0xdf8e1d);
        colors.version_control_renamed = h(0x209fb5);
        colors.version_control_conflict = h(0xfe640b);
        colors.version_control_ignored = h(0x9ca0b0);
        colors.version_control_conflict_marker_ours = ha(0x40a02b, 0.20);
        colors.version_control_conflict_marker_theirs = ha(0x1e66f5, 0.20);
        colors.terminal_background = h(0xeff1f5);
        colors.terminal_ansi_background = h(0xeff1f5);
        colors.terminal_foreground = h(0x4c4f69);
        colors.terminal_dim_foreground = h(0x8c8fa1);
        colors.terminal_bright_foreground = h(0x4c4f69);
        colors.terminal_ansi_black = h(0x5c5f77);
        colors.terminal_ansi_white = h(0xacb0be);
        colors.terminal_ansi_red = h(0xd20f39);
        colors.terminal_ansi_green = h(0x40a02b);
        colors.terminal_ansi_yellow = h(0xdf8e1d);
        colors.terminal_ansi_blue = h(0x1e66f5);
        colors.terminal_ansi_magenta = h(0xea76cb);
        colors.terminal_ansi_cyan = h(0x179299);
        colors.terminal_ansi_bright_black = h(0x6c6f85);
        colors.terminal_ansi_bright_white = h(0xbcc0cc);
        colors.terminal_ansi_bright_red = h(0xde293e);
        colors.terminal_ansi_bright_green = h(0x49af3d);
        colors.terminal_ansi_bright_yellow = h(0xeea02d);
        colors.terminal_ansi_bright_blue = h(0x456eff);
        colors.terminal_ansi_bright_magenta = h(0xfe85d8);
        colors.terminal_ansi_bright_cyan = h(0x2d9fa8);
        colors.terminal_ansi_dim_black = h(0x5c5f77);
        colors.terminal_ansi_dim_white = h(0xacb0be);
        colors.terminal_ansi_dim_red = h(0xd20f39);
        colors.terminal_ansi_dim_green = h(0x40a02b);
        colors.terminal_ansi_dim_yellow = h(0xdf8e1d);
        colors.terminal_ansi_dim_blue = h(0x1e66f5);
        colors.terminal_ansi_dim_magenta = h(0xea76cb);
        colors.terminal_ansi_dim_cyan = h(0x179299);

        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::hsla;

    #[test]
    fn fallback_theme_is_the_builtin_catppuccin_dark() {
        let theme = ctp_default_dark();
        assert_eq!(theme.id, "ctp-macchiato");
        assert_eq!(theme.name, DEFAULT_DARK_THEME);
    }

    #[test]
    fn fallback_family_has_only_the_dark_default() {
        let family = default_theme_family();
        let names: Vec<_> = family.themes.iter().map(|t| t.name.as_ref()).collect();
        // 只有一个深色主题：浅色外观在资产缺失时会退到深色（见
        // `default_theme_family` 文档里的取舍说明）。
        assert_eq!(names, ["Catppuccin Macchiato"]);
        assert_eq!(family.themes[0].id, ctp_default_dark().id);
        assert_eq!(family.themes[0].appearance, crate::Appearance::Dark);
    }

    #[test]
    fn missing_background_is_derived_from_foreground() {
        let mut status = StatusColorsRefinement::default();
        status.error = Some(hsla(0.0, 1.0, 0.5, 1.0));
        apply_status_color_defaults(&mut status);
        let background = status.error_background.expect("应派生 background");
        assert_eq!(background.a, 0.25, "派生背景应是 25% 透明");
        assert_eq!(background.h, 0.0, "色相应继承前景");
    }

    #[test]
    fn explicit_background_is_not_overwritten() {
        let mut status = StatusColorsRefinement::default();
        status.error = Some(hsla(0.0, 1.0, 0.5, 1.0));
        status.error_background = Some(hsla(0.5, 1.0, 0.5, 1.0));
        apply_status_color_defaults(&mut status);
        assert_eq!(
            status.error_background.unwrap().h,
            0.5,
            "作者明确给出的背景不应被覆盖"
        );
    }

    #[test]
    fn selection_background_falls_back_to_local_player() {
        let players = PlayerColors::dark();
        let mut colors = ThemeColorsRefinement::default();
        apply_theme_color_defaults(&mut colors, &players);
        let selection = colors.element_selection_background.expect("应派生选区背景");
        assert!(
            selection.a < 1.0,
            "选区背景应半透明，避免盖住文字: {selection:?}"
        );
    }

    #[test]
    fn fill_status_backgrounds_handles_concrete_status_colors() {
        let mut status = crate::StatusColors::dark();
        // 人为清空一个背景，模拟“作者没给”
        status.error_background = Hsla::default();
        fill_status_backgrounds(&mut status);
        assert!(
            status.error_background.a > 0.0,
            "应被前景色补齐: {:?}",
            status.error_background
        );
    }
}
