//! 分割线组件：对齐 zed `crates/ui/src/components/divider.rs`。
//!
//! zed 渲染（divider.rs:139-150）——就 Spacing + 一条 `bg`，颜色来自
//! [`DividerColor`]，默认 `Border` → `colors.border`：
//!
//! ```text
//! horizontal:  min_w_0().h_px().max_w_0().w_full()   [inset → mx_1p5()]
//! vertical:    min_w_0().w_px().h_4()               [inset → my_1p5()]
//! ```
//!
//! 本实现先提供实线版本（状态栏/工具栏图标按钮之间的竖线）。zed 的
//! Dashed/渐变靠 `window.paint_path` 画 `StructuralPath`，暂无需求不引入。

use aa_gpui_kit_theme::ActiveTheme;

use gpui::{App, Hsla, IntoElement, StyleRefinement, Styled, Window, div, prelude::*};

/// zed `DividerColor`：`Border`（默认）/ `BorderVariant`，映射主题 `border` / `border_variant`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerColor {
    Border,
    BorderVariant,
}

impl Default for DividerColor {
    fn default() -> Self {
        Self::Border
    }
}

/// zed `Divider` 的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

/// 分割线组件。
#[derive(Debug, IntoElement)]
pub struct Divider {
    color: DividerColor,
    direction: DividerDirection,
    inset: bool,
    /// 调用方经 `Styled` 叠加的 refinement（如 `.mx_1()`），渲染时并入
    /// （对齐 zed `Divider.style` + `impl Styled`）。
    style: StyleRefinement,
}

impl Divider {
    /// 水平分割线（`h_px w_full`）。
    pub fn horizontal() -> Self {
        Self {
            color: DividerColor::default(),
            direction: DividerDirection::Horizontal,
            inset: false,
            style: StyleRefinement::default(),
        }
    }

    /// 垂直分割线（`w_px h_4`）——状态栏/工具栏图标按钮之间的竖线。
    pub fn vertical() -> Self {
        Self {
            color: DividerColor::default(),
            direction: DividerDirection::Vertical,
            inset: false,
            style: StyleRefinement::default(),
        }
    }

    /// 让分割线两端内缩（zed `inset`）。
    pub fn inset(mut self) -> Self {
        self.inset = true;
        self
    }

    /// 指定颜色（zed `DividerColor`）。
    pub fn color(mut self, color: DividerColor) -> Self {
        self.color = color;
        self
    }

    /// 当前边框/分割线颜色（按主题 `colors` 解析）。
    fn color_hsla(&self, colors: &aa_gpui_kit_theme::ThemeColors) -> Hsla {
        match self.color {
            DividerColor::Border => colors.border,
            DividerColor::BorderVariant => colors.border_variant,
        }
    }
}

impl Styled for Divider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors().clone();

        let mut base = div()
            .min_w_0()
            .when_else(
                self.direction == DividerDirection::Horizontal,
                |el| el.h_px().max_w_0().w_full(),
                |el| el.w_px().h_4(),
            )
            .when(self.inset, |el| match self.direction {
                DividerDirection::Horizontal => el.mx_1p5(),
                DividerDirection::Vertical => el.my_1p5(),
            })
            .bg(self.color_hsla(&colors));

        // 调用方经 `Styled` 叠加的 refinement（对齐 zed：`base.style().refine(..)`）。
        base.style().refine(&self.style);

        base.flex_shrink_0().into_any_element()
    }
}
