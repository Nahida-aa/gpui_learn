//! theme/loaders：把 zed 主题扩展 JSON（[`content`]）转成本仓库的 [`Theme`]。
//!
//! 分两步：
//!
//! 1. [`parse_theme_family`]：`serde_json` 解析文件 → 中间 [`ThemeFamilyContent`]；
//! 2. [`theme_from_content`]：把扁平 dotted-key 经[`apply_color`]的映射表搬进
//!    [`ThemeColors`] / [`StatusColors`]，`syntax` 铺进 [`SyntaxTheme`]，
//!    `accents` 数组给 [`AccentColors`]，缺失字段回退到内置 Mocha/Latte 基色。
//!
//! 与 zed 的关系：本文件 ≈ zed `ThemeColorsRefinement` + `syntax_overrides`
//! 的极简版映射；不同的是 zed 的 JSON 用结构化 `style.colors.*` 子对象，
//! 而 catppuccin/zed 这种 v0.2.0 扩展用**扁平 dotted-key**，所以我们直接把
//! `"组.字段"` 当作查表 key。

use std::sync::Arc;

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, WindowBackgroundAppearance};

use crate::content::{AppearanceContent, StyleContent, SyntaxContent, ThemeFamilyContent};
use crate::state::{Appearance, Theme, ThemeFamily, ThemeStyles};
use crate::styles::{
    AccentColors, PlayerColors, StatusColors, SyntaxTheme, SystemColors, ThemeColors,
};

/// 解析一个主题家族文件（`themes/*.json`）。
pub fn parse_theme_family(bytes: &[u8]) -> serde_json::Result<ThemeFamily> {
    let content: ThemeFamilyContent = serde_json::from_slice(bytes)?;
    let family_name = content.name.clone();
    let author = content.author.clone();
    let themes = content.themes.into_iter().map(theme_from_content).collect();
    Ok(ThemeFamily {
        id: family_name.clone(),
        name: family_name,
        author,
        themes,
    })
}

fn theme_from_content(content: crate::content::ThemeContent) -> Theme {
    let appearance = match content.appearance {
        AppearanceContent::Light => Appearance::Light,
        AppearanceContent::Dark => Appearance::Dark,
    };
    let (colors, status, syntax, accents) = theme_from_style(&content.style, appearance);
    Theme {
        // 家族内主题名唯一（Catppuccin Latte/Frappé/Macchiato/Mocha），直接作 id。
        id: content.name.clone(),
        name: content.name,
        appearance,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            colors,
            status,
            accents,
            system: SystemColors::default(),
            // 主题 JSON 不描述协作者配色(v0.2.0 扩展里没有这组 key),
            // 按明暗回退到内置色板——与 colors/status 的缺失回退同一策略。
            players: match appearance {
                Appearance::Light => PlayerColors::light(),
                Appearance::Dark => PlayerColors::dark(),
            },
            syntax: Arc::new(syntax),
        },
    }
}

/// 由 `style` 构建四个样式子集。
fn theme_from_style(
    style: &StyleContent,
    appearance: Appearance,
) -> (ThemeColors, StatusColors, SyntaxTheme, AccentColors) {
    // 以内置 Mocha/Latte 为基色：缺失字段（如 selection_background）回退正确，
    // JSON 里出现的字段全覆盖成真实值。
    let mut colors = base_colors(appearance);
    let mut status = base_status(appearance);

    for (key, value) in &style.colors {
        let Some(hex) = value.as_str() else {
            continue;
        };
        let Some(hsla) = parse_hex(hex) else {
            continue;
        };
        apply_color(&mut colors, &mut status, key, hsla);
    }

    // v0.2.0 扩展里没有这两个字段：按 zed 的派生规则补齐。
    colors.selection_background = colors.element_selected;
    colors.editor_cursor = colors.editor_foreground;

    let accents = match style
        .colors
        .get("accents")
        .and_then(serde_json::Value::as_array)
    {
        Some(values) => {
            let hsla = values
                .iter()
                .filter_map(|v| v.as_str().and_then(parse_hex))
                .collect::<Vec<_>>();
            if hsla.is_empty() {
                AccentColors::default()
            } else {
                AccentColors(hsla.into())
            }
        }
        None => AccentColors::default(),
    };

    let syntax = SyntaxTheme::new(
        style
            .syntax
            .iter()
            .map(|(capture, entry)| (capture.clone(), highlight_from_syntax(entry)))
            .collect::<Vec<_>>(),
    );

    (colors, status, syntax, accents)
}

fn base_colors(appearance: Appearance) -> ThemeColors {
    match appearance {
        Appearance::Light => crate::builtin::theme_colors_latte(),
        Appearance::Dark => crate::builtin::theme_colors_mocha(),
    }
}

fn base_status(appearance: Appearance) -> StatusColors {
    match appearance {
        Appearance::Light => crate::builtin::status_colors_latte(),
        Appearance::Dark => crate::builtin::status_colors_mocha(),
    }
}

