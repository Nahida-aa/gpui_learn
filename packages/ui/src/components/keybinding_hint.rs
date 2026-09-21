//! 按键提示（对齐 zed `crates/ui/src/components/keybinding_hint.rs`）。
//!
//! 渲染成 `前缀 [⌘S] 后缀` 那种行内提示，比裸 [`KeyBinding`](crate::KeyBinding)
//! 多一层「等等，这里可以按…」的语境。常用于空状态页、命令面板底部。

use gpui::{AnyElement, App, BoxShadow, Hsla, IntoElement, Window};

use crate::prelude::*;
use crate::{KeyBinding, TextSize, rems_from_px};
use aa_gpui_kit_theme::Appearance;

/// Represents a hint for a keybinding, optionally with a prefix and suffix.
///
/// This struct allows for the creation and customization of a keybinding hint,
/// which can be used to display keyboard shortcuts or commands in a user interface.
#[derive(Debug, IntoElement, RegisterComponent)]
pub struct KeybindingHint {
    prefix: Option<SharedString>,
    suffix: Option<SharedString>,
    keybinding: KeyBinding,
    size: Option<Pixels>,
    background_color: Hsla,
}

impl KeybindingHint {
    /// Creates a new `KeybindingHint` with the specified keybinding.
    ///
    /// `background_color` 是提示所在容器的背景色——边框与底色要在它上面
    /// 混合出来，所以必须由调用方告诉我们。
    pub fn new(keybinding: KeyBinding, background_color: Hsla) -> Self {
        Self {
            prefix: None,
            suffix: None,
            keybinding,
            size: None,
            background_color,
        }
    }

    /// Creates a new `KeybindingHint` with a prefix and keybinding.
    pub fn with_prefix(
        prefix: impl Into<SharedString>,
        keybinding: KeyBinding,
        background_color: Hsla,
    ) -> Self {
        Self {
            prefix: Some(prefix.into()),
            suffix: None,
            keybinding,
            size: None,
            background_color,
        }
    }

    /// Creates a new `KeybindingHint` with a keybinding and suffix.
    pub fn with_suffix(
        keybinding: KeyBinding,
        suffix: impl Into<SharedString>,
        background_color: Hsla,
    ) -> Self {
        Self {
            prefix: None,
            suffix: Some(suffix.into()),
            keybinding,
            size: None,
            background_color,
        }
    }

    /// Sets the prefix for the keybinding hint.
    pub fn prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the suffix for the keybinding hint.
    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Sets the size of the keybinding hint.
    pub fn size(mut self, size: impl Into<Option<Pixels>>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for KeybindingHint {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let is_light = cx.theme().appearance() == Appearance::Light;

        let border_color = self
            .background_color
            .blend(colors.text.alpha(if is_light { 0.08 } else { 0.16 }));

        let bg_color = self
            .background_color
            .blend(colors.text_accent.alpha(if is_light { 0.05 } else { 0.1 }));

        let shadow_color = colors.text.alpha(if is_light { 0.04 } else { 0.08 });

        // 与 zed 的差异：zed 写 `TextSize::Small.rems(cx).to_pixels(window.rem_size())`；
        // 我们的 `TextSize` 直接提供 `pixels(cx)`，少一次换算。
        let size = self.size.unwrap_or(TextSize::Small.pixels(cx));

        let kb_size = size - px(2.0);

        // 与 zed 的差异：zed 需要 `let mut base` 是因为它随后直接改
        // `base.text_style().font_style`；我们改用链式的 `.italic()`，
        // 不必可变绑定。
        h_flex()
            .gap_1()
            .italic()
            .text_size(size)
            .text_color(colors.text_disabled)
            .children(self.prefix)
            .child(
                h_flex()
                    .rounded_sm()
                    .px_0p5()
                    .mr_0p5()
                    .border_1()
                    .border_color(border_color)
                    .bg(bg_color)
                    .shadow(vec![BoxShadow::new(px(0.), px(1.), shadow_color)])
                    .child(self.keybinding.size(rems_from_px(kb_size))),
            )
            .children(self.suffix)
    }
}

impl Component for KeybindingHint {
    fn scope() -> ComponentScope {
        ComponentScope::DataDisplay
    }

    fn description() -> &'static str {
        "Displays a keyboard shortcut hint with optional prefix and suffix text"
    }

    fn preview(_window: &mut Window, cx: &mut App) -> AnyElement {
        let bg_color = cx.theme().colors().surface_background;

        // 与 zed 的差异：zed 的预览用 `KeyBinding::for_action(&menu::Confirm, cx)`
        // （它依赖 menu crate 的动作类型）。我们没有那套 action 定义，
        // 改成从 keystroke 构造，效果一致。
        let enter = || {
            use gpui::{KeybindingKeystroke, Keystroke};

            let keystrokes: std::rc::Rc<[KeybindingKeystroke]> =
                vec![KeybindingKeystroke::from_keystroke(
                    Keystroke::parse("enter").expect("enter 是合法 keystroke"),
                )]
                .into();
            KeyBinding::from_keystrokes(keystrokes, false)
        };

        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Basic",
                    vec![
                        single_example(
                            "With Prefix",
                            KeybindingHint::with_prefix("Go to Start:", enter(), bg_color)
                                .into_any_element(),
                        ),
                        single_example(
                            "With Suffix",
                            KeybindingHint::with_suffix(enter(), "Go to End", bg_color)
                                .into_any_element(),
                        ),
                        single_example(
                            "With Prefix and Suffix",
                            KeybindingHint::new(enter(), bg_color)
                                .prefix("Confirm:")
                                .suffix("Execute selected action")
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Sizes",
                    vec![
                        single_example(
                            "Small",
                            KeybindingHint::new(enter(), bg_color)
                                .size(Pixels::from(12.0))
                                .prefix("Small:")
                                .into_any_element(),
                        ),
                        single_example(
                            "Medium",
                            KeybindingHint::new(enter(), bg_color)
                                .size(Pixels::from(16.0))
                                .suffix("Medium")
                                .into_any_element(),
                        ),
                        single_example(
                            "Large",
                            KeybindingHint::new(enter(), bg_color)
                                .size(Pixels::from(20.0))
                                .prefix("Large:")
                                .suffix("Size")
                                .into_any_element(),
                        ),
                    ],
                ),
            ])
            .into_any_element()
    }
}
