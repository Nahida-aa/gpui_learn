//! # 11_editor —— ui-gpui 的多行编辑器演示
//!
//! 基于 `ui-gpui::base::input::editor::Editor`(MultiLine 模式)。
//!
//! 验证清单(手动过一遍):
//! - 多行输入、Enter 换行(继承行首缩进)、Backspace 跨行合并;
//! - 上下移动在短行处列 clamp;Shift+上下选区;
//! - Ctrl/Cmd+A/C/V/X(secondary);Ctrl/Cmd+Z 撤销、Shift+Z 重做;
//! - 中文 IME 组字(下划线)与上屏;
//! - 行数超出视口时滚轮滚动、光标自动跟随;
//! - 单行对比:窗口上半部分是 SingleLine 输入框,Enter 触发 Submitted。

#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    div, prelude::*, px, rgb, App, Context, IntoElement, ParentElement, Render, Window,
    WindowBounds, WindowOptions, actions, size,
};
use gpui_platform::application;
use ui_gpui::base::input::editor::{bind_editor_keys, Editor, EditorMode, EDITOR_KEY_CONTEXT};
use ui_gpui::base::input::input_state::{bind_input_keys, InputState};

actions!(
    editor_demo,
    [Quit]
);

struct EditorDemo {
    multi: gpui::Entity<Editor>,
    single: gpui::Entity<InputState>,
}

impl EditorDemo {
    fn new(cx: &mut App) -> Self {
        let multi = cx.new(|cx| {
            Editor::new(
                EditorMode::MultiLine { rows: 8 },
                cx,
            )
            .placeholder("多行编辑器:输入代码或文本,Ctrl+Z 撤销…")
            .default_value("fn main() {\n    println!(\"hello editor\");\n}")
        });

        let single = cx.new(|cx| {
            InputState::new(cx).placeholder("单行对照:Enter 提交")
        });
        Self { multi, single }
    }
}

impl Render for EditorDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(EDITOR_KEY_CONTEXT)
            .track_focus(&cx.focus_handle().clone())
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x11111b))
            .text_color(rgb(0xcdd6f4))
            .p_4()
            .child(
                div()
                    .text_lg()
                    .child("11_editor —— ui-gpui 多行编辑器"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6c7086))
                    .child("Tab 聚焦编辑器;Enter 换行;Ctrl/Cmd+Z 撤销;滚轮滚动"),
            )
            .child(div().w_full().child(self.single.clone()))
            .child(div().w_full().child(self.multi.clone()))
            .on_action(cx.listener(|this, _: &Quit, _window, cx| {
                cx.quit();
                let _ = this;
            }))
    }
}

fn run_demo() {
    application().run(|cx: &mut App| {
        bind_input_keys(cx);
        bind_editor_keys(cx);
        cx.bind_keys([gpui::KeyBinding::new("cmd-q", Quit, None)]);

        let bounds = gpui::Bounds::centered(None, size(px(640.), px(480.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let focus = cx.focus_handle().clone();
                window.focus(&focus, cx);
                cx.new(|cx| EditorDemo::new(cx))
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_demo();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_demo();
}
