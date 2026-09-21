//! Checkbox 与 Switch（对齐 zed `crates/ui/src/components/toggle.rs`）。
//!
//! 出处：zed `crates/ui/src/components/toggle.rs`（GPL-3.0-or-later）。
//! 本文件只取 **Checkbox** 与 **Switch** 两个组件；zed 同文件里的
//! `ToggleButton` / `SwitchField` / `CheckboxWithLabel` 暂未移植。
//!
//! 与 zed 的差异：
//! - 少一个 `utils::is_light(cx)` helper，直接用 `theme().appearance()` 判断；
//! - `Icon` / `Label` 收 `Hsla` 而非语义 `Color`，取色处已展开。

use gpui::{
    AnyView, ClickEvent, ElementId, Hsla, IntoElement, Role, SharedString, Styled, Toggled, Window,
    div, prelude::*,
};
use std::{rc::Rc, sync::Arc};

use crate::prelude::*;
use crate::{
    DynamicSpacing, ElevationIndex, IconSize, KeyBinding, ToggleState, UiDensity,
};
use aa_gpui_kit_theme::Appearance;

/// Creates a new checkbox.
pub fn checkbox(id: impl Into<ElementId>, toggle_state: ToggleState) -> Checkbox {
    Checkbox::new(id, toggle_state)
}

/// Creates a new switch.
pub fn switch(id: impl Into<ElementId>, toggle_state: ToggleState) -> Switch {
    Switch::new(id, toggle_state)
}

/// The visual style of a toggle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ToggleStyle {
    /// Toggle has a transparent background
    #[default]
    Ghost,
    /// Toggle has a filled background based on the
    /// elevation index of the parent container
    ElevationBased(ElevationIndex),
    /// A custom style using a color to tint the toggle
    Custom(Hsla),
}

// 与 zed 的差异：zed 从 `theme_settings.ui_density(cx)` 读当前密度；我们没有
// 设置系统，密度显式取 Default。设置系统就绪后改为传当前值。
fn density() -> UiDensity {
    UiDensity::Default
}

