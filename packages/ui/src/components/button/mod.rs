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

use crate::{IconName, IconSize};
use std::rc::Rc;

use gpui::{
    Anchor, AnyView, App, ClickEvent, CursorStyle, DefiniteLength, ElementId, FocusHandle, Hsla,
    IntoElement, Pixels, SharedString, Window, prelude::*, px,
};

use crate::styles::ElevationIndex;
use crate::traits::{Clickable, Disableable, FixedWidth, SelectableButton, Toggleable};
use crate::{Color, Indicator};
use aa_gpui_kit_theme::ActiveTheme as _;
pub mod button;
pub mod button_like;
pub mod button_link;
pub mod copy_button;
pub mod split_button;
pub mod toggle_button;

pub use button::Button;
pub use button_like::{ButtonCommon, ButtonLike, ButtonSize, IconPosition};
pub use button_link::ButtonLink;
pub use copy_button::CopyButton;

/// 点击回调（`ButtonLike` / `IconButton` 的内部存储类型）。
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub use split_button::{SplitButton, SplitButtonKind, SplitButtonStyle};
pub use toggle_button::{
    ButtonBuilder, ButtonConfiguration, ToggleButtonGroup, ToggleButtonGroupSize, ToggleButtonGroupStyle,
    ToggleButtonPosition, ToggleButtonSimple, ToggleButtonWithIcon,
};

/// 按钮视觉语义（对齐 zed `ButtonStyle`，去掉需主题扩展的部分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonStyle {
    /// 实心背景（`element_background`），强调用。
    Filled,
    /// 实心背景 + 边框（`border_variant`）。
    Outlined,
    /// 透明背景 + 边框，比 Outlined 更弱。
    OutlinedGhost,
    /// 边框色由调用方指定（对齐 zed `ButtonStyle::OutlinedCustom(Hsla)`，
    /// editor 标题栏的自定义强调按钮用）。
    OutlinedCustom(Hsla),
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

