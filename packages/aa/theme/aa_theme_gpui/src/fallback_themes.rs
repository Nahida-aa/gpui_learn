//! 回退主题与「缺失字段补齐」规则，对齐 zed `crates/theme/src/fallback_themes.rs`。
//!
//! ## 用途
//!
//! 主题 JSON 可能载入失败、或目标主题不存在，此时需要一个**兜底主题**
//! 保证界面不空。本模块提供 [`ctp_default_dark`]——它是主题体系的最低
//! 保证，任何情况下都能取到。
//!
//! ## 职责边界（与 zed 一致的单向依赖）
//!
//! `fallback_themes → default_colors`（CTP 配色、色阶、基底都在那边），
//! 反向不引用。本模块只做三件事：兜底家族组装（[`default_theme_family`]）、
//! [`ctp_default_dark`] 的 Theme 组装、以及下面两条补齐规则。
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

use crate::default_colors::{
    h, ha, status_colors_macchiato, syntax_macchiato, theme_colors_macchiato,
};
use crate::styles::PlayerColors;
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
}
