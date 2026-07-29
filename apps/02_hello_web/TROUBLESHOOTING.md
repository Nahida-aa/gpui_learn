# 02_hello_web —— 排错笔记（debug 过程中的发现）

这份文档记录把官方 `hello_web` 跑通到浏览器**真正出画面**的过程中，踩过的坑和
对应的排查手段。README 里只列了结论，这里保留排查链路，方便以后复现 / 教别人。

---

## 0. 现象

`trunk serve` 起来、Chrome 打开 `http://localhost:8080`，页面**纯白**，控制台报错：

```
[WARN] gpui_web::dispatcher: Required WebAssembly threading APIs are unavailable; falling back to single-threaded dispatcher
[INFO] gpui_wgpu::wgpu_context: Selected GPU adapter: "" (BrowserWebGpu)
[INFO] gpui_web::platform: WebGPU context initialized successfully
[ERROR] gpui::window: app was released
```

注意一个很迷惑的点：**WebGPU 初始化成功了**，但页面还是白。说明白屏和 WebGPU
本身无关，是更上游的 App 生命周期问题。

排查按顺序走了三步，每一步都对应一个独立的坑。

---

## 1. 坑一：trunk 不读 `trunk.toml`（小写），COEP/COOP 没生效

**症状**：`Required WebAssembly threading APIs are unavailable` + 单线程回退。

**根因**：trunk 0.21.14 只认 `Trunk.toml`（大写）或 `.trunk.toml`，**不会**读小写
的 `trunk.toml`。写成小写时 trunk 静默忽略，于是 `[serve] headers` 里的
`Cross-Origin-Embedder-Policy` / `Cross-Origin-Opener-Policy` 根本不发。

浏览器因此**不是跨源隔离**的 → `SharedArrayBuffer` 不可用 → GPUI 回退单线程
dispatcher。单线程本身还能跑，但它会连带触发下一个坑（见 §3）。

**验证 header 是否真的发出去**：

```bash
curl -sI http://127.0.0.1:8080/ | grep -i "cross-origin"
# 期望看到 Cross-Origin-Embedder-Policy: require-corp
#           Cross-Origin-Opener-Policy: same-origin
```

**修法**：把 `trunk.toml` 重命名为 `Trunk.toml`（本目录已是这个名字）。

---

## 2. 坑二：wasm-bindgen 线程注入找不到 `__heap_base`

**症状**（构建期而非运行期）：

```
error: failed to prepare module for threading
Caused by:
    failed to find `__heap_base` for injecting thread id
```

**根因**：`wasm-bindgen` 0.2.121+ 在注入 worker 线程 id 时去 wasm 里找
`__heap_base` 符号；而新版 LLD 在 shared memory 配置下不再生成该符号。

**修法**：

- `Cargo.lock` 锁 `wasm-bindgen = 0.2.120`（与 zed 官方一致）。
- `Cargo.toml` 显式写 `wasm-bindgen = "=0.2.120"`。
- 安装对应 CLI：`cargo install wasm-bindgen-cli --version 0.2.120`（CLI 版本必须与
  lock 文件一致）。
- 在 `.cargo/config.toml` 追加链接参数 `"-C", "link-arg=--export=__heap_base"`，
  作为防御性补充（强制导出该符号）。

---

## 3. 坑三（真正的白屏根因）：`app was released`

**症状**：WebGPU 已初始化成功，但页面纯白，控制台报 `gpui::window: app was released`。

**根因**：`gpui_web` 的 `WebPlatform::run` 内部是
`spawn_local(async { WgpuContext::new_web().await; on_finish_launching(); })`。
`async` 块一结束，就把捕获的 `Application`(Rc) drop 掉。

- 桌面：`Application::run` 是**阻塞**调用，App 随调用栈一直存活，没事。
- Web：`run` 立刻返回，闭包跑在异步任务里；任务结束 → App 被释放 →
  窗口 / canvas 被销毁 → 白屏。

那个 `app was released` 字符串来自 `gpui/src/app/async_context.rs`，就是
`app.upgrade()` 拿到 `None` 时打的。