/// The shape of an [`IconButton`]（对齐 zed `components/button/icon_button.rs:12`）。
///
/// 与 [`ButtonRadius`] 是两套东西：`ButtonRadius` 是我们渲染 `Button` 时用的
/// 圆角档位；`IconButtonShape` 是 zed `IconButton` 的对外形状参数，
/// workspace 照搬 zed 代码时会直接写 `IconButtonShape::Square` / `::Wide`。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum IconButtonShape {
    Square,
    Wide,
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
    /// 形状（对齐 zed：Square 定宽方形 / Wide 跟随内容；zed 默认 `Wide`）。
    shape: IconButtonShape,
    radius: ButtonRadius,
    selected: bool,
    /// 选中态换用的样式（对齐 zed `IconButton::selected_style`，由
    /// [`SelectableButton`] 设入）。渲染时转发给 [`ButtonLike::selected_style`]。
    selected_style: Option<ButtonStyle>,
    disabled: bool,
    aria_label: Option<SharedString>,
    /// 弹出层展开态（dropdown / disclosure 触发器用，转发给 [`ButtonLike`]）。
    aria_expanded: Option<bool>,
    on_click: Option<ClickHandler>,
    /// 右键点击（转发给 [`ButtonLike::on_right_click`]，对齐 zed `IconButton::on_right_click`）。
    on_right_click: Option<ClickHandler>,
    /// 固定宽（对齐 zed `FixedWidth for IconButton`，标题栏的折叠按钮用）。
    width: Option<DefiniteLength>,
    /// 悬停提示（[`Tooltip::text`] 等工厂现场建实体）。
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    /// 可悬浮提示（转发给 [`ButtonLike::hoverable_tooltip`]，对齐 zed
    /// `IconButton::hoverable_tooltip`）。
    hoverable_tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    /// 提示锚点（默认 `Anchor::TopLeft`）。
    tooltip_anchor: Option<Anchor>,
    /// 提示 attachment（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    tooltip_attach: Option<Anchor>,
    /// 悬停时的光标样式；`None` 时可点状态用 pointer（trait [`Clickable`] 设置）。
    cursor_style: Option<CursorStyle>,
    /// 只在指定 group 被 hover 时显示（转发给 [`ButtonLike`]）。
    visible_on_hover: Option<SharedString>,
    /// 选中态下换用的图标（对齐 zed `IconButton::selected_icon`）。
    ///
    /// zed 的用法：`.toggle_state(zoomed).selected_icon(IconName::Minimize)`
    /// —— 同一个按钮在开/关两种状态下显示不同图标（最大化/还原）。
    selected_icon: Option<IconName>,
    /// 选中态下的图标色；`None` 时用 [`colors.icon_accent`] 那套默认
    /// （对齐 zed `IconButton::selected_icon_color`，zed 默认 `Color::Selected`）。
    selected_icon_color: Option<Color>,
    /// 整体不透明度系数（对齐 zed `IconButton::alpha`，仅作用于普通态图标色；
    /// 动画里做呼吸闪烁用）。
    alpha: Option<f32>,
    /// 图标右下角的徽标（对齐 zed `IconButton::indicator`）。
    indicator: Option<Indicator>,
    /// 徽标外圈描边色（对齐 zed `IconButton::indicator_border_color`）。
    ///
    /// zed 收 `Option<Hsla>`（已经是解析后的实色，不是语义色）。
    indicator_border_color: Option<Hsla>,
    /// 按下态底色覆盖（对齐 zed `IconButton::active_background`），
    /// 渲染时转发给 [`ButtonLike::active_background`]。
    active_background: Option<Hsla>,
    /// 悬停态底色覆盖（对齐 zed `IconButton::hover_background`），
    /// 渲染时转发给 [`ButtonLike::hover_background`]。
    hover_background: Option<Hsla>,
    // ---- 以下四项由 [`ButtonCommon`] 设置，渲染时转发给 [`ButtonLike`] ----
    /// Tab 键导航序号（`ButtonCommon::tab_index`）。
    tab_index: Option<isize>,
    /// 尺寸档位（`ButtonCommon::size`）。`None` 时用上面的 `size`（方形边长）。
    button_size: Option<ButtonSize>,
    /// 视觉层级（`ButtonCommon::layer`）。
    layer: Option<ElevationIndex>,
    /// 焦点跟踪（`ButtonCommon::track_focus`）。
    focus_handle: Option<FocusHandle>,
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
            shape: IconButtonShape::Wide,
            radius: ButtonRadius::Medium,
            selected: false,
            selected_style: None,
            disabled: false,
            aria_label: None,
            aria_expanded: None,
            on_click: None,
            on_right_click: None,
            width: None,
            tooltip: None,
            hoverable_tooltip: None,
            tooltip_anchor: None,
            tooltip_attach: None,
            cursor_style: None,
            visible_on_hover: None,
            selected_icon: None,
            selected_icon_color: None,
            alpha: None,
            indicator: None,
            indicator_border_color: None,
            active_background: None,
            hover_background: None,
            tab_index: None,
            button_size: None,
            layer: None,
            focus_handle: None,
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

    /// 按下态底色覆盖（对齐 zed `IconButton::active_background`）。
    pub fn active_background(mut self, background: Hsla) -> Self {
        self.active_background = Some(background);
        self
    }

    /// 悬停态底色覆盖（对齐 zed `IconButton::hover_background`）。
    pub fn hover_background(mut self, background: Hsla) -> Self {
        self.hover_background = Some(background);
        self
    }

    /// 图标色的不透明度系数（对齐 zed `IconButton::alpha`；只作用于普通态，
    /// disabled / selected 态不受影响）。
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(alpha);
        self
    }

    /// 形状（对齐 zed：Square 定宽方形 / Wide 跟随内容宽度）。
    pub fn shape(mut self, shape: IconButtonShape) -> Self {
        self.shape = shape;
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

    /// 弹出层展开态（对齐 zed：disclosure / dropdown 触发器设置）。
    pub fn aria_expanded(mut self, expanded: bool) -> Self {
        self.aria_expanded = Some(expanded);
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

    /// 右键点击回调（对齐 zed `IconButton::on_right_click`）。禁用时不会触发。
    pub fn on_right_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click = Some(Box::new(handler));
        self
    }

    /// 悬停提示（`Tooltip::text("...")` 等）。
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    /// 可悬浮的提示：鼠标移进提示本身时它不会消失，因此里面可以放链接 / 按钮。
    ///
    /// 对齐 zed `IconButton::hoverable_tooltip`（icon_button.rs:131）——zed 同样
    /// 只是把闭包转给 `ButtonLike`，最终落到 gpui 的
    /// `Stateful<Div>::hoverable_tooltip`。
    pub fn hoverable_tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.hoverable_tooltip = Some(Rc::new(tooltip));
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

    /// 选中态下换用的图标（对齐 zed `IconButton::selected_icon`）。
    ///
    /// 只在 `selected(true)`（或 `Toggleable::toggle_state(true)`）时生效。
    pub fn selected_icon(mut self, icon: impl Into<Option<IconName>>) -> Self {
        self.selected_icon = icon.into();
        self
    }

    /// 选中态下的图标色（对齐 zed `IconButton::selected_icon_color`）。
    pub fn selected_icon_color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.selected_icon_color = color.into();
        self
    }

    /// 图标右下角挂一个徽标（对齐 zed `IconButton::indicator`）。
    pub fn indicator(mut self, indicator: Indicator) -> Self {
        self.indicator = Some(indicator);
        self
    }

    /// 徽标外圈描边色（对齐 zed `IconButton::indicator_border_color`）。
    ///
    /// 常见用法是传按钮所在容器的背景色，让徽标看起来是"挖空"的。
    pub fn indicator_border_color(mut self, color: Option<Hsla>) -> Self {
        self.indicator_border_color = color;
        self
    }
}

