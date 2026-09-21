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
//! // 对齐 zed：不传密度、不传 rem_size，内部自己读设置
//! div().gap(DynamicSpacing::Base08.px(cx))
//! div().p(DynamicSpacing::Base16.rems(cx))
//! ```
//!
//! 密度与 UI 字号都来自 `aa_gpui_kit_theme`（读注册的
//! `ThemeSettingsProvider`）—— 与 zed 完全同形，调用方零参数。

use gpui::{Pixels, Rems, px, rems};

use aa_gpui_kit_ui_macros::derive_dynamic_spacing;

/// UI 密度：**定义在 `aa_gpui_kit_theme`**（对齐 zed `theme::UiDensity`），
/// 这里原样转出，所以外部路径 `aa_gpui_kit_ui::UiDensity` 不变。
///
/// 2026-09 合并：此前 `ui` 里另有一份同名 `UiDensity`，与 theme 的那份重复；
/// 已删除只留 theme 的 —— `ThemeSettingsProvider::ui_density` 返回的就是它。
pub use aa_gpui_kit_theme::UiDensity;

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
    use gpui::{App, Pixels};

    /// 测试用的固定设置 provider：给定密度与 UI 字号。
    ///
    /// 生产环境这些值来自注册的 `ThemeSettingsProvider`（用户在设置里选），
    /// 测试里固定住，断言才有确定的数值。
    struct FixedSettings {
        density: UiDensity,
        ui_font_size: Pixels,
    }

    impl aa_gpui_kit_theme::ThemeSettingsProvider for FixedSettings {
        fn ui_font(&self, _cx: &App) -> &gpui::Font {
            unimplemented!("测试不涉及字体")
        }
        fn buffer_font(&self, _cx: &App) -> &gpui::Font {
            unimplemented!("测试不涉及字体")
        }
        fn ui_font_size(&self, _cx: &App) -> Pixels {
            self.ui_font_size
        }
        fn buffer_font_size(&self, _cx: &App) -> Pixels {
            self.ui_font_size
        }
        fn ui_density(&self, _cx: &App) -> UiDensity {
            self.density
        }
    }

    fn with_settings<R>(
        density: UiDensity,
        ui_font_size: Pixels,
        cx: &mut App,
        f: impl FnOnce(&mut App) -> R,
    ) -> R {
        aa_gpui_kit_theme::set_theme_settings_provider(
            Box::new(FixedSettings {
                density,
                ui_font_size,
            }),
            cx,
        );
        f(cx)
    }

    /// 单值档：Base24 在 Default 密度、16px 字号下 = 24px。
    #[gpui::test]
    fn base24_default_at_16px(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Default, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base24.px(cx), px(24.0));
            });
        });
    }

    /// 单值档的标准公式：Base24 的 Compact = 20px、Comfortable = 28px。
    #[gpui::test]
    fn base24_standard_formula(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Compact, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base24.px(cx), px(20.0));
            });
            with_settings(UiDensity::Comfortable, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base24.px(cx), px(28.0));
            });
        });
    }

    /// 单值公式对 0 档不产生负值（(0-4).max(0) = 0）。
    #[gpui::test]
    fn base00_compact_clamps_to_zero(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Compact, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base00.px(cx), px(0.0));
            });
        });
    }

    /// 元组档：三档直接取值（(1,2,4) 在 16px 字号下 = 1/2/4px）。
    #[gpui::test]
    fn tuple_uses_values_directly(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Compact, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base02.px(cx), px(1.0));
            });
            with_settings(UiDensity::Default, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base02.px(cx), px(2.0));
            });
            with_settings(UiDensity::Comfortable, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base02.px(cx), px(4.0));
            });
        });
    }

    /// px 随 UI 字号缩放（不是窗口 rem_size）：Base24 在 8px 字号下 = 12px。
    #[gpui::test]
    fn px_scales_with_ui_font_size(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Default, px(8.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base24.px(cx), px(12.0));
            });
        });
    }

    /// rems 与 px 的一致性：Base16 的 ratio = 1.0。
    #[gpui::test]
    fn base16_ratio_is_one(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            with_settings(UiDensity::Default, px(16.0), cx, |cx| {
                assert_eq!(DynamicSpacing::Base16.rems(cx), rems(1.0));
                assert_eq!(DynamicSpacing::Base16.px(cx), px(16.0));
            });
        });
    }
}
