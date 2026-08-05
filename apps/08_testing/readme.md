# 08_testing —— GPUI 测试框架与异步任务示例（桌面 / Android / Waydroid 同构）

本例搬运自 GPUI 官方 `examples/testing.rs`，是一个「计数器」演示：展示 GPUI 的
**测试设施**（`#[gpui::test]`）和**异步任务**（`cx.spawn` / `Task` / `detach`）两大主题。
它和 07 一样是双形态工程：桌面走 bin，Android / Waydroid 走 cdylib `.so`（NativeActivity）。

---

## 1. 界面与四个按钮

计数器界面（`src/lib.rs` `Counter::render`）有四个可点控件：

| 控件 | 行为 | 代码 | 说明 |
|------|------|------|------|
| `+` | `count += 1` | `increment()` | 同时绑定键盘 ↑ |
| `−` | `count -= 1` | `decrement()` | 同时绑定键盘 ↓ |
| **Load** | `count = 100` | `load()` | **可等待**的后台任务（返回 `Task`） |
| **Reload** | `count += 50` | `reload()` | **fire-and-forget** 后台任务（`.detach()`） |

`Load` / `Reload` 不是业务功能，是用来直观感受「同步更新 vs 异步后台任务」区别的演示控件。

### Load / Reload 的异步语义

```rust
// src/lib.rs:52
fn load(&self, cx: &mut Context<Self>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        this.update(cx, |counter, _| { counter.count = 100; }).ok();
    })
}

// src/lib.rs:62
fn reload(&self, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
        this.update(cx, |counter, _| { counter.count += 50; }).ok();
    }).detach();   // ← 关键：丢到后台自己跑，不等它完成
}
```

- **`load`**：返回一个 `Task<()>`，调用方可以 `.await` 它。演示「**可等待**的」后台任务。
- **`reload`**：用 `.detach()` 把任务丢到后台，不阻塞、不等待。演示「**fire-and-forget**」后台任务。

在真实 app 里，这两者对应「后台加载数据回来再更新 UI」：比如 `load` 是「等网络返回后设值」，
`reload` 是「触发刷新后立刻返回，数据到了再自己更新」。

---

## 2. 测试模块（`#[cfg(test)]`）—— 与按钮一一对应

文件末尾 `mod tests` 用 7 个测试覆盖上面的能力，全部用 `cargo test -p testing_08
--features test-support` 运行（已验证 7 passed）。最关键的是 `test_async_operations`，
它正是 `Load` / `Reload` 按钮背后的机制证明：

```rust
// src/lib.rs:279
#[gpui::test]
async fn test_async_operations(cx: &mut TestAppContext) {
    let counter = cx.new(|cx| Counter::new(cx));

    counter.update(cx, |counter, cx| counter.load(cx)).await;   // 等 Load 完成
    assert_eq!(count, 100);                                      // → 100

    counter.update(cx, |counter, cx| counter.reload(cx));        // 触发 Reload（detach）
    assert_eq!(count, 100);                                      // 此刻还是 100！任务还没跑

    cx.run_until_parked();                                       // 让后台任务跑完
    assert_eq!(count, 150);                                      // → 150
}
```

要点：**GPUI 测试执行器是单线程的**，async 副作用（含 `detach` 的任务）不会在你
`await` / `run_until_parked` 之前运行。这正是 `Reload`「点了立刻 +50 但背后是异步」的
行为来源——在 UI 上你看到的是同步刷新（因为有 `cx.notify()`），而在测试里可以精确
控制「什么时候让后台任务跑」。

其他测试覆盖：基础更新 + 事件订阅（`basic_testing`）、窗口内 action 派发
（`test_counter_in_window`）、允许 park 的外部线程（`test_allow_parking`）、属性测试
（`test_counter_random_operations`）、以及多 app 上下文的分布式系统模拟
（`distributed_systems` 子模块）。

> 跑测试需要 `test-support` 特性 + `rand`/`futures` dev-dependency（已在 `Cargo.toml`
> 声明）。官方例子内嵌在 gpui crate 自动继承这些，独立成 crate 后要显式声明。

