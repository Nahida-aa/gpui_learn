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
├── Cargo.toml                 # cdylib：crate-type = ["cdylib"]，只有 android target 真正编出 .so
├── src/lib.rs                 # android_main 入口 + HelloAndroid 视图（仅 #[cfg(target_os="android")] 编译）
├── package.json               # 命令入口（见下），IDE 会显示运行按钮
└── gradle/                    # Gradle 宿主：把 .so 打包成 apk
    ├── settings.gradle.kts    # 含 AGP 版本/仓库镜像；rootProject.name 等
    ├── build.gradle.kts       # 根：声明 AGP 8.7.3（apply false）
    ├── gradle.properties      # ★ 本例子唯一需要改的配置：appId / appName / rustLibName / cargoPackage
    ├── gradle-wrapper.*       # 见下方「装 Gradle」：锁 Gradle 8.9
    └── app/
        ├── build.gradle.kts   # 只有一行：apply 共享模板（见下）
        └── src/main/
            ├── AndroidManifest.xml   # 用 ${nativeLibraryName} 占位符
            └── jniLibs/arm64-v8a/libhello_android.so   # 由 cargo ndk 产出（见下）
```

### 构建逻辑来自共享模板（Tauri 风格：不写 Kotlin/Gradle）

所有 Android 例子的 Gradle 构建逻辑**抽到了仓库根的 `gradle/android-app.gradle`**
（Groovy DSL 写的共享模板），由各例子的 `app/build.gradle.kts` 一行 apply 引入：

```kotlin
// apps/03_hello_android/gradle/app/build.gradle.kts
apply(from = rootProject.file("../../../gradle/android-app.gradle"))
```

模板从 `gradle.properties` 读配置（`appId` / `appName` / `rustLibName` /
`cargoPackage` / `rustTarget`），自动：

- 注册 `cargoBuild` 任务调 `cargo ndk` 编 `.so`；
- 把 `rustLibName` 注入 `AndroidManifest` 的 `android.app.lib_name`；
- 用 `resValue` 注入 `app_name`，并引用仓库根的共享 `GpuiTheme`（`gradle/android-res`），
  所以例子**不用写任何 `res/values/strings.xml` 或 `styles.xml`**；
- 复用 `crates/gpui-android/android/src/main/java` 里的 `GpuiActivity`，不复制。

> 为什么是 Groovy 而非 `.kts`：被 `apply from=` 的脚本在 Kotlin DSL 下拿不到
> android 插件的类型安全访问器，Groovy 的动态访问器没有这个限制，从而做到
> 「一份模板、多个例子零脚本复用」。这和 Tauri 用 `tauri.conf.json` 生成 Android
> 工程是同一个哲学——**配置驱动，模板生成**，应用层完全不碰 kt。
>
> `05_android_input` 就是直接复用这份模板的第二个例子：它和 `03` 的 `gradle/`
> 几乎一模一样，唯一的差异是 `gradle.properties` 里的那几个变量。

> `GpuiActivity.java` **不复制**到这里：它随 `crates/gpui-android/android/src/main/java`
> 一起 vendored 进了仓库，`app/build.gradle.kts` 通过 `java.srcDir` 直接引用，
> 保证 Java 侧和 Rust 侧始终同源。

## 命令入口（package.json）

本例用 `package.json` 的 `scripts` 管理常用命令，IDE（VS Code / Zed / …）会在
脚本旁自动显示 **▶ 运行按钮**，比 `justfile` 更顺手：

```bash
npm run rust:check    # 仅类型检查（aarch64-linux-android）
npm run rust:build    # 用 cargo ndk 编出 libhello_android.so 到 jniLibs
npm run apk           # ./gradlew assembleDebug（内部自动跑 cargo ndk）
npm run apk:release   # release 版
npm run install       # adb install 到真机
npm run launch        # adb am start 启动 Activity
npm run run           # apk + install + launch 一条龙
npm run logs          # adb logcat 看 hello_android / gpui-android 日志
npm run uninstall     # adb uninstall
```

> 需要 Node.js（仅用来跑 npm 脚本，不参与任何构建）。

## 工具链前置（一次性）

和 `docs/maintain-gpui-android.md` §1 一致：

| 工具                                             | 说明                                 |
| ------------------------------------------------ | ------------------------------------ |
| Rust + `rustup target add aarch64-linux-android` | 编 arm64 的 .so                      |
| `cargo install cargo-ndk`                        | 用 NDK 工具链打包 .so（已装 v4.1.2） |
| `ANDROID_HOME` / `NDK_HOME`                      | 指向 SDK / NDK                       |
| Java 21                                          | Gradle 需要                          |
| `adb`                                            | 真机安装 & 看 logcat                 |

## 两种构建方式

### A. 只编 Rust 侧（最快，验证 GPUI 能编进 .so）

```bash
# 在仓库根执行。-P 26 必须 ≥ 24，否则链接找不到 libnativewindow.so。
cargo ndk -t arm64-v8a -P 26 \
  -o apps/03_hello_android/gradle/app/src/main/jniLibs \
  build -p hello_android_03

