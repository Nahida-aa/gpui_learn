# GPUI 移动端后端对比：gpui-mobile vs gpui-toolkit

本仓库 `apps/02_hello_web` 走的是 **Web 路线**（trunk 编译 wasm，浏览器跑）。
但在研究「手机上跑 GPUI」时，发现了两个**原生移动端**项目，它们绕开了 Web 路线
所有的坑（WebGPU / 可信源 / HTTPS）。本文记录对这两个项目移动端后端的代码级对比，
作为 `gpui_learn` 的扩展阅读。

> 前置结论：原生移动端（iOS/Android 直接编译成 app）从根上不需要浏览器、不需要
> WebGPU、不被「可信源」限制——这正是我们 Web 路线在手机上处处碰壁时，另一条
> 完全可行的路。

---

## 0. 三个仓库定位

| 仓库                   | 定位                                        | GPUI 来源                  | 移动端                      |
| ---------------------- | ------------------------------------------- | -------------------------- | --------------------------- |
| `gpui_learn`（本仓库） | 教学 monorepo                               | zed `82aef443`             | 仅 Web 路线                 |
| `gpui-mobile`          | 单一 crate，给 GPUI 补 iOS/Android 平台层   | zed `5688167d`             | 原生 iOS/Android app        |
| `gpui-toolkit`         | 产品级 UI 组件库 + 工具链生态（20+ crates） | **zed v1.9.0（0.8.x 线）** | iOS/macOS/Android/tvOS/AUv3 |

`gpui-toolkit` 的 `gpui-android` 在 `lib.rs` 开头明写：

> _This crate is initially ported from `itsbalamurali/gpui-mobile`_

即 **`gpui-toolkit` 直接 fork 了 `gpui-mobile` 的 Android 后端**，再独立演进
（加了 accessibility、IME 事件队列、credential、packages 等），并和自家的
`gpui-ios` 配对，组成完整移动端后端。三者 GPUI 版本互不相同，无法合并进同一
workspace。

---

## 1. 启动 API：和本仓库 `02` 同源

`gpui-toolkit` 的总装入口 `crates/gpui-miniapp/src/misc.rs::current_platform()`
按 `target_os` 选后端：

```rust
#[cfg(target_os = "ios")]     { Ok(gpui_ios::current_platform(false)) }
#[cfg(target_os = "android")] { Ok(gpui_android::current_platform(false)) }
// macOS -> gpui_macos, linux -> gpui_linux, windows -> gpui_windows
```

然后 `crates/gpui-miniapp/src/mini_app.rs::run()` 用：

```rust
gpui::Application::with_platform(platform).run(|cx: &mut App| { /* open_window ... */ });
```

这和我们 `02_hello_web/src/main.rs` 里的
`gpui_platform::application().run(...)` **是同一套 `Application` API**——区别只是
`gpui-toolkit` 手动注入 platform，`gpui_platform` 在 Web 端自动选 web 后端。

移动端 `current_platform()` 签名（`gpui-android/src/lib.rs`、`gpui-mobile` 同款）：

```rust
pub fn current_platform(headless: bool) -> Rc<dyn gpui::Platform>
```

---

## 2. 关键架构差异：移动端不存在 Web 端的 `app was released`

回顾本仓库 `02` 的坑：`WebPlatform::run` 是 `spawn_local(async { ... })`，
async 块结束就把 `Application`(Rc) drop 掉 → 白屏 `app was released`。

**移动端不会犯这个错**，看 `gpui-android/src/android/platform.rs::run()`：

```rust
pub fn run(&self, on_finish_launching: Box<dyn FnOnce() + Send>) {
    // 把回调存进 state，然后【阻塞】在 native 事件循环
    if let Some(app) = super::jni::android_app() {
        super::jni::run_event_loop(&app);   // 阻塞直到 quit() / activity 销毁
    } else {
        // headless / test：直接调回调
        ...
    }
}
```

原生 app 有**永久事件循环**（`ALooper` on Android，`UIApplicationMain` on iOS），
App 随进程一直活着，不存在「async 块结束就释放」的问题。这是 **Web 异步任务模型
vs 原生永久事件循环** 的本质差异——也是为什么 Web 端必须 `run_embedded` +
`mem::forget`，而移动端直接 `run` 即可。