---

## 3. 工程结构（双形态，与 07 一致）

```
apps/08_testing/
├── Cargo.toml        # [lib] crate-type=["cdylib","rlib"] + [[bin]] + android target 依赖
├── package.json      # dev / check / android:* / waydroid:* 脚本
├── src/
│   ├── lib.rs        # 真正逻辑：Counter + run()(桌面) + android_main()(Android) + tests
│   └── main.rs       # 桌面薄壳：fn main() { testing_08::run(); }
└── gen/android/      # gpui-cli android init 生成的 Gradle 骨架（构建产物被 .gitignore）
```

- 桌面：`cargo run -p testing_08`，窗口 300×200，↑/↓ 或点按钮增减。
- Android / Waydroid：编译 cdylib `.so` 给 NativeActivity 加载；`window_bounds: None`
  （全屏）。入口在 `lib.rs` 的 `android_entry::android_main`。

---

## 4. 运行方式

### 桌面

```bash
cargo run -p testing_08
cargo test -p testing_08 --features test-support   # 7 个测试
```

### Waydroid（推荐，本机容器，无需 adb）

> 前提：waydroid session 已在跑（`waydroid session start`，且 `linux-zen` 内核自带
> `rust_binder`，无需额外 binder 模块）。`package.json` 里 `waydroid:*` 脚本已用
> `/usr/bin/python3 /usr/bin/waydroid` 绕开 shell 里 uv venv 的 python（否则报
> `No module named 'dbus'`）。

```bash
bun run waydroid:run        # = android:init → waydroid:apk → waydroid:install → waydroid:launch
# 或分步：
bun run waydroid:apk        # cd gen/android && ./gradlew assembleDebug
bun run waydroid:install    # waydroid app install app-debug.apk
bun run waydroid:launch     # waydroid app launch gpui_learn.testing_08
bun run waydroid:uninstall  # waydroid app uninstall gpui_learn.testing_08
```

装好后也可在 waydroid 启动器里点 `testing_08` 图标。看日志：`bun run waydroid:shell`
进容器后 `logcat -s testing_08:V gpui-android:V`。

### 真机 / 其他模拟器（adb）

```bash
bun run android:run         # = android:init → android:apk → android:install → android:launch
# 包名（applicationId）由 gpui-cli 从仓库名+包名推导：gpui_learn.testing_08
```

---

## 5. 踩坑记录

- **`gpui::test` 找不到 / `block_test` 不存在**：测试需要 `test-support` 特性。
  独立成 crate 后必须显式 `features.test-support = ["gpui/test-support"]` + dev-deps
  `rand`/`futures`，否则 `#[gpui::test]` 宏和 `TestAppContext` 都不可用。
- **`rand` API 版本**：官方示例用 `rng.random_bool()` / `rng.random_range()`，这是
  `rand` 0.9 的 `Random` trait 方法。dev-dependency 写 `rand = "0.8"` 会编译失败，
  必须 `rand = "0.9"`。
- **`No module named 'dbus'`**：交互 shell 的 `python3` 指向 uv 的 venv（缺 dbus），
  而 `waydroid` CLI 需要系统 python。解决：`/usr/bin/python3 /usr/bin/waydroid ...`
  （已写进 `waydroid:*` 脚本）。
- **waydroid 不需要 `/dev/binder` 节点**：新版 waydroid 用内核内置 `rust_binder`
  （`linux` / `linux-zen` 自带），`modprobe binder_linux` 报 "Device or resource busy"
  是正常的——内置已占坑，外部模块插不进也无需插。不要为此重编内核或装
  `binder_linux-dkms`（zen 下它和内置 rust_binder 冲突、等于无用）。
- **`gpui-android` 必须跟 workspace 同 gpui rev**：升级根 `[patch.crates-io]` 的 gpui
  pin 后，`crates/gpui-android/Cargo.toml` 里的 gpui / gpui_wgpu `rev` 也要同步升，
  否则 Android 构建会出现「两份 `gpui::Platform` trait」，`Application::with_platform`
  类型不匹配。
