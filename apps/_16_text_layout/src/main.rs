//! # _16_text_layout —— 文本对齐、装饰与字重（移植自官方 `text_layout.rs`）
//!
//! 官方线编号 `_16`，对应 zed `crates/gpui/examples/text_layout.rs`。
//!
//! 本例演示 GPUI 的文本排版三件事：
//!
//! 1. **对齐**：`text_left` / `text_center` / `text_right`（对应 CSS text-align）；
//! 2. **装饰**：`text_decoration_1`（下划线）、`line_through`（删除线）；
//! 3. **字重/字形混合**：`StyledText::with_highlights` 在同一串文字里给不同区间
//!    施加不同 `FontWeight` / `FontStyle`。
//!
//! ## 为什么这个例子对编辑器重要
//!
//! - **装饰 = IME 组字下划线**：gpui 的 `TextRun.underline` 与本例的
//!   `text_decoration_1` 是同一套底层能力；`apps/ug_05_editor` 里给组字区加
//!   下划线用的就是它。
//! - **`StyledText` 的区间高亮**正是语法高亮/选区高亮的最小模型：
//!   `(range, style)` 列表 → 一次排版。编辑器的 `runs` 就是它的批量版。
//! - **长文本 + `whitespace_nowrap` + `overflow_hidden`** 是「不换行」的处理
//!   组合；反过来（允许换行）就是软换行渲染要研究的对象，见 `_17_text_wrapper`。
//!
//! ## 与官方原版的差异
//!
//! 官方 `text_layout.rs` 顶部有
//! `example_support::load_fonts(cx)`（加载示例字体，保证各平台字形一致）。
//! 本仓库不引入 `example_support`，直接用平台默认字体——视觉上略有差异，
//! 但省掉一个内部依赖，符合本仓库「每个例子只依赖 gpui」的做法。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p _16_text_layout
//! ```

// WASM 目标下禁用 Rust 默认 main（启动交给 wasm-bindgen）。
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, FontStyle, FontWeight, StyledText, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, size,
};
// application() 是 GPUI 的平台入口；web_init() 用于 WASM 初始化。
use gpui_platform::application;

/// 本例没有交互状态，根 View 是一个空的 struct——
/// GPUI 里即使无状态也要有一个实现 [`Render`] 的实体作为窗口根。
struct TextLayout {}

impl Render for TextLayout {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // 白底，方便观察装饰线（下划线/删除线）的位置
            .bg(gpui::white())
            .flex()
            .flex_col()
            // 每行之间留 2 个单位间距
            .gap_2()
            .p_4()
            // 铺满窗口
            .size_full()
            // ---- 1. 对齐 ----
            // 默认左对齐（CSS text-align: left）
            .child(div().child("Text left"))
            // 居中（text-align: center）
            .child(div().text_center().child("Text center"))
            // 右对齐（text-align: right）
            .child(div().text_right().child("Text right"))
            // ---- 2. 对齐 × 下划线 ----
            // text_decoration_1 = 1px 下划线；对齐与装饰可叠加
            .child(div().text_decoration_1().child("Text left (underline)"))
            .child(
                div()
                    .text_center()
                    .text_decoration_1()
                    .child("Text center (underline)"),
            )
            .child(
                div()
                    .text_right()
                    .text_decoration_1()
                    .child("Text right (underline)"),
            )
            // ---- 3. 对齐 × 删除线 ----
            .child(div().line_through().child("Text left (line_through)"))
            .child(
                div()
                    .text_center()
                    .line_through()
                    .child("Text center (line_through)"),
            )
            .child(
                div()
                    .text_right()
                    .line_through()
                    .child("Text right (line_through)"),
            )
            // ---- 4. 长文本不换行 + 溢出裁剪 ----
            .child(
                div()
                    .flex()
                    .gap_2()
                    // 两个子元素分别贴左右两端（justify-content: space-between）
                    .justify_between()
                    .child(
                        div()
                            // 固定宽度 400px：容器比文字窄，用来观察溢出行为
                            .w(px(400.))
                            .border_1()
                            .border_color(gpui::blue())
                            .p_1()
                            // 关键组合：不换行 + 超出裁剪。
                            // 缺了 whitespace_nowrap 文字会自动折行；
                            // 缺了 overflow_hidden 溢出部分仍会画到容器外。
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_center()
                            .child("A long non-wrapping text align center"),
                    )
                    .child(
                        div()
                            // w_32 = 8rem ≈ 128px
                            .w_32()
                            .border_1()
                            .border_color(gpui::blue())
                            .p_1()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_right()
                            .child("100%"),
                    ),
            )
            // ---- 5. 同一串文字里混用字重/字形 ----
            // StyledText 把「区间 → 样式」的列表交给排版器，一次成型；
            // 编辑器里的语法高亮 runs 就是同一模型的批量版本。
            .child(div().flex().gap_2().justify_between().child(
                StyledText::new("ABCD").with_highlights([
                    // 第 1 个字符用超粗体
                    (0..1, FontWeight::EXTRA_BOLD.into()),
                    // 第 3 个字符用斜体
                    (2..3, FontStyle::Italic.into()),
                ]),
            ))
    }
}

/// 开一个 800x600 的居中窗口。
fn open_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(|_| TextLayout {}),
    )
    .unwrap();
    // 激活窗口，否则可能不开到前台
    cx.activate(true);
}

/// 桌面入口：`application().run` 是阻塞调用，App 随调用栈存活。
#[cfg(not(target_family = "wasm"))]
fn run_example() {
    application().run(|cx: &mut App| {
        open_window(cx);
    });
}

/// WASM 入口：必须用 `run_embedded`（返回 `ApplicationHandle`）并 forget 掉，
/// 否则 async 块结束后 App 被 drop → 窗口销毁、白屏。详见 `_01_hello_world`。
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