/// `#rrggbb` / `#rrggbbaa` → `Hsla`，解析失败返回 `None`。
///
/// 实际解析在 [`crate::schema::try_parse_color`]（对外的公开契约，
/// 与 zed 同名）；这里只做 `Result` → `Option` 的适配——加载器遇到
/// 单个坏颜色值的选择是**跳过并继续**（整份主题不该因为一个字段报废），
/// 而不是向上传播错误。
fn parse_hex(hex: &str) -> Option<Hsla> {
    crate::schema::try_parse_color(hex).ok()
}

fn highlight_from_syntax(entry: &SyntaxContent) -> HighlightStyle {
    let font_style = match entry.font_style.as_deref() {
        Some("italic") => Some(FontStyle::Italic),
        Some("normal") | None => None,
        Some(_) => None,
    };
    let font_weight = match entry.font_weight.as_ref() {
        Some(serde_json::Value::String(s)) => match s.as_str() {
            "bold" => Some(FontWeight::BOLD),
            "medium" => Some(FontWeight::MEDIUM),
            "normal" => None,
            _ => None,
        },
        Some(serde_json::Value::Number(n)) => Some(FontWeight(n.as_f64().unwrap_or(400.0) as f32)),
        None | Some(_) => None,
    };
    HighlightStyle {
        color: entry.color.as_deref().and_then(parse_hex),
        background_color: None,
        font_style,
        font_weight,
        ..Default::default()
    }
}

