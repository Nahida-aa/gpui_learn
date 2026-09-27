//! 分割线组件：对齐 zed `crates/ui/src/components/divider.rs`。
//!
//! zed 渲染（divider.rs:137-160）——Spacing + 实线 `bg` / 虚线 canvas，
//! 颜色来自 [`DividerColor`]，默认 `BorderVariant`：
//!
//! ```text
//! horizontal:  min_w_0().h_px().max_h_px().w_full()   [inset → mx_1p5()]
//! vertical:    min_w_0().w_px().h_4()                 [inset → my_1p5()]
//! ```
//!
//! 本实现与 zed 同构：实线走 `bg`，虚线走 `canvas` + `PathBuilder::stroke`
//! `dash_array`（zed divider.rs:100-128，`window.paint_path` 画线）。

use aa_gpui_kit_theme::ActiveTheme;

use gpui::{
    App, Div, Hsla, IntoElement, PathBuilder, Refineable as _, StyleRefinement, Styled, Window,
    canvas, div, point, prelude::*, px,
};

/// zed `DividerColor`：映射主题 `border` / `border_variant` / `border.opacity(0.6)`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DividerColor {
    Border,
    BorderFaded,
    #[default]
    BorderVariant,
}

/// zed `Divider` 的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

/// 线型（对齐 zed `DividerStyle`）：实线 `bg` 填充，虚线 canvas 画。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DividerStyle {
    Solid,
    Dashed,
}

/// 分割线组件。
#[derive(Debug, IntoElement)]
pub struct Divider {
    line_style: DividerStyle,
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
            line_style: DividerStyle::Solid,
            color: DividerColor::default(),
            direction: DividerDirection::Horizontal,
            inset: false,
            style: StyleRefinement::default(),
        }
    }

    /// 垂直分割线（`w_px h_4`）——状态栏/工具栏图标按钮之间的竖线。
    pub fn vertical() -> Self {
        Self {
            line_style: DividerStyle::Solid,
            color: DividerColor::default(),
            direction: DividerDirection::Vertical,
            inset: false,
            style: StyleRefinement::default(),
        }
    }

    /// 水平虚线分割线（对齐 zed `horizontal_dashed`）。
    pub fn horizontal_dashed() -> Self {
        Self {
            line_style: DividerStyle::Dashed,
            color: DividerColor::default(),
            direction: DividerDirection::Horizontal,
            inset: false,
            style: StyleRefinement::default(),
        }
    }

    /// 垂直虚线分割线（对齐 zed `vertical_dashed`）。
    pub fn vertical_dashed() -> Self {
        Self {
            line_style: DividerStyle::Dashed,
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
            DividerColor::BorderFaded => colors.border.opacity(0.6),
            DividerColor::BorderVariant => colors.border_variant,
        }
    }

    /// 实线：直接 `bg` 填充（对齐 zed `render_solid`）。
    fn render_solid(self, base: Div, cx: &mut App) -> impl IntoElement {
        base.bg(self.color_hsla(cx.theme().colors()))
    }

    /// 虚线：`canvas` 里用 `PathBuilder::stroke(..).dash_array(..)` 画线
    /// （对齐 zed `render_dashed`，dash 段 4px / 空 2px）。
    fn render_dashed(self, base: Div) -> impl IntoElement {
        base.relative().child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, cx| {
                    let mut builder = PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)]);
                    let (start, end) = match self.direction {
                        DividerDirection::Horizontal => {
                            let x = bounds.origin.x;
                            let y = bounds.origin.y + px(0.5);
                            (point(x, y), point(x + bounds.size.width, y))
                        }
                        DividerDirection::Vertical => {
                            let x = bounds.origin.x + px(0.5);
                            let y = bounds.origin.y;
                            (point(x, y), point(x, y + bounds.size.height))
                        }
                    };
                    builder.move_to(start);
                    builder.line_to(end);
                    if let Ok(line) = builder.build() {
                        window.paint_path(line, self.color_hsla(cx.theme().colors()));
                    }
                },
            )
            .absolute()
            .size_full(),
        )
    }
}

impl Styled for Divider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Divider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut base = match self.direction {
            DividerDirection::Horizontal => div()
                .min_w_0()
                .h_px()
                .max_h_px()
                .w_full()
                .when(self.inset, |this| this.mx_1p5()),
            DividerDirection::Vertical => div()
                .min_w_0()
                .w_px()
                .h_4()
                .when(self.inset, |this| this.my_1p5()),
        };

        // 调用方经 `Styled` 叠加的 refinement（对齐 zed：`base.style().refine(..)`）。
        base.style().refine(&self.style);

        match self.line_style {
            DividerStyle::Solid => self.render_solid(base, cx).into_any_element(),
            DividerStyle::Dashed => self.render_dashed(base).into_any_element(),
        }
    }
}
