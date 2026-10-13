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
///
/// 用 **Macchiato** 而非 Mocha:基底 `ThemeColors::dark()` 就是 Macchiato，
/// 兜底与基底同风味,免得「找不到主题」时突然换一套观感。
pub const DEFAULT_DARK_THEME: &str = "Catppuccin Macchiato";

/// zed_default_themes
/// 兜底主题家族（Catppuccin 三主题：Mocha + Macchiato + Latte）。
///
/// Zed 原版的 default_theme_family 只包含一个兜底主题（灰阶默认主题）。
/// 我们这里改为注册全部三个内置 Catppuccin 主题，方便用户在
/// ThemeSettings 里直接切换，不需要额外加载 theme JSON 扩展。
pub fn default_theme_family() -> crate::ThemeFamily {
    crate::ThemeFamily {
        id: "ui-gpui-fallback".into(),
        name: "ui-gpui Fallback".into(),
        author: "".into(),
        themes: vec![
            ctp_default_dark(),
            // Mocha 是另一个深色调（比 Macchiato 更深），留着给用户切；
            // Latte 是唯一的浅色，浅色系统外观下靠它。
            crate::default_colors::catppuccin_mocha(),
            crate::default_colors::catppuccin_latte(),
        ],
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
    use std::sync::Arc;

    use gpui::hsla;
    use crate::default_colors::{
        h, ha, status_colors_macchiato, syntax_macchiato, theme_colors_macchiato,
    };
    use crate::state::{Appearance, Theme, ThemeStyles};
    use crate::styles::{
        AccentColors, PlayerColor, PlayerColors, SystemColors,
    };
    use gpui::WindowBackgroundAppearance;

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
        name: "Catppuccin Macchiato".into(),
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
    fn fallback_family_contains_all_builtin_catppuccin_themes() {
        let family = default_theme_family();
        let names: Vec<_> = family.themes.iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(
            names,
            [
                "Catppuccin Macchiato",
                "Catppuccin Mocha",
                "Catppuccin Latte"
            ],
            "兜底家族要带上全部内置 Catppuccin 主题，用户才能直接切换"
        );
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
