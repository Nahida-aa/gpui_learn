# 01_hello_world —— 最小 GPUI 程序（纯 GPUI，无封装）

这是整个 gpui_learn 工作区里**第一个、也是最真实**的例子。
它直接 `use gpui::*`，**不依赖任何内部共享库**，目的就是让学习者先看到
GPUI 最原始的 API 长什么样——没有任何封装层挡在中间。

## 怎么跑

```bash
cargo run -p hello_world_01
```

因为根 `Cargo.toml` 的 `default-members = ["apps/*"]`，直接 `cargo run` 也会
默认构建并运行 apps 下的二进制；用 `-p hello_world_01` 显式指定包，推荐写法。
（目录名是 `01_hello_world` 以体现学习顺序，但 Cargo 包名不能以数字开头，故包名为 `hello_world_01`。）

## 这个例子演示了什么

1. **GPUI 程序的最小骨架**：`application().run(|cx| { cx.open_window(...) })`，
   入口来自 `gpui_platform::application`（注意不是 `Application::new()`）。
2. **`Render` trait**：`HelloWorld` 实现 `Render`，`render` 返回一个元素树
   （`div().child(...)`）。状态变化时 GPUI 会重新渲染。
3. **桌面 + WASM 双入口**：文件顶部 `#![cfg_attr(target_family = "wasm", no_main)]`，
   并为 WASM 写了 `#[wasm_bindgen(start)]` 入口（`start()` 里先 `web_init()` 再 `run_example()`）。
   这意味着**同一份代码可以编译成 HTML 在浏览器（含手机浏览器）里运行**——
   这是 GPUI 的亮点，也是本仓库后续 Web 专题的主线。
4. **直接依赖 gpui**：`Cargo.toml` 里只有 `gpui` 和 `gpui_platform`，
   它们来自根 `[workspace.dependencies]`（已通过 `[patch.crates-io]` 指向 zed 的 git 源）。

> **⚠️ 一个隐蔽的坑（曾导致「程序在跑但屏幕上没窗口」）**：
> `gpui_platform` 的 `default-features` 是**空**的（`default = []`）。
> 如果不在依赖里显式开窗口后端 feature，`open_window` 会「成功返回」但
> 底层没有任何 Wayland/X11 窗口创建逻辑——表现就是程序正常运行、无报错、
> 但屏幕上一个窗口都没有。本项目用 target-specific 写法在 Linux 下显式开启
> `features = ["font-kit", "wayland", "x11"]`（对齐官方 `crates/gpui/Cargo.toml:150`）。
> 这也是为什么 zed 自己能出窗口、而照搬 `application().run` 的最小例子却不出——
> 差异就在于这一行 feature。

5. **二进制 crate 的本质**：本包有 `main` 函数、能被 `cargo run`，是一个
   「二进制 crate」。对比 `crates/gpui_learn_common`（库 crate，没有 main）。

> **关于 WASM / HTML 入口（已实测验证）**：
>
> - 桌面编译 `cargo run -p hello_world_01` 已验证通过；wasm 分支被
>   `#[cfg(target_family = "wasm")]` 过滤，不影响桌面。
> - **wasm 必须用 nightly 工具链编译**，不能用 stable。原因：GPUI 的 Web 后端
>   依赖 `wasm_thread`，它用了 nightly 专属 feature `stdarch_wasm_atomic_wait`；
>   stable 下会报 `error[E0554]: #![feature] may not be used on the stable release channel`。
> - 已实测可编出 `hello_world_01.wasm`（nightly + `wasm32-unknown-unknown` target）：
>
>   ```bash
>   rustup toolchain install nightly --profile minimal --target wasm32-unknown-unknown
>   cargo +nightly build --target wasm32-unknown-unknown -p hello_world_01
>   ```
>
> - 本项目已为 wasm 加上 `wasm-bindgen` 依赖（target-specific，桌面不引入；
>   具体版本由 Cargo.lock 锁定，当前是 `0.2.120`，`wasm-bindgen-cli` 需装同版本）。
> - **编出 HTML 的完整链路已实测可跑**（见下）。GPUI 在 wasm 下会**自己创建
>   `<canvas>` 并挂到 `document.body`**，所以 HTML 里无需手写 canvas，只负责
>   加载 wasm-bindgen 生成的 JS 即可。
> - **wasm 必须加 `-Zbuild-std=std,panic_abort`**：根 `.cargo/config.toml` 给
>   `wasm32-unknown-unknown` 开了多线程（atomics + shared-memory + `--import-memory`
>   - 导出 TLS 符号），这些都依赖 nightly 的 `build-std` 重新编译 std；
>     不加会链接失败或运行时缺线程 API。
> - **`--export=__heap_base` 是必须的**：新版 LLD 在 shared-memory 下不再默认导出
>   `__heap_base`，而 wasm-bindgen 注入线程 id 时需要它，缺了会报
>   `failed to find __heap_base for injecting thread id`。根 `.cargo/config.toml`
>   已显式加了这个导出，无需手动处理。