# 校验产物确实是 native 库并导出了入口符号
llvm-nm -D apps/03_hello_android/gradle/app/src/main/jniLibs/arm64-v8a/libhello_android.so \
  | grep -E 'android_main|Java_dev_gpui_mobile_GpuiActivity_nativeIsInitialized'
```

### B. 打 apk（用仓库内的 Gradle wrapper）

本仓库**自带 `./gradlew`（锁定 Gradle 8.9）**，直接用即可，无需另装 Gradle：

```bash
cd apps/03_hello_android/gradle
./gradlew assembleDebug        # 首次会自动下载 Gradle 8.9 分发
# 产物：app/build/outputs/apk/debug/app-debug.apk
```

> ⚠️ **不要用系统 `gradle` 命令**。本机系统 Gradle 是 9.x，而 AGP 8.7.3 只支持
> Gradle 8.x（9.x 不被任何 AGP 8.x 接受），直接用 `gradle` 会在插件解析阶段失败。
> 仓库内的 `./gradlew` 锁了 8.9，已验证可正常构建。

> 💡 若 `./gradlew` 下载 Gradle 8.9 分发很慢（官方 services.gradle.org 有时被限速），
> 可手动从国内镜像取下 `gradle-8.9-bin.zip`，放进
> `~/.gradle/wrapper/dists/gradle-8.9-bin/<hash>/` 再跑 `./gradlew`，它会直接解压使用。
> 镜像例：`https://mirrors.cloud.tencent.com/gradle/gradle-8.9-bin.zip`。

`assembleDebug` 触发 `:app:preBuild` → 我们注册的 `cargoBuild` 任务，
它会自动跑 `cargo ndk` 把 `libhello_android.so` 放到 `jniLibs/arm64-v8a/`，
再交给 Android 打包。

> 若只想编 release：`./gradlew assembleRelease -Prelease`。
> 注：debug 包约 345 MB（含完整调试符号）；release 会小很多。

## 装到真机

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
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
  `NotoColorEmoji.ttf`（CBDT 格式，约 10 MB）作为 APK asset 打包，由 `gpui-android`
  在启动时从 `assets/fonts/` 加载，emoji（如标题里的 🤖）才能正常渲染。该字体文件已
  提交进仓库，所以 `./gradlew assembleDebug` 开箱即得，无需另下。若想减小 apk，可删掉
  `gradle/app/src/main/assets/fonts/NotoColorEmoji.ttf`（代价是 emoji 变豆腐块）。
- **`GpuiPlatformView` 可选类**：`gpui-android` 会反射 `dev.gpui.mobile.GpuiPlatformView`
  （platform-view 功能用），本例未启用该功能，找不到该类时只打 `debug` 级日志，不影响运行。
