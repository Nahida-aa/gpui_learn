//! button_like：通用按钮渲染器，对齐 zed `ButtonLike`。
//!
//! 「按钮」的样式计算与交互装配集中在这里，`IconButton`（以及将来的
//! `ToggleButton` / `SplitButton`）退化成薄壳：构造配置、算出图标色，
//! 然后委托给 [`ButtonLike`] 渲染。新增按钮形态（如 `ButtonLink` 那种
//! 链接样式）也只是十几行的包装。
//!
//! 用 `ButtonLike` 直接搭建自定义按钮也可以（zed 同样允许，但建议克制，
//! 避免按钮观感不一致）：
//!
//! ```ignore
//! ButtonLike::new("custom")
//!     .style(ButtonStyle::Subtle)
//!     .label("保存")
//!     .icon(IconName::Check, None, px(14.))
//!     .on_click(|_, window, cx| { /* ... */ })
//! ```

mod common;
mod style;

use std::rc::Rc;

use gpui::{
    Anchor, App, ClickEvent, CursorStyle, ElementId, Entity, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, Window, div, prelude::*,
    px,
};

use aa_gpui_kit_theme::ActiveTheme;

use crate::base::button::ClickHandler;
use crate::base::icon::{Icon, IconName};
use crate::components::button::{ButtonRadius, ButtonStyle};
use crate::components::tooltip::{Tooltip, TooltipHost};
use crate::traits::{Clickable, Disableable, Toggleable};

pub use common::ButtonCommon;
pub use style::{ButtonLikeColors, button_like_colors};

/// 通用按钮：可选图标 + 可选文字，统一的主题化样式与交互装配。
#[derive(IntoElement)]
pub struct ButtonLike {
    id: ElementId,
    style: ButtonStyle,
    /// 文字（与图标可同时存在，水平排列）。
    label: Option<SharedString>,
    icon: Option<IconName>,
    /// 显式图标色；`None` 时：selected → accent，否则取样式 fg。
    icon_color: Option<Hsla>,
    /// 图标边长（icon-only 时同时决定容器边长）。
    icon_size: Pixels,
    /// 容器边长（icon-only 时的方形边长）；`None` 时按内容自适应。
    size: Option<Pixels>,
    radius: ButtonRadius,
    disabled: bool,
    selected: bool,
    aria_label: Option<SharedString>,
    cursor_style: CursorStyle,
    on_click: Option<ClickHandler>,
    /// 悬停提示（工厂现场建 [`Tooltip`] 实体）。
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static>>,
    /// 提示锚点（默认 `Anchor::TopLeft`）。
    tooltip_anchor: Option<Anchor>,
    /// 提示 attachment（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    tooltip_attach: Option<Anchor>,
}

impl ButtonLike {
    /// 用给定 id 新建（id 在同一父容器内需唯一）。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: ButtonStyle::Subtle,
            label: None,
            icon: None,
            icon_color: None,
            icon_size: px(14.0),
            size: None,
            radius: ButtonRadius::Medium,
            disabled: false,
            selected: false,
            aria_label: None,
            cursor_style: CursorStyle::PointingHand,
            on_click: None,
            tooltip: None,
            tooltip_anchor: None,
            tooltip_attach: None,
        }
    }

    /// 应用 [`ButtonStyle`]。
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// 按钮文字。
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// 设置图标（颜色由 `icon_color` / 主题决定）。
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 指定图标颜色（`None` 时：selected → accent，否则样式 fg）。
    pub fn icon_color(mut self, color: Option<Hsla>) -> Self {
        self.icon_color = color;
        self
    }

    /// 图标边长（icon-only 时同时决定容器边长）。
    pub fn icon_size(mut self, size: impl Into<Pixels>) -> Self {
        self.icon_size = size.into();
        self
    }

    /// 容器边长（icon-only 时的方形边长）；有文字时忽略，按内容自适应。
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// 圆角形态（默认 [`ButtonRadius::Medium`]）。
    pub fn radius(mut self, radius: ButtonRadius) -> Self {
        self.radius = radius;
        self
    }

    /// 选中态（前景用 accent 色，背景不变）。
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 无障标签（纯图标按钮建议设置）。
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// 悬停时的光标样式（默认 pointer）。
    pub fn cursor_style(mut self, style: CursorStyle) -> Self {
        self.cursor_style = style;
        self
    }

    /// 点击回调。禁用时不会触发。
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// 悬停提示（`Tooltip::text("...")` 等）。
    pub fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static,
    ) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    /// 同 [`tooltip`](Self::tooltip)，但接收已打包的 `Rc` 工厂
    /// （`IconButton` 等薄壳内部持有 `Rc`，直接转发用）。
    pub fn tooltip_rc(
        mut self,
        tooltip: Rc<dyn Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static>,
    ) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// tooltip 锚点对齐到 trigger 的哪个角（覆盖默认 `Anchor::TopLeft`）。
    pub fn tooltip_anchor(mut self, anchor: Anchor) -> Self {
        self.tooltip_anchor = Some(anchor);
        self
    }

    /// trigger 的哪个角作为 tooltip 定位锚点（覆盖默认 `Anchor::BottomLeft`）。
    pub fn tooltip_attach(mut self, attach: Anchor) -> Self {
        self.tooltip_attach = Some(attach);
        self
    }
}

