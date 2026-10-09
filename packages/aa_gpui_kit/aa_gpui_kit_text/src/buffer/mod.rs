//! A concurrent, rope-backed text buffer.
//!
//! The [`Buffer`] type exposes an entity-like API for editing text, while
//! [`BufferSnapshot`] is an immutable, cheaply cloneable view of a fixed
//! version. All cross-replica synchronization, undo/redo history, and
//! anchor resolution live in this module.

mod buffer;
mod buffer_id;
mod constants;
#[cfg(debug_assertions)]
pub mod debug_ranges;
mod dimensions;
mod edit_snapshot;
mod edits_iter;
mod fragment;
mod history;
mod indent;
mod line_ending;
mod offset_traits;
mod operation;
mod rope_builder;
mod snapshot;
#[cfg(any(test, feature = "test-support"))]
mod test_support;
mod util;

pub use buffer::Buffer;
pub use buffer_id::BufferId;
pub use constants::MAX_INSERTION_LEN;
pub use dimensions::FullOffset;
pub use edit_snapshot::EditedBufferSnapshot;
pub use edits_iter::Edit;
pub use history::{HistoryEntry, Transaction};
pub use indent::LineIndent;
pub use line_ending::{LineEnding, chunks_with_line_ending};
pub use offset_traits::{FromAnchor, ToOffset, ToOffsetUtf16, ToPoint, ToPointUtf16};
pub use operation::{EditOperation, Operation, UndoOperation};
pub use snapshot::BufferSnapshot;

/// Identifier for an undo/redo transaction, identical to the lamport timestamp
/// that ticked it.
pub type TransactionId = clock::Lamport;

// Keep the original `debug::…` path working. Previously this was an inline
// `pub mod debug` inside `lib.rs`; now it lives in its own file.
#[cfg(debug_assertions)]
use crate::buffer::debug_ranges as debug;
