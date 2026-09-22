//! components/button：本包所有按钮。
//!
//! 三个层次，别混用：
//!
//! - [`button`]：**对齐 zed** 的 [`Button`]（`new(id, label)`），建在
//!   [`ButtonLike`] 之上；
//! - [`button_like`]：对齐 zed 的 `ButtonLike` / `ButtonCommon` / `ButtonStyle`，
//!   以及用它的 [`IconButton`]；
//! - [`split_button`]：对齐 zed 的 `SplitButton`。
//!
//! 图标来自 `aa_gpui_base`（独立的基础控件包，见本 crate 顶部文档）。

/// 按键提示在按钮上的位置（对齐 zed `KeybindingPosition`）。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum KeybindingPosition {
    /// 按键提示在文字之前。
    Start,
    /// 按键提示在文字之后（默认）。
    #[default]
    End,
}

use std::rc::Rc;

use gpui::{
    Anchor, AnyView, App, ClickEvent, CursorStyle, ElementId, Hsla, IntoElement, Pixels,
    SharedString, Window, prelude::*, px,
};

use aa_gpui_base::{IconName, IconSize};
use crate::traits::{Clickable, Disableable, Toggleable};
use crate::Color;
use aa_gpui_kit_theme::ActiveTheme;
pub mod button;
pub mod button_like;
pub mod copy_button;
pub mod split_button;

pub use button::Button;
pub use button_like::{ButtonCommon, ButtonLike, ButtonSize};
pub use copy_button::CopyButton;

/// 点击回调（`ButtonLike` / `IconButton` 的内部存储类型）。
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub use split_button::{SplitButton, SplitButtonKind, SplitButtonStyle};

/// 按钮视觉语义（对齐 zed `ButtonStyle`，去掉需主题扩展的部分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonStyle {
    /// 实心背景（`element_background`），强调用。
    Filled,
    /// 实心背景 + 边框（`border_variant`）。
    Outlined,
    /// 透明背景 + 边框，比 Outlined 更弱。
    OutlinedGhost,
    /// 默认：透明背景，hover/active 浮现 ghost 背景。
    #[default]
    Subtle,
    /// 完全透明，hover 也不出背景（仅前景色变化）。
    Transparent,
    /// 语义色调底（信息/错误/警告/成功状态色）。
    Tinted(TintColor),
}

/// [`ButtonStyle::Tinted`] 的语义色选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TintColor {
    #[default]
    Accent,
    Error,
    Warning,
    Success,
}

/// 一组待应用的按钮色（背景 / 边框 / 前景）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonRadius {
    /// 常规圆角 `rounded_md`（默认）。
    #[default]
    Medium,
    /// 圆形 `rounded_full`。
    Full,
    /// 直角（对齐 zed `IconButtonShape::Square`）。
    Square,
}

/// 图标按钮：不可变、一次性渲染的方形图标按钮。
///
/// 用法（与 zed 的 `IconButton` 同构）：
/// ```ignore
/// IconButton::new("app-menu", IconName::Menu)
///     .style(ButtonStyle::Subtle)
///     .icon_size(IconSize::Small)
///     .on_click(|_, window, cx| { /* ... */ })
/// ```
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    style: ButtonStyle,
    icon: IconName,
    /// 图标边长档位（对齐 zed：`IconButton::icon_size` 收 `IconSize`）。
    icon_size: IconSize,
    /// 图标颜色（对齐 zed：收语义色 `Color`，渲染时经 `cx.theme()` 解析）。
    icon_color: Color,
    /// 容器边长（默认 24px，适配标题栏/状态栏）。
    size: Pixels,
    radius: ButtonRadius,
    selected: bool,
    disabled: bool,
    aria_label: Option<SharedString>,
    on_click: Option<ClickHandler>,
    /// 悬停提示（[`Tooltip::text`] 等工厂现场建实体）。
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    /// 提示锚点（默认 `Anchor::TopLeft`）。
    tooltip_anchor: Option<Anchor>,
    /// 提示 attachment（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    tooltip_attach: Option<Anchor>,
    /// 悬停时的光标样式；`None` 时可点状态用 pointer（trait [`Clickable`] 设置）。
    cursor_style: Option<CursorStyle>,
    /// 只在指定 group 被 hover 时显示（转发给 [`ButtonLike`]）。
    visible_on_hover: Option<SharedString>,
}

