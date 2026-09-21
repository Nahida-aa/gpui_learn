//! `Button`：带文字的按钮（对齐 zed `crates/ui/src/components/button/button.rs`）。
//!
//! 它是 [`ButtonLike`](super::button_like::ButtonLike) 之上的薄壳：把
//! 「图标 + 文字 + 按键提示」按固定顺序拼好，再交给 `ButtonLike` 处理样式
//! 与交互。三种按钮的分工与 zed 一致：
//!
//! - 只有图标 → [`IconButton`](super::IconButton)
//! - 图标 + 文字 → **本组件**
//! - 完全自定义 → 直接用 `ButtonLike`
//!
//! 出处：zed `crates/ui/src/components/button/button.rs`（GPL-3.0-or-later）。
//! 与 zed 的差异集中在两处，见文件内 `// 与 zed 的差异` 注释。

use gpui::{AnyElement, ElementId, SharedString};

use crate::components::button::{ButtonLike, ButtonStyle, KeybindingPosition};
use crate::components::label::LabelLike;
use crate::prelude::*;
use crate::traits::SelectableButton;
use crate::{Color, DynamicSpacing, Icon, KeyBinding, LabelSize, UiDensity};

/// An element that creates a button with a label and optional icons.
///
/// Common buttons:
/// - Label, Icon + Label: [`Button`] (this component)
/// - Icon only: [`IconButton`](super::IconButton)
/// - Custom: [`ButtonLike`]
///
/// # Examples
///
/// ```
/// use aa_gpui_kit_ui::prelude::*;
///
/// Button::new("button_id", "Click me!").on_click(|_, _, _| {
///     // Handle click event
/// });
/// ```
///
/// 可切换的按钮用 `.toggle_state(..)` + `.selected_style(..)`：
///
/// ```
/// use aa_gpui_kit_ui::prelude::*;
/// use aa_gpui_kit_ui::TintColor;
///
/// Button::new("button_id", "Click me!")
///     .start_icon(Icon::new(IconName::Check))
///     .toggle_state(true)
///     .selected_style(ButtonStyle::Tinted(TintColor::Accent))
///     .on_click(|_, _, _| {});
/// ```
#[derive(IntoElement, Documented, RegisterComponent)]
pub struct Button {
    base: ButtonLike,
    label: SharedString,
    label_color: Option<Color>,
    label_size: Option<LabelSize>,
    selected_label: Option<SharedString>,
    selected_label_color: Option<Color>,
    start_icon: Option<Icon>,
    end_icon: Option<Icon>,
    key_binding: Option<KeyBinding>,
    key_binding_position: KeybindingPosition,
    alpha: Option<f32>,
    truncate: bool,
    loading: bool,
}

