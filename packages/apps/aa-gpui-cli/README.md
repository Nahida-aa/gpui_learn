# aa-gpui-cli —— GPUI 工程脚手架工具

开发期命令行工具，负责**生成 Android 工程**，让例子目录保持「只写 Rust + 按需一份
身份文件」，不碰任何 Gradle/Kotlin 脚本。定位等同于 Tauri 的 `tauri` CLI
（`tauri android init`），将来稳定后可 `cargo install aa-gpui-cli` 供外部使用。

## 子命令

```
aa-gpui-cli android init [-p <工程目录>] [--bundle-meta <身份来源>] [--targets arm64-v8a,x86_64]
```

工程目录默认**当前目录**（在例子目录下直接跑即可）；如需为别的目录生成，
用 `-p / --project-dir` 指定（相对或绝对路径均可）。

## 身份来源：`--bundle-meta`

**没有魔法配置文件**——不隐式读取任何约定文件名。身份（`identifier` / `name`）
要么从 Cargo.toml 推导，要么用 `--bundle-meta` 显式指定来源。

引用有两种形式：

| 形式 | 例 | 用途 |
| --- | --- | --- |
| `<文件>` | `gpui.conf.json` | 文件**根**即身份表（“文件里面的东西就是要的结构”） |
| `<文件>#[k1, k2, ...]` | `Cargo.toml#[package, metadata, bundle-stable]` | 要的结果在**深层**，用数组路径下钻 |

规则：

- 文件部分相对 `-p` 工程目录解析；类型按扩展名分派（`.toml` / `.json`）。
- 路径是**数组形式**而非点分——JSON key 合法含点，且 identifier 本身
  （`dev.aa.AaCode`）就含点，点分在这里会歧义。
- 分隔符用 `#` 而非 `:`——后者会与 Windows 盘符（`C:\...`）碰撞。
- 表内 `identifier` / `name` **均可缺省**，缺的走 Cargo.toml 推导。
- 字段名**对齐 cargo-bundle 的 `BundleSettings`**（`identifier` / `name`），
  因此可以直接指向 Cargo.toml 的 `[package.metadata.bundle*]` 表——
  桌面打包与 Android 生成从此共用一个身份来源。

```jsonc
// gpui.conf.json（根级身份表）
{
  "identifier": "gpui_learn.input_05",   // 缺省 → <仓库名>.<package.name>
  "name": "GPUI Learn · Input"           // 缺省 → <package.name>
}
```

**引用解不出来一律硬报错**（文件缺失 / 路径不存在 / 目标不是表 / 字段不是字符串），
不静默回落——路径写错、文件被移动这类漂移必须立刻暴露，而不是悄悄用推导值生成一个
错的包名。

默认 `identifier` 的命名空间前缀取**仓库名**（git 根目录名，git 不可用时向上找
含 `.git` 的祖先目录名）。**若不在 git 仓库内又没提供 identifier，`init` 直接报错**——
不兜底假名，避免默认包名语义失真。

## 读取汇总

- `<工程>/Cargo.toml` —— 取 `[package] name` 作为 cargo 包名（`cargo ndk -p`），
  取 `[lib] name` 作为 Rust 库名（→ `lib<name>.so`）。**不重复声明**。
- `--bundle-meta` 指向的身份表（见上）——可选；不传就是纯 Cargo.toml 推导。

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
ABI 是工具默认值，不写进任何配置文件——遵循「配置极简、工具补全其余」。

## 模板从哪来

所有模板以 `include_str!` / `include_bytes!` **编译期内嵌**在 `aa-gpui-cli` 二进制里
（见 `templates/` 目录），因此 `aa-gpui-cli` 完全自包含，不依赖散落在仓库各处的
`gradle/` 文件。改模板后重新编译 `aa-gpui-cli` 即可，例子目录无需任何改动。

## 与例子的配合

每个 Android 例子（`ug_01_hello_android` / `ug_02_android_input` 等）只需：

```
<例子>/
├── Cargo.toml          # cdylib，[lib] name 即 .so 名
├── src/lib.rs          # android_main 入口 + 视图
├── gpui.conf.json      # 可选：根级身份表（identifier / name），经 --bundle-meta 指定
├── assets/fonts/...    # 可选，init 时自动拷进工程
└── gen/android/        # 生成物（gitignore）
```

`gpui.conf.json` 只是**一个普通文件**（不再有约定地位），由调用方显式指向：

```bash
# justfile / package.json 的 init
cargo run -p aa-gpui-cli -- android init --bundle-meta gpui.conf.json

# 也可以指向 Cargo.toml 里的 bundle 表（与桌面打包共身份）
cargo run -p aa-gpui-cli -- android init --bundle-meta "Cargo.toml#[package, metadata, bundle]"
```

改了 `Cargo.toml` / 身份文件后重跑 `init` 即可更新工程。
