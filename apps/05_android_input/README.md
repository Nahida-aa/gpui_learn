# 05_android_input —— 用 `gpui-android` 后端在手机上跑文本输入

第四个 Android 例子（`03` 之后），也是 `04_input`（桌面文本输入）的 **Android 版**。
核心结论先说在前：

> **`gpui-android` 后端原生支持 input**——`handle_input` 是 `Window` 的方法，
> `text_system` / 剪贴板 / 键盘布局 / IME 合成（`Commit` / `SetComposing` /
> `FinishComposing`）在 `crates/gpui-android` 里都已实现。所以「把 input 移植到
> Android」不是给后端补功能，而是**应用层把同一个 `TextInput` 逻辑接到 Android 入口**
> 并在需要时弹软键盘。

## 和 04_input 比，改了什么

`TextInput` 那套逻辑（光标 / 选区 / IME / 剪贴板 / 键盘布局回显）**原样复用**，
只做了三处 Android 适配：

| 差异 | 桌面 `04_input` | Android `05` |
| --- | --- | --- |
| 入口 | `fn main()` → `application().run(...)` | `android_main(app)` → `Application::with_platform(...).run(...)`（阻塞事件循环） |
| 软键盘 | 物理键盘，无需处理 | 输入框获焦时调 `show_keyboard_android`，失焦时 `hide_keyboard_android`（Android 没物理键盘，必须主动弹 IME） |
| 工程 | `cargo run` | 共享 Gradle 模板 + `gradle.properties` 配置（见下） |

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

## 构建：零 Kotlin 脚本（复用 03 的共享模板）

本例子**不写任何 Gradle/Kotlin 脚本**。构建逻辑来自仓库根的
`gradle/android-app.gradle`（和 `03` 同一个模板），`app/build.gradle.kts` 只有一行
`apply`。所有可变项都在 `gradle.properties`：

```properties
appId=dev.gpui.learn.input_05      # 与 03 不同，两台例子可共存于同一手机
appName=GPUI Learn · Input
rustLibName=input_05               # → libinput_05.so
cargoPackage=input_05_android      # cargo ndk -p 的包名
rustTarget=arm64-v8a
```

`AndroidManifest.xml`、`GpuiTheme`、`GpuiActivity.java` 也都来自共享模板/`gpui-android`，
例子目录里只有 Rust 代码 + `gradle.properties`。

## 命令入口（package.json）

和 `03` 同一套，包名/Activity 换成 `input_05`：

```bash
npm run rust:build   # cargo ndk 编 libinput_05.so
npm run apk          # ./gradlew assembleDebug（自动跑 cargo ndk）
npm run install      # adb install 到真机
npm run launch       # adb am start -n dev.gpui.learn.input_05/dev.gpui.mobile.GpuiActivity
npm run run          # apk + install + launch 一条龙
npm run logs         # adb logcat -s input_05:V gpui-android:V
```

## 装到真机

```bash
cd apps/05_android_input/gradle
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
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
