# 维护自己的 gpui-android（vendor 进本仓库，对接自有 GPUI 版本）

目标：把社区 `gpui-toolkit` 里的 `gpui-android` 平台层**复制进本仓库**
（`packages/gpui-android/`），并把它依赖的 GPUI 改成**本仓库自己的版本**
（zed `82aef443`，见根 `Cargo.toml`），遇到编译不兼容就**改 `gpui-android`
的代码去适配本仓库的 GPUI**，而不是反过来升级本仓库的 GPUI。

> 原则：**以我为主**。本仓库的 `01`/`02` 例子和 GPUI 版本是基准，`gpui-android`
> 是迁就者。这与「fork 整个 gpui-toolkit 并升 GPUI」相反——后者会以别人版本为准。

---

## 1. 工具链（本机已具备）

实测本机已装齐，仅缺 `cargo-ndk`（已装）：

| 工具                       | 状态                       | 说明                                      |
| -------------------------- | -------------------------- | ----------------------------------------- |
| Rust + 4 个 android target | ✅ 已装                    | `aarch64/armv7/i686/x86_64-linux-android` |
| `ANDROID_HOME`             | ✅ `/home/aa/Android/Sdk`  | SDK 已存在                                |
| `NDK_HOME`                 | ✅ `.../ndk/29.0.13846066` | NDK r29 已存在                            |
| Java 21                    | ✅                         | NDK 构建需要                              |
| `adb`                      | ✅                         | 真机调试                                  |
| `cargo-ndk`                | ✅ 已装 v4.1.2             | 把 cargo 产物打包进 Android               |

安装命令（若换机器）：

```bash
cargo install cargo-ndk          # 需要 NDK + ANDROID_HOME 环境变量
rustup target add aarch64-linux-android
```

链路验证（已通过）：

```bash
cargo ndk -t arm64-v8a build     # 能在任意 crate 下编出 aarch64-linux-android 二进制
```

---

## 2. Vendor 步骤

```bash
# 1. 复制源码（保留其结构，不带动 toolkit 的其它 crate）
cp -r /home/aa/repos/ide_ls/gpui-toolkit/crates/gpui-android \
      /home/aa/repos/ide_ls/gpui_learn/packages/gpui-android

# 2. 加进 workspace（根 Cargo.toml 的 members 通配符通常会自动收纳；
#    若用显式列表则手动加 "packages/gpui-android"）

# 3. 改 packages/gpui-android/Cargo.toml 的 GPUI 依赖：
#    把 gpui / gpui_wgpu 从 zed v1.9.0 改成和根 Cargo.toml 一致的 82aef443 写法
```

`gpui-android/Cargo.toml` 里 GPUI 依赖要改成（与根 `Cargo.toml` 同源）：

```toml
gpui      = { git = "https://github.com/zed-industries/zed", rev = "82aef44308540b576e4e51fb379efa71614e5c91", version = "=0.2.2" }
gpui_wgpu = { git = "https://github.com/zed-industries/zed", rev = "82aef44308540b576e4e51fb379efa71614e5c91", version = "=0.1.0" }
```

注意 `gpui-android` **不依赖** `gpui-ui-kit`/`gpui-miniapp` 等上层（见其
`Cargo.toml` 的 dependencies），所以 vendoring 它不会连带拖进整个 toolkit。

---

## 3. 编译 & 改代码（遇问题改 gpui-android）

```bash
cargo check -p gpui-android --target aarch64-linux-android
```

预期会报错——因为 `gpui-android`（原配 v1.9.0）实现的 `gpui::Platform` trait 与
本仓库 `82aef443` 的 `gpui` 可能有签名差异。**按报错逐个改 `gpui-android/src/**`
去适配本仓库 GPUI**，不要去改本仓库的 GPUI 版本。

### 已解决的冲突（实测日志）

`cargo ndk -t arm64-v8a check -p gpui-android` 实测只撞到一个真正的 GPUI API
不兼容，已改 `gpui-android` 代码修复，**本仓库 GPUI 版本未动**：

| #   | 报错                                                                                                                                          | 根因（v1.9.0 → 82aef443）                                                                                                                                  | 修复（`gpui-android` 侧）                                                                                                                                                                                                                                              |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `E0046: not all trait items implemented, missing: on_system_wake`（同时出现在 `AndroidPlatform` 与 `SharedPlatform` 两个 `impl Platform` 块） | `82aef443` 的 `gpui::Platform` trait 比 v1.9.0 多了一个方法 `fn on_system_wake(&self, callback: Box<dyn FnMut()>)`（见 `crates/gpui/src/platform.rs:204`） | 在 `src/android/platform.rs` 的 `AndroidPlatform` impl 补一个空实现 `fn on_system_wake(&self, _callback: Box<dyn FnMut()>) {}`（Android 无系统睡眠/唤醒概念，永不回调）；`SharedPlatform` 用 `<AndroidPlatform as Platform>::on_system_wake(&self.0, callback)` 转发。 |

> 注：`WgpuRenderer::new`、`CosmicTextSystem`、`accesskit` 那一串**未**报错——
> `82aef443` 与 v1.9.0 在这几处的签名恰好兼容，所以不需要改 `window.rs` /
> `accessibility.rs`。后续若 GPUI 再升级碰到这些，按上表思路改即可。

构建链路本身还踩过两个**非 GPUI API** 的环境坑（已修，记此备查）：

- `android-activity` 必须开 `features = ["native-activity"]`，否则报错
  "Either game-activity or native-activity must be enabled"。
- 直接 `cargo check --target aarch64-linux-android` 找不到 `aarch64-linux-android-clang`
  （NDK 的 CC 没设）；必须用 `cargo ndk -t arm64-v8a check ...` 让它包一层 NDK 工具链。

### 本机可验证范围

- `cargo check --target aarch64-linux-android`：在本机 Linux 桌面即可跑，**不需要
  真机/模拟器**，用来把上面这些 API 不兼容改完。
- 但 `current_platform()` 在非 android target 直接 `panic`，所以桌面 `cargo run`
  跑不了——这是 `gpui-android` 的设计（见 `docs/mobile-backends.md`）。
- 真正出 apk / 上真机：需要 `cargo ndk` + Gradle 宿主（参考 `gpui-toolkit` 的
  `crates/gpui-showcase/android/` 或 `gpui-scaffolder` 生成的 `android/gradle/`）。

---

## 4. 与 Web 路线（02_hello_web）的关系

- `02` 用 `gpui_platform::application()`（Web 后端，依赖 `82aef443` 的 `gpui_web`）。
- `gpui-android` 是**另一条后端**，靠 `target_os = "android"` 条件编译接入，
  与 `02` 的 Web 路径不冲突——同一个 `gpui` crate（`82aef443`）同时支撑两者，
  只是编译目标不同（wasm vs aarch64-linux-android）。
- 若未来想让一个 app 同时支持 Web 和 Android，参考 `docs/mobile-backends.md`
  里 `gpui-miniapp` 的 `current_platform()` 分派器，但用本仓库的 `gpui-android`
  - `gpui_platform` 分别作为 android/web 后端。

---

## 5. 为什么不 fork 整个 gpui-toolkit

fork 整个 toolkit 意味着把 GPUI 升到 v1.9.0，会连累 `01`/`02` 的 Web 构建需要
重新验证（api 可能变），且 20+ crate、231KB lock 大部分用不到。本方案只 vendor
最小的 `gpui-android`，保持本仓库 GPUI 版本不变，代价最小、控制力最强。
