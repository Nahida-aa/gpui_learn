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
│   └── 01_hello_world/     # 第一个例子：纯 GPUI 最小窗口（无封装）
├── crates/                 # 库 crate（被 apps 共享的内部包）
│   └── gpui_learn_common/  # 后续例子的「共享库」演示（暂未被引用）
├── justfile                # 常用命令快捷方式
└── README.md               # 本文件
```

每个包目录下都有自己的 `README.md`，讲解该包「是什么、为什么这么设计」。
所有 GPUI 知识点都写在源码的**文档注释**里（`src/*.rs` 的 `//!` / `///`）——
代码即文档，读源码就能学。

## Rust 版本与 workspace

- `edition = "2024"`，`resolver = "3"`（写在根 `Cargo.toml`）。
- GPUI 通过 **git 源**引入（锁 `rev`），详见 `crates/gpui_learn_common/README.md`。
- 目前 zed/gpui 没有原生 iOS/Android 后端；「移动端」的现实路径是
  `gpui_web` 编译成 WASM 在移动端浏览器运行，后续例子会演示。

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

| 例子                  | 主题                                       |
| --------------------- | ------------------------------------------ |
| `apps/01_hello_world` | 纯 GPUI 最小窗口、`Render` trait、程序入口 |

> 教学顺序的设计：第一个例子**故意不用任何共享库**，让学习者先看 GPUI 原貌。
> 等例子变多、样板开始重复时，再引入 `crates/gpui_learn_common` 演示
> 「monorepo 如何用内部共享库收敛重复」——此时共享包的概念才自然出场。
>
> 后续会逐步加入：绘制图形、文本输入、布局（flex/taffy）、状态管理、
> 列表/组件化、以及 `gpui_web` 的 HTML/WASM 编译等。

## 给贡献者 / 学习者

想加新例子？在 `apps/` 下新建一个目录（建议 `NN_topic` 形式以排序），
包名用 `topic_nn` 这类合法名，依赖 `gpui`（必要时 `gpui_platform`），
把讲解写进源码文档注释和包内 `README.md` 即可。workspace 的 `members`
用通配符自动收纳，无需改根 `Cargo.toml`。
