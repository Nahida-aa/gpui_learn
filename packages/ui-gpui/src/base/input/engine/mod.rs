//! `engine`——自研 Rope 文本引擎（Rope/Chunk/TextSummary/坐标）。
//!
//! 参照 zed `crates/rope` 的精简移植，选型与精简范围见
//! `docs/editor-roadmap.md` 的「架构决策 3」。对外只 re-export 稳定类型，
//! 内部模块路径不构成公开 API。

mod chunk;
mod point;
mod rope;
mod summary;

pub use point::{OffsetUtf16, Point, PointUtf16};
pub use rope::{Chunks, Cursor, Rope};
pub use summary::{ChunkSummary, TextSummary};
