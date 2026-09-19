//! # _17_text_wrapper —— 文本换行、截断与省略（移植自官方 `text_wrapper.rs`）
//!
//! 官方线编号 `_17`，对应 zed `crates/gpui/examples/text_wrapper.rs`。
//!
//! `_16_text_layout` 管「一行内怎么摆」，本例管「一段文字放不下时怎么办」。
//! 演示文本刻意混排了英文 / 中文 / 日文 / 长单词 / URL——它们的换行点规则
//! 各不相同（英文按空格、CJK 按字符、URL 尽量不拆），正是软换行引擎的难点。
//!
//! ## 演示的六种模式
//!
//! | 模式 | 写法 | 效果 |
//! |---|---|---|
//! | 默认换行 | 什么都不加 | 按词边界折行，超出容器即换行 |
//! | 单行省略 | `truncate()` | 一行 + 末尾 `…`（等价 `text_ellipsis + nowrap`） |
//! | N 行省略 | `text_ellipsis() + line_clamp(n)` | 最多 n 行，第 n 行末尾 `…` |
//! | N 行硬截断 | `text_overflow(Truncate("")) + line_clamp(n)` | 最多 n 行，直接切断无省略号 |
//! | 不换行裁剪 | `whitespace_nowrap() + overflow_hidden()` | 单行，溢出部分直接裁掉 |
//! | flex 内省略 | flex 子项上加 `text_ellipsis()` | 宽度由 flex 分配时的省略 |
//!
//! ## 为什么这个例子对编辑器重要
//!
//! - **默认换行**就是 editor 软换行想要的效果——editor 的 display_map
//!   （见 `packages/editor/src/editor/display_map.rs`）要复刻的
//!   正是这套换行点规则，只是编辑器还需要把换行点记下来做坐标映射。
//! - **`truncate` / `line_clamp`** 是文件名列表、诊断消息、补全文档的常用手段。
//! - 演示文本里的 CJK 段落可以观察：中文/日文没有空格，换行点按字符走，
//!   这部分逻辑由排版器（底层是 cosmic-text 的 line breaking）负责。
//!
//! ## 与官方原版的差异
//!
//! 与 `_16` 相同：不引入 `example_support`（用平台默认字体）；
//! 桌面 / WASM 双入口对齐 `_01_hello_world` 风格。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p _17_text_wrapper
//! ```

// WASM 目标下禁用 Rust 默认 main（启动交给 wasm-bindgen）。
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Bounds, Context, TextOverflow, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    size,
};
// application() 是 GPUI 的平台入口；web_init() 用于 WASM 初始化。
use gpui_platform::application;

/// 本例无交互状态，空 struct 即可。
struct TextWrapper {}

impl Render for TextWrapper {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 演示文本：英文空格分词 / 中文日文无空格 / 超长单词 / 带参数的 URL。
        // 官方原样保留，便于对照官方截图。
        let text = "The longest word 你好世界这段是中文，こんにちはこの段落は日本語です in any of the major \
            English language dictionaries is pneumonoultramicroscopicsilicovolcanoconiosis, a word that \
            refers to a lung disease contracted from the inhalation of very fine silica particles, \
            a url https://github.com/zed-industries/zed/pull/35724?query=foo&bar=2, \
            specifically from a volcano; medically, it is the same as silicosis.";

        div()
            // id 用于 GPUI 内部元素标识（有交互状态的元素建议都给）
            .id("page")
            .size_full()
            .flex()
            .flex_col()
            .p_2()
            .gap_2()
            .bg(gpui::white())
            // ---- 1. flex 行内的省略：三个子元素宽度由 flex 分配 ----
            .child(
                div()
                    .flex()
                    .flex_row()
                    // 不被压缩（flex-shrink: 0），容器窄了就整体溢出
                    .flex_shrink_0()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .border_1()
                            .border_color(gpui::red())
                            // 单行 + 末尾省略号；flex 子项宽度不足时生效
                            .text_ellipsis()
                            .child("longer text in flex 1"),
                    )
                    .child(
                        div()
                            .flex()
                            .border_1()
                            .border_color(gpui::red())
                            .text_ellipsis()
                            .child("short flex"),
                    )
                    .child(
                        div()
                            .overflow_hidden()
                            .border_1()
                            .border_color(gpui::red())
                            .text_ellipsis()
                            .w_full()
                            .child("A short text in normal div"),
                    ),
            )
            // ---- 2. truncate：单行 + 末尾省略号（最常用的截断手法）----
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .truncate()
                    .border_1()
                    .border_color(gpui::blue())
                    .child("ELLIPSIS: ".to_owned() + text),
            )
            // ---- 3. 两行 + 省略号 ----
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .overflow_hidden()
                    .text_ellipsis()
                    // 最多显示 2 行，第 2 行末尾加 …
                    .line_clamp(2)
                    .border_1()
                    .border_color(gpui::blue())
                    .child("ELLIPSIS 2 lines: ".to_owned() + text),
            )
            // ---- 4. 三行硬截断（Truncate("") = 无省略号）----
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .overflow_hidden()
                    .text_overflow(TextOverflow::Truncate("".into()))
                    .line_clamp(3)
                    .border_1()
                    .border_color(gpui::green())
                    .child("TRUNCATE 3 lines: ".to_owned() + text),
            )
            // ---- 5. 不换行 + 裁剪：溢出部分直接消失 ----
            .child(
                div()
                    .flex_shrink_0()
                    .text_xl()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .border_1()
                    .border_color(gpui::black())
                    .child("NOWRAP: ".to_owned() + text),
            )
            // ---- 6. 默认换行：什么都不加，按词边界折满整行 ----
            // 这就是 editor 软换行想要的效果（editor 还需要记下换行点做坐标映射）
            .child(div().text_xl().w_full().child(text))
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
        |_, cx| cx.new(|_| TextWrapper {}),
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