---

## 3. 渲染后端：都用 `gpui_wgpu`，但后端不同

两边都用 `gpui_wgpu` 的 `GpuContext` + `WgpuRenderer`，接原生窗口：

- **Android**（`gpui-android/src/android/window.rs`）：`AndroidWindow` 包 `ANativeWindow *`，
  用 **Vulkan** 建 wgpu surface。
- **iOS**（`gpui-ios/src/ios/window/ios_window.rs`）：包 `CAMetalLayer`（`UIView`），
  用 **Metal**。注意 iOS 端有个细节：`gpui_wgpu::WgpuContext::instance()` 默认只开
  Vulkan+GL，所以 iOS 端**自己建了一个带 Metal 后端的 wgpu instance**
  （`metal_instance.create_surface_unsafe(...)` + 自建 `WgpuContext`）再喂给
  `WgpuRenderer::new()` 复用。

文本：Android 用 `cosmic-text` + `swash`；iOS 用 **CoreText**
（`gpui-ios/src/ios/text_system/`）。

---

## 4. 两端对称的能力（来自 fork 的共同结构）

`gpui-mobile` / `gpui-android` 都暴露一套移动特有的工具函数（签名几乎一致）：

- `set_system_chrome(StatusBarContentStyle)` — 状态栏/导航栏样式
- `show_keyboard()` / `hide_keyboard()` / `KeyboardType` — 软键盘
- `safe_area_insets()` — 安全区
- `TEXT_INPUT_DIRTY` + `set_text_input_callback` / `dispatch_text_input` — 软键盘输入回灌

`gpui-toolkit` 在此基础上额外做了（`gpui-android/src/lib.rs`）：

- `ImeEvent` 队列（Commit / SetComposing / FinishComposing / DeleteSurrounding）
- `credential_alias` — FNV-1a 稳定的 keychain 别名
- `accessibility` 模块（accesskit）
- `packages/` 子集（deeplink / media_session）

---

## 5. 与本仓库的关系 & 建议

- **若想做「手机上跑 GPUI」的教学例子**：`gpui-toolkit` 比 `gpui-mobile` 更完整
  （自带组件库、脚手架 CLI `gpui-scaffolder`、真上架 App 实例 SotF/StkOpt），但其
  GPUI 版本是 `v1.9.0`，和本仓库 `82aef443` 不兼容，**不能直接并入 workspace**。
- **若只想理解移动端 GPUI 怎么落地**：本文已给出核心文件索引，直接读
  `gpui-toolkit/crates/gpui-android/src/android/{platform,window}.rs` 和
  `gpui-ios/src/ios/{platform,window/ios_window}.rs` 即可，无需编译（本机无
  macOS/Xcode/Android NDK，且 `current_platform()` 非移动 target 直接 panic）。
- **Web 路线 vs 原生路线**：本仓库 `02` 的 Web 路线受浏览器 WebGPU 与「可信源」
  限制（iOS 基本无 WebGPU）；原生路线无此限制，但需各平台 SDK 才能构建真机包。
  两条路互补，不是替代。

---

## 6. 快速文件索引

gpui-toolkit（fork 自 gpui-mobile）：

- `packages/gpui-android/src/lib.rs` — 公开 API + 「ported from gpui-mobile」声明
- `packages/gpui-android/src/android/platform.rs` — `AndroidPlatform` 实现 `gpui::Platform`，`run()` 阻塞事件循环
- `packages/gpui-android/src/android/window.rs` — `ANativeWindow` + wgpu Vulkan surface
- `crates/gpui-ios/src/ios/platform.rs` / `window/ios_window.rs` — Metal/CAMetalLayer 后端
- `crates/gpui-miniapp/src/misc.rs` — 按 `target_os` 选后端的统一入口
- `crates/gpui-miniapp/src/mini_app.rs` — `Application::with_platform(platform).run(...)`

gpui-mobile（上游单 crate）：

- `src/lib.rs` — `current_platform()` + 移动工具函数
- `src/android/platform.rs` / `src/ios/platform.rs` — 平台后端