impl RenderOnce for IconButton {
    #[allow(refining_impl_trait)]
    fn render(self, window: &mut Window, cx: &mut App) -> ButtonLike {
        let disabled = self.disabled;
        let selected = self.selected;

        // 图标边长：`IconSize` 档位 → px（与 Button 内图标同一换算方式）。
        let icon_size = self.icon_size.rems() * window.rem_size();

        // 图标前景（对齐 zed icon_button.rs:243-252）：disabled → Disabled；
        // selected → 显式 `selected_icon_color`，否则 `Color::Selected`；
        // 普通态取语义色并乘 `alpha`。
        let icon_color = if disabled {
            Color::Disabled.color(cx)
        } else if selected {
            self.selected_icon_color
                .unwrap_or(Color::Selected)
                .color(cx)
        } else {
            self.icon_color.color(cx).opacity(self.alpha.unwrap_or(1.0))
        };

        // 选中态换图标（zed `selected_icon`）。
        let icon = if selected {
            self.selected_icon.unwrap_or(self.icon)
        } else {
            self.icon
        };

        let mut like = ButtonLike::new(self.id)
            .style(self.style)
            .icon(icon)
            .icon_color(Some(icon_color))
            .icon_size(icon_size)
            // 对齐 zed（icon_button.rs:268-275）：Square 用 `icon_size.square()`
            // 同时定宽与高；Wide 不设宽度，跟随内容。
            .map(|this| match self.shape {
                IconButtonShape::Square => this.box_size(self.icon_size.square(window, cx)),
                IconButtonShape::Wide => this,
            })
            .radius(self.radius)
            .selected(selected)
            // zed：`IconButton::selected_style` 同时喂给 base（zed icon_button.rs:157）
            // 和自己的 `selected_style` 字段；我们只有一个下游，直接转发。
            .when_some(self.selected_style, |this, style| this.selected_style(style))
            // 底色覆盖（对齐 zed icon_button.rs:88-97 直接写 base 的字段；
            // 我们的 ButtonLike 在渲染时才建，这里用 when_some 转发）。
            .when_some(self.hover_background, |this, background| {
                this.hover_background(background)
            })
            .when_some(self.active_background, |this, background| {
                this.active_background(background)
            })
            .disabled(disabled)
            .cursor_style(self.cursor_style.unwrap_or(CursorStyle::PointingHand));

        if let Some(aria_label) = self.aria_label {
            like = like.aria_label(aria_label);
        }
        if let Some(expanded) = self.aria_expanded {
            like = like.aria_expanded(expanded);
        }
        // on_click：ButtonLike 内部已处理 disabled 短路，这里只转发回调。
        let handler = self.on_click;
        like = like.on_click(move |event, window, cx| {
            if let Some(handler) = handler.as_ref() {
                handler(event, window, cx);
            }
        });
        if let Some(on_right_click) = self.on_right_click {
            like = like.on_right_click(move |event, window, cx| {
                (on_right_click)(event, window, cx);
            });
        }
        if let Some(width) = self.width {
            like = like.width(width);
        }
        if let Some(tooltip) = self.tooltip {
            like = like.tooltip_rc(tooltip);
        }
        if let Some(hoverable_tooltip) = self.hoverable_tooltip {
            like = like.hoverable_tooltip_rc(hoverable_tooltip);
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
        if let Some(indicator) = self.indicator {
            like = like.indicator(indicator);
        }
        if let Some(color) = self.indicator_border_color {
            like = like.indicator_border_color(Some(color));
        }
        // ButtonCommon 那四项
        if let Some(tab_index) = self.tab_index {
            like = like.tab_index(tab_index);
        }
        if let Some(button_size) = self.button_size {
            like = like.size(button_size);
        }
        if let Some(layer) = self.layer {
            like = like.layer(layer);
        }
        if let Some(focus_handle) = self.focus_handle {
            like = like.track_focus(&focus_handle);
        }
        like
    }
}

// zed 的 IconButton 有这个 impl（`icon_button.rs:195`），它是靠它对外提供
// `tab_index` / `size` / `layer` / `track_focus` 的；缺了它不仅 `.tab_index()`
// 调不到，`PopoverMenu::trigger_with_tooltip`（约束 `T: PopoverTrigger +
// ButtonCommon`）也没法拿 IconButton 当触发器。
//
// 与 zed 的差异：zed 的 IconButton 持有一个 `ButtonLike` 字段，trait 方法直接
// 转发给它；我们是在 `render` 时才建 `ButtonLike`，所以这里只记状态，
// 渲染阶段再转发。
impl ButtonCommon for IconButton {
    fn id(&self) -> &ElementId {
        &self.id
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }

    /// 尺寸档位（对齐 zed `ButtonCommon::size`）。
    ///
    /// zed 的 IconButton **没有**固有 `size`，所以 `.size(ButtonSize::None)`
    /// 走的是这里。我们原先有个收 `Pixels` 的固有 `size`，会把它挡住
    /// （固有方法优先，报 `Pixels: From<ButtonSize>`）——已删，方形边长
    /// 改用 [`ButtonSize`] 档位表达。
    fn size(mut self, size: ButtonSize) -> Self {
        self.button_size = Some(size);
        self
    }

    fn layer(mut self, layer: ElevationIndex) -> Self {
        self.layer = Some(layer);
        self
    }

    fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
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

impl SelectableButton for IconButton {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.selected_style = Some(style);
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

    /// 徽标（对齐 zed `IconButton::indicator`）：字段被设置，颜色能读回。
    #[test]
    fn icon_button_stores_indicator() {
        let button = IconButton::new("test", IconName::Bell)
            .indicator(Indicator::dot().color(Color::Accent))
            .indicator_border_color(Some(gpui::hsla(0., 0., 0., 1.)));

        let indicator = button.indicator.expect("indicator 应被设置");
        assert_eq!(indicator.color, Color::Accent);
        assert!(button.indicator_border_color.is_some());
    }
}

impl FixedWidth for IconButton {
    fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    fn full_width(mut self) -> Self {
        self.width = Some(DefiniteLength::Fraction(1.0));
        self
    }
}