/// # Checkbox
///
/// Checkboxes are used for multiple choices, not for mutually exclusive choices.
/// Each checkbox works independently from other checkboxes in the list,
/// therefore checking an additional box does not affect any other selections.
#[derive(IntoElement, RegisterComponent)]
pub struct Checkbox {
    id: ElementId,
    toggle_state: ToggleState,
    style: ToggleStyle,
    disabled: bool,
    placeholder: bool,
    filled: bool,
    visualization: bool,
    label: Option<SharedString>,
    label_size: LabelSize,
    label_color: Color,
    tooltip: Option<Arc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    on_click: Option<Arc<dyn Fn(&ToggleState, &ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    /// Creates a new [`Checkbox`].
    pub fn new(id: impl Into<ElementId>, checked: ToggleState) -> Self {
        Self {
            id: id.into(),
            toggle_state: checked,
            style: ToggleStyle::default(),
            disabled: false,
            placeholder: false,
            filled: false,
            visualization: false,
            label: None,
            label_size: LabelSize::Default,
            label_color: Color::Muted,
            // 与 zed 的差异：zed 这里用 Box，但整个 struct 要 derive Clone 时
            // Box<dyn Fn> 不行；我们统一用 Arc，语义相同且可 Clone。
            tooltip: None,
            on_click: None,
        }
    }

    /// Sets the disabled state of the [`Checkbox`].
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the placeholding state of the [`Checkbox`].
    pub fn placeholder(mut self, placeholder: bool) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Binds a handler to the [`Checkbox`] that will be called when clicked.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Arc::new(move |state, _, window, cx| {
            handler(state, window, cx)
        }));
        self
    }

    pub fn on_click_ext(
        mut self,
        handler: impl Fn(&ToggleState, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Sets the `fill` setting of the checkbox, indicating whether it should be filled.
    pub fn fill(mut self) -> Self {
        self.filled = true;
        self
    }

    /// Makes the checkbox look enabled but without pointer cursor and hover styles.
    /// Primarily used for uninteractive markdown previews.
    pub fn visualization_only(mut self, visualization: bool) -> Self {
        self.visualization = visualization;
        self
    }

    /// Sets the style of the checkbox using the specified [`ToggleStyle`].
    pub fn style(mut self, style: ToggleStyle) -> Self {
        self.style = style;
        self
    }

    /// Match the style of the checkbox to the current elevation using [`ToggleStyle::ElevationBased`].
    pub fn elevation(mut self, elevation: ElevationIndex) -> Self {
        self.style = ToggleStyle::ElevationBased(elevation);
        self
    }

    /// Sets the tooltip for the checkbox.
    pub fn tooltip(mut self, tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self {
        self.tooltip = Some(Arc::new(tooltip));
        self
    }

    /// Set the label for the checkbox.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn label_size(mut self, size: LabelSize) -> Self {
        self.label_size = size;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }
}

impl Checkbox {
    fn bg_color(&self, cx: &App) -> Hsla {
        let style = self.style.clone();
        match (style, self.filled) {
            (ToggleStyle::Ghost, false) => cx.theme().colors().ghost_element_background,
            (ToggleStyle::Ghost, true) => cx.theme().colors().element_background,
            (ToggleStyle::ElevationBased(_), false) => gpui::transparent_black(),
            (ToggleStyle::ElevationBased(elevation), true) => elevation.darker_bg(cx),
            (ToggleStyle::Custom(_), false) => gpui::transparent_black(),
            (ToggleStyle::Custom(color), true) => color.opacity(0.2),
        }
    }

    fn border_color(&self, cx: &App) -> Hsla {
        if self.disabled {
            return cx.theme().colors().border_variant;
        }

        match self.style.clone() {
            ToggleStyle::Ghost => cx.theme().colors().border,
            ToggleStyle::ElevationBased(_) => cx.theme().colors().border,
            ToggleStyle::Custom(color) => color.opacity(0.3),
        }
    }

    pub fn container_size() -> Pixels {
        px(20.0)
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let group_id = format!("checkbox_group_{:?}", self.id);
        let color = if self.disabled {
            Color::Disabled
        } else {
            Color::Selected
        };

        // 与 zed 的差异：我们的 `Icon::size` 收 `Pixels`、`.color()` 收 `Hsla`，
        // 所以把 `IconSize::Small` 用 rem_size 展开成 px、语义色展开成 Hsla。
        let icon_size = IconSize::Small.rems() * window.rem_size();

        let icon = match self.toggle_state {
            ToggleState::Selected => {
                if self.placeholder {
                    None
                } else {
                    Some(
                        Icon::new(IconName::Check)
                            .size(icon_size)
                            .color(color.color(cx)),
                    )
                }
            }
            ToggleState::Indeterminate => Some(
                Icon::new(IconName::Dash)
                    .size(icon_size)
                    .color(color.color(cx)),
            ),
            ToggleState::Unselected => None,
        };

        let bg_color = self.bg_color(cx);
        let border_color = self.border_color(cx);
        let hover_border_color = border_color.alpha(0.7);

        let size = Self::container_size();

        let checkbox = h_flex()
            .group(group_id.clone())
            .id(self.id.clone())
            .size(size)
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .justify_center()
                    .items_center()
                    .m_1()
                    .size_4()
                    .rounded_xs()
                    .bg(bg_color)
                    .border_1()
                    .border_color(border_color)
                    .when(self.disabled, |this| this.cursor_not_allowed())
                    .when(self.disabled, |this| {
                        this.bg(cx.theme().colors().element_disabled.opacity(0.6))
                    })
                    .when(!self.disabled && !self.visualization, |this| {
                        this.group_hover(group_id.clone(), |el| el.border_color(hover_border_color))
                    })
                    .when(self.placeholder, |this| {
                        this.child(
                            div()
                                .flex_none()
                                .rounded_full()
                                .bg(color.color(cx).alpha(0.5))
                                .size(px(4.)),
                        )
                    })
                    .children(icon),
            );

        h_flex()
            .id(self.id)
            .map(|this| {
                if self.disabled {
                    this.cursor_not_allowed()
                } else if self.visualization {
                    this.cursor_default()
                } else {
                    this.cursor_pointer()
                }
            })
            .gap(DynamicSpacing::Base06.rems(density()))
            .child(checkbox)
            .when_some(self.label, |this, label| {
                this.child(Label::new(label).color(self.label_color).size(self.label_size))
            })
            .when_some(self.tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| tooltip(window, cx))
            })
            .when_some(self.on_click.filter(|_| !self.disabled), |this, on_click| {
                this.on_click(move |click, window, cx| {
                    on_click(&self.toggle_state.inverse(), click, window, cx)
                })
            })
    }
}

/// Defines the color for a switch component.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum SwitchColor {
    #[default]
    Accent,
    Custom(Hsla),
}

impl SwitchColor {
    fn get_colors(&self, is_on: bool, cx: &App) -> (Hsla, Hsla) {
        if !is_on {
            return (
                cx.theme().colors().element_disabled,
                cx.theme().colors().border,
            );
        }

        match self {
            SwitchColor::Accent => {
                let status = cx.theme().status();
                let colors = cx.theme().colors();
                (status.info.opacity(0.4), colors.text_accent.opacity(0.2))
            }
            SwitchColor::Custom(color) => (*color, color.opacity(0.6)),
        }
    }
}

