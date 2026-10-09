//! Fixed-size text chunks with per-byte bitmaps.
//!
//! A `Chunk` holds up to `MAX_BASE` bytes of UTF-8 text, alongside four
//! `Bitmap`s whose bits parallel the bytes: `chars`, `chars_utf16`,
//! `newlines`, and `tabs`. These bitmaps answer offset/point/utf16
//! conversion queries in O(1) using popcount and bit-scan.
//!
//! Under `cfg(test)` the bitmap shrinks to `u16` and `MAX_BASE` becomes
//! 16, so randomized property tests hammer the boundary conditions that
//! would otherwise only occur at 128-byte edges.

mod bits;
mod chunk;
mod diagnostics;
mod slice;
mod tabs;

#[cfg(test)]
mod tests;

pub use chunk::Chunk;
pub use slice::ChunkSlice;
pub use tabs::{TabPosition, Tabs};

pub(crate) use bits::{nth_set_bit, saturating_shl_mask, saturating_shr_mask};
pub(crate) use diagnostics::{log_err_char_boundary, panic_char_boundary};

/// The width of the per-byte bitmaps stored on each [`Chunk`] and
/// [`ChunkSlice`]. Kept at `u128` in production so a chunk can cover a full
/// cache line; reduced to `u16` in tests to force the boundary paths to be
/// exercised with much smaller inputs.
#[cfg(not(all(test, not(rust_analyzer))))]
pub(crate) type Bitmap = u128;
#[cfg(all(test, not(rust_analyzer)))]
pub(crate) type Bitmap = u16;

pub(crate) const MIN_BASE: usize = MAX_BASE / 2;
pub(crate) const MAX_BASE: usize = Bitmap::BITS as usize;
