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

use crate::{Icon, IconName};
use std::rc::Rc;

use gpui::{
    Anchor, AnyElement, AnyView, App, ClickEvent, CursorStyle, DefiniteLength, ElementId,
    FocusHandle, Hsla, InteractiveElement, IntoElement, MouseButton, MouseClickEvent,
    MouseDownEvent, MouseUpEvent, ParentElement, Pixels, Rems, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px,
};

use aa_gpui_kit_theme::ActiveTheme;

use crate::components::button::ClickHandler;
use crate::components::button::{ButtonRadius, ButtonStyle};
use crate::components::icon::IconWithIndicator;
use crate::components::indicator::Indicator;
use crate::styles::{DynamicSpacing, ElevationIndex};
use crate::components::tooltip::TooltipHost;
use crate::traits::{Clickable, Disableable, StyledExt as _, Toggleable};
use crate::styles::units::rems_from_px;

/// 图标在按钮 / 菜单条目里的哪一侧（对齐 zed `IconPosition`，
/// zed `button_like.rs:49`）。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum IconPosition {
    /// 图标在文字之前（默认）。
    #[default]
    Start,
    /// 图标在文字之后。
    End,
}

/// 按钮尺寸档位（对齐 zed `ButtonSize`）。
///
/// `rems()` 给的是**容器高度**；宽度由左右内边距与内容决定（见渲染里的 px 分档）。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum ButtonSize {
    Large,
    Medium,
    #[default]
    Default,
    Compact,
    None,
}

impl ButtonSize {
    pub fn rems(self) -> Rems {
        match self {
            ButtonSize::Large => rems_from_px(32_f32),
            ButtonSize::Medium => rems_from_px(28_f32),
            ButtonSize::Default => rems_from_px(22_f32),
            ButtonSize::Compact => rems_from_px(18_f32),
            ButtonSize::None => rems_from_px(16_f32),
        }
    }
}

pub use common::ButtonCommon;
pub use style::{ButtonLikeColors, button_like_colors};

/// 角级圆角（对齐 zed `ButtonLikeRounding`）：可只圆某几个角，用来把相邻按钮
/// 拼成一条（如标题栏的左右分段按钮）。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ButtonLikeRounding {
    /// Top-left corner rounding
    pub top_left: bool,
    /// Top-right corner rounding
    pub top_right: bool,
    /// Bottom-right corner rounding
    pub bottom_right: bool,
    /// Bottom-left corner rounding
    pub bottom_left: bool,
}

