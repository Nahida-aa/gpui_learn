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
│   ├── 01_hello_world/     # 第一个例子：纯 GPUI 最小窗口（无封装，桌面）
│   ├── 02_hello_web/       # 第二个例子：GPUI 编译成 WASM 在浏览器跑（trunk）
│   ├── 03_hello_android/   # 第三个例子：自有 gpui-android 后端原生跑在 Android
│   ├── 04_input/           # 第四个例子：文本输入（IME / 选区 / 剪贴板 / 键盘布局），移植自官方 input.rs
│   ├── 05_android_input/   # 第五个例子：用 gpui-android 后端在手机跑文本输入（复用 04 逻辑 + 软键盘）
│   ├── 05_grid_layout/     # 第六个例子：CSS Grid 圣杯布局 + container_query 响应式（桌面）
│   ├── 06_text_area/       # 文本框 / 多行文本编辑（桌面 + Android）
│   ├── 07_uniform_list/    # 虚拟化列表（UniformList）
│   ├── 08_testing/         # GPUI 测试框架（#[gpui::test] / TestAppContext）
│   ├── _09_a11y/           # 无障碍（AccessKit）演示（桌面 / WASM / Android 同构）
│   └── 09_slider/          # Slider 组件
├── packages/               # 库 crate（被 apps 共享的内部包）
│   ├── assets/             # 内嵌资源（字体/图标），供各 app 引用
│   ├── gpui-android/       # vendored 的 Android 平台层（对接本仓库 GPUI 82aef443）
│   ├── gpui-cli/           # 开发工具：android init 等，配置驱动生成 Android 工程
│   └── ui-gpui/            # 组件库（后续例子的「共享库」演示）
├── justfile                # 常用命令快捷方式
└── README.md               # 本文件
```

每个包目录下都有自己的 `README.md`，讲解该包「是什么、为什么这么设计」。
所有 GPUI 知识点都写在源码的**文档注释**里（`src/*.rs` 的 `//!` / `///`）——
代码即文档，读源码就能学。

## Rust 版本与 workspace

- `edition = "2024"`，`resolver = "3"`（写在根 `Cargo.toml`）。
- GPUI 通过 **git 源**引入（锁 `rev`）；共享库与平台层见 `packages/ui-gpui/README.md`。
- 本仓库的「移动端」有两个路线：
  - **浏览器路线**：`apps/02_hello_web` 把 GPUI 编译成 WASM，在移动端浏览器运行
    （需注意可信源/HTTPS，见其 `TROUBLESHOOTING.md`）。
  - **原生路线**：`apps/03_hello_android` + `packages/gpui-android` 用 vendored 进来的
    Android 平台层，在 Vulkan/wgpu 上**原生渲染**（不经过浏览器），对接本仓库自己的
    GPUI `82aef443`。维护方式见 `docs/maintain-gpui-android.md`。

## 常用命令

```bash
cargo run -p hello_world_01      # 运行第一个例子（包名，目录名是 01_hello_world）
cargo build --workspace          # 构建全部（含库）
cargo build                      # 只构建默认成员（apps/*）
just run hello_world_01          # justfile 提供的等价快捷命令
```

> 包名 vs 目录名：Cargo 包名不能以数字开头，所以目录用 `01_hello_world`
> 体现学习顺序，包名则为 `hello_world_01`。运行/构建时一律用包名。

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
> 等例子变多、样板开始重复时，再引入 `packages/ui-gpui` 演示
> 「monorepo 如何用内部共享库收敛重复」——此时共享包的概念才自然出场。
>
> 后续会逐步加入：绘制图形、文本输入、布局（flex/taffy）、状态管理、
> 列表/组件化、以及 `gpui_web` 的 HTML/WASM 编译等。

## 给贡献者 / 学习者

想加新例子？在 `apps/` 下新建一个目录（建议 `NN_topic` 形式以排序），
包名用 `topic_nn` 这类合法名，依赖 `gpui`（必要时 `gpui_platform`），
把讲解写进源码文档注释和包内 `README.md` 即可。workspace 的 `members`
用通配符自动收纳，无需改根 `Cargo.toml`。

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
