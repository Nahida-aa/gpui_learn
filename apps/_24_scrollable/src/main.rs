//! # _24_scrollable —— 可滚动内容（移植自官方 `scrollable.rs`）
//!
//! 官方线编号 `_24`，对应 zed `crates/gpui/examples/scrollable.rs`。
//!
//! GPUI 的滚动是**纯声明式**的：给一个 stateful 的 `div` 加 `overflow_scroll()`
//! （类似 CSS `overflow: auto`），内容超出容器即自动可滚——不需要手写
//! 滚轮事件处理，gpui 内部接好了一切（滚轮、拖拽、惯性）。
//!
//! ## 三个关键点
//!
//! 1. **必须有 `.id(...)`**：`overflow_scroll` 只对 stateful 元素生效
//!    （滚动状态要存在元素状态里），无 id 的 div 加了也会被忽略。
//! 2. **内容比容器大才有得滚**：官方例子用一个 `h(px(5000.))` 的子 div
//!    撑出 5000px 的滚动面；`apps/ug_05_editor` 的做法同理——内容 div
//!    高度 = 总行数 × 行高。
//! 3. **嵌套滚动**：例子里水平滚动的 div 嵌在垂直滚动的 div 内，
//!    各自独立响应滚轮（gpui 按命中位置分发）。
//!
//! ## 为什么这个例子对编辑器重要
//!
//! `ug_05_editor` 第一版的垂直滚动是手搓的（`on_scroll_wheel` 里改
//! `scroll_top` 字段）；移植本例后已换成官方做法：
//! `overflow_scroll() + track_scroll(&ScrollHandle)`，见
//! `packages/editor/src/editor/mod.rs` 的 `Render` 实现。
//!
//! ## 与官方原版的差异
//!
//! 与 `_16` 相同：不引入 `example_support`（用平台默认字体）；
//! 桌面 / WASM 双入口对齐 `_01_hello_world` 风格。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p _24_scrollable
//! ```

// WASM 目标下禁用 Rust 默认 main（启动交给 wasm-bindgen）。
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, size};
// application() 是 GPUI 的平台入口；web_init() 用于 WASM 初始化。
use gpui_platform::application;

/// 无交互状态，空 struct 即可。
struct Scrollable {}

impl Render for Scrollable {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            // stateful 元素 id：overflow_scroll 的硬性前提
            .id("vertical")
            .p_4()
            // 垂直 + 水平双向滚动（overflow: auto）
            .overflow_scroll()
            .bg(gpui::white())
            .child("Example for test 2 way scroll in nested layout")
            .child(
                div()
                    // 内容高 5000px：撑出滚动面
                    .h(px(5000.))
                    .border_1()
                    .border_color(gpui::blue())
                    .bg(gpui::blue().opacity(0.05))
                    .p_4()
                    .child(
                        div()
                            .mb_5()
                            // 水平滚动：内容宽 2000px 超出此 div
                            .w_full()
                            .id("horizontal")
                            .overflow_scroll()
                            .child(
                                div()
                                    .w(px(2000.))
                                    .h(px(150.))
                                    .bg(gpui::green().opacity(0.1))
                                    // hover 高亮：滚动内容也是普通元素，交互照常
                                    .hover(|this| this.bg(gpui::green().opacity(0.2)))
                                    .border_1()
                                    .border_color(gpui::green())
                                    .p_4()
                                    .child("Scroll Horizontal"),
                            ),
                    )
                    .child("Scroll Vertical"),
            )
    }
}

/// 开一个 500x500 的居中窗口（刻意做小，滚动手感更明显）。
fn open_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(|_| Scrollable {}),
    )
    .unwrap();
    cx.activate(true);
}

/// 桌面入口：`application().run` 是阻塞调用，App 随调用栈存活。
#[cfg(not(target_family = "wasm"))]
fn run_example() {
    application().run(|cx: &mut App| {
        open_window(cx);
    });
}

/// WASM 入口：`run_embedded` + forget 钉住 App，防止白屏。详见 `_01_hello_world`。
#[cfg(target_family = "wasm")]
fn run_example() {
    let handle = application().run_embedded(|cx: &mut App| {
        open_window(cx);
    });
    std::mem::forget(handle);
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
