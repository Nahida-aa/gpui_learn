//! 时钟：两件不相干但都叫 clock 的东西（对齐 zed `crates/clock` 的划分，但自研）。
//!
//! - [`clock`](crate::clock)：**逻辑时钟** —— `ReplicaId` / `Lamport` / `Global`，
//!   用于确定事件顺序（CRDT 版本向量）。只有做协同编辑 / 事务 id 时才需要。
//! - [`SystemClock`]：**系统时钟** —— 一层 trait，让「现在几点」可以被注入，
//!   从而在测试里用 [`FakeSystemClock`] 把时间固定住。
//!
//! 本包**不依赖 gpui**（与 zed 一致，只吃 serde / smallvec / parking_lot），
//! 所以任意层都可以引用它。为什么自研而不 git 依赖 zed 的 clock，见
//! `docs/zed/clock-analysis.md`。

mod clock;
mod system_clock;

pub use clock::*;
pub use system_clock::*;
