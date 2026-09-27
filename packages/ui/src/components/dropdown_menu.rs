//! 下拉菜单（照搬 zed `crates/ui/src/components/dropdown_menu.rs`，GPL-3.0-or-later）。
//!
//! 与 `PopoverMenu` 的关系：`DropdownMenu` 是「带按钮外观的 `PopoverMenu`」——
//! 内部就是 `PopoverMenu::new(..).trigger(Button/ButtonLike)`，外加
//! `DropdownStyle` → `ButtonStyle` 的映射与 combobox 的 a11y 语义
//! （`Role::ComboBox` + Expand/Collapse 动作）。
//!
//! ## 与 zed 的差距
//!
//! | zed | 本文件 |
//! |---|---|
//! | `#[derive(IntoElement, RegisterComponent)]` + `impl Component`（preview） | 只留 `IntoElement`：我们的 `RegisterComponent` 要求同时 `impl Component` |
//! | 其余（1-314 行） | 原样 |

use gpui::{Anchor, AnyView, Entity, Pixels, Point, Role};

use crate::{ButtonLike, ContextMenu, PopoverMenu, prelude::*};

use super::PopoverMenuHandle;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropdownStyle {
    #[default]
    Solid,
    Outlined,
    Subtle,
    Ghost,
}

enum LabelKind {
    Text(SharedString),
    Element(AnyElement),
}

#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    label: LabelKind,
    trigger_size: ButtonSize,
    trigger_tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    trigger_icon: Option<IconName>,
    style: DropdownStyle,
    menu: Entity<ContextMenu>,
    full_width: bool,
    disabled: bool,
    handle: Option<PopoverMenuHandle<ContextMenu>>,
    attach: Option<Anchor>,
    offset: Option<Point<Pixels>>,
    tab_index: Option<isize>,
    chevron: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    aria_value: Option<SharedString>,
}

impl DropdownMenu {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        menu: Entity<ContextMenu>,
    ) -> Self {
        Self {
            id: id.into(),
            label: LabelKind::Text(label.into()),
            trigger_size: ButtonSize::Default,
            trigger_tooltip: None,
            trigger_icon: Some(IconName::ChevronUpDown),
            style: DropdownStyle::default(),
            menu,
            full_width: false,
            disabled: false,
            handle: None,
            attach: None,
            offset: None,
            tab_index: None,
            chevron: true,
            aria_label: None,
            aria_description: None,
            aria_value: None,
        }
    }

    pub fn new_with_element(
        id: impl Into<ElementId>,
        label: AnyElement,
        menu: Entity<ContextMenu>,
    ) -> Self {
        Self {
            id: id.into(),
            label: LabelKind::Element(label),
            trigger_size: ButtonSize::Default,
            trigger_tooltip: None,
            trigger_icon: Some(IconName::ChevronUpDown),
            style: DropdownStyle::default(),
            menu,
            full_width: false,
            disabled: false,
            handle: None,
            attach: None,
            offset: None,
            tab_index: None,
            chevron: true,
            aria_label: None,
            aria_description: None,
            aria_value: None,
        }
    }

    pub fn style(mut self, style: DropdownStyle) -> Self {
        self.style = style;
        self
    }

    pub fn trigger_size(mut self, size: ButtonSize) -> Self {
        self.trigger_size = size;
        self
    }

    pub fn trigger_tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.trigger_tooltip = Some(Box::new(tooltip));
        self
    }

    pub fn trigger_icon(mut self, icon: IconName) -> Self {
        self.trigger_icon = Some(icon);
        self
    }

    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    pub fn handle(mut self, handle: PopoverMenuHandle<ContextMenu>) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Defines which corner of the handle to attach the menu's anchor to.
    pub fn attach(mut self, attach: Anchor) -> Self {
        self.attach = Some(attach);
        self
    }

    /// Offsets the position of the menu by that many pixels.
    pub fn offset(mut self, offset: Point<Pixels>) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn tab_index(mut self, arg: isize) -> Self {
        self.tab_index = Some(arg);
        self
    }

    pub fn no_chevron(mut self) -> Self {
        self.chevron = false;
        self
    }

    /// Sets the label announced by assistive technology.
    /// Defaults to the trigger's visible label (typically the current value).
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets the supplementary description announced by assistive technology
    /// after the combobox's name, role, and value.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Sets the current value announced by assistive technology (the selected
    /// option). Defaults to the trigger's visible text label.
    pub fn aria_value(mut self, value: impl Into<SharedString>) -> Self {
        self.aria_value = Some(value.into());
        self
    }
}

impl Disableable for DropdownMenu {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let button_style = match self.style {
            DropdownStyle::Solid => ButtonStyle::Filled,
            DropdownStyle::Subtle => ButtonStyle::Subtle,
            DropdownStyle::Outlined => ButtonStyle::Outlined,
            DropdownStyle::Ghost => ButtonStyle::Transparent,
        };

