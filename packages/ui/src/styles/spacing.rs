//! styles/spacing：动态间距系统（对齐 zed `DynamicSpacing` / `UiDensity`）。
//!
//! [`DynamicSpacing`] 由 `aa_gpui_kit_ui_macros::derive_dynamic_spacing!`
//! 生成：每个变体的"Base 后数字"是该档在 **16px/rem、Default 密度**下的
//! 像素值。三档密度（Compact / Default / Comfortable）的取值规则：
//!
//! - 元组 `(a, b, c)`：三档直接取值。例：`(1, 2, 4)` → 1px/2px/4px；
//! - 单值 `n`：标准公式展开。例：`24` → 20px/24px/28px。
//!
//! 使用（对齐 zed 的调用习惯）：
//!
//! ```ignore
//! // 有 window 的场景（render 里）：
//! div().gap(DynamicSpacing::Base08.px(UiDensity::Default, window.rem_size()))
//! // 只需要 rems 的场景：
//! div().p(DynamicSpacing::Base16.rems(UiDensity::Default))
//! ```
//!
//! 与 zed 的差异：zed 从 `theme_settings.ui_density(cx)` 读当前密度（有
//! 设置系统）；我们尚无设置系统，密度由调用方显式传入。设置系统就绪后
//! 由 theme 包提供 `ui_density()`，调用方再改为传当前设置。

use gpui::{Pixels, Rems, px, rems};

use aa_gpui_kit_ui_macros::derive_dynamic_spacing;

/// UI 密度（对齐 zed `theme::UiDensity`）。
///
/// zed 把它放在 theme 包（因为由 `theme_settings` 提供当前值）；我们尚无
/// 设置系统，暂与间距定义同放，由调用方显式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UiDensity {
    /// 紧凑（每档比默认少 4px）。
    Compact,
    /// 默认。
    #[default]
    Default,
    /// 宽松（每档比默认多 4px）。
    Comfortable,
}

// 档位列表与 zed spacing.rs 保持一致（便于调用方代码与 zed 交互对照）。
derive_dynamic_spacing![
    (0, 0, 0),
    (1, 1, 2),
    (1, 2, 4),
    (2, 3, 4),
    (2, 4, 6),
    (3, 6, 8),
    (4, 8, 10),
    (10, 12, 14),
    (14, 16, 18),
    (18, 20, 22),
    24,
    32,
    40,
    48
];

#[cfg(test)]
mod tests {
    use super::*;

    /// 单值档：Base24 在 Default 密度、16px/rem 下 = 24px。
    #[test]
    fn base24_default_at_16px_rem() {
        assert_eq!(DynamicSpacing::Base24.px(UiDensity::Default, px(16.0)), px(24.0));
    }

    /// 单值档的标准公式：Base24 的 Compact = 20px、Comfortable = 28px。
    #[test]
    fn base24_standard_formula() {
        assert_eq!(DynamicSpacing::Base24.px(UiDensity::Compact, px(16.0)), px(20.0));
        assert_eq!(
            DynamicSpacing::Base24.px(UiDensity::Comfortable, px(16.0)),
            px(28.0)
        );
    }

    /// 单值公式对 0 档不产生负值（(0-4).max(0) = 0）。
    #[test]
    fn base00_compact_clamps_to_zero() {
        assert_eq!(DynamicSpacing::Base00.px(UiDensity::Compact, px(16.0)), px(0.0));
    }

    /// 元组档：三档直接取值（(1,2,4) 在 16px/rem 下 = 1/2/4px）。
    #[test]
    fn tuple_uses_values_directly() {
        assert_eq!(DynamicSpacing::Base02.px(UiDensity::Compact, px(16.0)), px(1.0));
        assert_eq!(DynamicSpacing::Base02.px(UiDensity::Default, px(16.0)), px(2.0));
        assert_eq!(
            DynamicSpacing::Base02.px(UiDensity::Comfortable, px(16.0)),
            px(4.0)
        );
    }

    /// px 随 rem_size 缩放：Base24 在 8px/rem 的 Default 密度 = 12px。
    #[test]
    fn px_scales_with_rem_size() {
        assert_eq!(DynamicSpacing::Base24.px(UiDensity::Default, px(8.0)), px(12.0));
    }

    /// rems 与 px 的一致性：Base16 的 ratio = 1.0。
    #[test]
    fn base16_ratio_is_one() {
        assert_eq!(DynamicSpacing::Base16.rems(UiDensity::Default), rems(1.0));
        assert_eq!(DynamicSpacing::Base16.px(UiDensity::Default, px(16.0)), px(16.0));
    }
}
