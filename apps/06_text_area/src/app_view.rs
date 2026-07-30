//! 顶层视图 `MultilineExample`：在窗口里放两个多行 `TextArea` 和一个 IME 诊断框。
//!
//! 平台无关：Android 入口会在获得焦点时弹软键盘，桌面入口用标准窗口即可。

use gpui::{App, Context, Entity, FocusHandle, Focusable, IntoElement, Render, Window, div, hsla, px, rgb, white};
use gpui::prelude::*;

use crate::editor::Editor;
use crate::text_area::TextArea;

pub struct MultilineExample {
    bio: Entity<String>,
    debug_log: Entity<String>,
    focus_handle: FocusHandle,
    /// 两个编辑器实体。在首次 render 经 `use_state` 创建后缓存到这里，
    /// 供 Android 的选区工具条 `SelectionHandler` 定位「当前聚焦的编辑器」。
    bio_editor: Option<Entity<Editor>>,
    notes_editor: Option<Entity<Editor>>,
}

impl Focusable for MultilineExample {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl MultilineExample {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let bio = cx.new(|_| String::new());
        let debug_log = cx.new(|_| String::new());
        Self {
            bio,
            debug_log,
            focus_handle: cx.focus_handle(),
            bio_editor: None,
            notes_editor: None,
        }
    }

    /// 两个编辑器实体（首次 render 后才有值），供 Android 选区工具条定位聚焦的编辑器。
    pub fn selection_editors(&self) -> [Option<Entity<Editor>>; 2] {
        [self.bio_editor.clone(), self.notes_editor.clone()]
    }
}

impl Render for MultilineExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 两个框都走 `Source::Editor`（TextArea::editor），各自用 Editor::over_with_log
        // 创建 editor 并挂 debug_log。这样 box1 不再走 TextArea::new 的 Source::Value
        // 路径（该路径在 render 里用 use_state 懒建 editor，会导致焦点/输入路由错乱、
        // 点击后锁死）。两个框完全对称，互不复用状态。
        let bio_editor = window.use_state(cx, {
            let bio = self.bio.clone();
            let log = self.debug_log.clone();
            move |window, cx| Editor::over_with_log(bio, Some(log), window, cx)
        });
        let notes_value = cx.new(|_| "multi\nline\nsample".to_string());
        let notes = window.use_state(cx, {
            let notes_value = notes_value.clone();
            let log = self.debug_log.clone();
            move |window, cx| Editor::over_with_log(notes_value, Some(log), window, cx)
        });
        // 缓存编辑器实体，供 Android 选区工具条使用（只在首次 render 写入）。
        if self.bio_editor.is_none() {
            self.bio_editor = Some(bio_editor.clone());
        }
        if self.notes_editor.is_none() {
            self.notes_editor = Some(notes.clone());
        }
        div()
            .bg(rgb(0xf0f0f0))
            .track_focus(&self.focus_handle(cx))
            .on_mouse_down(gpui::MouseButton::Left, |event: &gpui::MouseDownEvent, _window, _cx| {
                log::info!("[appview] on_mouse_down pos={:?} click={}", event.position, event.click_count);
            })
            .flex()
            .flex_col()
            .size_full()
            .p(px(24.))
            .gap(px(24.))
            // 诊断框放最上面，避免被软键盘遮住（NativeActivity 的 adjustResize 对
            // SurfaceView 不生效，底部内容会被键盘盖住）。
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(hsla(0., 0., 0.45, 1.))
                    .child("输入/回车记录（回车显示为 ⏎；普通字符经 IME 提交）："),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .bg(white())
                    .border_1()
                    .border_color(hsla(0., 0., 0.7, 1.))
                    .p(px(6.))
                    .min_h(px(40.))
                    .child(self.debug_log.read(cx).clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(hsla(0., 0., 0.3, 1.))
                    .child("Text area — from a String (multi-line, Enter = newline):"),
            )
            .child(TextArea::editor(bio_editor.clone(), 4))
            .child(
                div()
                    .text_sm()
                    .text_color(hsla(0., 0., 0.3, 1.))
                    .child("Text area — from an Editor (seeded with multi-line text):"),
            )
            .child(TextArea::editor(notes.clone(), 4).color(hsla(250. / 360., 0.7, 0.4, 1.)))
    }
}