impl From<SwitchColor> for Color {
    fn from(color: SwitchColor) -> Self {
        match color {
            SwitchColor::Accent => Color::Accent,
            SwitchColor::Custom(_) => Color::Default,
        }
    }
}

/// Defines the position of the label for a switch component.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum SwitchLabelPosition {
    Start,
    #[default]
    End,
}

/// # Switch
///
/// Switches are used to represent opposite states, such as enabled or disabled.
#[derive(IntoElement, RegisterComponent)]
pub struct Switch {
    id: ElementId,
    toggle_state: ToggleState,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&ToggleState, &mut Window, &mut App) + 'static>>,
    label: Option<SharedString>,
    label_position: Option<SwitchLabelPosition>,
    label_size: LabelSize,
    label_color: Color,
    full_width: bool,
    key_binding: Option<KeyBinding>,
    color: SwitchColor,
    tab_index: Option<isize>,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
}

impl Switch {
    /// Creates a new [`Switch`].
    pub fn new(id: impl Into<ElementId>, state: ToggleState) -> Self {
        Self {
            id: id.into(),
            toggle_state: state,
            disabled: false,
            on_click: None,
            label: None,
            label_position: None,
            label_size: LabelSize::Small,
            label_color: Color::Default,
            full_width: false,
            key_binding: None,
            color: SwitchColor::default(),
            tab_index: None,
            aria_label: None,
            aria_description: None,
        }
    }

    /// Sets the color of the switch using the specified [`SwitchColor`].
    pub fn color(mut self, color: SwitchColor) -> Self {
        self.color = color;
        self
    }

    /// Sets the disabled state of the [`Switch`].
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Binds a handler to the [`Switch`] that will be called when clicked.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ToggleState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Sets the label of the [`Switch`].
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn label_position(
        mut self,
        label_position: impl Into<Option<SwitchLabelPosition>>,
    ) -> Self {
        self.label_position = label_position.into();
        self
    }

    pub fn label_size(mut self, size: LabelSize) -> Self {
        self.label_size = size;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Display the keybinding that triggers the switch action.
    pub fn key_binding(mut self, key_binding: impl Into<Option<KeyBinding>>) -> Self {
        self.key_binding = key_binding.into();
        self
    }

    pub fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.tab_index = Some(tab_index.into());
        self
    }

    /// Sets the label announced by assistive technology.
    /// Defaults to the switch's visible label, if any.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the supplementary description announced by assistive technology
    /// after the switch's name, role, and state.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_on = self.toggle_state == ToggleState::Selected;
        // 与 zed 的差异：zed 用 `utils::is_light(cx)`（它内部读 appearance）；
        // 我们的 `Appearance` 没有 `is_light()`，直接模式匹配。
        let is_light = matches!(cx.theme().appearance(), Appearance::Light);
        let adjust_ratio = if is_light { 1.5 } else { 1.0 };

        let base_color = cx.theme().colors().text;
        let thumb_color = base_color;
        let (bg_color, border_color) = self.color.get_colors(is_on, cx);

        let bg_hover_color = if is_on {
            bg_color.blend(base_color.opacity(0.16 * adjust_ratio))
        } else {
            bg_color.blend(base_color.opacity(0.05 * adjust_ratio))
        };

        let thumb_opacity = match (is_on, self.disabled) {
            (_, true) => 0.2,
            (true, false) => 1.0,
            (false, false) => 0.5,
        };

        let group_id = format!("switch_group_{:?}", self.id);
        let label = self.label;
        let aria_label = self.aria_label.or_else(|| label.clone());
        let aria_description = self.aria_description;
        let aria_keyshortcuts = self
            .key_binding
            .as_ref()
            .and_then(|key_binding| key_binding.keyboard_shortcut_text(window, cx));

