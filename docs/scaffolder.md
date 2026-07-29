# gpui-scaffolder：一键生成跨平台 GPUI mini-app

`gpui-toolkit` 自带一个脚手架 CLI `gpui-scaffolder`，用来生成一个**独立、可跨
桌面/iOS/Android/tvOS** 的 GPUI app 骨架。本文记录它生成什么、怎么用，以及生成的
代码如何呼应 `docs/mobile-backends.md` 里讲的后端选择机制。

> 位置：`gpui-toolkit/crates/gpui-scaffolder/`（只有 `lib.rs` + `main.rs`，依赖仅
> `anyhow` + `clap`，是个极简生成器）。

---

## 1. 用法

```bash
# 在当前目录生成 my-app/
cargo run -p gpui-scaffolder -- my-app

# 生成到指定目录
cargo run -p gpui-scaffolder -- my-app --output-dir /tmp

# 只预览会创建什么，不写文件
cargo run -p gpui-scaffolder -- my-app --dry-run

# 覆盖一个已存在的空目录
cargo run -p gpui-scaffolder -- my-app --force
```

生成后：

```bash
cd my-app
cargo run      # 桌面直接跑
just run       # Justfile 封装
```

CLI 参数（`src/main.rs` 的 `clap::Args`）：`name`（必填）、`--output-dir`（默认
`.`）、`--force`、`--dry-run`。

---

## 2. 生成的项目结构

`scaffold_app()` 创建的目录（`src/lib.rs`）：

```
my-app/
├── Cargo.toml              # 依赖指回 toolkit 的各本地 crate
├── gpui-scaffold.toml      # 脚手架元数据（SCAFFOLD_TEMPLATE_VERSION）
├── Justfile
├── README.md
├── src/
│   ├── main.rs             # fn main() { my_app::run_desktop(); }
│   ├── lib.rs              # 桌面 run_desktop() + iOS/tvOS 的 #[no_mangle] FFI 入口
│   └── app.rs              # 真正的 View（用 MiniApp + ui-kit 写）
├── ios/
│   ├── project.yml         # XcodeGen 描述
│   └── <app>Source/AppDelegate.swift
└── android/gradle/...      # Gradle 宿主 + jniLibs 目录
```

---

## 3. 生成的 Cargo.toml 关键点

`cargo_toml()` 模板（`src/lib.rs`）展示了子项目怎么接 toolkit：

```toml
[dependencies]
gpui = { version = "0.2.2", git = "https://github.com/zed-industries/zed.git", tag = "v1.9.0" }
gpui-miniapp = { path = "../gpui-toolkit/crates/gpui-miniapp" }
gpui-ui-kit  = { path = "../gpui-toolkit/crates/gpui-ui-kit" }

[target.'cfg(any(target_os = "ios", target_os = "tvos"))'.dependencies]
gpui-ios = { path = "../gpui-toolkit/crates/gpui-ios" }

[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.6", features = ["native-activity"] }
android_logger = "0.15"
gpui-android   = { path = "../gpui-toolkit/crates/gpui-android" }
log = "0.4"

[patch."https://github.com/zed-industries/font-kit"]
zed-font-kit = { path = "../gpui-toolkit/crates/3rdparties/zed-font-kit" }
[patch.crates-io]
block = { path = "../gpui-toolkit/crates/3rdparties/block" }
```

要点：

- **所有 GPUI 相关 crate 锁在 zed `v1.9.0`**（`GPUI_ZED_TAG` 常量），与 toolkit
  workspace 一致——这也是为什么它和本仓库 `gpui_learn`（zed `82aef443`）不能合并。
- 子项目用**本地 path 依赖**指回 toolkit 的 crate，所以 scaffolder 必须在
  toolkit 仓库内运行（`toolkit_root()` 解析到仓库根）。
- `[patch]` 把 `font-kit` / `block` 换成 toolkit 自带的 fork，避免依赖冲突。

---

## 4. 生成的代码：跨平台入口怎么写

**`src/main.rs`**（极简）：

```rust
fn main() { my_app::run_desktop(); }
```

**`src/lib.rs`** 暴露两套入口（`lib_rs()` 模板）：

- 桌面：`pub fn run_desktop()`（内部走 `gpui-miniapp` 的桌面后端）
- iOS/tvOS：用 `#[unsafe(no_mangle)] pub extern "C" fn <ffi_start_symbol>()`，
  里面调 `gpui_ios::ios::ffi::set_app_callback(Box::new(|cx| { ...全局状态...; open_app_window(cx); }))`
  再 `gpui_ios::ios::ffi::run_app()`。这个符号被 Swift `AppDelegate` 调用。

注意 iOS 入口里手动 `set_global` 了 `ThemeState` / `DesignSystemState` /
`AccessibilityTree` / `I18nState`——这些都是 `gpui-ui-kit` 的设计系统全局状态，
和 `docs/mobile-backends.md` 里 `mini_app.rs::run()` 设置的内容对应。

**`src/app.rs`**（真正的 UI，`app_rs()` 模板）：

```rust
use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::{Button, ButtonVariant, Heading, Text, ThemeExt};

pub struct MyAppView;
impl MyAppView {
    pub fn new(_: &mut Context<Self>) -> Self { Self }
}
impl Render for MyAppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 用 gpui-ui-kit 的声明式组件（见 docs/ui-kit.md）
    }
}
```

---

## 5. 与本仓库的关系

- 这是**原生移动端路线**的「官方起手式」：比我们 `02` 的 Web 路线多了对 iOS/Android
  的一等支持，但代价是必须安装各平台 SDK（Xcode / Android NDK）才能出真机包。
- 生成的代码印证了 `docs/mobile-backends.md` 的结论：一个 GPUI app 通过
  `gpui-miniapp` 的 `current_platform()` 按 `target_os` 选后端，桌面/iOS/Android
  共用同一套 `View`/`Render` 代码。
- 本机（Linux 桌面）可以 `cargo run` 跑生成的**桌面**版（若 toolkit 能编译），
  但 iOS/Android 目标仍需对应 SDK。