impl Button {
    /// Creates a new [`Button`] with a specified identifier and label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            base: ButtonLike::new(id),
            label: label.into(),
            label_color: None,
            label_size: None,
            selected_label: None,
            selected_label_color: None,
            start_icon: None,
            end_icon: None,
            key_binding: None,
            key_binding_position: KeybindingPosition::default(),
            alpha: None,
            truncate: false,
            loading: false,
        }
    }

    /// Sets the color of the button's label.
    pub fn color(mut self, label_color: impl Into<Option<Color>>) -> Self {
        self.label_color = label_color.into();
        self
    }

    /// Sets the label announced by assistive technology.
    /// Defaults to the button's visible label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_label(label);
        self
    }

    /// Sets the supplementary description announced by assistive technology
    /// after the button's name, role, and value.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_description(description);
        self
    }

    /// Sets the current value reported to assistive technology. Use this when
    /// the button represents a control with a value, such as a combobox
    /// trigger whose value is the current selection.
    pub fn aria_value(mut self, value: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_value(value);
        self
    }

    /// Overrides the role reported to assistive technology.
    /// Defaults to [`gpui::Role::Button`].
    pub fn aria_role(mut self, role: gpui::Role) -> Self {
        self.base = self.base.aria_role(role);
        self
    }

    /// Sets the expanded state reported to assistive technology, for buttons
    /// that control a popup (e.g. dropdown or disclosure triggers).
    pub fn aria_expanded(mut self, expanded: bool) -> Self {
        self.base = self.base.aria_expanded(expanded);
        self
    }

    /// Registers a handler for an accessibility action (e.g.
    /// [`gpui::accesskit::Action::Expand`]) dispatched by assistive technology.
    pub fn on_a11y_action(
        mut self,
        action: gpui::accesskit::Action,
        listener: impl FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_a11y_action(action, listener);
        self
    }

    /// Defines the size of the button's label.
    pub fn label_size(mut self, label_size: impl Into<Option<LabelSize>>) -> Self {
        self.label_size = label_size.into();
        self
    }

    /// Sets the label used when the button is in a selected state.
    pub fn selected_label<L: Into<SharedString>>(mut self, label: impl Into<Option<L>>) -> Self {
        self.selected_label = label.into().map(Into::into);
        self
    }

    /// Sets the label color used when the button is in a selected state.
    pub fn selected_label_color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.selected_label_color = color.into();
        self
    }

    /// Sets an icon to display at the start (left) of the button label.
    ///
    /// The icon's color will be overridden to `Color::Disabled` when the button is disabled.
    pub fn start_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.start_icon = icon.into();
        self
    }

    /// Sets an icon to display at the end (right) of the button label.
    ///
    /// The icon's color will be overridden to `Color::Disabled` when the button is disabled.
    pub fn end_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.end_icon = icon.into();
        self
    }

    /// Display the keybinding that triggers the button action.
    pub fn key_binding(mut self, key_binding: impl Into<Option<KeyBinding>>) -> Self {
        self.key_binding = key_binding.into();
        self
    }

    /// Sets the position of the keybinding relative to the button label.
    pub fn key_binding_position(mut self, position: KeybindingPosition) -> Self {
        self.key_binding_position = position;
        self
    }

    /// Sets the alpha property of the color of label.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = Some(alpha);
        self
    }

    /// Truncates overflowing labels with an ellipsis (`…`) if needed.
    ///
    /// Buttons with static labels should _never_ be truncated, ensure
    /// this is only used when the label is dynamic and may overflow.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Displays a rotating loading spinner in place of the `start_icon`.
    ///
    /// When `loading` is `true`, any `start_icon` is ignored, and a rotating
    /// loading spinner is shown instead.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl Toggleable for Button {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.base = self.base.toggle_state(selected);
        self
    }
}

impl SelectableButton for Button {
    fn selected_style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.style(style);
        self
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
}

impl Clickable for Button {
    fn on_click(
        mut self,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = Clickable::on_click(self.base, handler);
        self
    }

    fn cursor_style(mut self, cursor_style: gpui::CursorStyle) -> Self {
        self.base = self.base.cursor_style(cursor_style);
        self
    }
}

impl FixedWidth for Button {
    fn width(mut self, width: impl Into<gpui::DefiniteLength>) -> Self {
        self.base = self.base.width(width);
        self
    }

    fn full_width(mut self) -> Self {
        self.base = self.base.full_width();
        self
    }
}

impl ButtonCommon for Button {
    fn id(&self) -> &ElementId {
        self.base.id_ref()
    }

    /// 设置 tab 键导航序号。
    fn tab_index(mut self, tab_index: impl Into<isize>) -> Self {
        self.base = self.base.tab_index(tab_index);
        self
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.style(style);
        self
    }

