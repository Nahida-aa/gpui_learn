//! # ug_05_editor —— ui-gpui 的多行编辑器演示
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
//! - 多光标:Ctrl/Cmd+Alt+↑/↓ 上下加光标,Ctrl/Cmd+D 选下一处相同文本,
//!   Alt+点击加/撤光标,Esc 收拢为单个光标;打字/退格对全部光标生效。

#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Context, IntoElement, ParentElement, Render, Window, WindowBounds, WindowOptions, actions,
    div, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use theme_settings::{GlobalAssets, init};
use tracing_subscriber;
use aa_gpui_kit_theme::LoadThemes;
use editor::{EDITOR_KEY_CONTEXT, Editor, EditorMode, bind_editor_keys};
use ui_gpui::base::input::input::{InputState, bind_input_keys};

actions!(editor_demo, [Quit]);

struct EditorDemo {
    multi: gpui::Entity<Editor>,
    single: gpui::Entity<InputState>,
}

impl EditorDemo {
    fn new(cx: &mut App) -> Self {
        let multi = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 8 }, cx)
                .placeholder("多行编辑器:输入代码或文本,Ctrl+Z 撤销…")
                .default_value("fn main() {\n    println!(\"hello editor\");\n}")
        });

        let single = cx.new(|cx| InputState::new(cx).placeholder("单行对照:Enter 提交"));
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
            .child(div().text_lg().child("ug_05_editor —— ui-gpui 多行编辑器"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6c7086))
                    .child("Tab 聚焦编辑器;Enter 换行;Ctrl/Cmd+Z 撤销;滚轮滚动")
                    .child(div().child("多光标:Ctrl/Cmd+Alt+↑↓ 加光标、Ctrl/Cmd+D 选下一处、Alt+点击加/撤、Esc 收拢")),
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
    // 调试日志: RUST_LOG=debug cargo run -p ug_05_editor 2> log.txt
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .without_time()
        .init();
    application().run(|cx: &mut App| {
        // 资产仍在 gpui 全局里,用适配器桥给主题注册表
        init(
            LoadThemes::All(Box::new(GlobalAssets(cx.asset_source().clone()))),
            cx,
        );
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
