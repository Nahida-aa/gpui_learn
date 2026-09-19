# zed `crates/collections` 依赖分析 —— git 依赖还是自己写？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 与 [util-analysis.md](./util-analysis.md)、[path-analysis.md](./path-analysis.md)、
> [util-macros-analysis.md](./util-macros-analysis.md) 是同一套框架的第四次应用。

## 结论

1. **依赖，而且边际成本是零** —— 它不是「要不要拉进来」的问题，它**已经在我们的
   编译图里**：`gpui` 自己就依赖它（zed `crates/gpui/Cargo.toml:56`），Linux 上
   每次 `cargo build` 都已经编译过一遍。加一行 workspace 依赖只是让**我们自己的
   代码也能 `use collections::HashMap`**。
2. **但别为了用它而用** —— 只在两种情况用它：（a）照抄/对齐 zed 代码时对方已经
   写了 `collections::HashMap`,别手贱改回 `std::collections::HashMap`；
   （b）真的需要 Fx 哈希或 `VecMap` 的语义。
3. **依赖时必须显式写 `rev = f6838a7c`** —— 否则可能拿到仓库里已有的那份旧 rev
   副本（见下方「陷阱」），于是编译图里出现第三份 `collections`。

## 解剖（418 行，本质是「别名层」）

```
src/collections.rs     15 行   ← 全部是 type alias + re-export
src/vecmap.rs         192 行   ← 唯一的实质实现：VecMap
src/vecmap_tests.rs   211 行   ← 测试（比实现还多）
```

`collections.rs` 全文就这些导出：

```rust
pub type HashMap<K, V> = FxHashMap<K, V>;
pub type HashSet<T>    = FxHashSet<T>;
pub type IndexMap<K,V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub type IndexSet<T>   = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
pub type TypeIdHashMap<V> = std::collections::HashMap<TypeId, V, gpui_util::TypeIdHashBuilder>;
pub type TypeIdHashSet    = std::collections::HashSet<TypeId, gpui_util::TypeIdHashBuilder>;
pub use indexmap::Equivalent;
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};
pub use std::collections::*;      // ← 所以 collections::BTreeMap 之类也能用
pub mod vecmap;
```

它解决的是 zed 的一个**全局约定**问题：「全仓库的 HashMap 都用 rustc 的 FxHasher」。
这不是能力问题（std 也有 HashMap），是**一致性和性能**问题 —— FxHasher 对短 key
（指针、u64、短字符串）比 SipHash 快得多，且结果确定（可复现）。

`VecMap<K, V>` 是唯一有肉的部分：Vec-backed 的有序小映射，带完整 entry API
（`entry` / `entry_ref` / `or_insert_with_key` / `OccupiedEntry` / `VacantEntry`），
适用于「key 数量很小、遍历比查找多」的场景。

**但它是死代码**：全 zed 仓库搜 `vecmap` 只命中两处 —— `collections.rs` 的
`pub mod vecmap;` 和它自己的 `vecmap_tests.rs`。**没有任何 crate 在用 `VecMap`**
（大概是某次重构被替换后留下的）。所以别因为它「看起来是个有用的数据结构」
就去用 —— 上游自己都没在用，`collections` 对你的价值实际上只有那 15 行别名。

**依赖闭包**：`indexmap` + `rustc-hash` + `gpui_util` —— 三个**全在我们的
Cargo.lock 里**（:3608 / :5797 / :3106），一个都不用下载。

## 关键事实：它已经在编译

```console
$ cargo tree -p aa_gpui_kit_ui -i collections -e normal --target x86_64-unknown-linux-gnu
collections v0.1.0 (zed?rev=f6838a7c#f6838a7c)
├── gpui v0.2.2 (zed?rev=f6838a7c#f6838a7c)
└── zlog v0.1.0 (zed?rev=f6838a7c#f6838a7c)
    └── ztracing …
```

`gpui` 直接依赖它，`zlog` 也依赖它 —— **两条路径都在 Linux 上生效**。所以：

- 「git 依赖 collections」的**真实成本 = 0**（不新增包、不新增 clone、不新增编译单元）；
- 反过来说，**我们现在不用它，也照样在为它付编译时间** —— 那不如该用时就用。

## 决策表

| 场景 | 怎么做 | 理由 |
|---|---|---|
| 抄/对齐 zed 代码遇到 `collections::HashMap` / `IndexMap` | **原样保留 + 加一行 git 依赖** | 改成 `std::collections::HashMap` 会引入与上游的 diff，后续同步更痛；且哈希器不同会改性能特征 |
| 需要「类型作 key」的注册表（对齐 gpui 的 Global 存储） | 用 `TypeIdHashMap` / `TypeIdHashSet` | 已经配好 `gpui_util::TypeIdHashBuilder`，自己配一遍纯属重复 |
| 需要一个「小 key 空间 + 保序 + 遍历快」的 map | **别用 `collections::VecMap`** | 全 zed 仓库零消费者（只有它自己的测试在用），是死代码；真需要就自己写或用 `indexmap` |
| 只是想要个 HashMap | **用 `std::collections::HashMap`** | 别为了「对齐 zed」在没有性能需求的地方换哈希器 |
| 处理**不可信输入**作 key 的场景（网络/文件内容） | **不要用 Fx** | FxHasher 非抗碰撞，可被构造碰撞做 HashDoS；用 std 的 SipHash |

## 陷阱

1. **仓库里有两份 `collections`**：`Cargo.lock` 的 `:1446`（rev `d88f682`，被
   `gpui-android` 锁旧 rev 拖进来）和 `:1456`（rev `f6838a7c`，gpui 用的这份）。
   根因记录在 [util-analysis.md 附录](./util-analysis.md)（gpui-android 锁旧 rev）。
   所以新增依赖**必须写全 rev**：

   ```toml
   collections = { git = "https://github.com/zed-industries/zed", rev = "f6838a7c4afecd7c212c43c70165588b92a5a11f" }
   ```

   否则 cargo 可能解析到旧 rev 那份，编译图里同时出现三份 —— 编译时间三倍、且
   两个版本的 `VecMap` 类型不互通。

2. **`pub use std::collections::*` 会污染**：`use collections::*` 会把 `HashMap`
   换成 Fx 版，同时把 `BTreeMap` / `VecDeque` / `BinaryHeap` 也一并导入。**别写
   `use collections::*`**,要什么点什么（`use collections::{HashMap, HashSet};`）。

3. **`VecMap` 不是 `HashMap` 的替代品**：它是线性查找，元素多了是 O(n) 查找。
   zed 只在元素极少时用。别因为「名字好听」就拿来装几千个条目。

## 附：四次分析的判据对照

| crate | 闭包 | 通用库? | 现在用得上? | 结论 |
|---|---|---|---|---|
| `crates/util` | 20+ 依赖 + 平台件 | 否（应用杂物间） | 否 | 不依赖，散函数按需抄 |
| `crates/path` | anyhow + dunce（都在树上） | 是（纯路径不变量） | 暂不需要 | **可以依赖，先搁着** |
| `crates/util_macros` | 一个包都不新增 | 否（测试糖 + zed CI client） | 否（Linux 上是恒等函数） | 不依赖，需要时抄 12 行 |
| `crates/collections` | 一个包都不新增（已在编译） | 是（哈希器约定；VecMap 是死代码） | 抄 zed 代码时必然遇到 | **依赖，零成本** |

规律：闭包大小**从来不是决定性因素**（`util_macros` 免费我们也不想要，
`util` 贵但里面真有通用件值得抄）。决定性的三个问题是 ——
**是不是通用库 / 我们现在用得上吗 / 需要的那部分有多少行**。