impl RenderOnce for ButtonLike {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let colors = theme.colors();
        let style_colors = button_like_colors(self.style, &theme);

        let disabled = self.disabled;
        let selected = self.selected;

        // 前景色：disabled → icon_disabled；selected → accent；
        // 显式 icon_color → 取之；否则样式 fg（同时用于图标与文字）。
        let fg = if disabled {
            colors.icon_disabled
        } else if selected {
            colors.icon_accent
        } else {
            self.icon_color.unwrap_or(style_colors.fg)
        };

        let has_label = self.label.is_some();
        let button_id = self.id.clone();
        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .when(has_label, |this| this.gap(px(6.)))
            .aria_label(self.aria_label.clone().unwrap_or_default());

        // 尺寸：icon-only 用方形（size 覆盖 icon_size）；有文字按内边距自适应。
        button = if has_label {
            button.px_3().py_0p5().text_size(px(14.))
        } else {
            let side = self.size.unwrap_or(self.icon_size * 12. / 7.);
            button.size(side)
        };

        button = match self.radius {
            ButtonRadius::Medium => button.rounded_md(),
            ButtonRadius::Full => button.rounded_full(),
            ButtonRadius::Square => button.rounded_none(),
        };

        if let Some(icon) = self.icon {
            button = button.child(Icon::new(icon).size(self.icon_size).color(fg));
        }
        if let Some(label) = self.label {
            button = button.child(div().text_color(fg).child(label));
        }

        if disabled {
            button = button.bg(colors.ghost_element_disabled);
        } else {
            let (bg, hover_bg, active_bg) =
                (style_colors.bg[0], style_colors.bg[1], style_colors.bg[2]);
            button = button
                .bg(bg)
                .border_1()
                .border_color(style_colors.border)
                .hover(move |style| style.bg(hover_bg))
                .active(move |style| style.bg(active_bg));
            button = button.cursor(self.cursor_style);
        }

        button = button.on_click(move |event, window, cx| {
            if disabled {
                return;
            }
            if let Some(handler) = self.on_click.as_ref() {
                handler(event, window, cx);
            }
        });

        // 有悬停提示时包进 TooltipHost（host 与按钮各用自己的 ElementId）。
        match self.tooltip {
            Some(tooltip) => {
                let mut host =
                    TooltipHost::new(ElementId::Name(format!("tip-{button_id:?}").into()))
                        .tooltip(move |window, cx| (tooltip)(window, cx))
                        .trigger(move |_, _window, _cx| button);
                if let Some(anchor) = self.tooltip_anchor {
                    host = host.anchor(anchor);
                }
                if let Some(attach) = self.tooltip_attach {
                    host = host.attach(attach);
                }
                host.into_any_element()
            }
            None => button.into_any_element(),
        }
    }
}

impl Clickable for ButtonLike {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = cursor_style;
        self
    }
}

impl Disableable for ButtonLike {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for ButtonLike {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl ButtonCommon for ButtonLike {
    fn id(&self) -> &ElementId {
        &self.id
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static,
    ) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::button::TintColor;
    use gpui::{TestAppContext, hsla};

    /// 冒烟：builder 链（含 trait 方法）走一遍，字段确实被设置。
    #[test]
    fn button_like_builds_with_traits() {
        let button = ButtonLike::new("test")
            .style(ButtonStyle::Tinted(TintColor::Error))
            .label("保存")
            .icon(IconName::Check)
            .selected(true)
            .disabled(false)
            .on_click(|_, _, _| {})
            .cursor_style(CursorStyle::PointingHand);

        assert!(button.on_click.is_some());
        assert_eq!(button.label.as_deref(), Some("保存"));
        assert!(button.icon.is_some());
        assert!(button.selected);
        assert!(!button.disabled);
    }

    /// 色值计算：Tinted 样式应取到对应状态色的 background/border/fg。
    #[gpui::test]
    fn tinted_style_resolves_status_colors(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let colors = button_like_colors(
                ButtonStyle::Tinted(TintColor::Error),
                cx.theme(),
            );
            let status = cx.theme().status();
            assert_eq!(colors.bg[0], status.error_background);
            assert_eq!(colors.border, status.error_border);
        });
    }

    /// Subtle 常态背景透明、hover 浮现 ghost（语义锁定，防误改）。
    #[gpui::test]
    fn subtle_style_is_transparent_until_hover(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let colors = button_like_colors(ButtonStyle::Subtle, cx.theme());
            let transparent = hsla(0., 0., 0., 0.);
            assert_eq!(colors.bg[0], transparent, "常态应透明");
            assert_eq!(colors.bg[1], cx.theme().colors().ghost_element_hover);
        });
    }
}