        let switch = div()
            .id((self.id.clone(), "switch"))
            .role(Role::Switch)
            .when_some(aria_label, |this, label| this.aria_label(label))
            .when_some(aria_keyshortcuts, |this, keyshortcuts| {
                this.aria_keyshortcuts(keyshortcuts)
            })
            .when_some(aria_description, |this, description| {
                this.aria_description(description)
            })
            .aria_toggled(match self.toggle_state {
                ToggleState::Selected => Toggled::True,
                ToggleState::Indeterminate => Toggled::Mixed,
                ToggleState::Unselected => Toggled::False,
            })
            .p(px(1.0))
            .border_2()
            .border_color(cx.theme().colors().border_transparent)
            .rounded_full()
            .when_some(
                self.tab_index.filter(|_| !self.disabled),
                |this, tab_index| {
                    this.tab_index(tab_index).focus_visible(|mut style| {
                        style.border_color = Some(cx.theme().colors().border_focused);
                        style
                    })
                },
            )
            .when_some(self.on_click.clone().filter(|_| !self.disabled), |this, on_click| {
                this.on_click(move |_, window, cx| {
                    on_click(&self.toggle_state.inverse(), window, cx)
                })
            })
            .child(
                h_flex()
                    .w(DynamicSpacing::Base32.rems(density()))
                    .h(DynamicSpacing::Base20.rems(density()))
                    .group(group_id.clone())
                    .child(
                        h_flex()
                            .when(is_on, |on| on.justify_end())
                            .when(!is_on, |off| off.justify_start())
                            .size_full()
                            .rounded_full()
                            .px(DynamicSpacing::Base02.px(density(), window.rem_size()))
                            .bg(bg_color)
                            .when(!self.disabled, |this| {
                                this.group_hover(group_id.clone(), |el| el.bg(bg_hover_color))
                            })
                            .border_1()
                            .border_color(border_color)
                            .child(
                                div()
                                    .size(DynamicSpacing::Base12.rems(density()))
                                    .rounded_full()
                                    .bg(thumb_color)
                                    .opacity(thumb_opacity),
                            ),
                    ),
            );

        h_flex()
            .id(self.id)
            .cursor_pointer()
            .gap(DynamicSpacing::Base06.rems(density()))
            .when(self.full_width, |this| this.w_full().justify_between())
            .when(
                self.label_position == Some(SwitchLabelPosition::Start),
                |this| {
                    this.when_some(label.clone(), |this, label| {
                        this.child(Label::new(label).size(self.label_size).color(self.label_color))
                    })
                },
            )
            .child(switch)
            .when(
                self.label_position == Some(SwitchLabelPosition::End),
                |this| {
                    this.when_some(label, |this, label| {
                        this.child(Label::new(label).size(self.label_size).color(self.label_color))
                    })
                },
            )
            .when_some(self.on_click.filter(|_| !self.disabled), |this, on_click| {
                this.on_click(move |_, window, cx| {
                    on_click(&self.toggle_state.inverse(), window, cx)
                })
            })
    }
}

impl Component for Checkbox {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn description() -> &'static str {
        "Checkboxes allow the user to select one or more options from a set. \
        They are typically used in forms and settings."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Checkbox States",
                    vec![
                        single_example(
                            "Unselected",
                            Checkbox::new("unselected", ToggleState::Unselected).into_any_element(),
                        ),
                        single_example(
                            "Selected",
                            Checkbox::new("selected", ToggleState::Selected).into_any_element(),
                        ),
                        single_example(
                            "Indeterminate",
                            Checkbox::new("indeterminate", ToggleState::Indeterminate)
                                .into_any_element(),
                        ),
                        single_example(
                            "Disabled",
                            Checkbox::new("disabled", ToggleState::Selected)
                                .disabled(true)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Checkbox with Label",
                    vec![
                        single_example(
                            "With Label",
                            Checkbox::new("labeled", ToggleState::Unselected)
                                .label("Enable notifications")
                                .into_any_element(),
                        ),
                        single_example(
                            "Selected with Label",
                            Checkbox::new("labeled-selected", ToggleState::Selected)
                                .label("Enable notifications")
                                .into_any_element(),
                        ),
                    ],
                ),
            ])
            .into_any_element()
    }
}

impl Component for Switch {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn description() -> &'static str {
        "Switches are used to represent opposite states, such as enabled or disabled."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Switch States",
                    vec![
                        single_example(
                            "Off",
                            Switch::new("off", ToggleState::Unselected).into_any_element(),
                        ),
                        single_example(
                            "On",
                            Switch::new("on", ToggleState::Selected).into_any_element(),
                        ),
                        single_example(
                            "Disabled",
                            Switch::new("disabled-switch", ToggleState::Selected)
                                .disabled(true)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Switch with Label",
                    vec![
                        single_example(
                            "Label End (Default)",
                            Switch::new("label-end", ToggleState::Unselected)
                                .label("Enable feature")
                                .into_any_element(),
                        ),
                        single_example(
                            "Label Start",
                            Switch::new("label-start", ToggleState::Selected)
                                .label("Enable feature")
                                .label_position(SwitchLabelPosition::Start)
                                .into_any_element(),
                        ),
                        single_example(
                            "Full Width",
                            Switch::new("full-width", ToggleState::Unselected)
                                .label("Enable feature")
                                .full_width(true)
                                .into_any_element(),
                        ),
                    ],
                ),
            ])
            .into_any_element()
    }
}
