//! component/button：复合按钮（在 [`crate::base::button`] 基础上扩展）。
//!
//! 基础层 [`crate::base::icon`]/[`crate::base::button`] 只负责"单个图标"与
//! "文字按钮"；这里把图标 + 按钮交互 + 主题样式拼成更高层的组件：
//!
//! - [`ButtonStyle`]：对齐 zed `ButtonStyle` 的视觉语义（Filled/Outlined/
//!   OutlinedGhost/Subtle/Transparent/Tinted），颜色从当前主题读取
//!   （`cx.theme().colors()` / `status()`），而非基础层那样硬编码。
//! - [`IconButton`]：图标按钮。zed 里它是最高频的组件（标题栏、状态栏、
//!   tab 全部用它），所以单独成件放在 component 层。
//!
//! 与基础层的差异：基础按钮一次性 `IntoElement` 直接产出 `Div`（无 App，
//! 只能硬编码色）；本组件实现 gpui 的 `RenderOnce`，render 阶段能拿到
//! `&mut App` 读主题，与 zed 的 `IconButton` 一致。

use std::rc::Rc;

use gpui::{
    Anchor, App, ClickEvent, ElementId, Entity, Hsla, IntoElement, Pixels, SharedString, Window,
    div, hsla, prelude::*, px,
};

use crate::base::button::ClickHandler;
use crate::base::icon::{Icon, IconName};
use crate::base::theme::{ActiveTheme, Theme};
use crate::component::tooltip::{Tooltip, TooltipHost};

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

impl TintColor {
    /// 取该语义色的 (background, border)（对齐 zed `TintColor::button_like_style`）。
    fn status_color(self, theme: &Theme) -> (Hsla, Hsla) {
        let status = theme.status();
        let color = match self {
            // zed 的 Accent 实际套用 info 状态色。
            TintColor::Accent => &status.info,
            TintColor::Error => &status.error,
            TintColor::Warning => &status.warning,
            TintColor::Success => &status.success,
        };
        (color.background, color.border)
    }
}

/// 一组待应用的按钮色（背景 / 边框 / 前景）。
#[derive(Clone, Copy)]
struct ButtonColors {
    /// 常态 / hover / active 各自的背景。
    bg: [Hsla; 3],
    /// 边框色（默认透明）。
    border: Hsla,
    /// 图标 / 文字前景色。
    fg: Hsla,
}

impl ButtonStyle {
    /// 解析该样式在 `enabled` / `hovered` / `active` 三个状态下的颜色。
    fn colors(self, theme: &Theme) -> ButtonColors {
        let colors = theme.colors();
        let transparent = hsla(0., 0., 0., 0.);
        let text = colors.text;

        match self {
            ButtonStyle::Filled => {
                let mut hover_bg = colors.element_background;
                hover_bg.fade_out(0.5);
                ButtonColors {
                    bg: [colors.element_background, hover_bg, colors.element_active],
                    border: transparent,
                    fg: text,
                }
            }
            ButtonStyle::Outlined => ButtonColors {
                bg: [
                    colors.element_background,
                    colors.ghost_element_hover,
                    colors.element_active,
                ],
                border: colors.border_variant,
                fg: text,
            },
            ButtonStyle::OutlinedGhost => ButtonColors {
                bg: [
                    transparent,
                    colors.ghost_element_hover,
                    colors.ghost_element_active,
                ],
                border: colors.border_variant,
                fg: text,
            },
            ButtonStyle::Subtle => ButtonColors {
                bg: [
                    transparent,
                    colors.ghost_element_hover,
                    colors.ghost_element_active,
                ],
                border: transparent,
                fg: text,
            },
            ButtonStyle::Transparent => ButtonColors {
                bg: [transparent, transparent, transparent],
                border: transparent,
                // zed：Transparent 前景随 hover 变为 muted。
                fg: colors.text_muted,
            },
            ButtonStyle::Tinted(tint) => {
                let (bg, border) = tint.status_color(theme);
                ButtonColors {
                    bg: [bg, bg, colors.element_active],
                    border,
                    fg: text,
                }
            }
        }
    }
}

/// 图标按钮圆角。
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
///     .icon_size(px(14.))
///     .on_click(|_, window, cx| { /* ... */ })
/// ```
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    style: ButtonStyle,
    icon: IconName,
    icon_size: Pixels,
    /// 图标颜色；`None` 时跟随样式 `fg`（disabled/selected 有更高优先级）。
    icon_color: Option<Hsla>,
    /// 容器边长（默认 24px，适配标题栏/状态栏）。
    size: Pixels,
    radius: ButtonRadius,
    selected: bool,
    disabled: bool,
    aria_label: Option<SharedString>,
    on_click: Option<ClickHandler>,
    /// 悬停提示（[`Tooltip::text`] 等工厂现场建实体）。
    tooltip: Option<Rc<dyn Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static>>,
    /// 提示锚点（默认 `Anchor::TopLeft`）。
    tooltip_anchor: Option<Anchor>,
    /// 提示 attachment（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    tooltip_attach: Option<Anchor>,
}

impl IconButton {
    /// 用给定 id 与图标新建按钮。id 在同一父容器内需唯一。
    pub fn new(id: impl Into<ElementId>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            style: ButtonStyle::Subtle,
            icon,
            icon_size: px(14.0),
            icon_color: None,
            size: px(24.0),
            radius: ButtonRadius::Medium,
            selected: false,
            disabled: false,
            aria_label: None,
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

    /// 图标边长。
    pub fn icon_size(mut self, size: impl Into<Pixels>) -> Self {
        self.icon_size = size.into();
        self
    }

    /// 指定图标颜色（默认取样式的 `fg`）。
    pub fn icon_color(mut self, color: impl Into<Hsla>) -> Self {
        self.icon_color = Some(color.into());
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
    pub fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static,
    ) -> Self {
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
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let colors = theme.colors();
        let style_colors = self.style.colors(&theme);

        let disabled = self.disabled;
        let selected = self.selected;
        let handler = self.on_click;

        // 图标前景：disabled → 置灰；显式 icon_color → 取之；
        // selected → accent；否则样式 fg。
        let icon_color = if disabled {
            colors.icon_disabled
        } else if let Some(icon_color) = self.icon_color {
            icon_color
        } else if selected {
            colors.icon_accent
        } else {
            style_colors.fg
        };

        let icon = Icon::new(self.icon).size(self.icon_size).color(icon_color);

        let button_id = self.id.clone();

        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .size(self.size)
            .child(icon);

        button = match self.radius {
            ButtonRadius::Medium => button.rounded_md(),
            ButtonRadius::Full => button.rounded_full(),
            ButtonRadius::Square => button.rounded_none(),
        };

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
                .active(move |style| style.bg(active_bg))
                .cursor_pointer();
        }

        button = button.on_click(move |event, window, cx| {
            if disabled {
                return;
            }
            if let Some(handler) = handler.as_ref() {
                handler(event, window, cx);
            }
        });

        if let Some(aria_label) = self.aria_label {
            button = button.aria_label(aria_label);
        }

        let button = button.into_any_element();

        // 有悬停提示时把按钮包进 TooltipHost（host 与按钮各用自己的 ElementId）。
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
            None => button,
        }
    }
}
