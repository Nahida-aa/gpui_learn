//! # 03_hello_android —— 用**自有** `gpui-android` 后端在 Android 原生跑 GPUI
//!
//! 这是第三个例子，对应「移动端原生后端」路线：
//!
//! - `01_hello_world` / `02_hello_web` 走的是 `gpui_platform`（桌面 / wasm 后端）。
//! - 本例子走 `crates/gpui-android`（vendored 进本仓库、**对接本仓库自己的 GPUI 82aef443**），
//!   由 Android 的 `NativeActivity` 加载这个 `.so`，在 Vulkan/wgpu 上渲染。
//!
//! ## 入口机制（与 02 的关键区别）
//!
//! Android 没有 `fn main()`。系统的 `NativeActivity` 加载本 crate 编出的
//! `libhello_android.so` 后，由 `android-activity` 胶水层调用我们导出的
//! `android_main(app: AndroidApp)`。在那里：
//!
//! 1. `gpui_android::android::jni::init_platform(&app)` 把 `AndroidApp` 存成全局，
//!    并创建 `AndroidPlatform`（实现 `gpui::Platform`）。
//! 2. `shared_platform()` 取出它，交给 `Application::with_platform(..).run(..)`。
//!    `run` 在 Android 上**阻塞**在事件循环里（见 `crates/gpui-android/src/android/platform.rs`），
//!    因此 `Application`（持有 `Rc<AppContext>`）随栈帧一直存活 —— 不会出现 02 wasm 那种
//!    `app was released` 白屏问题。
//!
//! 构建与部署见同目录 `README.md`。

#[cfg(target_os = "android")]
mod imp {
    // prelude 把所有样式 trait（Styled / InteractiveElement / …）和
    // div / px / rgb 等常用项一次性拉进作用域，否则 .flex()/.text_sm()/.id()
    // 这些方法会因为 trait 不在作用域而报 E0599。
    use gpui::prelude::*;
    use gpui::{App, AppContext, Application, Context, Render, Window, WindowOptions, div, rgb};

    // ── 视图状态 ────────────────────────────────────────────────────────────
    //
    // 刻意做成一个最简单的「Hello」界面：标题 + 一行系统信息 + 一个可点的按钮。
    // 目的不是功能，而是演示「自有 gpui-android 后端能正常接收事件、渲染、重绘」。
    struct HelloAndroid {
        tapped: u32,
    }

    impl HelloAndroid {
        fn new(_cx: &mut Context<Self>) -> Self {
            Self { tapped: 0 }
        }
    }

    impl Render for HelloAndroid {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let tap_line = if self.tapped == 0 {
                "按钮还没被点过。点一下试试？".to_string()
            } else {
                format!("你点了 {} 次 👆", self.tapped)
            };

            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(rgb(0x1e1e2e))
                .justify_center()
                .items_center()
                .gap_4()
                .p_6()
                .child(
                    div()
                        .text_2xl()
                        .text_color(rgb(0xcdd6f4))
                        .child("Hello, Android 🤖"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0xa6adc8))
                        .child("GPUI on Vulkan/wgpu via 自有 gpui-android (zed 82aef443)"),
                )
                .child(
                    div()
                        .id("tap")
                        .px_5()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(0xa6e3a1))
                        .text_color(rgb(0x1e1e2e))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            this.tapped += 1;
                            cx.notify();
                        }))
                        .child("Tap me"),
                )
                .child(div().text_sm().text_color(rgb(0x6c7086)).child(tap_line))
        }
    }

    // ── Android 入口 ──────────────────────────────────────────────────────────
    //
    // `#[no_mangle]` + `pub fn android_main` 是 android-activity 约定的入口符号。
    // NativeActivity 加载 .so 后会调用它（在一个专有的 native 线程上）。
    #[unsafe(no_mangle)]
    pub fn android_main(app: android_activity::AndroidApp) {
        // 把日志导向 logcat（用 `adb logcat -s hello_android:T` 看）。
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("hello_android"),
        );

        // panic 也打到 logcat，方便真机排查。
        std::panic::set_hook(Box::new(|info| {
            log::error!("hello_android panic: {info}");
        }));

        log::info!("android_main: entered");

        // 1) 创建并全局存储 AndroidPlatform。
        let _platform = gpui_android::android::jni::init_platform(&app);

        // 2) 取出 SharedPlatform，交给 GPUI 作为本进程的平台实现。
        let Some(shared_platform) = gpui_android::android::jni::shared_platform() else {
            log::error!("android_main: shared_platform() 返回 None，平台未初始化");
            return;
        };

        // 3) 阻塞式运行：run 内部会驱动事件循环，直到 App 退出/Activity 被销毁。
        //    on_finish_launching 回调里打开第一个窗口（GPUI 会复用平台已建好的
        //    AndroidWindow，见 platform.rs 的 run() 注释）。
        Application::with_platform(shared_platform.into_rc()).run(|cx: &mut App| {
            log::info!("Application::run 回调：打开窗口");
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: None, // Android 上窗口即整个屏幕
                    ..Default::default()
                },
                |_, cx| cx.new(HelloAndroid::new),
            );
            if let Err(e) = result {
                log::error!("打开 Android 窗口失败：{e:#}");
                return;
            }
            cx.activate(true);
        });

        log::info!("android_main: Application::run 返回（App 已退出）");
    }
}

// 非 android target 下本 crate 是空壳（cdylib 但无任何导出符号）。
// 这样 `cargo check --workspace`（host）不会因为缺 android-activity 而报错。
#[cfg(not(target_os = "android"))]
pub fn placeholder() {}
