//! Zed's shared utility library.

#[cfg(test)]
extern crate self as util;

#[cfg(not(target_family = "wasm"))]
pub mod archive;
#[cfg(not(target_family = "wasm"))]
pub mod command;
#[cfg(not(target_family = "wasm"))]
pub mod fs;
#[cfg(not(target_family = "wasm"))]
pub mod process;
#[cfg(not(target_family = "wasm"))]
pub mod shell;
#[cfg(not(target_family = "wasm"))]
pub mod shell_builder;
#[cfg(not(target_family = "wasm"))]
pub mod shell_env;

pub mod disambiguate;
pub mod markdown;
pub mod path_list;
pub mod paths;
pub mod redact;
pub mod schemars;
pub mod serde;
pub mod size;
#[cfg(any(test, feature = "test-support"))]
pub mod test;
pub mod time;

mod connection;
mod dev;
mod embed;
mod json;
mod misc;
mod os;
mod ranges;
mod sort;
mod text;
mod truncate;

#[cfg(any(test, feature = "test-support"))]
mod rng;

#[cfg(test)]
mod tests;

// ---- re-exports ----

pub use connection::ConnectionResult;
pub use dev::dev_repo_root;
pub use embed::{__rust_embed, asset_str};
// 这两个只存在于 dev 分支（`#[cfg(all(debug_assertions, not(feature = "debug-embed")))]`）：
// release 走 rust_embed 的编译期内嵌，根本不调用它们。不加同样的门控的话，
// `cargo build --release` 会在这一行报 E0432。
#[cfg(all(debug_assertions, not(feature = "debug-embed")))]
pub use embed::{__fs_embed_get, __fs_embed_iter};
pub use json::{
    merge_json_lenient_value_into, merge_json_value_into, merge_non_null_json_value_into,
    union_json_value_into,
};
pub use misc::default;
pub use os::{
    increase_open_file_limit, parse_os_release,
    set_pre_exec_to_start_new_session,
};
pub use ranges::{RangeExt, expanded_and_wrapped_usize_range, wrapped_usize_outward_from};
pub use sort::{NumericPrefixWithSuffix, extend_sorted};
pub use text::{split_str_with_ranges, word_consists_of_emojis};
pub use truncate::{
    is_utf8_char_boundary, truncate, truncate_and_remove_front, truncate_and_trailoff,
    truncate_lines_and_trailoff, truncate_lines_to_byte_limit, truncate_to_byte_limit,
};

#[cfg(any(test, feature = "test-support"))]
pub use rng::RandomCharIter;

// ---- 第三方 re-exports（保持不变） ----

pub use gpui_util::*;
pub use path::PathExt;
pub use path::normalize_path;
pub use path::rel_path;
pub use take_until::*;
#[cfg(any(test, feature = "test-support"))]
pub use util_macros::{line_endings, path, uri};

#[cfg(not(target_family = "wasm"))]
pub use self::shell::{
    get_default_system_shell, get_default_system_shell_preferring_bash, get_system_shell,
};
