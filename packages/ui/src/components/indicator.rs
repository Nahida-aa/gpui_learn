//! 状态指示器（对齐 zed `crates/ui/src/components/indicator.rs`）。
//!
//! 三种形态：小圆点（Dot）、条（Bar）、图标（Icon）。常挂在头像、图标、
//! 标签的角上表示状态/未读。

use crate::{Icon, prelude::*};

#[derive(Default)]
enum IndicatorKind {
    #[default]
    Dot,
    Bar,
    Icon(Icon),
}

#[derive(IntoElement, RegisterComponent)]
pub struct Indicator {
    kind: IndicatorKind,
    border_color: Option<Color>,
    pub color: Color,
}

impl Indicator {
    pub fn dot() -> Self {
        Self {
            kind: IndicatorKind::Dot,
            border_color: None,
            color: Color::Default,
        }
    }

    pub fn bar() -> Self {
        Self {
            kind: IndicatorKind::Bar,
            border_color: None,

            color: Color::Default,
        }
    }

    /// 图标形态。因为 [`Icon`] 有 `From<IconName>`，这里也可以直接传
    /// `IconName`（`Indicator::icon(IconName::Check)`）。
    pub fn icon(icon: impl Into<Icon>) -> Self {
        Self {
            kind: IndicatorKind::Icon(icon.into()),
            border_color: None,

            color: Color::Default,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }
}

impl RenderOnce for Indicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let container = div().flex_none();
        let container = if let Some(border_color) = self.border_color {
            if matches!(self.kind, IndicatorKind::Dot | IndicatorKind::Bar) {
                container.border_1().border_color(border_color.color(cx))
            } else {
                container
            }
        } else {
            container
        };

        match self.kind {
            // 与 zed 的差异：zed 用 `icon.custom_size(rems_from_px(8.))`，
            // 因为它的 Icon 收 `IconSize`；我们的 `Icon::size` 收 `Pixels`、
            // `.color()` 收 `Hsla`（见 base/icon.rs），所以直接给 8px + 语义色。
            IndicatorKind::Icon(icon) => container
                .child(icon.size(px(8.)).color(self.color.color(cx))),
            IndicatorKind::Dot => container
                .w_1p5()
                .h_1p5()
                .rounded_full()
                .bg(self.color.color(cx)),
            IndicatorKind::Bar => container
                .w_full()
                .h_1p5()
                .rounded_t_sm()
                .bg(self.color.color(cx)),
        }
    }
}

impl Component for Indicator {
    fn scope() -> ComponentScope {
        ComponentScope::Status
    }

    fn description() -> &'static str {
        "Visual indicators used to represent status, notifications, \
        or draw attention to specific elements."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Dot Indicators",
                    vec![
                        single_example("Default", Indicator::dot().into_any_element()),
                        single_example(
                            "Success",
                            Indicator::dot().color(Color::Success).into_any_element(),
                        ),
                        single_example(
                            "Warning",
                            Indicator::dot().color(Color::Warning).into_any_element(),
                        ),
                        single_example(
                            "Error",
                            Indicator::dot().color(Color::Error).into_any_element(),
                        ),
                        single_example(
                            "With Border",
                            Indicator::dot()
                                .color(Color::Accent)
                                .border_color(Color::Default)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Bar Indicators",
                    vec![
                        single_example("Default", Indicator::bar().into_any_element()),
                        single_example(
                            "Success",
                            Indicator::bar().color(Color::Success).into_any_element(),
                        ),
                        single_example(
                            "Warning",
                            Indicator::bar().color(Color::Warning).into_any_element(),
                        ),
                        single_example(
                            "Error",
                            Indicator::bar().color(Color::Error).into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Icon Indicators",
                    vec![
                        single_example(
                            "Default",
                            Indicator::icon(IconName::Circle).into_any_element(),
                        ),
                        single_example(
                            "Success",
                            Indicator::icon(IconName::Check)
                                .color(Color::Success)
                                .into_any_element(),
                        ),
                        single_example(
                            "Warning",
                            Indicator::icon(IconName::Warning)
                                .color(Color::Warning)
                                .into_any_element(),
                        ),
                        single_example(
                            "Error",
                            Indicator::icon(IconName::Close)
                                .color(Color::Error)
                                .into_any_element(),
                        ),
                    ],
                ),
            ])
            .into_any_element()
    }
}
