//! 回退主题与「缺失字段补齐」规则，对齐 zed `crates/theme/src/fallback_themes.rs`。
//!
//! ## 用途
//!
//! 主题 JSON 可能载入失败、或目标主题不存在，此时需要一个**兜底主题**
//! 保证界面不空。本模块提供 [`ctp_default_dark`]——它是主题体系的最低
//! 保证，任何情况下都能取到。
//!
//! ## 与 zed 的差异
//!
//! zed 这里是 `zed_default_themes()`：zed 自己的默认灰阶配色，约 300 行
//! 手写色值。我们改用 **Catppuccin Mocha**（`ctp` = Catppuccin）：它已经
//! 是本仓库的内置主题（见 [`default_colors`](crate::default_colors)），
//! 色值不必重复定义一遍，视觉也与其余内置主题一致。
//!
//! 另外 zed 的 `ThemeFamily` 带 `scales`（色板集合）字段，我们的色板是
//! 全局函数（[`default_colors`](crate::default_colors)）不随家族走，
//! 故没有这个字段。
//!
//! ## 两个补齐规则
//!
//! [`apply_status_color_defaults`] 与 [`apply_theme_color_defaults`]
//! 作用于 `Refineable` 生成的 refinement 类型：主题 JSON 只写部分字段时，
//! 未写的那些按语义**派生**（而不是硬编码一个值）。

use gpui::Hsla;

use crate::{PlayerColors, StatusColorsRefinement, ThemeColorsRefinement};

/// 兜底主题的展示名(对齐 zed `DEFAULT_DARK_THEME` 的角色:找不到就退到它)。
pub const DEFAULT_DARK_THEME: &str = "Catppuccin Mocha";

/// Catppuccin Mocha 兜底主题。
///
/// 直接复用 [`default_colors`](crate::default_colors) 的内置主题构造，
/// 不再绕道注册表——兜底要的就是「一定能拿到」。
pub fn ctp_default_dark() -> crate::Theme {
    crate::default_colors::catppuccin_mocha()
}

/// 兜底主题家族（Catppuccin Mocha 单主题）。
pub fn default_theme_family() -> crate::ThemeFamily {
    crate::ThemeFamily {
        id: "ui-gpui-fallback".into(),
        name: "ui-gpui Fallback".into(),
        author: String::new(),
        themes: vec![ctp_default_dark()],
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::hsla;

    #[test]
    fn fallback_theme_is_the_builtin_catppuccin_dark() {
        let theme = ctp_default_dark();
        assert_eq!(theme.id, "ui-gpui-default-dark");
        assert_eq!(theme.name, "ui-gpui Dark");
    }

    #[test]
    fn fallback_family_contains_exactly_the_fallback_theme() {
        let family = default_theme_family();
        assert_eq!(family.themes.len(), 1);
        assert_eq!(family.themes[0].id, ctp_default_dark().id);
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
