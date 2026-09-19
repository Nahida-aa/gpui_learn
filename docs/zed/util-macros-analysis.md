# zed `crates/util_macros` 依赖分析 —— git 依赖还是自己写？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 与 [util-analysis.md](./util-analysis.md)、[path-analysis.md](./path-analysis.md)
> 是同一套判断框架的第三次应用。

## 结论

1. **不 git 依赖。** 不是因为闭包大（实测下来其实挺小），而是因为它给的东西我们
   **一个都用不上**：3 个宏是「跨平台测试字面量助手」，第 4 个是 zed 自己的 CI
   性能回归 harness 的一半。
2. **也不要自己写当前版本。** 我们是在 Linux 上开发、Linux 上跑测试，`path!` /
   `uri!` / `line_endings!` 在这三个宏在这里是**恒等变换**，抄过来也只会得到恒等函数。
3. **真需要时：抄那一个宏（约 15 行），不要拉整个 crate。** Apache-2.0 允许抄。
   落点是已有的 `packages/ui_macros`（对齐 zed 命名的话再建 `packages/util_macros`，
   但同样「不预建空壳」）。

## 解剖（单文件 286 行）

`[lib] path = "src/util_macros.rs"`,`proc-macro = true`,`doctest = false`。

四个 proc-macro，性质分成**完全无关的两组**：

| 宏 | 行数 | 干什么 | 我们要吗 |
|---|---|---|---|
| `path!(lit)` | ~15 | Windows 上把 `/`→`\\` 并给绝对路径补 `C:` | 否 |
| `uri!(lit)` | ~12 | Windows 上把 `file:///`→`file:///C:/` | 否 |
| `line_endings!(lit)` | ~12 | Windows 上把 `\n`→`\r\n` | 否（除非开始写跨平台编辑行为测试） |
| `#[perf(...)]` | ~100 | 把 `#[test]` 改写成「读 env var 决定循环 N 次 + 额外生成一个往 stdout 打元数据的假 test」 | 否 —— 缺 host 侧就退化成 `#[test]` |

前三个的**全部实现**就是把字符串取出来、Windows 下 `replace` 一下、再 `quote!` 回去。
它们的存在意义只有一个：**让同一份测试源码能在 Windows CI 上通过**。

第四个 `#[perf]` 是 `tooling/perf`（zed 的性能回归工具）的**client 半边**：
- 生成的代码读 `std::env::var(ITER_ENV_VAR)` 决定循环次数；
- 再生成一个 `_mdata` 后缀的假 `#[test]`,往 stdout 打带 `MDATA_LINE_PREF` 前缀的
  元数据行供 `tooling/perf` 的 binary 解析;
- 靠 feature `perf-enabled` 开关，没开就退化成普通 `#[test]`。

—— **没有 zed 的 `tooling/perf` harness + CI 约定，`#[perf]` == `#[test]`。**

## 实测（闭包到底多大）

`/tmp` 探针，`util_macros = { git = …, rev = f6838a7c }`（用 `package =` 改名避免
`util_macros::` 太泛，实测改名可用）：

```console
     Locking 41 packages to latest compatible versions
   …
   Compiling gpui_util v0.1.0 (zed?rev=f6838a7c)
   Compiling collections v0.1.0 (zed?rev=f6838a7c)
   Compiling perf v0.1.0 (zed?rev=f6838a7c)
   Compiling util_macros v0.1.0 (zed?rev=f6838a7c)
    Finished `dev` profile in 16.15s
```

**看起来很吓人（41 包 / 16 秒），实际边际成本很小 —— 逐个核对我们自己的 Cargo.lock：**

| 闭包成员 | 我们树上已有？ |
|---|---|
| `serde` `serde_json` `indexmap` `rustc-hash` `quote` `syn` | ✅ 全在（`Cargo.lock` 里分别位于 6009 / 6069 / 3608 / 5797 / 5397 / 6563 行） |
| `collections` `gpui_util` @ `f6838a7c` | ✅ 已在（随 gpui 进来，`Cargo.lock:1456` / `:3106`） |
| `perf` | ✅ 已编译（见下） |
| `util_macros` | ✅ 已编译（见下） |

**更正（写完本文后用 `cargo tree` 复核发现）**：`perf` 与 `util_macros` 也**已经在
我们的编译图里** —— `gpui` 自己就依赖 `util_macros`（proc-macro），后者拉进 `perf`:

```console
$ cargo tree -p aa_gpui_kit_ui -i util_macros -i perf -e normal --target x86_64-unknown-linux-gnu
perf v0.1.0 (zed?rev=f6838a7c#f6838a7c)
└── util_macros v0.1.0 (proc-macro) (zed?rev=f6838a7c#f6838a7c)
    └── gpui v0.2.2 (zed?rev=f6838a7c#f6838a7c)
        ├── aa_gpui_kit_assets … ├── aa_gpui_kit_component … ├── aa_gpui_kit_theme …
        ├── aa_gpui_kit_ui … └── editor …
```

即：**git 依赖 `util_macros` 的边际成本是 0 个新包**（41 包 / 16 秒那个数字是空
目录探针的冷启动，不是我们仓库的增量）。

—— 这反而让结论更干脆：**否决它的理由从头到尾就不是成本，是「货不对板」**。
四样东西我们一样用得上（Linux 上前三个是恒等函数、第四个退化成 `#[test]`），
成本再低也不该引进来。

## 决策表

| 场景 | 怎么做 | 理由 |
|---|---|---|
| 现在 | **什么都不做** | 4 个宏全部无消费方；全仓无人写跨平台路径测试，也无人跑 zed 的 perf harness |
| 将来 editor 测试里要断言 CRLF 行为 | **抄 `line_endings!` 一个宏**（~12 行）到 `ui_macros` | 12 行的宏不值得让自家 crate 多一个 git 依赖位点；且原版 `cfg(target_os)` 判的是 host（见下方陷阱），抄时可以顺手改对 |
| 将来要引入 zed 的 `#[perf]` | **整 crate git 依赖**（并同步把 `tooling/perf` 的 harness 也要过来） | 只拿 client 半边没意义，宏会退化成 `#[test]` |
| 只是想要某个宏的「跨平台」能力 | 直接写 `#[cfg(target_os = "windows")]` 的普通函数 | proc-macro 不是必需的，见下方「陷阱」 |

## 陷阱：proc-macro 里的 `cfg(target_os)` 判断的是 **host**,不是 target

```rust
#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    let mut path = path.value();
    #[cfg(target_os = "windows")]      // ← 这里
    { path = path.replace("/", "\\"); … }
```

proc-macro crate 是为 **host** 编译的，`target_os` 在它里面等于**编译它的那台机器**。
所以在 Linux 上 `cargo build --target x86_64-pc-windows-msvc` 时，`path!` **不会**做替换
—— 交叉编译场景下这个宏是错的（zed 自己不 care，它不交叉编译）。

推论两点，对我们有用：

1. 这类「按平台改字面量」的需求，**普通 `const fn` / `#[cfg]` 函数就能做且更正确**，
   不该迷信 proc-macro；
2. 如果哪天真抄，要把 `cfg` 挪出 proc-macro（在生成的代码里判断 target），而不是照抄。

## `packages/util_macros` 的启动条件（沿用 util 的同一条规则）

**触发信号**（满足其一才建，且出生时就要有内容）：

- 第 2 个以上的 crate 需要同一批「测试用字面量助手」（重复出现即该下沉）；
- 开始写 **Windows CI**，且 editor/fs 测试里有大量绝对路径 / URI / CRLF 断言；
- 需要写我们自己的第二个/第三个 proc-macro，且 `packages/ui_macros` 的定位装不下
  （`ui_macros` 现在只放 UI 相关的宏，见它的 Cargo.toml）。

**建成形态**：包名 `aa_gpui_kit_util_macros`（下划线，全仓约定）；依赖上限
`quote` + `syn` + `proc-macro2`；**禁止引入 `perf` 之类的运行时 crate** ——
一旦 proc-macro crate 需要运行时依赖，说明它装错了东西。

**为什么现在不建**：零需求。空包会腐化成杂物抽屉（这条在 util-analysis.md 里已经
论证过一次，`packages/fs` 的空壳就是反面例子）。

## 附：和 `path` 的结论为什么相反

同一个 zed 仓库、同一个 rev、邻居 crate，结论一个「可以依赖」一个「不要依赖」，
差别在三个问题上：

| 判据 | `crates/path` | `crates/util_macros` |
|---|---|---|
| 依赖闭包 | anyhow + dunce（且 dunce 已在树上） | 一个包都不新增（`perf` / `util_macros` 已随 gpui 编译） |
| 是不是通用库 | 是 —— 纯粹的路径不变量 | 否 —— 一半是 zed CI 的 client，一半是测试糖 |
| 我们现在用得上吗 | 用不上（所以也先不引） | 用不上，且未来也只有 12 行的量 |

`path` 是「纯度通过、暂不需要」→ 结论 1 可以依赖但先搁着；
`util_macros` 是「纯度勉强通过、需要的部分只有 12 行」→ 结论直接是抄。
