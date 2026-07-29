# gpui_learn_common —— 共享库 crate

这个包是**库（library）**，不是能直接运行的程序。它的职责是：
把「如何接入 GPUI」这件事只做一次，然后让所有 `apps/*` 例子复用。

## 为什么需要它（monorepo 的共享包概念）

Rust monorepo 的典型结构是「库 + 二进制」：

- **库（crate 类型 = lib）**：提供可复用代码，没有 `main`。
- **二进制（crate 类型 = bin）**：有 `main`，能 `cargo run`，依赖若干库。

当有很多个例子都想用 GPUI 时，如果每个例子都自己写 git 源依赖、
自己选平台 feature，就会出现「同一个 gpui 被配置了 N 遍」的问题——
一旦要升级 `rev` 或调整 feature，就要改 N 个文件，而且很可能改漏导致
版本不一致。

解法：把 GPUI 的接入收敛进**一个**库，`apps/*` 用 `path` 依赖指向它：

```toml
# apps/hello_window/Cargo.toml
[dependencies]
gpui_learn_common = { path = "../../crates/gpui_learn_common" }
```

`path = "..."` 表示「依赖同一工作区内的另一个本地 crate」，cargo 会把它
当作 workspace 成员直接编译，不需要发布到 crates.io。这就是 monorepo
「本地共享包」的本质。

## 为什么 GPUI 用 git 源而不是 crates.io

`gpui` 虽然在 crates.io 上有发布版（`0.2.x`），但它内部大量依赖 zed 的
自有 fork（例如 `font-kit` 指向 `zed-industries/font-kit` 的特定 rev）。
crates.io 的发布版经常无法独立编译通过，社区实际几乎都通过 git 源使用：

```toml
gpui = { git = "https://github.com/zed-industries/zed", rev = "<commit>", package = "gpui" }
```

用 git 源还有两个对学习很有价值的好处：

1. **能用最新特性**：比如 `gpui_web` 这个 crate 提供了把 GPUI 编译成
   HTML/WASM 的能力（在浏览器、包括手机浏览器里运行），它通常还没进
   crates.io 稳定版，git 源才能拿到。
2. **可锁定、可复现**：`rev` 锁到具体 commit，`Cargo.lock` 就不会漂移，
   你和别人、未来的自己 `cargo build` 拿到的都是同一份代码。

> 关于「移动端」：目前 zed/gpui 没有原生的 iOS/Android 后端 crate。
> 在手机上跑 GPUI 的现实路径是「用 `gpui_web` 编译成 WASM，在移动端浏览器里运行」。
> 这部分我们会在后续例子（web target）里演示，不会夸大能力。

## 这里的 `run_app` 封装了什么

`src/lib.rs` 里的 [`run_app`](src/lib.rs) 把「建 App → 开窗口 → 放入根 View」
的最小样板收敛成一行调用，让例子代码聚焦在「学 GPUI 本身」而不是「配环境」。
需要更多控制（自定义窗口选项、多窗口等）时，例子可以直接用重新导出的
`gpui_learn_common::gpui` 写完整流程。
