# 03_hello_android —— 用**自有** `gpui-android` 后端在 Android 原生跑 GPUI

第三个例子，对应「移动端原生后端」路线。它和 `01`/`02` 最大的不同：

- `01_hello_world` / `02_hello_web` 走 `gpui_platform`（桌面 / wasm 后端，浏览器或 X11/wayland）。
- `03` 走 `crates/gpui-android`（vendored 进本仓库、**对接本仓库自己的 GPUI `82aef443`**），
  由 Android 的 `NativeActivity` 加载本例子编出的 `libhello_android.so`，
  在 **Vulkan / wgpu** 上真正原生渲染 —— 不经过任何浏览器。

```
GPUI 应用 (src/lib.rs, android_main)
   │  Application::with_platform(shared_platform.into_rc()).run(...)
   ▼
crates/gpui-android  (AndroidPlatform: impl gpui::Platform)
   │  Vulkan / wgpu 渲染 + 触控/键盘/剪贴板/无障碍
   ▼
NativeActivity (GpuiActivity.java)  ──  系统窗口 / ANativeWindow / JNI
```

## 目录结构

```
03_hello_android/
├── Cargo.toml          # cdylib：crate-type = ["cdylib"]，只有 android target 真正编出 .so
├── src/lib.rs          # android_main 入口 + HelloAndroid 视图（仅 #[cfg(target_os="android")] 编译）
├── gpui.conf.json      # ★ 唯一需要维护的 Android 配置：identifier / app_name
├── assets/fonts/       # 例子自带的资源（如 NotoColorEmoji.ttf），init 时自动拷进工程
├── package.json        # 命令入口（见下），IDE 会显示运行按钮
└── gen/android/        # 由 gpui-cli android init 生成（已 gitignore，重跑即更新）
```

### 工程由 `gpui-cli android init` 生成（Tauri 风格：不写 Kotlin/Gradle）

本例子**不写任何 Gradle/Kotlin 脚本**。`gen/android/` 整个工程由开发工具
`gpui-cli`（见 `crates/gpui-cli`）根据两个输入生成：

- 例子的 `Cargo.toml` —— 自动取 `[package] name`（cargo 包名）与 `[lib] name`（→ `libXXX.so`）
- 例子的 `gpui.conf.json` —— 只放真正属于 Android、Cargo.toml 里没有的两个字段：

```json
{
  "identifier": "dev.gpui.learn.hello_android",
  "app_name": "GPUI Learn · Hello Android"
}
```

`gpui-cli` 内嵌了 Gradle 模板与 Gradle 8.9 wrapper，生成时自动注入：
`rustLibName` / `cargoPackage`（来自 Cargo.toml）、`appId` / `appName`（来自
gpui.conf.json）、默认 ABI 列表（`arm64-v8a` + `x86_64`）；并把例子的 `assets/`
复制进 `app/src/main/assets/`。`AndroidManifest.xml`、`GpuiTheme`、`GpuiActivity.java`
（来自 `gpui-android`）也都一并生成/引用。

> 这和 Tauri 的 `tauri android init` 同一个哲学——**配置极简，工具生成工程**，
> 应用层完全不碰 kt。改了 `Cargo.toml` / `gpui.conf.json` 后重跑 `bun run init` 即可。

## 命令入口（package.json）

本例用 `package.json` 的 `scripts` 管理常用命令，IDE（VS Code / Zed / …）会在
脚本旁自动显示 **▶ 运行按钮**，比 `justfile` 更顺手：

```bash
bun run init        # gpui-cli android init → 生成 gen/android/
bun run rust:check  # 仅类型检查（aarch64-linux-android）
bun run apk         # cd gen/android && ./gradlew assembleDebug
bun run apk:release # release 版
bun run install       # adb install 到真机
bun run launch        # adb am start 启动 Activity
bun run run           # apk + install + launch 一条龙
bun run logs          # adb logcat 看 hello_android / gpui-android 日志
bun run uninstall     # adb uninstall
```

> 需要 Node.js（仅用来跑 bun 脚本，不参与任何构建）。

## 工具链前置（一次性）

和 `docs/maintain-gpui-android.md` §1 一致：

| 工具                                             | 说明                                 |
| ------------------------------------------------ | ------------------------------------ |
| Rust + `rustup target add aarch64-linux-android` | 编 arm64 的 .so                      |
| `cargo install cargo-ndk`                        | 用 NDK 工具链打包 .so（已装 v4.1.2） |
| `ANDROID_HOME` / `NDK_HOME`                      | 指向 SDK / NDK                       |
| Java 21                                          | Gradle 需要                          |
| `adb`                                            | 真机安装 & 看 logcat                 |

## 构建方式

### A. 先生成工程（首次或改配置后）

```bash
bun run init
# = cargo run -p gpui-cli -- android init
#   如需为别的目录生成：android init -p apps/03_hello_android
# 生成 gen/android/（含 Gradle 8.9 wrapper + 从 Cargo.toml/gpui.conf.json 注入的配置）
```