### 编译为 HTML 并在浏览器运行（已实测）

同一份 `src/main.rs`，用以下三步就能在浏览器（含手机浏览器）里看到窗口：

```bash
# 前置（一次性）
rustup toolchain install nightly --profile minimal --target wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.120   # 版本须与 Cargo.lock 一致

# 1) nightly 编 wasm（必须 -Zbuild-std，见下）
cargo +nightly build -Zbuild-std=std,panic_abort --target wasm32-unknown-unknown -p hello_world_01

# 2) wasm-bindgen 生成 JS 绑定（--target web 对应浏览器 ES module）
wasm-bindgen target/wasm32-unknown-unknown/debug/hello_world_01.wasm \
    --target web --no-typescript --out-dir web_dist --out-name hello_world_01

# 3) 拷贝 index.html 后，用带跨源隔离头的服务器打开
cp apps/01_hello_world/web/index.html web_dist/
cd web_dist && bun run ../apps/01_hello_world/web/serve.ts 8080
# 浏览器访问 http://127.0.0.1:8080/
```

或者用本仓库提供的两种方式之一（都已包含上述全部步骤）：

**方式 A：bash 脚本（在仓库根运行）**

```bash
./apps/01_hello_world/web/build_web.sh --serve
```

**方式 B：bun/npm 脚本（在 apps/01_hello_world/ 下运行）**

```bash
cd apps/01_hello_world
bun install            # 仅生成本地 node_modules 锁（本包无运行时依赖，可跳过）
bun run start          # = build:wasm + build:web + serve，一步到位
# 或分步：bun run build:wasm / bun run build:web / bun run serve
```

> **两个硬性要求**（缺了浏览器会白屏/报错）：
>
> 1. **必须用 nightly 编 wasm**（原因见上，`wasm_thread` 的 nightly feature）。
> 2. **serve 时必须带 COEP/COOP 头**（`Cross-Origin-Embedder-Policy: require-corp`、
>    `Cross-Origin-Opener-Policy: same-origin`）——WebGPU 与 SharedArrayBuffer 的硬性要求。
>    本仓库的 `serve.ts`（bun 写）已自动加这两个头；`file://` 直接打开或普通静态服务器都不行。
>
> **⚠️ 浏览器必须支持 WebGPU**：GPUI 的 Web 后端（`gpui_web`）是 **WebGPU-only，
> 没有 WebGL 回退**（`crates/gpui_web/src/platform.rs` 只调 `WgpuContext::new_web()`，
> 失败就弹出 "Failed to initialize WebGPU. This application requires a browser with
> WebGPU support."）。这不是构建问题，是浏览器能力问题。解决：
>
> - **首选桌面版 Chrome / Edge（113+）**：默认开启 WebGPU，直接能用。
> - **Firefox**：地址栏 `about:config` 把 `dom.webgpu.enabled` 设为 `true`，重启。
> - **Safari**：升级到 17.4+（macOS）/ 18+（iOS）。
> - **Linux Chrome 仍报错**：多半缺 Vulkan 驱动，试启动参数
>   `chrome --enable-unsafe-webgpu --enable-features=Vulkan`，或确保系统有 Mesa。
> - **手机浏览器**：多数尚不稳定，Android Chrome 较新版本可能可用，iOS 需 18+。
>
> **为什么本地服务器用 bun 而不是 Python**：Windows 默认没有 Python，需自行安装；
> 而 Web 学习者大多已有 bun/node。bun 是单二进制、零额外依赖，`bun run serve.ts` 即可。
> （zeb 官方用的是 `trunk serve`，更重更封装；我们选 bun 是为轻量 + 每步可观测。）
>
> 桌面用 stable，wasm 用 nightly——这是 GPUI Web 后端的硬性要求。

## 为什么这里不先引入共享库

`crates/gpui_learn_common` 是一个「把样板收敛起来的共享库」，属于 **monorepo
共享包**的演示内容。如果第一个例子就用它，学习者会先看到封装、后看到 GPUI 本身，
顺序就反了。所以：先看懂这个纯 GPUI 例子 → 后续再学「如何用库消除重复样板」。

## 代码即文档

所有讲解都写在 `src/main.rs` 的文档注释（`//!` / `///`）里。把源码当教程读即可。
