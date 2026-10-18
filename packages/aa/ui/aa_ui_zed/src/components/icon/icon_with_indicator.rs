//! 图标 + 右下角小徽标（对齐 zed `crates/ui/src/components/icon.rs:241`）。
//!
//! 用法与 zed 一致：`IconWithIndicator::new(icon, Some(Indicator::dot()))`，
//! 边框色默认取 [`colors.elevated_surface_background`]（徽标浮在按钮上时
//! 用背景色描一圈，视觉上"挖空"底下的按钮）。

use gpui::Hsla;

use crate::components::indicator::Indicator;
use crate::prelude::*;

#[derive(IntoElement)]
pub struct IconWithIndicator {
    icon: Icon,
    indicator: Option<Indicator>,
    indicator_border_color: Option<Hsla>,
}

impl IconWithIndicator {
    pub fn new(icon: Icon, indicator: Option<Indicator>) -> Self {
        Self {
            icon,
            indicator,
            indicator_border_color: None,
        }
    }

    pub fn indicator(mut self, indicator: Option<Indicator>) -> Self {
        self.indicator = indicator;
        self
    }

    /// 覆盖徽标自身的语义色（不传时用 [`Indicator`] 自己设的颜色）。
    pub fn indicator_color(mut self, color: Color) -> Self {
        if let Some(indicator) = self.indicator.as_mut() {
            indicator.color = color;
        }
        self
    }

    /// 徽标外圈描边色；`None` 时取主题 `elevated_surface_background`。
    pub fn indicator_border_color(mut self, color: Option<Hsla>) -> Self {
        self.indicator_border_color = color;
        self
    }
}

impl RenderOnce for IconWithIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let indicator_border_color = self
            .indicator_border_color
            .unwrap_or_else(|| cx.theme().colors().elevated_surface_background);

        div()
            .relative()
            .child(self.icon)
            .when_some(self.indicator, |this, indicator| {
                this.child(
                    div()
                        .absolute()
                        .size_2p5()
                        .border_2()
                        .border_color(indicator_border_color)
                        .rounded_full()
                        .bottom_neg_0p5()
                        .right_neg_0p5()
                        .child(indicator),
                )
            })
    }
}
