//! button：基础按钮。
//!
//! 与 slider / input 不同，按钮**没有**持久状态（按下态由 gpui 每帧重绘时
//! 根据 hover 推导即可），所以这里不做 `Entity<ButtonState>`，而是一个
//! 一次性元素：`IntoElement` 直接产出内部的 `Div`。
//!
//! 用法：
//! ```ignore
//! Button::new("download")
//!     .label("下载")
//!     .primary()
//!     .on_click(|_event, _window, cx| { /* ... */ })
//! ```
//! `disabled(true)` 时不挂点击回调，视觉置灰且不响应 hover。

use gpui::{
    App, ClickEvent, Div, ElementId, IntoElement, Rgba, SharedString, Stateful, Window, div,
    prelude::*, px, rgb,
};

/// 按钮视觉变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// 中性按钮（默认）。
    #[default]
    Default,
    /// 主行动按钮，蓝色实心。
    Primary,
    /// 危险操作，红色实心。
    Danger,
}

impl ButtonVariant {
    /// (常态背景, hover 背景, 文字色)
    fn colors(self) -> (Rgba, Rgba, Rgba) {
        match self {
            ButtonVariant::Default => (rgb(0x313244), rgb(0x45475a), rgb(0xcdd6f4)),
            ButtonVariant::Primary => (rgb(0x1e66f5), rgb(0x3b7dfa), rgb(0xffffff)),
            ButtonVariant::Danger => (rgb(0xd20f39), rgb(0xe64553), rgb(0xffffff)),
        }
    }
}

/// 点击回调。
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// 一次性按钮元素。
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// 用给定的 element id 新建按钮（id 在同一父容器内需唯一）。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::default(),
            variant: ButtonVariant::Default,
            disabled: false,
            on_click: None,
        }
    }

    /// 按钮文字。
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// 主行动样式。
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// 危险操作样式。
    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
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
}

impl IntoElement for Button {
    // `.on_click()` 会把 `Div` 提升成 `Stateful<Div>`，所以元素类型按提升后的写。
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let disabled = self.disabled;
        let handler = self.on_click;
        let (bg, hover_bg, text_color) = self.variant.colors();
        // 禁用：置灰、无 hover。on_click 仍然挂上（保持元素类型一致），
        // 由回调内部直接忽略，避免 `Div` / `Stateful<Div>` 类型分叉。
        let (bg, text_color, hover_bg) = if disabled {
            (rgb(0x45475a), rgb(0x6c7086), None)
        } else {
            (bg, text_color, Some(hover_bg))
        };

        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .px_4()
            .py_1()
            .rounded_md()
            .text_size(px(14.))
            .bg(bg)
            .text_color(text_color)
            .child(self.label)
            .on_click(move |event, window, cx| {
                if disabled {
                    return;
                }
                if let Some(handler) = handler.as_ref() {
                    handler(event, window, cx);
                }
            });

        if let Some(hover_bg) = hover_bg {
            button = button.hover(|style| style.bg(hover_bg)).cursor_pointer();
        }

        button
    }
}
