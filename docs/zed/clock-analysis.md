# zed `crates/clock` 依赖分析 —— git 依赖还是自己写？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 与 [util-analysis.md](./util-analysis.md)、[path-analysis.md](./path-analysis.md)、
> [util-macros-analysis.md](./util-macros-analysis.md)、
> [collections-analysis.md](./collections-analysis.md) 同一套框架的第五次应用。

## 结论

> **2026-09-22 后续**：已按结论 2 自建，见 `packages/clock`（包名 `aa_clock`，
> 零 gpui 依赖，两半都在，12 个单测通过）。下面保留原始分析。

1. **现在两个都别做** —— `clock` 在 gpui_learn 里**一个引用都没有**
   （全仓库搜 `clock|Lamport|ReplicaId` 零命中）。没有消费者就不要先造依赖。
2. **将来需要时：自己写** —— 这是目前分析过的 crate 里**最没有依赖价值**的一个：
   它不提供任何跨 crate 必须同型的东西（见下「决定性判据」），338 行全是
   自包含逻辑，且 `license = "GPL-3.0-or-later"` 而我们是 Apache-2.0。
3. 它只有**两半**，需要哪半抄哪半，别整体搬：
   - `ReplicaId` / `Lamport` / `Global`（285 行）—— CRDT 逻辑时钟，**只有当我们
     真去移植 `text` / `buffer` 的 CRDT 编辑才需要**；
   - `SystemClock` / `FakeSystemClock`（53 行）—— 时间的可测试缝合点，
     自己写 20 行就够，而且 zed 那版有个命名坑（见「陷阱」）。

## 解剖（338 行，零 zed 内部依赖）

```
src/clock.rs        285 行  ← ReplicaId / Lamport / Global（版本向量）
src/system_clock.rs  53 行  ← SystemClock trait + Real/Fake 实现
```

```toml
[dependencies]
parking_lot = { workspace = true, optional = true }   # 仅 test-support
serde.workspace = true
smallvec.workspace = true
```

**依赖闭包 = serde + smallvec + parking_lot**，三个全在 `Cargo.lock` 里
（:6027 / :6282 / :4942），一个都不用新下载；也没有 `gpui`、没有 zed 内部 crate。
所以「技术可行性」这一关它是**满分** —— 比 `path`、`collections` 还干净。

### 第一半：逻辑时钟（`clock.rs`）

| 类型 | 是什么 | 我们什么时候会需要 |
|---|---|---|
| `ReplicaId(u16)` | 分布式副本 id，`LOCAL/REMOTE_SERVER/AGENT/LOCAL_BRANCH/FIRST_COLLAB_ID` | 只有在做多人协同的时候；这几个常量是 **zed collab 的语义**，与我们无关 |
| `Lamport { value: Seq, replica_id }` | Lamport 时间戳，`tick()` / `observe()` / `as_u64()` | `text::TransactionId` 就是它；移植 CRDT undo 栈时需要 |
| `Global` | 版本向量（`SmallVec<[u32;4]>`），`observe/join/meet/observed_all/changed_since` | `text::BufferSnapshot::version`、`buffer::saved_version` 全靠它 |

它们在 zed 里是**纯协同编辑的骨架**：`text`、`language::Buffer`、`buffer_diff`、
`multi_buffer`、`project::lsp_store` 里到处是 `clock::Global` 的版本比较。
我们现在的 `packages/editor` 是**单机单用户**内核（多光标 + 增量，没有版本向量），
所以这一半现在**一个字节都用不上**。

### 第二半：系统时钟（`system_clock.rs`）

```rust
pub trait SystemClock: Send + Sync { fn utc_now(&self) -> Instant; }
pub struct RealSystemClock;          // Instant::now()
pub struct FakeSystemClock;          // cfg(test / test-support)，可 set_now / advance
```

是个**让时间可注入**的测试缝合点，zed 里被 `editor`、`worktree`、`project`、
`auto_update`、`client::telemetry` 用来做确定性测试。我们如果哪天要测
「防抖 / 超时 / 自动保存」,自己写 20 行的 `trait Clock` 完全等价。

## 决定性判据：**不存在类型同一性压力**

前四次分析里，最硬的一条判据是「**这个 crate 提供的类型会不会和 gpui 手上的
那个必须同型**」：

- `collections::HashMap` —— gpui 自己在用，不同型就无法把 map 传进 gpui API
  → **必须依赖**（而且反正是零成本，已经在编译图里）。
- `clock::Lamport` —— **谁都不和我们共享**。

实测：`gpui`、`gpui_util`、`sum_tree` 的 `Cargo.toml` 里**都没有 clock**
（唯一的 grep 命中为 0），而 `clock` 的上游消费者是
`text / language / project / editor / collab / worktree / client / agent` 这一层
（应用层，我们一个都没引入）。也就是说：

```console
$ grep -n clock crates/gpui/Cargo.toml crates/gpui_util/Cargo.toml crates/sum_tree/Cargo.toml
（无输出）
```

