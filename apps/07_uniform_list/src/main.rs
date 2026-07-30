#![cfg_attr(target_family = "wasm", no_main)]

//! 07_uniform_list —— GPUI `uniform_list` 定高虚拟列表练习。
//!
//! `uniform_list` 是 GPUI 里长列表 / 滚动渲染的基础构件：所有 item 高度相同，
//! 它只渲染可视区内的 item（虚拟化），滚动再远也不会卡。官方自带示例只演示了
//! 「渲染 + 点击打印」，本例补全真实 app 必需、官方却省略的部分：
//!   - 可变数据源（item 列表存进 view 字段，可动态增删）
//!   - 点击 / 键盘选中高亮（选中项换底色）
//!   - 键盘上下导航（↑/↓）+ 回车选中，并自动滚动到可视区
//!
//! 关键 API（当前 gpui rev 82aef44）：
//!   - `uniform_list(id, item_count, processor)`：processor 拿到可视 `range`，
//!     返回该范围内的 item 元素；range 之外不渲染（虚拟化）。
//!   - `UniformListScrollHandle::scroll_to_item(ix, ScrollStrategy::Nearest)`：
//!     把第 ix 项滚到可视区（Nearest = 不到边界不滚，到边界才滚）。
//!   - `.track_scroll(&handle)`：把滚动状态接到 handle，键盘导航时用来跟随选中项。

use gpui::{
    actions, App, Bounds, Context, FocusHandle, ScrollStrategy, UniformListScrollHandle, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size, uniform_list,
};
use gpui_platform::application;
use std::ops::Range;

// 上下导航 + 回车选中：自定义 action，绑定键盘事件（↑/↓/Enter）。
actions!(uniform_list_07, [SelectNext, SelectPrev, Confirm]);

struct UniformListExample {
    /// 数据源：可变，真实 app 里可来自网络 / 文件。
    items: Vec<String>,
    /// 当前选中项下标（usize::MAX 表示无选中）。
    selected: usize,
    /// 滚动句柄：键盘导航时把选中项滚进可视区。
    scroll_handle: UniformListScrollHandle,
    /// 焦点句柄：列表需要聚焦才能接收键盘事件。
    focus_handle: FocusHandle,
}

impl UniformListExample {
    const ITEM_HEIGHT: f32 = 50.0;

    fn new(items: Vec<String>, cx: &mut Context<Self>) -> Self {
        Self {
            items,
            selected: usize::MAX,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    /// 选中第 ix 项：更新状态并滚到可视区（键盘导航与点击共用）。
    fn select(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.items.len() {
            return;
        }
        self.selected = ix;
        self.scroll_handle
            .scroll_to_item(ix, ScrollStrategy::Nearest);
        window.refresh();
        cx.notify();
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        let next = if self.selected == usize::MAX {
            0
        } else if self.selected + 1 < self.items.len() {
            self.selected + 1
        } else {
            0 // 到尾部回到开头（循环）
        };
        self.select(next, window, cx);
    }

    fn select_previous(&mut self, _: &SelectPrev, window: &mut Window, cx: &mut Context<Self>) {
        let prev = if self.selected == usize::MAX {
            0
        } else if self.selected > 0 {
            self.selected - 1
        } else {
            self.items.len() - 1 // 到头部回到末尾（循环）
        };
        self.select(prev, window, cx);
    }

    fn confirm(&mut self, _: &Confirm, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(self.selected) {
            log::info!("[uniform_list] confirmed item {}={}", self.selected, item);
        }
    }
}

impl Render for UniformListExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xffffff))
            // 让列表容器可聚焦，键盘事件才会进来。
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::confirm))
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_size(px(14.))
                    .text_color(rgb(0x666666))
                    .child(format!(
                        "uniform_list：{} 项 · 选中 {} · ↑/↓ 导航 · Enter 确认 · 点击选中",
                        self.items.len(),
                        if self.selected == usize::MAX {
                            "无".to_string()
                        } else {
                            self.selected.to_string()
                        }
                    )),
            )
            .child(
                uniform_list(
                    "entries",
                    self.items.len(),
                    cx.processor(|this, range: Range<usize>, _window, cx| {
                        let entity = cx.entity();
                        range
                            .map(|ix| {
                                let is_selected = ix == this.selected;
                                div()
                                    .id(ix)
                                    .h(px(Self::ITEM_HEIGHT))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    // 选中项高亮；hover 轻微底色（仅桌面有效）。
                                    .bg(if is_selected {
                                        rgb(0x2563eb)
                                    } else {
                                        rgb(0xffffff)
                                    })
                                    .text_color(if is_selected {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0x111111)
                                    })
                                    .hover(|s| {
                                        s.bg(if is_selected {
                                            rgb(0x2563eb)
                                        } else {
                                            rgb(0xf1f5f9)
                                        })
                                    })
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_event, window, cx| {
                                            // 点击选中：通过 entity 拿到 view，
                                            // 统一走 select（含滚动跟随 + notify）。
                                            // 必须经由 entity.update 而非 this，
                                            // 否则闭包无法 'static（this 是借用）。
                                            entity.update(cx, |this, cx| {
                                                this.select(ix, window, cx)
                                            });
                                        }
                                    })
                                    .child(this.items[ix].clone())
                            })
                            .collect()
                    }),
                )
                .track_scroll(&self.scroll_handle)
                .h_full()
                .border_1()
                .border_color(rgb(0xdddddd)),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(360.0), px(520.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    UniformListExample::new(
                        (1..=100).map(|i| format!("Item {i}")).collect::<Vec<_>>(),
                        cx,
                    )
                })
            },
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    // 桌面端 logger：env_logger 初始化后 log::info! 才能输出到终端。
    env_logger::init();
    log::info!("[uniform_list] starting 07_uniform_list");
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
