# zed `crates/settings_json` 分析 —— git 依赖还是自己实现？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 同一套框架的第五次应用，见 [util-analysis.md](./util-analysis.md)、
> [path-analysis.md](./path-analysis.md)、[util-macros-analysis.md](./util-macros-analysis.md)、
> [collections-analysis.md](./collections-analysis.md)。
>
> **本次的前提变了**：用户明确「一直以功能完整对齐为基础要求」。所以「精简版」
> 不是候选，候选只有「依赖」和「完整抄」。

## 结论

1. **不 git 依赖。** 决定性理由不是它大，是它为了 **1 行代码** 拖进整个
   `crates/util`（详见下节）。
2. **自己实现 = 完整抄 2703 行（含 1942 行测试）。** 功能完整对齐只有这一条路
   —— 这 2703 行是 tree-sitter 驱动的「原地改 JSON 文本但保留注释和格式」，
   原创重写必然在边界情况上偏离上游。
3. **分两阶段落地**：先把 `parse_json_with_comments`（8 行，不需要 tree-sitter）
   补上，它已经是被调用方等着的状态；`editing` 那一半（tree-sitter）作为第二阶段。

## 解剖（2703 行，单文件）

```
src/settings_json.rs   2703 行
├── 15-751    editing 部分（全部 #[cfg(feature = "editing")]）
└── 753-760   parse_json_with_comments（唯一不受 feature 门控的 pub fn）
761-2702      mod tests（1942 行 —— 比实现还多 7 倍）
```

8 个函数，**17 处 `#[cfg(feature = "editing")]`**：

| 函数 | 需要 tree-sitter | 干什么 |
|---|---|---|
| `update_value_in_json_text` | ✅ | 按 key path 递归更新，逐 key 比较，**保留未改动部分的注释与格式** |
| `replace_value_in_json_text` | ✅ | 替换某 key path 的值 |
| `replace_top_level_array_value_in_json_text` | ✅ | 替换顶层数组的某个元素 |
| `append_top_level_array_value_in_json_text` | ✅ | 往顶层数组追加元素 |
| `infer_json_indent_size` | ✅ | 从文本结构推断缩进宽度（默认 2） |
| `to_pretty_json` | ✅ | 带 `indent_prefix_len` 的漂亮打印（缩进前缀用于嵌套插入） |
| `construct_json_value`（private） | ✅ | 序列化时插入缩进前缀 |
| **`parse_json_with_comments`** | ❌ | **8 行**：`serde_json_lenient` + `serde_path_to_error`，容忍注释和尾逗号 |

关掉 `default = ["editing"]` 之后，这个 crate **只剩 8 行代码**。
所以「完整对齐」的成本全部集中在 editing 那一半。

## 决定性发现：`util` 只被用了 1 行

```rust
#[cfg(feature = "editing")]
use util::RangeExt;          // src/settings_json.rs:12
```

全文件搜 `util::` 只有这一处 import；搜 `RangeExt` 的方法调用，**只有一处**：

```rust
// src/settings_json.rs:116
if last_value_range.contains_inclusive(&value_range) {
```

而 `RangeExt::contains_inclusive` 的本体在 zed `crates/util/src/util.rs:916` 是两行：

```rust
fn contains_inclusive(&self, other: &Range<T>) -> bool {
    self.start <= other.start && other.end <= self.end
}
```

**为了这 1 行调用，git 依赖会把整个 `crates/util` 拖进来** —— 而我们在
[util-analysis.md](./util-analysis.md) 已经论证过 util 是「应用层杂物间」：20+ 个
crates.io 依赖 + `nix`/`mach2`/`windows`/`command-fds` 平台件 + 绑定 zed 安装
目录结构。**这是本次否决依赖的最强论据，没有之一。**

（对照：`util` 本身是 Apache-2.0，所以这里的障碍是体积而非许可；真正的许可
障碍在 settings_json 自己身上，见下。）

## 依赖 vs 抄 的真实成本对比

| 项 | git 依赖 | 完整抄 |
|---|---|---|
| `crates/util` 及它的 20+ 依赖 + 平台件 | 全拉进来 | **不需要**（自己写那 2 行） |
| `tree-sitter` + `tree-sitter-json` | 需要 | 需要（一样） |
| `serde_json_lenient` + `serde_path_to_error` | 需要 | 需要（一样） |
| `anyhow` / `serde` / `serde_json` | 需要 | 需要（一样） |
| GPL-3.0-or-later 传染 | 有 | 有（一样，见下） |
| 与上游漂移 | 跟随 rev 自动同步 | 手工同步（但能改） |

**关键观察**：除了 `util`，两边要付的成本**完全相同**。所以「依赖省事」这个论点
在这里不成立 —— 依赖唯一能省的就是那两行 `contains_inclusive`。

## 抄的话要动几处（适配清单）

