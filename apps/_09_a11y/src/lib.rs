#![cfg_attr(target_family = "wasm", no_main)]

//! # 09_a11y —— GPUI 无障碍（AccessKit）演示（桌面 / WASM / Android 同构）
//!
//! 本工程是官方 `crates/gpui/examples/a11y.rs` 的搬运，演示 GPUI 如何把
//! **结构化无障碍树**暴露给操作系统，让辅助技术（AT，如屏幕阅读器 / 自动化
//! 驱动）能以编程方式「看到」并「操作」UI。
//!
//! 工程结构同 08：本文件（`lib.rs`）是条件编译入口——Android 走 `android_main`
//! （产出 cdylib `.so` 给 NativeActivity 加载），桌面/WASM 走 `run()`/`start()`，
//! 三者共用 `open_window()` 装配 `A11yDemo`。桌面二进制 `src/main.rs` 只是调用
//! `run()` 的薄壳。
//!
//! 依赖 GPUI 通过 AccessKit 实现无障碍层，映射到各平台系统接口：
//! - macOS → macOS 辅助功能 API
//! - Windows → UI Automation (UIA)
//! - Linux → AT-SPI（dbus）
//! - Android → 自有 `gpui-android` 后端采集 TreeUpdate（见
//!   `packages/gpui-android/src/accessibility.rs`）
//!
//! 运行：`cargo run -p _09_a11y`（桌面）。
//!
//! 本应用行为：
//! - 打开单个窗口，标题为 "GPUI Accessibility Demo"。
//! - 窗口内竖向堆叠以下元素：
//!   - 一级标题 "Accessibility Demo"（角色 Heading）。
//!   - 一行两个元素：
//!     - 微调框（`SpinButton`）"Counter: <n>"：提供 `Increment` / `Decrement`
//!       可访问动作，同时支持点击自增；数值最小值钳制为 0。
//!     - 按钮 "Reset counter"，将计数清零。
//!   - 一行两个元素：
//!     - 开关（`Switch`），初始关闭，开关切换动作无副作用。
//!     - 文本 "Enable feature"。
//!   - 一个待办列表（角色 List），含三个 ListItem：
//!     - "1. Write code" / "2. Run tests" / "3. Ship it"

use gpui::{
    AccessibleAction, App, Bounds, Context, FocusHandle, Role, SharedString, Toggled, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size, text,
};
#[cfg(not(target_os = "android"))]
use gpui::KeyBinding;

// 声明两个聚焦遍历动作（Tab 前进 / Shift+Tab 后退）。
actions!(a11y_example, [Tab, TabPrev]);

/// 演示状态：持有焦点句柄、一个整数计数和一个布尔开关。
struct A11yDemo {
    focus_handle: FocusHandle,
    count: i32,
    enabled: bool,
}

impl A11yDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        // 创建后立即把焦点交给本视图，便于键盘(Tab 遍历)直接进入元素树。
        window.focus(&focus_handle, cx);
        Self {
            focus_handle,
            count: 0,
            enabled: false,
        }
    }
}

impl Render for A11yDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .role(Role::Application)
            .aria_label("Accessibility Demo")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|_, _: &Tab, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &TabPrev, window, cx| window.focus_prev(cx)))
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            // Heading
            .child(
                div()
                    .id("heading")
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label("Accessibility Demo")
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(text!("Accessibility Demo")),
            )
            // Counter — uses a SpinButton role with Increment/Decrement
            // actions so screen readers can adjust the value directly.
            // Click also works via the built-in handler.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("counter")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::SpinButton)
                            .aria_label(SharedString::from(format!("Counter: {}", self.count)))
                            .aria_numeric_value(self.count as f64)
                            .aria_min_numeric_value(0.0)
                            .on_a11y_action(AccessibleAction::Increment, {
                                let this = cx.entity().downgrade();
                                move |_, _, cx| {
                                    this.update(cx, |this, cx| {
                                        this.count += 1;
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
                            .on_a11y_action(AccessibleAction::Decrement, {
                                let this = cx.entity().downgrade();
                                move |_, _, cx| {
                                    this.update(cx, |this, cx| {
                                        this.count = (this.count - 1).max(0);
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }))
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .cursor_pointer()
                            .child(text!(format!("Count: {}", self.count))),
                    )
                    .child(
                        div()
                            .id("reset")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::Button)
                            .aria_label("Reset counter")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x585b70))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count = 0;
                                cx.notify();
                            }))
                            .child(text!("Reset")),
                    ),
            )
            // A toggle switch
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("toggle")
                            .focusable()
                            .tab_stop(true)
                            .role(Role::Switch)
                            .aria_label("Enable feature")
                            .aria_toggled(if self.enabled {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .w(px(44.))
                            .h(px(24.))
                            .rounded_full()
                            .cursor_pointer()
                            .when(self.enabled, |el| el.bg(rgb(0x89b4fa)))
                            .when(!self.enabled, |el| el.bg(rgb(0x585b70)))
                            .child(
                                div()
                                    .size(px(20.))
                                    .rounded_full()
                                    .bg(gpui::white())
                                    .mt(px(2.))
                                    .when(self.enabled, |el| el.ml(px(22.)))
                                    .when(!self.enabled, |el| el.ml(px(2.))),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.enabled = !this.enabled;
                                cx.notify();
                            })),
                    )
                    .child(text!("Enable feature")),
            )
            // A short list
            .child(
                div()
                    .id("task-list")
                    .role(Role::List)
                    .aria_label("Tasks")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(
                        ["Write code", "Run tests", "Ship it"]
                            .iter()
                            .enumerate()
                            .map(|(i, label)| {
                                div()
                                    .id(("task", i))
                                    .role(Role::ListItem)
                                    .aria_label(SharedString::from(*label))
                                    .aria_position_in_set(i + 1)
                                    .aria_size_of_set(3)
                                    .py_1()
                                    .px_2()
                                    // Note: even though this `text!` macro
                                    // produces multiple elements, it doesn't
                                    // need its own unique ID because the parent
                                    // div has different IDs for each string.
                                    .child(text!(format!("{}. {}", i + 1, label)))
                            }),
                    ),
            )
    }
}

