# zed `crates/path` 依赖分析 —— git 依赖还是自己写？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 与 [util-analysis.md](./util-analysis.md) 是同一套判断框架的第二次应用。

## 结论

1. **可以 git 依赖，而且是「教科书级」的自足 crate** —— 实测独立编译通过，
   闭包只有 `anyhow` + `dunce`（`serde` 是 optional）。与 `sum_tree` /
   `refineable` 同一档，**比 `crates/util` 干净得多**。
2. **但现在不要引入** —— 我们仓库里零消费方（`rg rel_path|RelPath|abs_path`
   全仓无命中）。等出现真实需求时再加一行依赖，成本为零。
3. **不要自己实现** —— 1967 行里全是路径不变量的坑（`..` 归一化、Windows
   分隔符、`\\?\` verbatim 前缀、UTF-8 校验、join 时 `..` 溢出到父目录），自己趟一遍
   就是用我们的 bug 换 zed 的 bug。真想要「自己的」，也应该直接**抄**
   （Apache-2.0 允许），而不是重写。
4. **依赖时改名**：用 `package = "path"` 改成 `zed_path`，避免 `path::RelPath`
   这种读起来像 std 的名字。

## 解剖（3 个文件，1967 行）

| 文件 | 行数 | 内容 |
|---|---|---|
| `src/path.rs` | 858 | `PathStyle`（Unix/Windows 分隔符风格）、`PathExt::to_rel_path_buf`、路径比较/遍历辅助 |
| `src/rel_path.rs` | 823 | `RelPath` / `RelPathBuf`：内部**恒为 POSIX `/` 分隔**、必相对于根、已归一化（无 `.` / `..`）、必为合法 unicode |
| `src/abs_path.rs` | 286 | `AbsPath` / `AbsPathBuf`：绝对 + 归一化 + unicode，带 `dunce` 处理 Windows verbatim |

**Makefile 要点**：`[lib] path = "src/path.rs"` —— lib 根不是 `lib.rs`，且三个文件
全部手写 `mod` 声明关系（`path.rs` 里 `pub mod abs_path; pub mod rel_path;`）。
这是 zed 的老式组织，不是我们那种 `mod.rs` 风格；git 依赖的话不影响使用。

**零 zed 内部 imports**（决定性证据）：

```console
$ rg -n "^(use|pub use|pub mod|mod)\b" crates/path/src/*.rs
path.rs:9:     use std::{...}
path.rs:14:    use crate::rel_path::RelPath;
abs_path.rs:1  use std::{...};  use anyhow::Context;  use crate::{PathStyle, rel_path::RelPath};
rel_path.rs:1  use anyhow::{Context as _, Result, anyhow};  use std::{...};  use crate::{...}
```

除 `crate::` 外没有任何 zed crate。`[dev-dependencies]` 只有 `tempfile` /
`pretty_assertions`（git 依赖不会编译 dev-deps）。`[lints] workspace = true`
也不构成问题（checkout 里带 workspace）。

## 实测（这是本次唯一真正的增量）

在 `/tmp/path_probe` 里写了个 1 行 main + 一行依赖做了实拉实编译：

```toml
[dependencies]
path = { git = "https://github.com/zed-industries/zed", rev = "f6838a7c4afecd7c212c43c70165588b92a5a11f" }
```

```console
    Updating git repository `https://github.com/zed-industries/zed`
     Locking 3 packages to latest compatible versions
   Compiling anyhow v1.0.104
    Checking dunce v1.0.5
    Checking path v0.1.0 (zed?rev=f6838a7c#f6838a7c)
    Checking path_probe v0.0.0 (/tmp/path_probe)
    Finished `dev` profile in 1.01s
```

**只进来 3 个包，1 秒完成**。两条推论：

- **`dunce` 不需要新增下载**：`Cargo.lock` 里已经有 `dunce v1.0.5`
  （被 `gpui_windows` 引入），版本正好一致 → 编译图零增长。
- **不需要新 clone**：repo + rev 与 `[patch.crates-io]` 的 gpui 完全相同，
  cargo 复用同一个 checkout（已在 `~/.cargo/git/checkouts/zed-a70e2ad075855582/f6838a7`）。

对照 `crates/util`（20+ 依赖、绑定 zed 安装结构、`publish = false`、拖拽
`nix`/`async_zip`/`globset` 等平台件）—— 两者位于谱系的两端。

## 决策表

| 场景 | 怎么做 | 理由 |
|---|---|---|
| 现在（无消费方） | **什么都不做** | 引入无用依赖纯属负债，`packages/path` 空壳同理不预建 |
| 将来做文件树 / project panel / LSP worktree 相对路径 | git 依赖 **`zed_path`**（`package = "path"`，rev 与 gpui 对齐） | 实测可行、闭包括 0 新增、不变量 hard-won |
| 只需要「UTF-8 路径」这一个保证 | 用 crates.io 的 **`camino`**（`Utf8PathBuf`）| 比 zed path 职责更窄，且是发布包，不占 git 依赖位点 |
| 只想抄一两个函数（如 `path_compare`） | **抄**（Apache-2.0，加出处注释） | 和 util 散函数同一条先例 |
| 哪天 zed 把它发布到 crates.io | 换 registry 依赖，去掉 rev | 目前 `publish.workspace = true` = zed workspace 的 `publish = false`，所以只能走 git |

## 为什么这次敢依赖（对照 `crates/component` 那次）

不重复犯上次的分析错：那次是「看着小、实际不自足」，拖出 GPL + 巨大的
`zed::theme` 闭包。这次三个硬指标都过了：

1. **闭包极小**（anyhow + dunce，且 dunce 已在树上）；
2. **无 zed 应用语义**（Closured std 类型里的通用路径不变量）；
3. **许可干净**（Apache-2.0，与 GPL 的 `ui` / `component` / `ui_macros` 不同 ——
   真不能依赖时，**抄也是合规的**）。

## 附：名称冲突提醒

crates.io 上存在一个同名占位 crate `path`。我们的依赖是 git 源，不走 registry，
一般不会冲突；但一旦某个传递依赖从 crates.io 拉 `path`，cargo 会报
"multiple packages named `path`"。用 `package = "path"` 重命名成 `zed_path`
可以同时规避冲突和语义歧义 —— **这就是结论 4 的来源**。

## 附：如果哪天真要自己实现，最小集合是什么

不要一次性抄 1967 行。先只做 editor / file-tree 真正踩到的不变量：

1. `RelPath`：内部 `/` 分隔 + 拒绝绝对 + 拒绝 `.` / `..`（`AbsPath` 一样）；
2. `join` 溢出。清理 `..` 时小心 `a/../..` 不能清成负深度，且要保留前导 `..`；
3. Windows：`\\` 与 `/` 都算分隔符，`\\?\` verbatim 前缀要 `dunce::simplified` 处理；
4. `to_string_lossy()` 一律替换：转 unicode 失败要 `anyhow` 报错而不是替 `U+FFFD`。

—— 抄完之后你会发现：这就是 zed `path` crate。所以结论回到 1：**不要自己写**。
