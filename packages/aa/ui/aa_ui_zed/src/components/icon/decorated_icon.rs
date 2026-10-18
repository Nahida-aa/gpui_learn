//! 带装饰的图标（对齐 zed `crates/ui/src/components/icon/decorated_icon.rs`）。

use gpui::{AnyElement, IntoElement, Point};

use crate::components::icon::icon_decoration::{IconDecoration, IconDecorationKind};
use crate::prelude::*;

#[derive(IntoElement, RegisterComponent)]
pub struct DecoratedIcon {
    icon: Icon,
    decoration: Option<IconDecoration>,
}

impl DecoratedIcon {
    pub fn new(icon: Icon, decoration: Option<IconDecoration>) -> Self {
        Self { icon, decoration }
    }
}

impl RenderOnce for DecoratedIcon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // 与 zed 的差异：zed 直接读 `self.icon.size`（同模块树能访问私有字段）；
        // 我们的 `Icon` 在 base/icon.rs，与这里不同模块，用 `size_px()` 取值。
        let icon_size = self.icon.size_px();

        div()
            .relative()
            .size(icon_size)
            .child(self.icon)
            .children(self.decoration)
    }
}

impl Component for DecoratedIcon {
    fn scope() -> ComponentScope {
        ComponentScope::Images
    }

    fn description() -> &'static str {
        "An icon with an optional decoration overlay (like an X, triangle, or dot) \
        that can be positioned relative to the icon"
    }

    fn preview(_window: &mut Window, cx: &mut App) -> AnyElement {
        let decoration_x = IconDecoration::new(
            IconDecorationKind::X,
            cx.theme().colors().surface_background,
            cx,
        )
        .color(cx.theme().status().error)
        .position(Point {
            x: px(-2.),
            y: px(-2.),
        });

        let decoration_triangle = IconDecoration::new(
            IconDecorationKind::Triangle,
            cx.theme().colors().surface_background,
            cx,
        )
        .color(cx.theme().status().error)
        .position(Point {
            x: px(-2.),
            y: px(-2.),
        });

        let decoration_dot = IconDecoration::new(
            IconDecorationKind::Dot,
            cx.theme().colors().surface_background,
            cx,
        )
        .color(cx.theme().status().error)
        .position(Point {
            x: px(-2.),
            y: px(-2.),
        });

        v_flex()
            .gap_6()
            .children(vec![example_group_with_title(
                "Decorations",
                vec![
                    single_example(
                        "No Decoration",
                        DecoratedIcon::new(Icon::new(IconName::FileDoc), None).into_any_element(),
                    ),
                    single_example(
                        "X Decoration",
                        DecoratedIcon::new(Icon::new(IconName::FileDoc), Some(decoration_x))
                            .into_any_element(),
                    ),
                    single_example(
                        "Triangle Decoration",
                        DecoratedIcon::new(Icon::new(IconName::FileDoc), Some(decoration_triangle))
                            .into_any_element(),
                    ),
                    single_example(
                        "Dot Decoration",
                        DecoratedIcon::new(Icon::new(IconName::FileDoc), Some(decoration_dot))
                            .into_any_element(),
                    ),
                ],
            )])
            .into_any_element()
    }
}