/// 打开窗口并装配 `A11yDemo`（桌面 / WASM / Android 共用）。
/// Android 上窗口即整个屏幕（`window_bounds: None`）；桌面给固定居中窗口。
fn open_window(cx: &mut App) {
    let bounds = if cfg!(target_os = "android") {
        None
    } else {
        Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(500.), px(400.0)),
            cx,
        )))
    };
    cx.open_window(
        WindowOptions {
            window_bounds: bounds,
            // 显式设置窗口标题，附带平台原生标题栏。
            #[cfg(not(target_os = "android"))]
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("GPUI Accessibility Demo".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        // 窗口根视图即 A11yDemo（Render + 无障碍树）。
        |window, cx| cx.new(|cx| A11yDemo::new(window, cx)),
    )
    .unwrap();
}

// ===========================================================================
// 桌面入口
// ===========================================================================
#[cfg(all(not(target_os = "android"), not(target_family = "wasm")))]
pub fn run() {
    use tracing_subscriber::prelude::*;

    // 初始化 tracing 订阅者：默认级别 Warn，gpui 模块提级到 Info，
    // 便于观察无障碍树构建日志。
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gpui=info".parse().unwrap())
                .add_directive("_09_a11y=trace".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    // 桥接 `log` 宏：gpui 内部仍用 log 输出，统一汇入上面的 tracing 订阅者。
    tracing_log::LogTracer::init().ok();

    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys([
            // Tab / Shift-Tab 切换焦点，供键盘导航（也是屏幕阅读器遍历的一种方式）。
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);

        open_window(cx);
        cx.activate(true);
    });
}

// ===========================================================================
// WASM 入口
// ===========================================================================
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    use tracing_subscriber::prelude::*;

    // WASM 下没有时间戳支持，用无时间层的订阅者，输出到浏览器 console。
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();
    tracing_log::LogTracer::init().ok();

    gpui_platform::web_init();
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);

        open_window(cx);
        cx.activate(true);
    });
}

// ===========================================================================
// Android 入口
// ===========================================================================
#[cfg(target_os = "android")]
mod android_entry {
    use super::*;
    use gpui::{App, Application};

    /// `#[no_mangle]` + `pub fn android_main` 是 android-activity 约定的入口符号，
    /// NativeActivity 加载 `.so` 后调用（在一个专有的 native 线程上）。
    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {
        // 日志导向 logcat（`adb logcat -s _09_a11y:V`）。
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("_09_a11y"),
        );
        std::panic::set_hook(Box::new(|info| {
            log::error!("_09_a11y panic: {info}");
        }));
        log::info!("android_main: entered (_09_a11y)");

        // 1) 创建并全局存储 AndroidPlatform（自带无障碍采集，见
        //    packages/gpui-android/src/accessibility.rs）。
        let _platform = gpui_android::android::jni::init_platform(&app);

        // 2) 取出 SharedPlatform，交给 GPUI 作为本进程的平台实现。
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {
            log::error!("android_main: shared_platform() 返回 None");
            return;
        };

        // 3) 阻塞式运行：run 内部驱动事件循环直到 App 退出/Activity 销毁。
        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {
            log::info!("Application::run 回调：打开窗口");
            open_window(cx);
        });
        log::info!("android_main: Application::run 返回");
    }
}

// 非 android/wasm target 下若直接作为库被引用，至少给出一个符号。
#[cfg(not(target_os = "android"))]
pub fn placeholder() {}