    fn tooltip(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> gpui::Entity<crate::components::Tooltip> + 'static,
    ) -> Self {
        self.base = ButtonLike::tooltip(self.base, tooltip);
        self
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_disabled = self.base.is_disabled();
        let is_selected = self.base.is_selected();

        let label = self
            .selected_label
            .filter(|_| is_selected)
            .unwrap_or(self.label);

        // 无障碍名称缺省用可见文字（与 zed 一致）。
        let mut base = self.base;

        // 与 zed 的差异 1：zed 在这里读 `self.base.aria_label` / `aria_keyshortcuts`
        // 这些私有字段直接写；我们把它们收在 ButtonLike 上，用方法设置。
        if base.aria_label_ref().is_none() {
            base = base.aria_label(label.clone());
        }

        // 把可见的快捷键串也告诉辅助技术（aria-keyshortcuts）。
        if let Some(keyshortcuts) = self
            .key_binding
            .as_ref()
            .and_then(|key_binding| key_binding.keyboard_shortcut_text(window, cx))
        {
            base = base.aria_keyshortcuts(keyshortcuts);
        }

        let label_color = if is_disabled {
            Color::Disabled
        } else if is_selected {
            self.selected_label_color.unwrap_or(Color::Selected)
        } else {
            self.label_color.unwrap_or_default()
        };

        // 与 zed 的差异 2：zed 的 `Icon` 收 `IconSize`、`.color()` 收语义 `Color`，
        // 且 loading 用 `.with_keyed_rotate_animation(...)`；我们的 `Icon` 收
        // `Pixels` / `Hsla`，也还没有 keyed 旋转动画，所以 loading 退化成
        // 静态的 spinner 图标。补动画机制后再对齐。
        let start_icon_size = IconSize::Small.rems() * window.rem_size();
        let gap = DynamicSpacing::Base04.rems(UiDensity::Default);
        let label_gap = DynamicSpacing::Base06.rems(UiDensity::Default);

        let icon_color = |color: Color| color.color(cx);

        let start: Option<AnyElement> = if self.loading {
            Some(
                Icon::new(IconName::LoadCircle)
                    .size(start_icon_size)
                    .color(icon_color(Color::Muted))
                    .into_any_element(),
            )
        } else {
            self.start_icon.map(|icon| {
                if is_disabled {
                    icon.color(icon_color(Color::Disabled))
                } else {
                    icon
                }
                .into_any_element()
            })
        };

        let end: Option<AnyElement> = self.end_icon.map(|icon| {
            if is_disabled {
                icon.color(icon_color(Color::Disabled))
            } else {
                icon
            }
            .into_any_element()
        });

        let mut label_like = LabelLike::new()
            .child(
                crate::components::label::Label::new(label)
                    .color(label_color)
                    .size(self.label_size.unwrap_or_default()),
            )
            .when_some(self.alpha, |this, alpha| LabelCommon::alpha(this, alpha));

        if self.truncate {
            label_like = LabelCommon::truncate(label_like);
        }

        let mut row = h_flex()
            .when(self.truncate, |this| this.min_w_0().overflow_hidden())
            .gap(gap);

        if let Some(start) = start {
            row = row.child(start);
        }

        let mut inner = h_flex()
            .when(self.truncate, |this| this.min_w_0().overflow_hidden())
            .when(
                self.key_binding_position == KeybindingPosition::Start,
                |this| this.flex_row_reverse(),
            )
            .gap(label_gap)
            .justify_between()
            .child(label_like);

        if let Some(key_binding) = self.key_binding {
            inner = inner.child(key_binding);
        }

        row = row.child(inner);

        if let Some(end) = end {
            row = row.child(end);
        }

        base.child(row)
    }
}

impl Component for Button {
    fn scope() -> ComponentScope {
        ComponentScope::Input
    }

    fn sort_name() -> &'static str {
        "ButtonA"
    }

    fn description() -> &'static str {
        "A button triggers an event or action."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Button Styles",
                    vec![
                        single_example(
                            "Default",
                            Button::new("default", "Default").into_any_element(),
                        ),
                        single_example(
                            "Filled",
                            Button::new("filled", "Filled")
                                .style(ButtonStyle::Filled)
                                .into_any_element(),
                        ),
                        single_example(
                            "Subtle",
                            Button::new("subtle", "Subtle")
                                .style(ButtonStyle::Subtle)
                                .into_any_element(),
                        ),
                        single_example(
                            "Outlined",
                            Button::new("outlined", "Outlined")
                                .style(ButtonStyle::Outlined)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "With Icons",
                    vec![
                        single_example(
                            "Start Icon",
                            Button::new("start-icon", "Start Icon")
                                .start_icon(Icon::new(IconName::Check))
                                .into_any_element(),
                        ),
                        single_example(
                            "End Icon",
                            Button::new("end-icon", "End Icon")
                                .end_icon(Icon::new(IconName::ChevronDown))
                                .into_any_element(),
                        ),
                        single_example(
                            "Loading",
                            Button::new("loading", "Loading")
                                .loading(true)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "States",
                    vec![
                        single_example(
                            "Disabled",
                            Button::new("disabled", "Disabled")
                                .disabled(true)
                                .into_any_element(),
                        ),
                        single_example(
                            "Toggleable",
                            Button::new("toggle", "Toggle")
                                .toggle_state(true)
                                .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                                .into_any_element(),
                        ),
                        single_example(
                            "Selected Label",
                            Button::new("selected-label", "Follow")
                                .toggle_state(true)
                                .selected_label("Following")
                                .into_any_element(),
                        ),
                    ],
                ),
            ])
            .into_any_element()
    }
}