**修法**（见 `src/main.rs`）：Web 目标改用 `run_embedded`，它返回
`ApplicationHandle`，把 App 钉在手里；再 `std::mem::forget` 掉 handle，防止函数
返回时 handle 被 drop 再次释放 App：

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

> `run_embedded` 不在桌面平台暴露，所以必须按 `target_family` 分两个入口。

---

## 4. 怎么确认「这次真的出画面了」（而不是又白屏）

只用肉眼看不可靠（尤其排错中途反复白屏会让人怀疑自己）。下面这套是验证过的手段。

### 4.1 canvas 绘制缓冲区尺寸

GPUI 的 Web 窗口初始 `bounds` 是 `0×0`，真实尺寸靠 `ResizeObserver` 回调设置。
所以 canvas 一开始是 `1×1`（或默认 `300×150`），**几秒后**才会变成真实物理尺寸。

在 Chrome 开发者工具控制台（或 CDP 的 `Runtime.evaluate`）执行：

```js
const c = document.querySelector("canvas");
console.log(c.width, c.height, getComputedStyle(c).width);
```

- 仍是 `300×150` / `1×1` → GPUI 的 `draw` 没执行到（生命周期问题，回到 §3）。
- 变成真实尺寸（如 `1389×914`）→ draw 路径走通。

### 4.2 ResizeObserver 是否触发（排除「size 0」假设）

曾怀疑过「纯白是因为 canvas 尺寸一直是 0」。用一段临时 HTML 验证
`devicePixelContentBoxSize` 是否有值，结论：**有值**（inline=1885 block=1588），
所以不是 size 0 的问题，根因在 §3。该临时文件（`diag.html`）已删除。

### 4.3 截图 + 像素分析（最硬的证据）

headless Chrome 在这台机器上**起不了 WebGPU**（会卡死 / 零日志），必须用
**headful** Chrome + 真实显示（Wayland/X11，`DISPLAY=:0`）。启动方式：

```bash
google-chrome-stable --no-sandbox \
  --user-data-dir=/tmp/cdp-chrome \
  --enable-unsafe-webgpu --enable-features=Vulkan \
  --window-size=900,700 \
  --remote-debugging-port=9341
```

然后用 CDP（`Target.createTarget` → `Target.attachToTarget {flatten:true}` →
拿 `sessionId`）驱动页面，最后 `Page.captureScreenshot` 存 PNG。注意 Node 23 的
全局 `WebSocket` 用 `addEventListener`，不是 `.on`。

对截图做像素统计，确认 UI 真上屏（而非纯色）：

| 颜色      | 含义                     | 占比（本次验证） |
| --------- | ------------------------ | ---------------- |
| `#1e1e2e` | 页面背景 bgBase          | ~79%             |
| `#313244` | 面板 bgSurface           | ~18%             |
| `#a6e3a1` | 绿色「Count Primes」按钮 | ~0.9%            |
| `#89b4fa` | 蓝色选中 preset / 进度条 | ~0.3%            |
| `#cdd6f4` | 标题 / 文字 textPrimary  | ~0.08%           |

> 别用「把 WebGPU canvas drawImage 到 2D canvas 再 getImageData」来读像素——
> 那对 WebGPU canvas 经常读不到内容（返回透明），会误报 `distinctColors: 1`。
> 直接 `Page.captureScreenshot` 拿 PNG 才靠谱。

---

## 5. 顺带发现的工具链细节

- 本包 `rust-toolchain.toml` 固定 **nightly** + `wasm32-unknown-unknown` + `rust-src`。
  从仓库根目录直接 `cargo build --target wasm32...` 会用根 toolchain（stable）而失败
  （`wasm_thread` 需要 `feature(stdarch_wasm_atomic_wait)`）。**必须 cd 进本目录**再构建，
  让目录级 `rust-toolchain.toml` 生效。
- trunk 在未知路径上也会回退到 `index.html`（SPA fallback），所以访问不存在的
  `/diag.html` 会返回首页 HTML，别被这个误导。
- 本机默认浏览器是 `google-chrome-stable`（有谷歌版），`chromium` 是另一个无谷歌版，
  调试 WebGPU 时用有谷歌的那个。
