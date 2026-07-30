#![cfg_attr(target_family = "wasm", no_main)]

//! # 07_uniform_list —— GPUI `uniform_list` 定高虚拟列表示例（桌面 / Android 同构）
//!
//! 工程结构与 06 一致：本文件（`lib.rs`）是条件编译入口，Android 走 `android_main`
//! （产出 cdylib `.so` 给 NativeActivity 加载），桌面走 `run()` + `gpui_platform::
//! application()`，二者共用 `open_window()` 装配列表视图。桌面二进制 `src/main.rs`
//! 只是一个调用 `run()` 的薄壳。
//!
//! uniform_list 的关键能力与踩坑见 `docs/uniform_list.md`。

use gpui::{
    actions, App, Bounds, Context, FocusHandle, ScrollStrategy, UniformListScrollHandle, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size, uniform_list,
};
use std::ops::Range;

// 上下导航 + 回车选中：自定义 action，绑定键盘事件（↑/↓/Enter）。
actions!(uniform_list_07, [SelectNext, SelectPrev, Confirm]);

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// 打开窗口并装配 `UniformListExample`（桌面 / Android 共用）。
/// Android 上窗口即整个屏幕（`window_bounds: None`）；桌面端给一个固定居中窗口。
fn open_window(cx: &mut App) -> gpui::WindowHandle<UniformListExample> {
    let bounds = if cfg!(target_os = "android") {
        None
    } else {
        Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(360.0), px(520.0)),
            cx,
        )))
    };
    cx.open_window(
        WindowOptions {
            window_bounds: bounds,
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
    .expect("打开窗口失败")
}

// ===========================================================================
// 桌面入口
// ===========================================================================
#[cfg(not(target_os = "android"))]
pub fn run() {
    // 初始化桌面 logger backend，使 log::info! 能输出到终端。
    env_logger::init();
    log::info!("[uniform_list] starting 07_uniform_list v{}", APP_VERSION);
    gpui_platform::application().run(|cx: &mut App| {
        let window = open_window(cx);
        // 桌面端：自动聚焦顶层视图，键盘（↑/↓/Enter）即可直接操作。
        let _ = window.update(cx, |view, window, cx| {
            let focus = view.focus_handle.clone();
            window.focus(&focus, cx);
            cx.activate(true);
        });
    });
}

// ===========================================================================
// Android 入口
// ===========================================================================
#[cfg(target_os = "android")]
mod android_entry {
    use super::*;
    use gpui::{App, Application};

    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("uniform_list_07"),
        );
        std::panic::set_hook(Box::new(|info| {
            log::error!("uniform_list_07 panic: {info}");
        }));
        log::info!("android_main: entered (uniform_list_07 v{})", APP_VERSION);

        let _platform = gpui_android::android::jni::init_platform(&app);
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {
            log::error!("android_main: shared_platform() 返回 None");
            return;
        };
        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {
            log::info!("Application::run 回调：打开窗口");
            let _window = open_window(cx);
        });
        log::info!("android_main: Application::run 返回");
    }
}
