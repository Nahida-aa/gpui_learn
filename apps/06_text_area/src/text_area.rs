//! `TextArea` —— 多行文本框外壳：建立在 `Editor` 之上，Enter 插入换行。
//!
//! （移植自 zed crates/gpui/examples/view_example/example_text_area.rs，
//! 简化掉 `editor.cached()` 以兼容本节点的 gpui rev，直接 child(editor)。）
//!
//! 本模块平台无关：`on_mouse_down` 里「点击弹软键盘」只在 Android 上调用
//! （见 `focus_and_show_keyboard` 的 cfg 分支），桌面端点击只聚焦，不弹键盘。

use gpui::{
    App, CursorStyle, Entity, EntityId, Hsla, IntoElement, MouseButton, MouseDownEvent, View,
    Window, div, hsla, px, white,
};
use gpui::prelude::*;

use crate::editor::{Editor, Enter, standard_actions};

enum Source {
    Value(Entity<String>),
    Editor(Entity<Editor>),
}

#[derive(IntoElement)]
pub struct TextArea {
    source: Source,
    rows: usize,
    color: Option<Hsla>,
}

impl TextArea {
    pub fn new(value: Entity<String>, rows: usize) -> Self {
        Self {
            source: Source::Value(value),
            rows,
            color: None,
        }
    }

    pub fn editor(editor: Entity<Editor>, rows: usize) -> Self {
        Self {
            source: Source::Editor(editor),
            rows,
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// 点击输入框：聚焦；Android 上同时主动弹出软键盘（Android 不会自动弹）。
fn focus_and_show_keyboard(focus_handle: &gpui::FocusHandle, window: &mut Window, cx: &mut App) {
    window.focus(focus_handle, cx);
    #[cfg(target_os = "android")]
    {
        gpui_android::android::jni::show_keyboard_android(gpui_android::KeyboardType::MultiLine);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = focus_handle;
    }
}

impl View for TextArea {
    fn entity_id(&self) -> Option<EntityId> {
        Some(match &self.source {
            Source::Value(value) => value.entity_id(),
            Source::Editor(editor) => editor.entity_id(),
        })
    }

    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let editor = match self.source {
            Source::Value(value) => {
                window.use_state(cx, move |window, cx| Editor::over(value, window, cx))
            }
            Source::Editor(editor) => editor,
        };

        let focus_handle = editor.read(cx).focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let text_color = self.color.unwrap_or(hsla(0., 0., 0.1, 1.));
        let row_height = px(24.);
        let box_height = row_height * self.rows as f32 + px(16.);

        let border = if is_focused {
            hsla(220. / 360., 0.8, 0.5, 1.)
        } else {
            hsla(0., 0., 0.75, 1.)
        };

        div()
            .key_context("TextInput")
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                let editor_id = editor.entity_id();
                move |_event: &MouseDownEvent, window, cx| {
                    log::info!("[textarea] on_mouse_down focus editor id={:?}", editor_id);
                    focus_and_show_keyboard(&focus_handle, window, cx);
                }
            })
            .map(standard_actions(editor.clone()))
            // Enter 是多行框与单行输入唯一的区别：插入换行而非忽略/提交。
            .on_action({
                let editor = editor.clone();
                let editor_id = editor.entity_id();
                move |_: &Enter, _window, cx| {
                    log::info!("[textarea] Enter action -> editor id={:?}", editor_id);
                    editor.update(cx, |e, cx| e.insert_newline(cx))
                }
            })
            .w_full()
            .h(box_height)
            .p(px(8.))
            .bg(white())
            .border_1()
            .border_color(border)
            .rounded(px(4.))
            .overflow_hidden()
            .line_height(row_height)
            .text_size(px(18.))
            .text_color(text_color)
            .child(editor)
    }
}
