#![cfg_attr(target_family = "wasm", no_main)]

//! # 07_example_text_area —— 多行文本输入示例（桌面 / Android 同构）
//!
//! 本例直接移植自 zed 仓库 `crates/gpui/examples/view_example/` 的三个文件：
//! `example_editor.rs`（`Editor` 引擎）+ `example_text_area.rs`（`TextArea`
//! 多行框）+ `view_example_main.rs`（actions / keybindings / 装配）。
//!
//! 代码按平台无关 / 平台相关拆分：
//! - `editor.rs` —— `Editor` 引擎（光标 / 闪烁 / IME / 逐行渲染），平台无关。
//! - `text_area.rs` —— `TextArea` 外壳，平台无关（弹软键盘那行用 cfg 隔离）。
//! - `app_view.rs` —— `MultilineExample` 顶层视图，平台无关。
//! - 本文件（`lib.rs`）—— 条件编译入口：Android 走 `android_main`，
//!   桌面走 `main` + `gpui_platform::application()`，二者共用 `run()`。
//!
//! 为什么不直接用 `crates/editor`（Zed 主编辑器、也是 Zed agent 聊天输入框
//! 用的那个）？因为它依赖整个 IDE 后端，对一个 Android 小示例太重。这里的
//! `Editor`/`TextArea` 是 zed **官方教学示例**，从零演示多行输入怎么实现。
//!
//! 运行：
//! - 桌面：`cargo run -p text_area_07_android`（在窗口里用硬键盘测试换行）。
//! - Android：见 `package.json` 的 `apk` / `install` / `launch`。

mod app_view;
mod editor;
mod text_area;

pub use app_view::MultilineExample;
pub use editor::{Editor, Enter};

/// 编译期从 Cargo.toml 的 `version` 读入，启动时打印，便于确认设备上
/// 跑的是不是最新构建（升小版本见 `npm run bump` / package.json）。
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

use gpui::{App, AppContext, Focusable, KeyBinding, WindowOptions};

// 动作与 keybinding 与 zed view_example_main.rs 一致（绑定到全局）。
fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", editor::Backspace, None),
        KeyBinding::new("delete", editor::Delete, None),
        KeyBinding::new("left", editor::Left, None),
        KeyBinding::new("right", editor::Right, None),
        KeyBinding::new("home", editor::Home, None),
        KeyBinding::new("end", editor::End, None),
        KeyBinding::new("enter", editor::Enter, None),
        KeyBinding::new("cmd-q", editor::Quit, None),
    ]);
}

/// 打开窗口并装配 `MultilineExample`。Android / 桌面共用。返回窗口句柄，
/// 调用方再决定要不要自动聚焦 + 弹软键盘。
fn open_example_window(cx: &mut App) -> gpui::WindowHandle<MultilineExample> {
    bind_keys(cx);
    cx.open_window(
        WindowOptions {
            window_bounds: None, // Android 上窗口即整个屏幕；桌面端由平台决定
            ..Default::default()
        },
        |_, cx| cx.new(MultilineExample::new),
    )
    .expect("打开窗口失败")
}

// ===========================================================================
// 桌面入口
// ===========================================================================
#[cfg(not(target_os = "android"))]
pub fn run() {
    // 初始化桌面 logger backend，使 log::info! 能输出到终端（RUST_LOG 控制级别，
    // 例如 RUST_LOG=info）。不初始化时 log 宏被静默丢弃。
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("text_area_07", log::LevelFilter::Info)
        .try_init();
    log::info!("text_area_07 v{} 桌面端启动", APP_VERSION);
    gpui_platform::application().run(|cx: &mut App| {
        let window = open_example_window(cx);
        // 桌面端：自动聚焦顶层视图，硬键盘即可直接输入（含回车换行）。
        let _ = window.update(cx, |view, window, cx| {
            let focus = view.focus_handle(cx);
            window.focus(&focus, cx);
            cx.activate(true);
        });
        cx.on_action(|_: &editor::Quit, cx| cx.quit());
    });
}

// ===========================================================================
// Android 入口
// ===========================================================================
#[cfg(target_os = "android")]
mod android_entry {
    use super::*;
    use gpui::{App as _, Application};

    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("text_area_07"),
        );
        std::panic::set_hook(Box::new(|info| {
            log::error!("text_area_07 panic: {info}");
        }));
        log::info!("android_main: entered (text_area_07 v{})", APP_VERSION);

        let _platform = gpui_android::android::jni::init_platform(&app);
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {
            log::error!("android_main: shared_platform() 返回 None");
            return;
        };
        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {
            log::info!("Application::run 回调：打开窗口");
            let window = open_example_window(cx);
            cx.on_action(|_: &editor::Quit, cx| cx.quit());

            window
                .update(cx, |view, window, cx| {
                    let ta_focus = view.focus_handle(cx);
                    window.focus(&ta_focus, cx);
                    cx.activate(true);

                    // Android 没有物理键盘，获得焦点时主动弹软键盘。
                    let _ = window.on_focus_in(&ta_focus, cx, |_window, _cx| {
                        gpui_android::android::jni::show_keyboard_android(
                            gpui_android::KeyboardType::MultiLine,
                        );
                    });
                    let _ = window.on_focus_out(&ta_focus, cx, |_event, _window, _cx| {
                        gpui_android::android::jni::hide_keyboard_android();
                    });

                    // 兜底弹出一次（初始聚焦时 on_focus_in 未必触发）。
                    gpui_android::android::jni::show_keyboard_android(
                        gpui_android::KeyboardType::MultiLine,
                    );
                })
                .unwrap();
        });
        log::info!("android_main: Application::run 返回");
    }
}
