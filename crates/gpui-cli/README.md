# gpui-cli —— GPUI 工程脚手架工具

开发期命令行工具，负责**生成 Android 工程**，让例子目录保持「只写 Rust + 一份
极简 `gpui.conf.json`」，不碰任何 Gradle/Kotlin 脚本。定位等同于 Tauri 的
`tauri` CLI（`tauri android init`），将来稳定后可 `cargo install gpui-cli` 供外部使用。

## 子命令

```
gpui-cli android init [-p <工程目录>] [--targets arm64-v8a,x86_64]
```

工程目录默认**当前目录**（在例子目录下直接跑即可）；如需为别的目录生成，
用 `-p / --project-dir` 指定（相对或绝对路径均可）。

读取：

- `<工程>/Cargo.toml` —— 取 `[package] name` 作为 cargo 包名（`cargo ndk -p`），
  取 `[lib] name` 作为 Rust 库名（→ `lib<name>.so`）。**不重复声明**。
- `<工程>/gpui.conf.json` —— 只需两个真正属于 Android、Cargo.toml 里没有的字段：

  ```json
  {
    "identifier": "dev.gpui.learn.input_05",
    "app_name": "GPUI Learn · Input"
  }
  ```

生成：`<工程>/gen/android/` 完整 Gradle 工程（已 `gitignore`），内含：

- `settings.gradle.kts` / `build.gradle.kts`（根，声明 AGP 8.7.3 `apply false`）
- `app/build.gradle.kts` —— 由内嵌模板注入 `rustLibName` / `cargoPackage` / ABI 列表，
  自动注册 `cargoBuild` 任务（多 ABI `cargo ndk`）
- `AndroidManifest.xml`（用 `${nativeLibraryName}` 占位符）
- `res/values/styles.xml`（共享全屏主题 `GpuiTheme`）
- `gradle.properties`（`appId` / `appName` + 通用 Android 配置）
- `gradlew` / `gradlew.bat` / `gradle-wrapper.*`（Gradle 8.9，内嵌进二进制）
- `<例子>/assets/` 整个复制到 `app/src/main/assets/`（emoji 字体等）

## 目标 ABI（默认与可选）

默认 `arm64-v8a` + `x86_64`（覆盖真机 + 模拟器）。完整可选集与 Tauri 默认一致：

```
arm64-v8a  armeabi-v7a  x86  x86_64
```

用 `--targets` 任选组合，例如 `--targets arm64-v8a,x86_64,armeabi-v7a`。
ABI **不写进 `gpui.conf.json`**——它是工具默认值，遵循「配置极简、工具补全其余」。

## 模板从哪来

所有模板以 `include_str!` / `include_bytes!` **编译期内嵌**在 `gpui-cli` 二进制里
（见 `templates/` 目录），因此 `gpui-cli` 完全自包含，不依赖散落在仓库各处的
`gradle/` 文件。改模板后重新编译 `gpui-cli` 即可，例子目录无需任何改动。

## 与例子的配合

每个 Android 例子（`03_hello_android` / `05_android_input`）只需：

```
<例子>/
├── Cargo.toml          # cdylib，[lib] name 即 .so 名
├── src/lib.rs          # android_main 入口 + 视图
├── gpui.conf.json      # identifier / app_name
├── assets/fonts/...    # 可选，init 时自动拷进工程
└── gen/android/        # 生成物（gitignore）
```

`package.json` 里的 `npm run init` 即调用本工具；改了 `Cargo.toml` / `gpui.conf.json`
后重跑 `init` 即可更新工程。
