# 02_hello_web —— 官方的「质数筛」Web 例子（trunk 构建）

完全照搬 zed 官方 `crates/gpui_web/examples/hello_web`，用来演示 **trunk** 这条
官方推荐的 GPUI Web 构建路线。例子本身是一个 CPU 密集的质数计数（分块丢到后台
线程），借以验证 GPUI 在浏览器里也能跑多线程、出画面。

## 与 01 的区别

- `01_hello_world` 是**手写** WASM 入口 + 自己用 `wasm-bindgen`/`bun` 起静态服务，
  适合理解底层机制。
- `02` 用 **trunk**（`trunk serve` 一条命令完成编译 + wasm-bindgen + 起带正确
  COEP/COOP 头的本地服务器）。`trunk.toml` 与 `index.html` 照搬官方，零胶水代码。

### ⚠️ 配置文件必须叫 `Trunk.toml`（大写 T）

trunk 只会按 `Trunk.toml`（或 `.trunk.toml`）读取配置，**不会**读小写的
`trunk.toml`。写成小写时 trunk 静默忽略，于是 `[serve] headers` 里的
COEP/COOP 根本不会生效——浏览器因此**不是跨源隔离**的，`SharedArrayBuffer`
不可用，GPUI 会回退到单线程 dispatcher，进而出现
`Required WebAssembly threading APIs are unavailable` 以及随后的
`gpui::window: app was released` 报错。**本目录用的是 `Trunk.toml`（大写）。**

> **为什么 zed 官方示例是小写 `trunk.toml`？**
> 翻 trunk 0.21.14 源码 `src/config/models/source/mod.rs`，配置候选列表只含
> `"Trunk.toml" / ".trunk.toml" / "Trunk.yaml" / ...`，**根本没有小写**。
> 注释写着 `Trunk.toml goes first, as it was the default for a long time`——
> 说明更早的 trunk 版本曾把小写 `trunk.toml` 当作默认候选，官方写这个 Web 示例
> （PR #50228「GPUI on the web」）时用的就是旧版 trunk，所以小写能正常读到配置。
> 之后 trunk 把规范统一成大写并从候选列表里删掉了小写，新版（含你本机的
> 0.21.14）便不再认小写。官方示例文件从加入起就是小写、也一直没改，属于「写于
> 旧版、规范变更后没跟上」的遗留状态，**不是我们应该模仿的用法**。

trunk 自动把 `fn main()` 包成 wasm 入口，所以这里：

- **不要**写 `#![cfg_attr(target_family = "wasm", no_main)]`（会让 wasm 下 `main`
  被禁用、整段逻辑变成 dead_code、wasm-bindgen 线程注入失败）。
- `web_init()` 只在 wasm 下存在（`#[cfg(target_family = "wasm")]`），桌面下调用会
  编译失败，所以 `main` 里用 cfg 包住它。

## 运行

```bash
# 桌面
cargo run -p hello_web_02

# 浏览器（自带正确响应头，直接支持 WebGPU / SharedArrayBuffer）
cd apps/02_hello_web
trunk serve
# 打开 http://localhost:8080
```

浏览器需要支持 **WebGPU**（Chrome/Edge 较新版本，必要时加启动参数
`--enable-unsafe-webgpu`）。`gpui_web` 仅支持 WebGPU，没有 WebGL 回退。

> **想在手机 / 局域网另一台设备访问？** 只把 `Trunk.toml` 的 `addresses` 改成
> `0.0.0.0` 不够——局域网 IP（`http://192.168.x.x`）被浏览器判为「不可信源」，
> 会忽略 COOP 头并拒绝 WebGPU 初始化。必须套一层本地 **HTTPS**（自签证书 +
> Caddy 反代），让 origin 变可信。完整步骤见 `TROUBLESHOOTING.md` 第 6 节。
> 本目录 `Trunk.toml` 的 `addresses` 已设为 `0.0.0.0`，配合 §6 的 HTTPS 代理即可。

## 多线程构建要点（坑）

`apps/02_hello_web/.cargo/config.toml` 里的链接参数是官方同款：

- `+atomics,+bulk-memory,+mutable-globals` + `--shared-memory`：开 wasm 多线程。
- `--import-memory` + 导出 `__wasm_init_tls` / `__tls_size` / `__tls_align` /
  `__tls_base`：让 worker 线程能共享内存与 TLS。
- `build-std = ["std,panic_abort"]`：用 nightly 自带 std 构建。
- 本包 `rust-toolchain.toml` 固定用 nightly + `wasm32-unknown-unknown` + `rust-src`。

### 关键：wasm-bindgen 必须锁 0.2.120

本仓库 `Cargo.lock` 已把 `wasm-bindgen` 锁定到 **0.2.120**（与 zed 官方 `Cargo.lock`
一致），且 `Cargo.toml` 显式写了 `wasm-bindgen = "=0.2.120"`。

`wasm-bindgen` **0.2.121+** 在注入 worker 线程 id 时，会去 wasm 里找 `__heap_base`
符号；而新版 LLD 在 shared memory 配置下不再生成该符号，于是报：

```
error: failed to prepare module for threading
Caused by:
    failed to find `__heap_base` for injecting thread id
```

锁定 0.2.120（并安装对应 CLI：`cargo install wasm-bindgen-cli --version 0.2.120`）
即可避开。CLI 版本必须与 `Cargo.lock` 里的 `wasm-bindgen` 版本一致，否则
wasm-bindgen 步骤会报版本不匹配。

## ⚠️ 纯白屏的根因：`app was released`（Web 上必须用 `run_embedded`）

即使 COEP/COOP 都正确、WebGPU 也初始化成功，浏览器里仍可能**纯白**，控制台只报：

```
[ERROR] gpui::window: app was released
```

原因在 `gpui_web` 的 `WebPlatform::run`：它内部是
`spawn_local(async { WgpuContext::new_web().await; on_finish_launching(); })`，
`async` 块一结束就把捕获的 `Application`(Rc) 给 drop 了。也就是说桌面下
`Application::run` 是**阻塞**的、App 随调用栈一直活着；而 Web 上 `run` 立刻返回、
闭包跑在异步任务里，任务结束 → App 被释放 → 窗口/canvas 被销毁 → 白屏。

**修法（见 `src/main.rs`）**：Web 目标下改用 `run_embedded`，它返回
`ApplicationHandle`，把 App 钉在手里；`run` 不再阻塞也不会释放 App。再把 handle
`std::mem::forget` 掉，防止函数返回时 handle 被 drop 再次释放 App：

```rust
#[cfg(not(target_family = "wasm"))]
fn run_app() {
    gpui_platform::application().run(|cx: &mut App| { /* open_window ... */ });
}

#[cfg(target_family = "wasm")]
fn run_app() {
    let _app = gpui_platform::application().run_embedded(|cx: &mut App| {
        /* open_window ... */
    });
    std::mem::forget(_app); // 钉住 App，否则白屏
}
```

验证要点：用支持 WebGPU 的 Chrome 打开后，canvas 的绘制缓冲区尺寸应从初始的
`1×1` 被 `ResizeObserver` 改成真实物理尺寸（如 `1389×914`），且页面背景是
`#1e1e2e`、面板是 `#313244`——说明 `draw` 路径走通、UI 真的上屏了。