**我们现有编译图里根本没有 `clock`，而且未来也不会有第二条路径把它带进来。**
于是「两个版本的 `clock::Global` 类型不互通」这个经典痛点**不存在** ——
自己写一份，和谁都不冲突。这正是它和 `collections` 的根本区别。

反过来，git 依赖它要付出：多一个编译单元（很小）+ GPL-3.0 代码进树 +
为 338 行锁定一个 zed rev。收益只有「将来抄 `text` 时不用批量替换 `clock::` 前缀」
—— 那是一道 `sed` 就能解决的事。

## 决策表

| 场景 | 怎么做 | 理由 |
|---|---|---|
| 现在（gpui_learn 里零引用） | **什么都不做** | 没有消费者的依赖就是负担；将来需要时再加 |
| 将来要测防抖/超时/自动保存，需要假时间 | **自己写 `trait Clock`**（20 行），别依赖 | 半个 crate 的体量，不值得拉 GPL 代码；顺手修掉 `utc_now()->Instant` 的命名坑 |
| 将来真去移植 zed 的 `text` / CRDT undo | **自己写 `ReplicaId/Lamport/Global`**（~150 行，去掉 collab 常量） | 同上；且我们的副本模型更简单（大概率只有 LOCAL） |
| 抄 zed 代码遇到 `clock::Lamport` | **看规模**：零星几处 → 换成我们的类型；整包移植 `text` → 再重新评估 git 依赖 | 那时需要权衡的是「整包 `text` 的依赖闭包」，不是 `clock` 一个 |
| 有人提议「反正依赖闭包是零，先加上」 | **拒绝** | 零下载 ≠ 零成本：许可证 + 编译单元 + 一个用不上的 API 面（见「陷阱」1） |

## 陷阱

1. **`clock` 同时是「逻辑时钟」和「系统时钟」两个不相干的东西**，名字极具误导性。
   `use clock::*` 会同时把 `Global`、`Lamport` 和 `SystemClock` 灌进来 ——
   如果一个 crate 只想做「可测试的时间」却依赖了整个 `clock`，
   就会白白吃下 285 行 CRDT 逻辑和 `smallvec` 依赖。**别整包依赖半个 crate。**

2. **`utc_now()` 返回的是 `Instant`，不是 UTC 时间**。这是 zed 自己的历史包袱
   （早期可能返回过 `DateTime<Utc>`）：`Instant` 是单调时钟，没有时区概念，
   和「UTC」毫无关系。自己写时直接用 `fn now(&self) -> Instant` 或
   `fn utc_now(&self) -> DateTime<Utc>`（配 `chrono`），别复制这个坑。

3. **`ReplicaId` 的常量是 zed 协同协议的一部分**：`AGENT = 2`、
   `FIRST_COLLAB_ID = 8`、`is_remote()` 的判定都绑定 zed 的 collab 服务端语义。
   即使自己实现，也**别照抄这些常量** —— 我们的副本模型大概率只需要 `LOCAL`。

4. **`Global` 的 `meet()` 有微妙行为**：只在 `other` 不长于 `self` 时才
   `truncate`（`clock.rs:151-157` 的注释解释了为什么不无条件截断）。
   自己实现时最容易写错的就是这一段，抄的时候**连注释一起抄**。

5. **GPL-3.0-or-later**：`crates/clock/LICENSE-GPL -> ../../LICENSE-GPL`。
   我们仓库是 `license = "Apache-2.0"`（根 `Cargo.toml:45`）。
   之前 ui / component / keybinding / settings_json 已经复制过 GPL 文件，
   这个历史问题还没结论；但在**没有类型同一性压力**的时候，
   没必要主动再添一份 —— 这是「自己写」最实际的一条理由。

## 附：五次分析的判据对照

| crate | 闭包 | 通用库? | 现在用得上? | 类型同一性压力 | 结论 |
|---|---|---|---|---|---|
| `crates/util` | 20+ 依赖 + 平台件 | 否（应用杂物间） | 否 | 无 | 不依赖，散函数按需抄 |
| `crates/path` | anyhow + dunce（都在树上） | 是（纯路径不变量） | 暂不需要 | 无 | 可以依赖，先搁着 |
| `crates/util_macros` | 零新增 | 否（测试糖） | 否（Linux 上恒等） | 无 | 不依赖，需要时抄 12 行 |
| `crates/collections` | 零新增（**已在编译**） | 是（哈希器约定） | 抄 zed 代码时必然遇到 | **有**（gpui 在用） | **依赖，零成本** |
| `crates/clock` | 零新增（**但不在编译图**） | 是（CRDT + 假时间） | **否（零引用）** | **无**（gpui 不用） | **暂不动；需要时自己写** |

规律更新：闭包大小仍然不是决定性因素；**决定性的是
「通用库 / 现在用得上吗 / 需要的那部分多少行 / 有没有类型同一性压力」**，
其中最后一条是 `clock` 与 `collections` 分道扬镳的唯一原因 ——
`collections` 已经在我们的编译图里且 gpui 共享它的类型，
`clock` 两条都不占。