### B. 打 apk（用生成的 Gradle wrapper）

`gen/android/` 自带 `./gradlew`（锁定 Gradle 8.9），直接用即可，无需另装 Gradle：

```bash
cd gen/android
./gradlew assembleDebug        # 首次会自动下载 Gradle 8.9 分发
# 产物：app/build/outputs/apk/debug/app-debug.apk
```

> ⚠️ **不要用系统 `gradle` 命令**。本机系统 Gradle 是 9.x，而 AGP 8.7.3 只支持
> Gradle 8.x（9.x 不被任何 AGP 8.x 接受），直接用 `gradle` 会在插件解析阶段失败。
> 生成的 `./gradlew` 锁了 8.9，已验证可正常构建。

> 💡 若 `./gradlew` 下载 Gradle 8.9 分发很慢（官方 services.gradle.org 有时被限速），
> 可手动从国内镜像取下 `gradle-8.9-bin.zip`，放进
> `~/.gradle/wrapper/dists/gradle-8.9-bin/<hash>/` 再跑 `./gradlew`，它会直接解压使用。
> 镜像例：`https://mirrors.cloud.tencent.com/gradle/gradle-8.9-bin.zip`。

`assembleDebug` 触发 `:app:preBuild` → `gpui-cli` 生成的 `cargoBuild` 任务，
它会自动跑 `cargo ndk`（多 ABI）把 `libhello_android.so` 放到 `jniLibs/`，
再交给 Android 打包。

> 若只想编 release：`bun run apk:release`（= `./gradlew assembleRelease -Prelease`）。
> 注：debug 包约 345 MB（含完整调试符号）；release 会小很多。

## 装到真机

```bash
adb install -r gen/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.gpui.learn.hello_android/dev.gpui.mobile.GpuiActivity
# 看日志（Rust 侧 tag 是 hello_android，平台层是 gpui-android）：
adb logcat -s hello_android:T gpui-android:T
```

期望：启动后全屏深色界面，居中显示「Hello, Android 🤖」+ 一行版本信息 +
绿色 `Tap me` 按钮，点按后计数。能点按即说明「自有 gpui-android 后端
正常接收事件 / 渲染 / 重绘」。

## 与 02 的对照（同一套 GPUI，不同后端）

|          | 02_hello_web                                       | 03_hello_android                                                                          |
| -------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| 平台后端 | `gpui_platform`（wasm / WebGPU）                   | `crates/gpui-android`（Vulkan/wgpu）                                                      |
| 入口     | `fn main()`（trunk 包成 wasm）                     | `android_main(app)`（NativeActivity 调起）                                                |
| 运行模型 | `run_embedded` + `mem::forget` 防止 app 被释放白屏 | `Application::with_platform(...).run(...)` **阻塞**在事件循环，App 随栈帧存活，无白屏问题 |
| 编译目标 | wasm32-unknown-unknown                             | aarch64-linux-android（cdylib .so）                                                       |
| 部署     | trunk serve + HTTPS（手机需可信源）                | Gradle 打 apk + adb 安装                                                                  |

三个例子共用同一个 `gpui`（`82aef443`）：`crates/gpui-android` 是把社区
`gpui-toolkit` 的 Android 后端迁就到我们版本的结果（见 `docs/maintain-gpui-android.md`）。

## 已知限制（教学向）

- 只编 `arm64-v8a` 一种 ABI（最常见真机）。要 x86_64 模拟器另加 `-t x86_64` 并改 Gradle `abiFilters`。
- 没接 `gpui-ui-kit`：界面是手写 `div`（和 `02` 同范式），保持依赖最小、便于理解原生后端本身。
- 软键盘 / 无障碍走 `GpuiActivity` 的 `InputConnection` 与 AccessibilityNodeProvider 桥接，
  由 `gpui-android` 已实现的 JNI 方法支撑，本例未额外用到。
- **emoji 字体**：Android 系统 emoji 字体是 COLR v1，cosmic-text 用不了；本例把
  `NotoColorEmoji.ttf`（CBDT 格式，约 10 MB）放在例子的 `assets/fonts/` 目录，
  `gpui-cli android init` 会把它复制进 `gen/android/app/src/main/assets/fonts/`，
  由 `gpui-android` 在启动时从 `assets/fonts/` 加载，emoji（如标题里的 🤖）才能正常
  渲染。该字体文件已提交进例子目录的 `assets/`，所以 `bun run init && bun run apk`
  开箱即得，无需另下。若想减小 apk，可从例子 `assets/` 删掉它（代价是 emoji 变豆腐块）。
- **`GpuiPlatformView` 可选类**：`gpui-android` 会反射 `dev.gpui.mobile.GpuiPlatformView`
  （platform-view 功能用），本例未启用该功能，找不到该类时只打 `debug` 级日志，不影响运行。