/// 扁平 dotted-key → `ThemeColors` / `StatusColors` 字段。
///
/// 未识别的 key（`vim.*` / `terminal.*` / `players`…）直接忽略。
fn apply_color(colors: &mut ThemeColors, status: &mut StatusColors, key: &str, value: Hsla) {
    match key {
        // ---- 状态色：`状态[.background|.border]`（扁平字段，与 JSON 一一对应）----
        "conflict" => status.conflict = value,
        "conflict.background" => status.conflict_background = value,
        "conflict.border" => status.conflict_border = value,
        "created" => status.created = value,
        "created.background" => status.created_background = value,
        "created.border" => status.created_border = value,
        "deleted" => status.deleted = value,
        "deleted.background" => status.deleted_background = value,
        "deleted.border" => status.deleted_border = value,
        "error" => status.error = value,
        "error.background" => status.error_background = value,
        "error.border" => status.error_border = value,
        "hidden" => status.hidden = value,
        "hidden.background" => status.hidden_background = value,
        "hidden.border" => status.hidden_border = value,
        "ignored" => status.ignored = value,
        "ignored.background" => status.ignored_background = value,
        "ignored.border" => status.ignored_border = value,
        "info" => status.info = value,
        "info.background" => status.info_background = value,
        "info.border" => status.info_border = value,
        "modified" => status.modified = value,
        "modified.background" => status.modified_background = value,
        "modified.border" => status.modified_border = value,
        "renamed" => status.renamed = value,
        "renamed.background" => status.renamed_background = value,
        "renamed.border" => status.renamed_border = value,
        "success" => status.success = value,
        "success.background" => status.success_background = value,
        "success.border" => status.success_border = value,
        "warning" => status.warning = value,
        "warning.background" => status.warning_background = value,
        "warning.border" => status.warning_border = value,
        // ---- border 组 ----
        "border" => colors.border = value,
        "border.variant" => colors.border_variant = value,
        "border.focused" => colors.border_focused = value,
        "border.selected" => colors.border_selected = value,
        "border.disabled" => colors.border_disabled = value,
        "border.transparent" => colors.border_transparent = value,
        // background 组
        "background" => colors.background = value,
        "surface.background" => colors.surface_background = value,
        "elevated_surface.background" => colors.elevated_surface_background = value,
        // element 组
        "element.background" => colors.element_background = value,
        "element.hover" => colors.element_hover = value,
        "element.active" => colors.element_active = value,
        "element.selected" => colors.element_selected = value,
        "element.disabled" => colors.element_disabled = value,
        // ghost_element 组
        "ghost_element.background" => colors.ghost_element_background = value,
        "ghost_element.hover" => colors.ghost_element_hover = value,
        "ghost_element.active" => colors.ghost_element_active = value,
        "ghost_element.selected" => colors.ghost_element_selected = value,
        "ghost_element.disabled" => colors.ghost_element_disabled = value,
        // text / icon 组
        "text" => colors.text = value,
        "text.muted" => colors.text_muted = value,
        "text.placeholder" => colors.text_placeholder = value,
        "text.disabled" => colors.text_disabled = value,
        "text.accent" => colors.text_accent = value,
        "icon" => colors.icon = value,
        "icon.muted" => colors.icon_muted = value,
        "icon.disabled" => colors.icon_disabled = value,
        "icon.accent" => colors.icon_accent = value,
        // editor 组
        "editor.foreground" => colors.editor_foreground = value,
        "editor.background" => colors.editor_background = value,
        "editor.gutter.background" => colors.editor_gutter_background = value,
        "editor.active_line.background" => colors.editor_active_line_background = value,
        "editor.line_number" => colors.editor_line_number = value,
        "editor.active_line_number" => colors.editor_active_line_number = value,
        "editor.wrap_guide" => colors.editor_wrap_guide = value,
        "editor.indent_guide" => colors.editor_indent_guide = value,
        "editor.indent_guide_active" => colors.editor_indent_guide_active = value,
        "editor.invisible" => colors.editor_invisible = value,
        "editor.document_highlight.read_background" => {
            colors.editor_document_highlight_read_background = value
        }
        "editor.document_highlight.write_background" => {
            colors.editor_document_highlight_write_background = value
        }
        "editor.cursor" => colors.editor_cursor = value,
        // 面板 / 杂项
        "panel.background" => colors.panel_background = value,
        "pane.focused_border" => colors.pane_focused_border = value,
        "pane_group.border" => colors.pane_group_border = value,
        "search.match_background" => colors.search_match_background = value,
        "search.active_match_background" => colors.search_active_match_background = value,
        "scrollbar.thumb.background" => colors.scrollbar_thumb_background = value,
        "scrollbar.thumb.hover_background" => colors.scrollbar_thumb_hover_background = value,
        "scrollbar.thumb.active_background" => colors.scrollbar_thumb_active_background = value,
        "scrollbar.thumb.border" => colors.scrollbar_thumb_border = value,
        "scrollbar.track.background" => colors.scrollbar_track_background = value,
        "scrollbar.track.border" => colors.scrollbar_track_border = value,
        "link_text" | "link_text.hover" => colors.link = value,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Rgba;

    /// 容差比较两个颜色：palette 与 gpui 的 sRGB→HSL 实现有末位差异。
    fn assert_color_close(actual: Hsla, expected: Hsla) {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(
            close(actual.h, expected.h)
                && close(actual.s, expected.s)
                && close(actual.l, expected.l)
                && close(actual.a, expected.a),
            "颜色应一致(容差 1e-6):\n  actual   = {actual:?}\n  expected = {expected:?}"
        );
    }

    #[test]
    fn parses_catppuccin_family() {
        let bytes =
            include_bytes!("../../../assets/themes/catppuccin/catppuccin-mauve.json");
        let family = parse_theme_family(bytes).expect("parse catppuccin-mauve");
        assert_eq!(family.name, "Catppuccin");
        assert_eq!(family.themes.len(), 4);

        let macchiato = family
            .themes
            .iter()
            .find(|t| t.name == "Catppuccin Macchiato")
            .expect("macchiato present");
        assert_eq!(macchiato.appearance, Appearance::Dark);
        // 颜色用容差比较：解析走 schema::try_parse_color（palette 的
        // sRGB→HSL），与 gpui 自带转换在浮点末位差 1e-7 量级。
        assert_color_close(
            macchiato.colors().editor_background,
            Hsla::from(Rgba::try_from("#24273a").unwrap()),
        );
        assert_color_close(
            macchiato.colors().background,
            Hsla::from(Rgba::try_from("#2c2f46").unwrap()),
        );
        assert_color_close(
            macchiato.colors().text,
            Hsla::from(Rgba::try_from("#cad3f5").unwrap()),
        );
        // v0.2.0 没有 selection/editor_cursor 字段 → 回退派生
        assert_eq!(
            macchiato.colors().selection_background,
            macchiato.colors().element_selected
        );
        assert_eq!(
            macchiato.colors().editor_cursor,
            macchiato.colors().editor_foreground
        );
        // 状态组（扁平 key `error` / `error.background` / `error.border`）
        assert_color_close(
            macchiato.status().error,
            Hsla::from(Rgba::try_from("#ed8796").unwrap()),
        );
        // accents 数组 [7]
        assert_eq!(macchiato.accents().0.len(), 7);
        // players 不从 JSON 来(扩展里没有这组 key) → 按明暗回退到内置色板
        assert_eq!(
            macchiato.players(),
            &PlayerColors::dark(),
            "深色主题应回退到深色协作者配色"
        );
        // syntax capture 名原样保留（dotted），可精确命中
        let syntax = macchiato.syntax();
        assert!(syntax.get("keyword").is_some());
        assert!(syntax.get("variable.member").is_some());
    }

    #[test]
    fn parses_no_italics_family() {
        let bytes =
            include_bytes!("../../../assets/themes/catppuccin/catppuccin-no-italics-mauve.json");
        let family = parse_theme_family(bytes).expect("parse catppuccin-no-italics-mauve");
        assert_eq!(family.name, "Catppuccin");
        assert_eq!(family.themes.len(), 4);
    }
}
