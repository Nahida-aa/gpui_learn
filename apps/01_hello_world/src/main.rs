//! # 01_hello_world —— 第一个 GPUI 例子（纯 GPUI，含 WASM/HTML 入口）
//!
//! 这是 gpui_learn 里**最简单、最真实**的例子：直接 `use gpui::*`，
//! 不依赖任何内部共享库。目的是让学习者先看到 GPUI 最原始的 API 长什么样，
//! 而不是一上来就被一层封装挡住。
//!
//! 与后续「只跑桌面」的例子不同，这个例子**从一开始**就考虑了编译成
//! HTML/WASM 在浏览器里运行（包括手机浏览器）——这是 GPUI 的一大亮点。
//! 桌面和 WASM 两套入口都写在这里，用 `#[cfg(target_family = "wasm")]` 区分。
//!
//! 运行（桌面）：
//!
//! ```bash
//! cargo run -p hello_world_01
//! ```
//!
//! 编译成 Web（浏览器 / 移动端浏览器），后续会专门演示，要点是：
//!
//! ```bash
//! cargo build --target wasm32-unknown-unknown -p hello_world_01
//! ```
//! 然后用 `wasm-bindgen` 等工具产出 HTML。具体见仓库后续 Web 专题。

// 如果在 WASM 目标下编译，禁用 Rust 默认的 main 入口（由 JS/wasm-bindgen 接管启动）。
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    rgb, size,
};
// application() 是启动 GPUI 程序的平台入口；web_init() 是 WASM 下的初始化。
use gpui_platform::application;

/// 根 View：持有一段标题文字。
///
/// 在 GPUI 里，「View」是 UI 的状态容器。它实现 [`Render`] 来声明自己长什么样。
struct HelloWorld {
    text: SharedString,
}

// Render trait 是 GPUI 的核心：描述「这个 View 长什么样」。
// 每次状态变化，GPUI 会重新调用 render 得到新的元素树（一段声明式描述）。
// render 的第二参数是 &mut Context<Self>，用来访问/修改本 View 的状态与订阅事件。
impl Render for HelloWorld {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}!", &self.text))
            .child(
                // 一行彩色方块，演示 div 的组合与背景色。
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::red())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::green())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::blue())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::yellow())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::black())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .rounded_md()
                            .border_color(gpui::white()),
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(gpui::white())
                            .border_1()
                            .border_dashed()
                            .rounded_md()
                            .border_color(gpui::black()),
                    ),
            )
    }
}

/// 程序主体：开一个居中、500x500 的窗口，里面放根 View。
///
/// 桌面和 WASM 的区别在于「`run` 还是 `run_embedded`」：
/// - 桌面：`application().run(...)` 是阻塞调用，App 随调用栈一直存活。
/// - WASM：`run` 内部是 `spawn_local(async { ... })`，async 块结束即把持有的
///   `Application`(Rc) drop 掉 → 窗口/canvas 被销毁、报 `app was released`、白屏。
///   所以 WASM 下改用 `run_embedded`（返回 `ApplicationHandle`，把 App 钉住），
///   并 `mem::forget` 掉 handle，run 不阻塞也不会释放 App。
#[cfg(not(target_family = "wasm"))]
fn run_example() {
    application().run(|cx: &mut App| {
        open_hello_window(cx);
    });
}

#[cfg(target_family = "wasm")]
fn run_example() {
    let _app = application().run_embedded(|cx: &mut App| {
        open_hello_window(cx);
    });
    // 钉住 App，防止 handle 被 drop 后 App 释放（白屏的根因）。
    std::mem::forget(_app);
}

/// 开窗口这一段桌面/WASM 完全一致，抽出来避免重复。
fn open_hello_window(cx: &mut App) {
    // Bounds::centered 计算一个居中、500x500 的窗口矩形。
    let bounds = Bounds::centered(None, size(px(500.0), px(500.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        // 开窗口的闭包第二参是 &mut App，这里用它 new 出根 View。
        |_, cx| {
            cx.new(|_| HelloWorld {
                text: "World".into(),
            })
        },
    )
    .unwrap();
    // 激活窗口，使其获得焦点。
    cx.activate(true);
}

// 桌面入口：标准 Rust 二进制从 main 启动。
#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

// WASM 入口：编译到浏览器时，由 wasm-bindgen 把 start 作为导出函数，
// JS 加载后调用它来启动 GPUI。先 web_init() 初始化 Web 平台后端。
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
