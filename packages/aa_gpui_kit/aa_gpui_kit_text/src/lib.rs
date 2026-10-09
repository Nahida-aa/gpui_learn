//! Text primitives for Zed.
//!
//! This crate provides:
//!
//! - [`Buffer`] / [`BufferSnapshot`]: a concurrent, rope-backed editable text
//!   buffer with anchors and undo/redo history.
//! - The [`ToOffset`] / [`ToPoint`] family of conversions between offsets,
//!   `Point`s, and UTF-16 `Point`s.
//! - Small utility modules (`patch`, `selection`, `subscription`, `undo_map`,
//!   `operation_queue`, `locator`) reused across the rest of the editor.
//!
//! The rope implementation itself lives in the external `rope` crate and is
//! re-exported here for convenience.

mod anchor;
mod buffer;
pub mod locator;
#[cfg(any(test, feature = "test-support"))]
pub mod network;
pub mod operation_queue;
mod patch;
mod selection;
pub mod subscription;
mod undo_map;

#[cfg(test)]
mod tests;

pub use anchor::*;
pub use buffer::*;
// 对齐 zed `text::debug`（`crates/text/src/text.rs` 里的
// `#[cfg(debug_assertions)] pub mod debug`）。multi_buffer 的
// `debug_with_key` 从外部调 `text::debug::GlobalDebugRanges`，所以必须
// 在 crate 根可达 —— 只放在 buffer 模块里的 `use ... as debug` 不够。
#[cfg(debug_assertions)]
pub use buffer::debug_ranges as debug;
pub use patch::Patch;
pub use rope::*;
pub use selection::*;
pub use subscription::*;
pub use sum_tree::Bias;

// `ReplicaId` is used pervasively alongside `Buffer`, so keep it available at
// the crate root as well.
pub use clock::ReplicaId;
