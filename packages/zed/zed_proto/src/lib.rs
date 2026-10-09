#![allow(non_snake_case)]

pub mod error;
mod macros;
mod typed_envelope;

mod constants;
mod nonce;
mod split;
mod timestamp;

#[cfg(test)]
mod tests;

pub use error::*;
pub use prost::{DecodeError, Message};
pub use typed_envelope::*;

// 协议类型由 build script 生成的 `zed.messages.rs` 提供。
// 这一段必须留在 crate 根：所有宏调用都依赖它生成的类型名。
include!(concat!(env!("OUT_DIR"), "/zed.messages.rs"));

// 宏调用列表。`include!` 是文本内联，展开后仍在 crate 根，
// 因此宏内部对生成类型的引用方式不变。

include!("envelope.rs");
include!("lsp_query.rs");
include!("messages.rs");
include!("request_messages.rs");
include!("lsp_messages.rs");
include!("entity_messages.rs");

pub use constants::{REMOTE_SERVER_PEER_ID, REMOTE_SERVER_PROJECT_ID};
pub use split::{
    MAX_WORKTREE_UPDATE_MAX_CHUNK_SIZE, split_repository_update, split_worktree_update,
};
