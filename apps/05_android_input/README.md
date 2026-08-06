# 05_android_input —— 用 `gpui-android` 后端在手机上跑文本输入

第四个 Android 例子（`03` 之后），也是 `04_input`（桌面文本输入）的 **Android 版**。
核心结论先说在前：

> **`gpui-android` 后端原生支持 input**——`handle_input` 是 `Window` 的方法，
> `text_system` / 剪贴板 / 键盘布局 / IME 合成（`Commit` / `SetComposing` /
> `FinishComposing`）在 `packages/gpui-android` 里都已实现。所以「把 input 移植到
> Android」不是给后端补功能，而是**应用层把同一个 `TextInput` 逻辑接到 Android 入口**
> 并在需要时弹软键盘。

## 和 04_input 比，改了什么

`TextInput` 那套逻辑（光标 / 选区 / IME / 剪贴板 / 键盘布局回显）**原样复用**，
只做了三处 Android 适配：

| 差异   | 桌面 `04_input`                        | Android `05`                                                                                                 |
| ------ | -------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 入口   | `fn main()` → `application().run(...)` | `android_main(app)` → `Application::with_platform(...).run(...)`（阻塞事件循环）                             |
| 软键盘 | 物理键盘，无需处理                     | 输入框获焦时调 `show_keyboard_android`，失焦时 `hide_keyboard_android`（Android 没物理键盘，必须主动弹 IME） |
| 工程   | `cargo run`                            | 共享 Gradle 模板 + `gradle.properties` 配置（见下）                                                          |

软键盘的接法（在 `src/lib.rs` 的窗口 setup 里）：

```rust
let input_focus = view.text_input.focus_handle(cx);
window.on_focus_in(&input_focus, cx, |_window, _cx| {
    gpui_android::android::jni::show_keyboard_android(gpui_android::KeyboardType::Default);
});
window.on_focus_out(&input_focus, cx, |_event, _window, _cx| {
    gpui_android::android::jni::hide_keyboard_android();
});
```

## 构建：零 Kotlin 脚本（`gpui-cli android init` 生成）

本例子**不写任何 Gradle/Kotlin 脚本**。工程由 `gpui-cli` 这个开发工具根据配置
**生成**到 `gen/android/`（已 gitignore）。例子目录里只有两样东西：

- `src/lib.rs` —— Rust 代码（TextInput + android_main + 软键盘）
- `gpui.conf.json` —— 仅两个真正属于 Android、Cargo.toml 里没有的字段：

```json
{
  "identifier": "dev.gpui.learn.input_05",
  "app_name": "GPUI Learn · Input"
}
```

> 为什么只有这两个？`cargo_package`（`input_05_android`）和 `rust_lib_name`
> （`input_05`）由 `gpui-cli` 读本例子的 `Cargo.toml` 自动获取，不重复声明；
> 目标 ABI 用默认值（`arm64-v8a` + `x86_64`，覆盖真机 + 模拟器），不写进配置。
> 这正是 Tauri `tauri android init` 的哲学——配置极简，生成器补全其余。

生成与构建：

```bash
bun run init     # gpui-cli android init → 生成 gen/android/
bun run apk      # cd gen/android && ./gradlew assembleDebug
```

`gpui-cli` 的模板里自动注入：`rustLibName` / `cargoPackage`（来自 Cargo.toml）、
`appId` / `appName`（来自 gpui.conf.json）、默认 ABI 列表；`AndroidManifest.xml`、
`GpuiTheme`、`GpuiActivity.kt`（来自 `gpui-android`）也都一并生成/引用，无需手写。

## 命令入口（package.json）

```bash
bun run init     # 生成 gen/android/（首次或改了 Cargo.toml / gpui.conf.json 后重跑）
bun run apk      # ./gradlew assembleDebug（内部自动跑 cargo ndk，多 ABI）
bun run install  # adb install 到真机
bun run launch   # adb am start -n dev.gpui.learn.input_05/dev.gpui.mobile.GpuiActivity
bun run run      # init + apk + install + launch 一条龙
bun run logs     # adb logcat -s input_05:V gpui-android:V
```

## 装到真机

```bash
bun run init          # 生成 gen/android/
bun run apk           # cd gen/android && ./gradlew assembleDebug
adb install -r gen/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.gpui.learn.input_05/dev.gpui.mobile.GpuiActivity
```

期望：全屏界面，顶部显示当前键盘布局，下面是输入框（点一下弹出软键盘，能打字 /
移动光标 / 选区 / 复制粘贴），再下面回显最近的按键。说明 **input 在 Android 后端上
完全可用**。

## 已知限制

- 只编 `arm64-v8a`；软键盘 `KeyboardType` 目前用 `Default`，需要邮箱/数字键盘改传
  对应枚举即可（API 已支持）。
- emoji 字体同 `03`：APK 自带 `NotoColorEmoji.ttf`（CBDT），由 `gpui-android` 从
  `assets/fonts/` 加载。
- 关于「移动端 input 应排在桌面 input 之后学」：本例正是这个顺序的体现——先 `04`
  （桌面）吃透 input 原理，再 `05`（Android）只补入口与软键盘。
