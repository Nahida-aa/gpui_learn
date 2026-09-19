# zed `crates/util` 依赖分析 —— git 依赖还是自己写？

> 背景：editor 内核选型时（见 `docs/editor-roadmap.md` §3.3）判断过"rope 硬依赖 util
> 所以自研"。本篇把 util 整体分析一遍，作为「以后遇要不要 GitHub 依赖 zed crate」
> 的通用判断记录。
>
> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。

## 结论

1. **`crates/util` 不直接 git 依赖** —— 它是 zed 的应用层杂物间，不是通用库。
2. **通用小件的真实出处是 `gpui_util`** —— zed 已经亲手把 util 拆过一次，拆出的
   `gpui_util`（依赖仅 log + anyhow）**已随 gpui 进了我们的依赖树**，需要
   `ResultExt` / `debug_panic` / `measure` 时直接声明依赖即可，成本≈0。
3. **散函数（各 10-30 行的独立纯函数）按需抄** —— 这是我们 rope 选型时的既有先例。
4. **`packages/util` 不预建空壳** —— 等第一个真实需求出现再建（启动条件见下文）。

## zed util 的解剖（8555 行）

| 模块 | 行数 | 性质 |
|---|---|---|
| `paths.rs` | 3291 | **zed 专属**：`data_dir` / `support_dir` 等目录约定绑定 zed 的安装结构 |
| `shell.rs` + `shell_builder` + `shell_env` | ~1700 | **应用级**：登录 shell 环境加载、zed 的 shell 集成协议 |
| `util.rs`（lib 本体） | 1502 | 混合：`get_zed_cli_path` / `load_login_shell_environment` /
    `prevent_root_execution` 等是 zed 专属；`merge_json_value_into` 系列、
    `truncate` 系列、`extend_sorted` 是通用散函数 |
| `markdown.rs` / `redact.rs` / `archive.rs` | ~1200 | zed 遥测脱敏 / 扩展下载专用 |
| `process.rs` / `command.rs` / `fs.rs` | ~600 | zed 进程管理（依赖 `nix` / `command-fds` 平台件） |
| size / time / serde / test 等 | ~400 | 小杂件 |

**依赖闭包**：20+ 个 crates.io 依赖（`async_zip` / `globset` / `regex` /
`schemars` / `rust-embed` / `serde_json_lenient` / `tempfile` / `url`…）
+ zed 内部 4 个（`collections` / `path` / `gpui_util` / `util_macros`）
+ 平台件（`nix` / `mach2` / `windows` / `command-fds`）。且 `publish = false`。

## 关键事实（决定结论的三条）

1. **zed 正在拆 util**。通用件已搬去 `gpui_util`（`crates/gpui_util`，依赖只有
   log + anyhow，~1100 行：`ResultExt` / `log_err` / `debug_panic` /
   `measure` / `maybe!` / `Deferred` / `ArcCow` / `post_inc` /
   `TryFutureExt` / `truncate_to_bottom_n_sorted_by`）。util 只留下应用层杂物。
   —— zed 的意图：能被 gpui 生态复用的东西不进 util。

2. **`gpui_util` 已在我们的依赖树里**（随 gpui 进来）：

   ```console
   $ cargo tree | grep gpui_util
   gpui_util v0.1.0 (zed@f6838a7c)   # 随 gpui 白拿，已在编译
   ```

   要用通用件，加一行 git 依赖即可，边际编译成本为零。

3. **`sum_tree` 不依赖 util**（该 rev 已清理，只依赖 heapless/rayon/log/ztracing）；
   **`rope` 也不依赖 util**（用 ztracing/log）。即我们已在用的 zed 底层数据结构
   crate 都不经过 util —— util 在 zed 的依赖图里位于**应用层**（editor、zed 主
   crate 等），不在基础设施层。

## 依赖决策

| 需要什么 | 怎么办 | 理由 |
|---|---|---|
| `ResultExt` / `debug_panic` / `measure` / `ArcCow` / `Deferred` | git 依赖 **gpui_util**（rev 与 gpui 对齐） | 已在依赖树；闭包 log+anyhow；与 zed 同源不漂移 |
| `merge_json_value_into` 系列 / `truncate` 系列 / `extend_sorted` 等散函数 | **抄**（各 10-30 行独立纯函数，抄进自家的 util 包或就地实现） | 拉整个 util 换 100 行代码不值；先例：rope 选型 |
| `paths.rs` / shell 集成 / 进程管理 / archive | **不碰** | 绑定 zed 应用约定；我们用不到 |

与既有决策同构：`sum_tree`、`refineable` 我们敢 git 依赖，是因为它们是
**自足的底层组件**（依赖少、无 zed 应用语义）；util 是应用杂物间，方向相反。

## `packages/util` 的启动条件（什么时候建、建成什么样）

**触发信号**（满足其一就建）：
- 第 2 个以上的 crate 需要同一批"抄来的散函数"（重复出现即该下沉）；
- editor 内核开始需要 `debug_panic` / `log_err` 风格的日志辅助且 gpui_util
  不适合（例如想要零依赖的教学实现）；
- 需要写第一个我们自己的宏（对齐 `util_macros` 的角色）。

**建成形态**（到时执行 `cargo new packages/util --lib` 后按此约束）：
- 包名 `util`（内部包，无 `aa-gpui-kit-` 前缀，与 zed 同名同位）；
- **依赖上限：`anyhow` + `log`**（对齐 gpui_util 的边界）—— 一旦想加第三个
  依赖，先问"这真的是 util 吗"；
- 只装**真实被用到**的东西，每个条目带测试与出处注释（"抄自 zed util.rs:xxx"
  或"自研"）；
- 禁止收编任何绑定 zed 语义的东西（目录约定 / shell 集成 / 进程管理）。

**为什么现在不建**：当前零需求（rope 已不依赖 util，我们自研 rope 完全自足，
仓库里没有任何"抄来的散函数"）。zed 的 gpui_util 也不是预建的空包，是从 util
里"长"出来的 —— 空包会腐化成杂物抽屉，出生时就有内容才立得住。

## 附：顺带发现的依赖树问题

`packages/gpui-android/Cargo.toml` 锁旧 rev `d88f682`，与根 `[patch.crates-io]`
的 `f6838a7` 并存 → 依赖树里同时编译**两份** `gpui` / `gpui_util` /
`util_macros` / `collections`。后果：编译时间翻倍、两套宏可能不兼容。
gpui-android 跟上主 rev 后即消除；vendored 后端有意锁旧 rev，暂不动。