        let full_width = self.full_width;
        let trigger_size = self.trigger_size;
        // Ensure a handle exists so assistive technology can open/close the menu
        // via the Expand/Collapse accessibility actions (used by UIA on Windows
        // and AX on macOS; on Linux/AT-SPI the click action is used instead).
        let handle = self.handle.unwrap_or_default();
        let expanded = handle.is_deployed();

        // A combobox should announce its current value (the selected option).
        // Default to the trigger's visible text label when no explicit value
        // is provided.
        let aria_value = self.aria_value.clone().or_else(|| match &self.label {
            LabelKind::Text(text) => Some(text.clone()),
            LabelKind::Element(_) => None,
        });
        let aria_description = self.aria_description.clone();

        let a11y_actions = |button: Button| {
            let show_handle = handle.clone();
            let hide_handle = handle.clone();
            button
                .on_a11y_action(gpui::accesskit::Action::Expand, move |_, window, cx| {
                    show_handle.show(window, cx);
                })
                .on_a11y_action(gpui::accesskit::Action::Collapse, move |_, _window, cx| {
                    hide_handle.hide(cx);
                })
        };
        let a11y_actions_element = |button: ButtonLike| {
            let show_handle = handle.clone();
            let hide_handle = handle.clone();
            button
                .on_a11y_action(gpui::accesskit::Action::Expand, move |_, window, cx| {
                    show_handle.show(window, cx);
                })
                .on_a11y_action(gpui::accesskit::Action::Collapse, move |_, _window, cx| {
                    hide_handle.hide(cx);
                })
        };

        let (text_button, element_button) = match self.label {
            LabelKind::Text(text) => (
                Some(
                    a11y_actions(Button::new(self.id.clone(), text))
                        .aria_role(Role::ComboBox)
                        .when_some(self.aria_label, |this, label| this.aria_label(label))
                        .when_some(aria_description, |this, description| {
                            this.aria_description(description)
                        })
                        .when_some(aria_value, |this, value| this.aria_value(value))
                        .aria_expanded(expanded)
                        .style(button_style)
                        .when_some(self.trigger_icon.filter(|_| self.chevron), |this, icon| {
                            this.end_icon(
                                Icon::new(icon).size(IconSize::XSmall).color(Color::Muted),
                            )
                        })
                        .when(full_width, |this| this.full_width())
                        .size(trigger_size)
                        .disabled(self.disabled)
                        .when_some(self.tab_index, |this, tab_index| this.tab_index(tab_index)),
                ),
                None,
            ),
            LabelKind::Element(element) => (
                None,
                Some(
                    a11y_actions_element(ButtonLike::new(self.id.clone()))
                        .aria_role(Role::ComboBox)
                        .when_some(self.aria_label, |this, label| this.aria_label(label))
                        .when_some(aria_description, |this, description| {
                            this.aria_description(description)
                        })
                        .when_some(aria_value, |this, value| this.aria_value(value))
                        .aria_expanded(expanded)
                        .child(element)
                        .style(button_style)
                        .when(self.chevron, |this| {
                            this.child(
                                Icon::new(IconName::ChevronUpDown)
                                    .size(IconSize::XSmall)
                                    .color(Color::Muted),
                            )
                        })
                        .when(full_width, |this| this.full_width())
                        .size(trigger_size)
                        .disabled(self.disabled)
                        .when_some(self.tab_index, |this, tab_index| this.tab_index(tab_index)),
                ),
            ),
        };

        // When the menu opens, move the selection to the current value (or the
        // first item) so assistive technology announces a meaningful item
        // immediately, instead of focusing the bare menu container and
        // announcing only "menu". See the ARIA menu button pattern.
        let menu_for_open = self.menu.clone();
        let mut popover = PopoverMenu::new((self.id.clone(), "popover"))
            .full_width(self.full_width)
            .with_handle(handle)
            .on_open(std::rc::Rc::new(move |window, cx| {
                menu_for_open.update(cx, |menu, cx| {
                    menu.select_toggled_or_first(window, cx);
                });
            }))
            .menu(move |_window, _cx| Some(self.menu.clone()));

        popover = match (text_button, element_button, self.trigger_tooltip) {
            (Some(text_button), None, Some(tooltip)) => {
                popover.trigger_with_tooltip(text_button, tooltip)
            }
            (Some(text_button), None, None) => popover.trigger(text_button),
            (None, Some(element_button), Some(tooltip)) => {
                popover.trigger_with_tooltip(element_button, tooltip)
            }
            (None, Some(element_button), None) => popover.trigger(element_button),
            _ => popover,
        };

        popover
            .attach(match self.attach {
                Some(attach) => attach,
                None => Anchor::BottomRight,
            })
            .when_some(self.offset, |this, offset| this.offset(offset))
    }
}