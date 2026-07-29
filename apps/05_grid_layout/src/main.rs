//! # 05_grid_layout —— GPUI 响应式网格布局（Holy Grail）
//!
//! 移植自 zed 官方例子 `crates/gpui/examples/grid_layout.rs`，演示两个核心布局能力：
//!
//! 1. **CSS Grid 风格网格**：`div().grid()` + `grid_cols()` / `grid_rows()` +
//!    `col_span()` / `row_span()` / `col_span_full()` 等，用「网格线」而非 flex
//!    来摆放区域。这是实现经典「圣杯布局」（Header / 侧栏 / 内容 / 广告 / Footer）
//!    最自然的方式。
//! 2. **`container_query` 响应式**：根据容器**实测宽度**切换布局——窗口太窄时
//!    从三栏网格塌缩成单列堆叠。和「媒体查询看视口宽度」不同，`container_query`
//!    看的是元素自身被分到的尺寸，更贴合组件化思维。
//!
//! 这是桌面端例子（对应 `03` 是 Android 端）。之后会在它基础上加文本框做
//! 「桌面 input 例子」，再之后才把 input 移植到 `gpui-android` 后端（移动端 input
//! 应该排在桌面 input 之后学，道理见 `docs/mobile-backends.md`）。
//!
//! 运行：`cargo run -p grid_layout_05`（或 `npm run dev`）。拖动改变窗口大小，
//! 观察 < 400px 时布局塌缩成单列。

use gpui::{
    container_query, div, prelude::*, px, rgb, size, App, Bounds, Context, Hsla, Window,
    WindowBounds, WindowOptions,
};
use gpui_platform::application;

// https://en.wikipedia.org/wiki/Holy_grail_(web_design)
//
// 拖动窗口改变大小：`container_query` 会按容器实测宽度选择布局——
// 太窄放不下三栏网格时，塌缩成单列堆叠。
struct HolyGrailExample {}

impl Render for HolyGrailExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // container_query 把「当前容器被分到的尺寸」交给闭包，我们据此决定布局。
        // 注意：闭包里拿到的 container_size 是 GPUI 在布局阶段实测出来的，不是
        // 我们硬编码的——所以窗口缩放时布局会实时重排。
        container_query(|container_size, _window, _cx| {
            // 一个复用的「色块」构造器：占满、底色、虚线边框、圆角、内容居中。
            let block = |color: Hsla| {
                div()
                    .size_full()
                    .bg(color)
                    .border_1()
                    .border_dashed()
                    .rounded_md()
                    .border_color(gpui::white())
                    .items_center()
            };

            let header = block(gpui::white()).child(format!("Header — {}", container_size.width));
            let table_of_contents = block(gpui::red()).child("Table of contents");
            let content = block(gpui::green()).child("Content");
            let ad = block(gpui::blue()).child("AD :(").text_color(gpui::white());
            let footer = block(gpui::black())
                .text_color(gpui::white())
                .child("Footer");

            // 容器：间距 1、深灰底、大阴影、占满。
            let container = div().gap_1().bg(rgb(0x505050)).shadow_lg().size_full();

            // 关键分支：窗口（容器）宽度 < 400px 时塌缩为单列。
            if container_size.width < px(400.) {
                // —— 窄屏：单列 flex 堆叠 ——
                // h_12 / h_20 是固定高（flex_none 不参与拉伸），
                // content 用 flex_1 吃掉中间剩余空间。
                container
                    .flex()
                    .flex_col()
                    .child(header.h_12().flex_none())
                    .child(table_of_contents.h_20().flex_none())
                    .child(content.flex_1())
                    .child(ad.h_20().flex_none())
                    .child(footer.h_12().flex_none())
            } else {
                // —— 宽屏：5×5 CSS 网格 ——
                // grid_cols(5) / grid_rows(5) 定义 5 条列线、5 条行线。
                // col_span / row_span 让某个格子跨多列/多行；
                // col_span_full / row_span_full 表示横跨整行/整列。
                container
                    .grid()
                    .grid_cols(5)
                    .grid_rows(5)
                    .child(header.row_span(1).col_span_full())
                    .child(table_of_contents.col_span(1).h_56())
                    .child(content.col_span(3).row_span(3))
                    .child(ad.col_span(1).row_span(3))
                    .child(footer.row_span(1).col_span_full())
            }
        })
    }
}

fn run_example() {
    // 桌面端用 application().run(...)（阻塞式，App 随栈帧存活）。
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| HolyGrailExample {}),
        )
        .unwrap();
        cx.activate(true);
    });
}

fn main() {
    run_example();
}
