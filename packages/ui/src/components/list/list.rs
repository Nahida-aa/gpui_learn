//! List：列表容器（对齐 zed `crates/ui/src/components/list/list.rs`）。
//!
//! `v_flex` 打底，空列表时显示 `empty_message`。
//!
//! 出处：zed `crates/ui/src/components/list/list.rs`（GPL-3.0-or-later）。
//! 差异：zed 还有 `header` 字段收一个 `ListHeader`（我们没搬 `ListHeader`，
//! 见 `list/mod.rs` 的清单），故这里没有 header 槽位。

use crate::prelude::*;
use gpui::AnyElement;
use smallvec::SmallVec;

/// 空列表时显示什么。
pub enum EmptyMessage {
    /// 一段文字（默认 "No items"）。
    Text(SharedString),
    /// 调用方自绘的元素。
    Element(AnyElement),
}

/// A container component for displaying a collection of list items.
#[derive(IntoElement)]
pub struct List {
    /// Message to display when the list is empty.
    /// Defaults to "No items"
    empty_message: EmptyMessage,
    /// Some(false) 时连 `empty_message` 也不显示（zed `List::toggle`）。
    toggle: Option<bool>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self {
        Self {
            empty_message: EmptyMessage::Text("No items".into()),
            toggle: None,
            children: SmallVec::new(),
        }
    }

    pub fn empty_message(mut self, message: impl Into<EmptyMessage>) -> Self {
        self.empty_message = message.into();
        self
    }

    pub fn toggle(mut self, toggle: impl Into<Option<bool>>) -> Self {
        self.toggle = toggle.into();
        self
    }
}

impl ParentElement for List {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl From<String> for EmptyMessage {
    fn from(s: String) -> Self {
        EmptyMessage::Text(SharedString::from(s))
    }
}

impl From<&str> for EmptyMessage {
    fn from(s: &str) -> Self {
        EmptyMessage::Text(SharedString::from(s.to_owned()))
    }
}

impl From<AnyElement> for EmptyMessage {
    fn from(element: AnyElement) -> Self {
        EmptyMessage::Element(element)
    }
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .py(DynamicSpacing::Base04.rems(cx))
            .map(|this| match (self.children.is_empty(), self.toggle) {
                (false, _) => this.children(self.children),
                (true, Some(false)) => this,
                (true, _) => match self.empty_message {
                    // zed 这里给文字版加了 `px_2()`，元素版不加——原样保留。
                    EmptyMessage::Text(text) => {
                        this.px_2().child(Label::new(text).color(Color::Muted))
                    }
                    EmptyMessage::Element(element) => this.child(element),
                },
            })
    }
}