1. `use util::RangeExt;` → 删掉；`:116` 那行改成内联表达式
   `last_value_range.start <= value_range.start && value_range.end <= value_range.end`
   （或在自己的 util 包里放一个同名 2 行 trait）。
2. `Cargo.toml` 去掉 `util.workspace = true`。
3. 补 4 个依赖：`tree-sitter`（zed 用 git rev `43623ec9`，注意它**不在 crates.io**
   走的是 git）、`tree-sitter-json = "0.24"`、`serde_json_lenient = "0.2"`、
   `serde_path_to_error = "0.1.17"`。
4. `serde_json` 要开 zed 同款 features：`["preserve_order", "raw_value"]`
   —— `preserve_order` 保证对象 key 顺序不被打乱，是这些文本级编辑函数的前提。
5. `dev-dependencies` 补 `unindent = "0.2.0"`（1942 行测试里到处在用）。

## 许可：必须拍板的一件事

`crates/settings_json` 是 **GPL-3.0-or-later**（`Cargo.toml:6`）。依赖也好、抄也好，
只要这份代码进我们的 binary，整个链接产物就受 GPL-3.0 约束。

这与我们已经抄过的 `crates/ui`（GPL）、`crates/component`（GPL）是**同一类问题**，
不是 settings_json 独有的。三个选项：

- **(A) 接受 GPL**：整个产品以 GPL-3.0 发布。`packages/settings_json` 标
  `license = "GPL-3.0-or-later"`,并在文件头加 zed 出处。
- **(B) 只抄非 editing 的 8 行**：`parse_json_with_comments` 是「对
  `serde_json_lenient` 的一行调用」，这种 trivial 的胶水可主张不构成可版权表达；
  editing 部分则原创重写。**代价：不满足「功能完整对齐」。**
- **(C) 完整原创重写**：对齐输入输出契约 + 把 1942 行测试照搬当验收，实现自己写。
  工作量最大，且 tree-sitter 查询逻辑重写后行为一致性风险高。

**按「功能完整对齐是硬要求」这个前提，只有 (A) 成立。** 这需要用户确认，
不是我能替你决定的 —— 而且它会影响整个仓库的 license 字段，不只是这一个包。

## 现状：AAgent 已经建了壳，而且是断的

```
packages/settings_json/
├── Cargo.toml        ← zed 原样（含 util / tree-sitter / serde_json_lenient …）
└── src/lib.rs  14 行 ← cargo new 的 add() 桩，什么都没有
```

- `packages/settings_content/src/project.rs:11` 已经写了
  `use settings_json::parse_json_with_comments;` —— **但 settings_json 里没有这个
  函数**，所以这条链现在是断的；
- AAgent 根 `Cargo.toml` 只声明了 `util = { path = "packages/util" }`，
  **没有** `tree-sitter` / `tree-sitter-json` / `serde_json_lenient` /
  `serde_path_to_error`，所以就算把 zed 源码填进去也编不过；
- zed 那侧的消费者是 `crates/settings`（`pub use settings_json::*;`）+ `settings_content`
  + `migrator`，即 **完整抄才是 zed 的 API 面对齐**。

**最小止血**：先填 `parse_json_with_comments`（8 行）+ 加两个 registry 依赖
（`serde_json_lenient` / `serde_path_to_error`）+ 去掉 `util` 依赖。
不需要 tree-sitter，也不碰 editing。这一步能让 AAgent 那条链先通。

## 决策表

| 场景 | 怎么做 |
|---|---|
| 现在（gpui_learn 侧） | **不引入** —— gpui_learn 的 `theme-settings` 只吃纯 JSON，零需求 |
| AAgent `settings_content` 等着用 | **先填 8 行的 `parse_json_with_comments`**（阶段 1） |
| 要做到 zed settings 的完整 API 面（`pub use settings_json::*`） | **完整抄 2703 行**（阶段 2），含 tree-sitter + 1942 行测试 |
| 许可 | 拍板 (A)/(B)/(C)。前提「功能完整对齐」下只有 (A) |

## 附：五次分析的判据对照

| crate | 依赖闭包 | 通用库? | 结论 |
|---|---|---|---|
| `util` | 20+ 依赖 + 平台件 | 否（应用杂物间） | 不依赖，散函数按需抄 |
| `path` | anyhow + dunce（都在树上） | 是 | 可以依赖，先搁着 |
| `util_macros` | 一个包都不新增 | 否（测试糖 + CI client） | 不依赖，需要时抄 12 行 |
| `collections` | 一个包都不新增（已在编译） | 是 | 依赖，零成本 |
| `settings_json` | **为了 1 行拖进 util** | 是（JSON 文本编辑） | **不依赖；完整抄** |

新出现的一条判据：**「上游内部依赖被实际用到的行数」**。
`settings_json` 对 `util` 的耦合是 1 行 —— 这种「名义上依赖、实际上擦边」的情况，
单独看 Cargo.toml 会误判，必须 grep 实际使用点才知道能不能一刀切断。
