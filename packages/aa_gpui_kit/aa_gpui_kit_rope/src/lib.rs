//! A rope of fixed-size [`Chunk`]s, providing O(log n) indexing and
//! point/offset/utf16 conversions via `SumTree` summaries.

mod chunk;
mod cursor;
mod dimension;
mod iterators;
mod offset_utf16;
mod point;
mod point_utf16;
mod rope;
mod summary;
mod unclipped;

#[cfg(test)]
mod tests;

pub use chunk::{Chunk, ChunkSlice};
pub use cursor::Cursor;
pub use dimension::{DimensionPair, TextDimension};
pub use iterators::{Bytes, ChunkBitmaps, ChunkWithBitmaps, Chunks, Lines};
pub use offset_utf16::OffsetUtf16;
pub use point::Point;
pub use point_utf16::PointUtf16;
pub use rope::Rope;
pub use summary::{ChunkSummary, TextSummary};
pub use unclipped::Unclipped;

// 子模块内部共享
pub(crate) use chunk::Bitmap;
