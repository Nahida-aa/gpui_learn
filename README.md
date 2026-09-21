# gpui_learn

一个用 **Rust monorepo** 的方式学习 [GPUI](https://gpui.rs) 的仓库。
目标有二：

1. **循序渐进学 GPUI** —— 每个 `apps/*` 是一个独立可运行的小例子。
2. **顺带学 Rust monorepo 工程实践** —— workspace 成员、默认成员、二进制 vs 库、
   内部共享包（path 依赖）等概念，都直接体现在仓库结构里。

## 仓库结构

```
gpui_learn/
├── Cargo.toml              # 工作区根：members / default-members / 统一依赖
├── apps/                   # 二进制 crate（每个可 `cargo run`，是学习例子）
│   │                       # 两条线，用前缀区分（详见 apps/README.md）
│   ├── _01…_45/            # 官方线：一一对应 zed `crates/gpui/examples/`
│   │                       #   编号即官方索引；_03 / _05 空缺（对应的是自研
│   │                       #   Android 移植，已归入 ug_ 线）
│   ├── ug_01_hello_android/# 自研线（ug_ = aa_gpui_kit_ui）：Android 平台的 hello world
│   ├── ug_02_android_input/# 自研线：Android 软键盘输入
│   ├── ug_03_slider/       # 自研线：aa_gpui_kit_ui Slider 组件
│   ├── ug_04_input_button/ # 自研线：aa_gpui_kit_ui Input（单行）+ Button
│   └── ug_05_editor/       # 自研线：aa_gpui_kit_ui Editor（多行编辑器）
├── packages/               # 库 crate（被 apps 共享的内部包）
│   ├── assets/             # 内嵌资源（字体/图标），供各 app 引用
│   ├── gpui-android/       # vendored 的 Android 平台层（对接本仓库 GPUI 82aef443）
│   ├── gpui-cli/           # 开发工具：android init 等，配置驱动生成 Android 工程
│   └── ui/                 # 组件库（aa_gpui_kit_ui）（后续例子的「共享库」演示）
├── justfile                # 常用命令快捷方式
└── README.md               # 本文件
```

每个包目录下都有自己的 `README.md`，讲解该包「是什么、为什么这么设计」。
所有 GPUI 知识点都写在源码的**文档注释**里（`src/*.rs` 的 `//!` / `///`）——
代码即文档，读源码就能学。

## Rust 版本与 workspace

- `edition = "2024"`，`resolver = "3"`（写在根 `Cargo.toml`）。
- GPUI 通过 **git 源**引入（锁 `rev`）；共享库与平台层见 `packages/ui/README.md`。
- 本仓库的「移动端」有两个路线：
  - **浏览器路线**：`apps/02_hello_web` 把 GPUI 编译成 WASM，在移动端浏览器运行
    （需注意可信源/HTTPS，见其 `TROUBLESHOOTING.md`）。
  - **原生路线**：`apps/03_hello_android` + `packages/gpui-android` 用 vendored 进来的
    Android 平台层，在 Vulkan/wgpu 上**原生渲染**（不经过浏览器），对接本仓库自己的
    GPUI `82aef443`。维护方式见 `docs/maintain-gpui-android.md`。

## 常用命令

```bash
cargo run -p _01_hello_world     # 运行某个例子（包名 == 目录名）
cargo run -- android init        # 裸 cargo run 走 gpui-cli（需带子命令）
cargo build                      # 只构建默认成员（packages/gpui-cli，秒完）
cargo build --workspace          # 构建全部（48 个示例 + 库）
just test                        # 跑 aa_gpui_kit_ui 库测试（engine / editor 单测）
just run _01_hello_world         # justfile 提供的等价快捷命令
```

> **包名 == 目录名**：`_01_hello_world` / `ug_05_editor` 既是目录名也是 Cargo 包名
> （Cargo 允许下划线开头的包名），所以 `-p` 后面直接写目录名即可，无需记忆两套名字。
>
> **默认成员只有 `gpui-cli`**：`apps/` 现有 48 个示例包，全量默认构建很慢，而
> `gpui-cli` 不依赖 GPUI 运行时。裸 `cargo build`/`cargo run` 因此很快；
> 注意裸 `cargo test` 也只测 gpui-cli，库测试请用 `just test`、
> 全量用 `cargo test --workspace`。

## apps 编号规则

- `_01`–`_45`：官方线，编号即 zed `crates/gpui/examples/` 的索引（`_03`/`_05`
  空缺，那两个是自研 Android 移植，已归入 `ug_` 线）；
- `ug_01`–：自研线（`ug_` = aa_gpui_kit_ui），编号独立增长；
- 号位一经分配即保留，未实现的留空，不挪作他用。

## 学习路线（例子索引）

| 例子                    | 主题                                                                             |
| ----------------------- | -------------------------------------------------------------------------------- |
| `apps/01_hello_world`   | 纯 GPUI 最小窗口、`Render` trait、程序入口                                       |
| `apps/02_hello_web`     | GPUI 编译成 WASM，trunk 构建，浏览器/手机运行                                    |
| `apps/03_hello_android` | 自有 `gpui-android` 后端，Android 原生渲染（Vulkan/wgpu）                        |
| `apps/04_input`         | 文本输入（IME / 选区 / 剪贴板 / 键盘布局），移植自官方 `input.rs`                |
| `apps/05_android_input` | 用 `gpui-android` 后端在手机跑文本输入（复用 04 逻辑 + 软键盘；零 kt，配置驱动） |
| `apps/05_grid_layout`   | CSS Grid 圣杯布局 + `container_query` 响应式（桌面，移植自官方例子）             |

> 教学顺序的设计：第一个例子**故意不用任何共享库**，让学习者先看 GPUI 原貌。
> 等例子变多、样板开始重复时，再引入 `packages/ui` 演示
> 「monorepo 如何用内部共享库收敛重复」——此时共享包的概念才自然出场。
>
> 后续会逐步加入：绘制图形、文本输入、布局（flex/taffy）、状态管理、
> 列表/组件化、以及 `gpui_web` 的 HTML/WASM 编译等。

## 给贡献者 / 学习者

想加新例子？在 `apps/` 下新建一个目录（建议 `NN_topic` 形式以排序），
包名用 `topic_nn` 这类合法名，依赖 `gpui`（必要时 `gpui_platform`），
把讲解写进源码文档注释和包内 `README.md` 即可。workspace 的 `members`
用通配符自动收纳，无需改根 `Cargo.toml`。

## 许可证（两套，默认 GPL）

与上游 zed 同款的分法：

| 范围 | 许可证 | 说明 |
|---|---|---|
| **默认**（绝大多数包） | **GPL-3.0-or-later** | 这些包含照搬 / 改写自 zed 的代码（`ui`、`theme`、`theme-settings`、`component`、`assets`、`ui_input`、`ui_macros`、`base`、`editor`、`gpui_fuzzy`、`clock`…），上游是 GPL-3.0-or-later，因此必须同许可 |
| `packages/gpui-cli` | Apache-2.0 | 构建/开发工具，零 zed 代码 |
| `packages/gpui-android` | Apache-2.0 | Android 平台适配，只依赖 zed 的 Apache-2.0 侧（`gpui` / `gpui_wgpu`） |

默认值写在根 `Cargo.toml` 的 `[workspace.package].license`，Apache 的两个包在各自
`Cargo.toml` 里显式覆盖。许可证全文：根目录 `LICENSE-GPL` / `LICENSE-APACHE`。

> **下游注意**：任何链接了默认（GPL）包的二进制，分发时整体必须是
> GPL-3.0-or-later —— 这是 copyleft 的传染性，与「只是当依赖用」无关。

## 扩展阅读（`docs/`）

对 `gpui_learn` 之外的 GPUI 生态做代码级调研的笔记，均基于实际仓库阅读：

- [docs/mobile-backends.md](docs/mobile-backends.md) — 移动端 GPUI 怎么落地：
  对比 `gpui-mobile` 与 `gpui-toolkit` 的 iOS/Android 平台层，以及它们为何
  从根上不存在 Web 端的 `app was released` 问题。
- [docs/scaffolder.md](docs/scaffolder.md) — `gpui-scaffolder` CLI：如何一键
  生成跨桌面/iOS/Android 的 GPUI mini-app 骨架。
- [docs/ui-kit.md](docs/ui-kit.md) — `gpui-ui-kit` 组件库：~80 个成品组件、
  声明式 builder API（与 `02` 手写 `div` 同范式）、主题/设计系统机制。
- [docs/charts.md](docs/charts.md) — `gpui-d3rs` / `gpui-px`：D3 风格可视化原语
  与 Plotly Express 风格图表 API（scatter/line/bar/heatmap/3D…）。
- [docs/maintain-gpui-android.md](docs/maintain-gpui-android.md) — 如何把社区
  `gpui-toolkit` 的 Android 后端 **vendor 进本仓库并对接自有 GPUI 版本**，以及
  `apps/03_hello_android` 怎么用它跑起来。

> `docs/` 里除了 `maintain-gpui-android.md` 之外的笔记，涉及的 crate 锁在 zed
> `v1.9.0`，与本仓库（zed `82aef443`）不兼容，不能直接作为依赖并入 workspace；
> 笔记仅作「GPUI 能长到什么程度」的参考标杆。只有 `gpui-android` 被我们主动
> vendor 并适配到了 `82aef443`（见 `packages/gpui-android` 与 `maintain-gpui-android.md`）。
