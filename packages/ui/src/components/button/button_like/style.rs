//! button_like/style：按钮各状态的色值计算。
//!
//! 对齐 zed `button_like.rs` 里 `ButtonStyle::{enabled, hovered, active,
//! disabled} -> ButtonLikeStyles` 的职责：每种 [`ButtonStyle`] 在
//! enabled / hovered / active 三个状态下各有一套 (背景, 边框, 前景)。
//!
//! 与 zed 的差异：zed 另有 `focused`（键盘焦点环）与 `Tinted` 的 darken
//! 派生，需要 `StyleRefinement`/`darken` 等我们尚未引入的机制，暂缺。

use gpui::Hsla;

use aa_gpui_kit_theme::Theme;

use crate::components::button::ButtonStyle;

/// 一组待应用的按钮色（背景 / 边框 / 前景）。
#[derive(Clone, Copy)]
pub struct ButtonLikeColors {
    /// 常态 / hover / active 各自的背景。
    pub bg: [Hsla; 3],
    /// 边框色（默认透明）。
    pub border: Hsla,
    /// 图标 / 文字前景色。
    pub fg: Hsla,
}

/// 解析该样式在 `enabled` / `hovered` / `active` 三个状态下的颜色。
///
/// 自 `crate::components::button` 的旧实现（原 `ButtonStyle::colors`）
/// 原样迁入，语义不变。
pub fn button_like_colors(style: ButtonStyle, theme: &Theme) -> ButtonLikeColors {
    use gpui::hsla;

    let colors = theme.colors();
    let transparent = hsla(0., 0., 0., 0.);
    let text = colors.text;

    match style {
        ButtonStyle::Filled => {
            let mut hover_bg = colors.element_background;
            hover_bg.fade_out(0.5);
            ButtonLikeColors {
                bg: [colors.element_background, hover_bg, colors.element_active],
                border: transparent,
                fg: text,
            }
        }
        ButtonStyle::Outlined => ButtonLikeColors {
            bg: [
                colors.element_background,
                colors.ghost_element_hover,
                colors.element_active,
            ],
            border: colors.border_variant,
            fg: text,
        },
        ButtonStyle::OutlinedGhost => ButtonLikeColors {
            bg: [
                transparent,
                colors.ghost_element_hover,
                colors.ghost_element_active,
            ],
            border: colors.border_variant,
            fg: text,
        },
        ButtonStyle::Subtle => ButtonLikeColors {
            bg: [
                transparent,
                colors.ghost_element_hover,
                colors.ghost_element_active,
            ],
            border: transparent,
            fg: text,
        },
        ButtonStyle::Transparent => ButtonLikeColors {
            bg: [transparent, transparent, transparent],
            border: transparent,
            // zed：Transparent 前景随 hover 变为 muted。
            fg: colors.text_muted,
        },
        ButtonStyle::Tinted(tint) => {
            let status = theme.status();
            let (bg, border) = match tint {
                // zed 的 Accent 实际套用 info 状态色。
                crate::components::button::TintColor::Accent => {
                    (status.info_background, status.info_border)
                }
                crate::components::button::TintColor::Error => {
                    (status.error_background, status.error_border)
                }
                crate::components::button::TintColor::Warning => {
                    (status.warning_background, status.warning_border)
                }
                crate::components::button::TintColor::Success => {
                    (status.success_background, status.success_border)
                }
            };
            ButtonLikeColors {
                bg: [bg, bg, colors.element_active],
                border,
                fg: text,
            }
        }
    }
}