impl ButtonLikeRounding {
    pub const ALL: Self = Self {
        top_left: true,
        top_right: true,
        bottom_right: true,
        bottom_left: true,
    };
    pub const LEFT: Self = Self {
        top_left: true,
        top_right: false,
        bottom_right: false,
        bottom_left: true,
    };
    pub const RIGHT: Self = Self {
        top_left: false,
        top_right: true,
        bottom_right: true,
        bottom_left: false,
    };
}

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
    /// 图标右下角的徽标（对齐 zed `IconButton::indicator`，经
    /// [`IconWithIndicator`] 落到按钮上）。
    indicator: Option<Indicator>,
    /// 徽标外圈描边色；`None` 时取主题 `elevated_surface_background`。
    indicator_border_color: Option<Hsla>,
    /// 容器边长（icon-only 时的方形边长）；`None` 时按内容自适应。
    box_size: Option<Pixels>,
    /// 宽度覆盖（`FixedWidth::width` / `full_width` 设入）；`None` 时按内容自适应。
    width: Option<DefiniteLength>,
    /// Tab 键导航序号（`ButtonCommon::tab_index`）。
    tab_index: Option<isize>,
    /// 尺寸档位（`ButtonCommon::size`）。
    button_size: ButtonSize,
    /// 高度覆盖；`None` 时取 `button_size.rems()`。
    height: Option<DefiniteLength>,
    /// 视觉层级（`ButtonCommon::layer`），影响取色。
    layer: Option<ElevationIndex>,
    /// 焦点跟踪（`ButtonCommon::track_focus`）。
    focus_handle: Option<FocusHandle>,
    radius: ButtonRadius,
    /// 角级圆角覆盖（对齐 zed `ButtonLike.rounding`）；设了就以它为准，
    /// 否则退回 [`Self::radius`] 的整体档位。
    rounding: Option<ButtonLikeRounding>,
    disabled: bool,
    selected: bool,
    /// 选中态下改用这套样式（对齐 zed `ButtonLike.selected_style`）：
    /// 未设置时选中与否都是 [`Self::style`]。
    selected_style: Option<ButtonStyle>,
    /// 悬停底色覆盖（对齐 zed `ButtonLike.hover_background`）；
    /// `None` 时用样式的悬停色。
    hover_background: Option<Hsla>,
    /// 按下底色覆盖（对齐 zed `ButtonLike.active_background`）；
    /// `None` 时用样式的按下色。
    active_background: Option<Hsla>,
    aria_label: Option<SharedString>,
    cursor_style: CursorStyle,
    on_click: Option<ClickHandler>,
    /// 右键点击（对齐 zed `ButtonLike::on_right_click`：手动拼 Right 键的
    /// `ClickEvent`，在 mouse_up 时触发）。editor 的 runnables 右键菜单用它。
    on_right_click: Option<ClickHandler>,
    /// 悬停提示（工厂现场建视图）。
    ///
    /// 与 zed 一致收 `AnyView`（不是 `Entity<Tooltip>`）—— 这样
    /// `Tooltip::text(..)` / `Tooltip::with_meta(.., cx)` / 任意自定义视图
    /// 都能直接传。
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    /// **可悬浮**的提示（鼠标移进提示本身时它不消失，所以里面能放链接 / 按钮）。
    ///
    /// 与 zed 一致：zed 的 `ButtonLike::hoverable_tooltip` 只是把闭包存下来，
    /// render 时转发给 gpui 的 `Stateful<Div>::hoverable_tooltip`；我们同样
    /// 转给 gpui，不复用上面的 `TooltipHost`（那个 host 移进提示就会收起）。
    hoverable_tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    /// 提示锚点（默认 `Anchor::TopLeft`）。
    tooltip_anchor: Option<Anchor>,
    /// 提示 attachment（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    tooltip_attach: Option<Anchor>,
    /// 追加的子元素（`Button` / `IconButton` 这类壳把自己的内容挂进来）。
    ///
    /// 对齐 zed：zed 的 `ButtonLike` 也是 `ParentElement`，`Button::render`
    /// 直接 `self.base.child(...)`。这比让壳自己去拼 `div` 更贴 zed 的写法。
    children: Vec<AnyElement>,
    // ---- 无障碍（对齐 zed ButtonLike 的 aria_* 系列）----
    /// 无障碍名称；缺省时 `Button` 会用可见文字补上。
    aria_description: Option<SharedString>,
    /// 无障碍当前值（如 combobox 触发器显示当前选项）。
    aria_value: Option<SharedString>,
    /// 覆盖无障碍 role（默认由 gpui 按元素推断）。
    aria_role: Option<gpui::Role>,
    /// 弹出层展开态（dropdown / disclosure 触发器用）。
    aria_expanded: Option<bool>,
    /// 无障碍快捷键串（`aria-keyshortcuts`），如 `"Ctrl-S"`。
    pub(crate) aria_keyshortcuts: Option<SharedString>,
    /// 无障碍动作回调（如辅助技术派发 `Action::Expand`）。
    on_a11y_action: Option<(
        gpui::accesskit::Action,
        Box<dyn FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App) + 'static>,
    )>,
    /// 只在指定 group 被 hover 时显示（对齐 zed `ButtonLike::visible_on_hover`）。
    visible_on_hover: Option<SharedString>,
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
            indicator: None,
            indicator_border_color: None,
            box_size: None,
            width: None,
            tab_index: None,
            button_size: ButtonSize::default(),
            height: None,
            layer: None,
            focus_handle: None,
            radius: ButtonRadius::Medium,
            rounding: None,
            disabled: false,
            selected: false,
            selected_style: None,
            hover_background: None,
            active_background: None,
            aria_label: None,
            cursor_style: CursorStyle::PointingHand,
            on_click: None,
            on_right_click: None,
            tooltip: None,
            hoverable_tooltip: None,
            tooltip_anchor: None,
            tooltip_attach: None,
            children: Vec::new(),
            aria_description: None,
            aria_value: None,
            aria_role: None,
            aria_expanded: None,
            aria_keyshortcuts: None,
            on_a11y_action: None,
            visible_on_hover: None,
        }
    }

    /// 应用 [`ButtonStyle`]。
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// 只在 `group` 被 hover 时显示（否则 `invisible`）。
    pub fn visible_on_hover(mut self, group: impl Into<SharedString>) -> Self {
        self.visible_on_hover = Some(group.into());
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

    /// 图标右下角的徽标（对齐 zed `IconButton::indicator`）。
    pub fn indicator(mut self, indicator: Indicator) -> Self {
        self.indicator = Some(indicator);
        self
    }

    /// 徽标外圈描边色（`None` 时取主题 `elevated_surface_background`）。
    pub fn indicator_border_color(mut self, color: Option<Hsla>) -> Self {
        self.indicator_border_color = color;
        self
    }

    /// 容器边长（icon-only 时的方形边长）；有文字时忽略，按内容自适应。
    ///
    /// 与 `ButtonCommon::size(ButtonSize)` 区别：那是**尺寸档位**（高度档），
    /// 这是**显式边长**。名字取 `box_size` 以避开冲突。
    pub fn box_size(mut self, size: impl Into<Pixels>) -> Self {
        self.box_size = Some(size.into());
        self
    }

    /// 圆角形态（默认 [`ButtonRadius::Medium`]）。
    pub fn radius(mut self, radius: ButtonRadius) -> Self {
        self.radius = radius;
        self
    }

    /// 只圆右侧两角（对齐 zed `ButtonLike::new_rounded_right`）。
    pub fn new_rounded_right(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::RIGHT)
    }

    /// 只圆左侧两角（对齐 zed，与 [`Self::new_rounded_right`] 成对）。
    pub fn new_rounded_left(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::LEFT)
    }

    /// 四角都圆（对齐 zed `ButtonLike::new_rounded_all`）。
    pub fn new_rounded_all(id: impl Into<ElementId>) -> Self {
        Self::new(id).rounding(ButtonLikeRounding::ALL)
    }

    /// 角级圆角（对齐 zed `ButtonLike::rounding`）。
    pub fn rounding(mut self, rounding: impl Into<Option<ButtonLikeRounding>>) -> Self {
        self.rounding = rounding.into();
        self
    }

    /// 选中态（前景用 accent 色，背景不变）。
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 选中态使用的样式（对齐 zed `ButtonLike::selected_style`）。
    pub fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.selected_style = Some(style);
        self
    }

    /// 悬停底色覆盖（对齐 zed `ButtonLike` 的 `hover_background` 字段；
    /// zed 的壳直接写字段，我们字段私有，经此转发）。
    pub fn hover_background(mut self, background: Hsla) -> Self {
        self.hover_background = Some(background);
        self
    }

    /// 按下底色覆盖（对齐 zed `ButtonLike` 的 `active_background` 字段）。
    pub fn active_background(mut self, background: Hsla) -> Self {
        self.active_background = Some(background);
        self
    }

    /// 无障标签（纯图标按钮建议设置）。
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// 无障碍补充说明（在 name / role / value 之后播报）。
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// 无障碍当前值（按钮代表"有值的控件"时用，如 combobox 触发器）。
    pub fn aria_value(mut self, value: impl Into<SharedString>) -> Self {
        self.aria_value = Some(value.into());
        self
    }

    /// 覆盖无障碍 role（默认 [`gpui::Role::Button`]）。
    pub fn aria_role(mut self, role: gpui::Role) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// 弹出层展开态（dropdown / disclosure 触发器用）。
    pub fn aria_expanded(mut self, expanded: bool) -> Self {
        self.aria_expanded = Some(expanded);
        self
    }

    /// 注册无障碍动作处理（如辅助技术派发 `Action::Expand`）。
    pub fn on_a11y_action(
        mut self,
        action: gpui::accesskit::Action,
        listener: impl FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_a11y_action = Some((action, Box::new(listener)));
        self
    }

    /// 追加一个子元素（供 `Button` / `IconButton` 这类壳挂内容）。
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// 批量追加子元素。
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(IntoElement::into_any_element));
        self
    }

    /// 设置无障碍快捷键串（`aria-keyshortcuts`），如 `"Ctrl-S"`。
    ///
    /// 由 [`Button`](super::button::Button) 从可见的 `KeyBinding` 推出后写入 ——
    /// 让读屏用户听到与视觉用户看到的是同一个快捷键。
    pub fn aria_keyshortcuts(mut self, keyshortcuts: impl Into<SharedString>) -> Self {
        self.aria_keyshortcuts = Some(keyshortcuts.into());
        self
    }

    /// 固定宽度（对齐 zed `FixedWidth::width`）。
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// 撑满容器宽度（对齐 zed `FixedWidth::full_width`）。
    pub fn full_width(mut self) -> Self {
        self.width = Some(gpui::relative(1.).into());
        self
    }

    /// Tab 键导航序号（对齐 zed `ButtonCommon::tab_index`）。
    pub fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }

    /// 尺寸档位（对齐 zed `ButtonCommon::size`）。
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.button_size = size;
        self
    }

    /// 高度覆盖（高于 `size` 的默认高度）。
    pub fn height(mut self, height: DefiniteLength) -> Self {
        self.height = Some(height);
        self
    }

    /// 视觉层级（对齐 zed `ButtonCommon::layer`），影响取色。
    pub fn layer(mut self, elevation: ElevationIndex) -> Self {
        self.layer = Some(elevation);
        self
    }

    /// 焦点跟踪（对齐 zed `ButtonCommon::track_focus`）。
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    // ---- 供壳（Button / IconButton）读取状态的访问器 ----
    //
    // 与 zed 的差异：zed 的壳与 `ButtonLike` 同模块，直接读私有字段。
    // 我们分了文件，所以开这几个只读访问器。

    /// 当前是否禁用。
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// 元素 id 的只读引用（`ButtonCommon::id` 用它）。
    pub fn id_ref(&self) -> &ElementId {
        &self.id
    }

    /// 当前是否选中。
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// 已设置的无障碍名称（`None` 表示壳可以用可见文字补上）。
    pub fn aria_label_ref(&self) -> Option<&SharedString> {
        self.aria_label.as_ref()
    }

    /// 已设置的无障碍快捷键串。
    pub fn aria_keyshortcuts_ref(&self) -> Option<&SharedString> {
        self.aria_keyshortcuts.as_ref()
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

    /// 右键点击回调（对齐 zed `ButtonLike::on_right_click`）。禁用时不会触发。
    pub fn on_right_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click = Some(Box::new(handler));
        self
    }



    /// 悬停提示（`Tooltip::text("...")` 等）。
    #[inline(always)]
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Rc::new(tooltip));
        self
    }

    /// 同 [`tooltip`](Self::tooltip)，但接收已打包的 `Rc` 工厂
    /// （`IconButton` 等薄壳内部持有 `Rc`，直接转发用）。
    pub fn tooltip_rc(
        mut self,
        tooltip: Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>,
    ) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// 可悬浮的提示（对齐 zed `ButtonLike::hoverable_tooltip`）。
    ///
    /// 与 [`tooltip`](Self::tooltip) 的区别：提示自身可 hover，鼠标移进去不会消失，
    /// 因此里面能放可交互内容。
    pub fn hoverable_tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.hoverable_tooltip = Some(Rc::new(tooltip));
        self
    }

    /// 同 [`hoverable_tooltip`](Self::hoverable_tooltip)，接收已打包的 `Rc` 工厂
    /// （`IconButton` 等薄壳内部持有 `Rc`，直接转发用）。
    pub fn hoverable_tooltip_rc(
        mut self,
        tooltip: Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>,
    ) -> Self {
        self.hoverable_tooltip = Some(tooltip);
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
        // 对齐 zed：选中时优先用 selected_style，未设置则仍是 self.style。
        let style = self
            .selected_style
            .filter(|_| self.selected)
            .unwrap_or(self.style);
        let style_colors = button_like_colors(style, &theme);

        let disabled = self.disabled;
        let selected = self.selected;

        // 前景色：disabled → `Color::Disabled`（= `text_disabled`，与 zed
        // `styles/color.rs:100` 同）；selected → accent；
        // 显式 icon_color → 取之；否则样式 fg（同时用于图标与文字）。
        let fg = if disabled {
            colors.text_disabled
        } else if selected {
            colors.icon_accent
        } else {
            self.icon_color.unwrap_or(style_colors.fg)
        };

        let button_id = self.id.clone();
        // 对齐 zed button_like.rs:767 —— base 用 `self.base.h_flex()`，
        // **不带 items_center() / justify_center()**。
        //
        // 原先这里写成 `div().flex().items_center().justify_center()`，
        // 把按钮内容整体强制居中。Button 的内容是
        // `h_flex().justify_between().child(Label).when_some(key_binding)` ——
        // 外层再叠一层 items_center + justify_center 后，「标签 + 快捷键」这一对
        // 被推到中间，表现为 ProjectSearch 空状态引导页的按钮文字居中，
        // 而 zed 是左对齐。h_flex() 自带 items_center，足够对齐图标与文字。
        // zed 是 `self.base.h_flex()`；aacode 的 ButtonLike 不是 Div，
        // 故等价地写 `div().h_flex()`。
        let mut button = div()
            .h_flex()
            .id(self.id.clone())
            .flex_none()
            .aria_label(self.aria_label.clone().unwrap_or_default());

        // 无障碍属性（对齐 zed ButtonLike 的 aria_* 系列）。
        if let Some(description) = self.aria_description.clone() {
            button = button.aria_description(description);
        }
        if let Some(value) = self.aria_value.clone() {
            button = button.aria_value(value);
        }
        if let Some(role) = self.aria_role {
            button = button.role(role);
        }
        if let Some(expanded) = self.aria_expanded {
            button = button.aria_expanded(expanded);
        }
        if let Some(keyshortcuts) = self.aria_keyshortcuts.clone() {
            button = button.aria_keyshortcuts(keyshortcuts);
        }
        if let Some((action, mut listener)) = self.on_a11y_action {
            button = button.on_a11y_action(action, move |data, window, cx| {
                listener(data, window, cx)
            });
        }

        // 焦点跟踪（ButtonCommon::track_focus）。
        if let Some(focus_handle) = self.focus_handle.clone() {
            button = button.track_focus(&focus_handle);
        }

        // 尺寸：高度按档位、左右内边距按档位分档（对齐 zed）。
        //
        // zed 的对照（button_like.rs:786-806）：
        //   .h(self.height.unwrap_or(self.size.rems().into()))
        //   .map(|this| match self.size {
        //       Large | Medium   => this.px(DynamicSpacing::Base08.rems(cx)),
        //       Default | Compact=> this.px(DynamicSpacing::Base04.rems(cx)),
        //       None             => this.px_px(),
        //   })
        //
        // `gap` 也按 zed 取 `Base04`。
        button = button
            .h(self.height.unwrap_or_else(|| self.button_size.rems().into()))
            .gap(DynamicSpacing::Base04.rems(cx))
            .map(|this| match self.button_size {
                ButtonSize::Large | ButtonSize::Medium => {
                    this.px(DynamicSpacing::Base08.rems(cx))
                }
                ButtonSize::Default | ButtonSize::Compact => {
                    this.px(DynamicSpacing::Base04.rems(cx))
                }
                // 与 zed 一致：None 档只在左右各留 1px 的视觉呼吸位。
                ButtonSize::None => this.px(px(1.)),
            });

        // 正方形：只在显式给了 `box_size` 时才定死（对齐 zed —— zed 的
        // `ButtonLike` 本身从不猜方形，方形由 `IconButton` 的
        // `IconButtonShape::Square` 下发，见 icon_button.rs:268-275）。
        // 没给 `box_size` 时宽度完全交给内容（文字按钮按文字伸缩）。
        if let Some(box_size) = self.box_size {
            button = button.w(box_size).h(box_size);
        }

        // 宽度覆盖（FixedWidth）与 tab 序号（ButtonCommon）。
        // 对齐 zed（button_like.rs:801-803）：给了宽度就居中内容。
        if let Some(width) = self.width {
            button = button.w(width).justify_center().text_center();
        }
        if let Some(tab_index) = self.tab_index {
            // 对齐 zed（button_like.rs:788-792）：已聚焦的按钮被禁用时仍留在
            // tab 序列里，焦点不会被挤到 view 外面。
            button = button.tab_index(tab_index).tab_stop(!disabled);
        }

        button = match self.rounding {
            Some(rounding) => button
                .when(rounding.top_left, |this| this.rounded_tl_md())
                .when(rounding.top_right, |this| this.rounded_tr_md())
                .when(rounding.bottom_right, |this| this.rounded_br_md())
                .when(rounding.bottom_left, |this| this.rounded_bl_md()),
            None => match self.radius {
                ButtonRadius::Medium => button.rounded_md(),
                ButtonRadius::Full => button.rounded_full(),
                ButtonRadius::Square => button.rounded_none(),
            },
        };

        if let Some(icon) = self.icon {
            let icon_element = Icon::new(icon).size(self.icon_size).color(fg.into());
            // 与 zed 的差异：zed 的 `IconButton` 自己拼
            // `IconWithIndicator::new(icon_element, Some(indicator))`（icon_button.rs:264）；
            // 我们的 `IconButton` 到渲染时才建 `ButtonLike`，所以这一步下移到这里。
            button = button.child(if self.indicator.is_some() {
                IconWithIndicator::new(icon_element, self.indicator)
                    .indicator_border_color(self.indicator_border_color)
                    .into_any_element()
            } else {
                icon_element.into_any_element()
            });
        }
        if let Some(label) = self.label {
            button = button.child(div().text_color(fg).child(label));
        }
        // 壳（Button / IconButton）挂进来的内容。
        //
        // 与 zed 的差异：zed 的 `ButtonLike` 是「纯容器」，内容全由壳通过
        // `child()` 挂入，内置的 `label` / `icon` 字段其实是给直接使用
        // `ButtonLike` 的场景用的。我们保留那套字段，同时接受 children ——
        // 壳走 children，直接使用者走字段。
        button = button.children(self.children);

        // 只在 group 被 hover 时显示（对齐 zed `ButtonLike::visible_on_hover`）。
        if let Some(group) = self.visible_on_hover.clone() {
            button = button.invisible().group_hover(group, |el| el.visible());
        }

        // 对齐 zed（button_like.rs:819-829）：背景/边框**只看样式**，disabled
        // 不换底色 —— zed 的 `ButtonStyle::disabled()` 是死代码（标了
        // `#[allow(unused)]`，全仓无人调用），禁用态只改前景色与光标。
        // 旧实现给 disabled 一律刷 `ghost_element_disabled`，于是禁用的
        // Subtle 图标按钮（如 tab bar 的前后导航）变成一整块纯色，且底色与
        // `icon_disabled` 前景接近到看不出图标。
        let (bg, hover_bg, active_bg) = (style_colors.bg[0], style_colors.bg[1], style_colors.bg[2]);
        // 对齐 zed：壳可以显式指定悬停 / 按下底色（`IconButton::hover_background`
        // / `active_background`），没指定才用样式本身的。
        let hover_bg = self.hover_background.unwrap_or(hover_bg);
        let active_bg = self.active_background.unwrap_or(active_bg);
        // 对齐 zed（button_like.rs:758-804）：只有 Outlined / OutlinedGhost /
        // OutlinedCustom 这类「描边」样式才真的画 1px 边框；其余样式（含
        // Tinted）的 border_color 只是占位，不渲染边框。否则 Tinted(Accent)
        // 会多出一圈 info_border 高亮边，与 zed 不一致（如 ACP Registry 的
        // Install 下载按钮）。
        let is_outlined = matches!(
            style,
            ButtonStyle::Outlined | ButtonStyle::OutlinedGhost | ButtonStyle::OutlinedCustom(_)
        );
        button = button
            .bg(bg)
            .when(is_outlined, |this| this.border_1())
            .border_color(style_colors.border);
        if disabled {
            // zed：默认光标是指手时改成 not-allowed，否则保留自定义光标。
            button = if self.cursor_style == CursorStyle::PointingHand {
                button.cursor_not_allowed()
            } else {
                button.cursor(self.cursor_style)
            };
        } else {
            button = button
                .cursor(self.cursor_style)
                .hover(move |style| style.bg(hover_bg))
                .active(move |style| style.bg(active_bg));
        }

        button = button.on_click(move |event, window, cx| {
            if disabled {
                return;
            }
            if let Some(handler) = self.on_click.as_ref() {
                handler(event, window, cx);
            }
        });

        // 右键：gpui 的 on_click 只认左键，所以照 zed 手动拦 Right 键的
        // mouse_down / mouse_up，在 mouse_up 时拼一个 ClickEvent 触发回调。
        // 禁用时不挂（对齐 zed 的 filter(|_| !self.disabled)）。
        if !disabled && self.on_right_click.is_some() {
            button = button
                .on_mouse_down(MouseButton::Right, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_mouse_up(
                    MouseButton::Right,
                    move |event, window, cx| {
                        cx.stop_propagation();
                        if let Some(handler) = self.on_right_click.as_ref() {
                            let click_event = ClickEvent::Mouse(MouseClickEvent {
                                down: MouseDownEvent {
                                    button: MouseButton::Right,
                                    position: event.position,
                                    modifiers: event.modifiers,
                                    click_count: 1,
                                    first_mouse: false,
                                },
                                up: MouseUpEvent {
                                    button: MouseButton::Right,
                                    position: event.position,
                                    modifiers: event.modifiers,
                                    click_count: 1,
                                },
                            });
                            handler(&click_event, window, cx);
                        }
                    },
                );
        }

        // 可悬浮提示：直接交给 gpui（zed 也是转发给
        // `Stateful<Div>::hoverable_tooltip`，见 button_like.rs:890）。
        if let Some(hoverable_tooltip) = self.hoverable_tooltip.clone() {
            button = button.hoverable_tooltip(move |window, cx| (hoverable_tooltip)(window, cx));
        }

        // 普通悬停提示：直接转发给 gpui Div 的 .tooltip()，与 zed 原版 L887-888 对齐。
        // 不走 TooltipHost——那是自定义 Element，相对按钮 corner 加了额外 gap=px(8)，
        // 会和 tooltip_container 的 padding 叠加，导致 tooltip 位置偏差。
        if let Some(tooltip) = self.tooltip.clone() {
            button = button.tooltip(move |window, cx| tooltip(window, cx));
        }

        button.into_any_element()
    }
}

impl Clickable for ButtonLike {
    #[inline(always)]
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

    fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.button_size = size;
        self
    }

    fn layer(mut self, elevation: ElevationIndex) -> Self {
        self.layer = Some(elevation);
        self
    }

    fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
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