impl IconButton {
    /// 用给定 id 与图标新建按钮。id 在同一父容器内需唯一。
    pub fn new(id: impl Into<ElementId>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            style: ButtonStyle::Subtle,
            icon,
            icon_size: IconSize::default(),
            icon_color: Color::Default,
            size: px(24.0),
            radius: ButtonRadius::Medium,
            selected: false,
            disabled: false,
            aria_label: None,
            on_click: None,
            tooltip: None,
            tooltip_anchor: None,
            tooltip_attach: None,
            cursor_style: None,
            visible_on_hover: None,
        }
    }

    /// 应用 [`ButtonStyle`]。
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// 图标边长（对齐 zed：收 `IconSize` 档位）。
    pub fn icon_size(mut self, size: IconSize) -> Self {
        self.icon_size = size;
        self
    }

    /// 图标颜色（对齐 zed：收语义色 [`Color`]）。
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = color;
        self
    }

    /// 容器边长（默认 24px）。
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// 圆角形态（默认 [`ButtonRadius::Medium`]）。
    pub fn radius(mut self, radius: ButtonRadius) -> Self {
        self.radius = radius;
        self
    }

    /// 选中态（图标用 accent 色，背景不变）。
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 无障标签（纯图标按钮建议设置）。
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// 禁用：不响应点击与 hover，视觉置灰。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
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

    /// 只在指定 group 被 hover 时显示（转发给 `ButtonLike`）。
    pub fn visible_on_hover(mut self, group: impl Into<SharedString>) -> Self {
        self.visible_on_hover = Some(group.into());
        self
    }
}

impl RenderOnce for IconButton {
    #[allow(refining_impl_trait)]
    fn render(self, window: &mut Window, cx: &mut App) -> ButtonLike {
        let theme = cx.theme().clone();
        let colors = theme.colors();
        let style_colors = button_like::button_like_colors(self.style, &theme);
        let disabled = self.disabled;
        let selected = self.selected;

        // 图标边长：`IconSize` 档位 → px（与 Button 内图标同一换算方式）。
        let icon_size = self.icon_size.rems() * window.rem_size();

        // 图标前景：disabled → 置灰；selected → accent；
        // 否则取语义色（默认档位回落样式的 fg）。
        let icon_color = if disabled {
            colors.icon_disabled
        } else if selected {
            colors.icon_accent
        } else if self.icon_color == Color::Default {
            style_colors.fg
        } else {
            self.icon_color.color(cx)
        };

        let mut like = ButtonLike::new(self.id)
            .style(self.style)
            .icon(self.icon)
            .icon_color(Some(icon_color))
            .icon_size(icon_size)
            .box_size(self.size)
            .radius(self.radius)
            .selected(selected)
            .disabled(disabled)
            .cursor_style(self.cursor_style.unwrap_or(CursorStyle::PointingHand));

        if let Some(aria_label) = self.aria_label {
            like = like.aria_label(aria_label);
        }
        // on_click：ButtonLike 内部已处理 disabled 短路，这里只转发回调。
        let handler = self.on_click;
        like = like.on_click(move |event, window, cx| {
            if let Some(handler) = handler.as_ref() {
                handler(event, window, cx);
            }
        });
        if let Some(tooltip) = self.tooltip {
            like = like.tooltip_rc(tooltip);
        }
        if let Some(anchor) = self.tooltip_anchor {
            like = like.tooltip_anchor(anchor);
        }
        if let Some(attach) = self.tooltip_attach {
            like = like.tooltip_attach(attach);
        }
        if let Some(group) = self.visible_on_hover {
            like = like.visible_on_hover(group);
        }
        like
    }
}

impl Clickable for IconButton {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn cursor_style(mut self, cursor_style: CursorStyle) -> Self {
        self.cursor_style = Some(cursor_style);
        self
    }
}

impl Disableable for IconButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Toggleable for IconButton {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// trait 体系冒烟:builder 链经 trait 方法走一遍,字段确实被设置。
    #[test]
    fn icon_button_impls_interactive_traits() {
        let button = IconButton::new("test", IconName::Close)
            .on_click(|_, _, _| {})
            .cursor_style(CursorStyle::Crosshair)
            .toggle_state(true)
            .disabled(false);

        assert!(button.on_click.is_some(), "Clickable::on_click 应生效");
        assert!(
            button.cursor_style.is_some(),
            "Clickable::cursor_style 应生效"
        );
        assert!(button.selected, "Toggleable::toggle_state 应生效");
        assert!(!button.disabled, "Disableable::disabled(false) 应生效");
    }
}
