//! Git integration for Zed: a thin wrapper over `git2`/CLI with a shared
//! [`Oid`] type and a set of [`actions`] the UI can dispatch.

pub mod blame;
pub mod commit;
mod hosting_provider;
mod remote;
pub mod repository;
pub mod stash;
pub mod status;

mod actions;
mod constants;
mod oid;
mod run_hook;

#[cfg(test)]
mod tests;

pub use crate::hosting_provider::*;
pub use crate::remote::*;
pub use actions::{RenameBranch, RestoreFile};
pub use constants::{
    BISECT_LOG, COMMIT_MESSAGE, DOT_GIT, FETCH_HEAD, FSMONITOR_DAEMON, GC_PID, GITIGNORE,
    HOOKS_DIR, INFO_DIR, LFS_DIR, LOGS_DIR, LOGS_REF_STASH, OBJECTS_DIR, ORIG_HEAD,
    REBASE_APPLY_DIR, REBASE_MERGE_DIR, REFS_DIR, REFTABLE_DIR, REPO_EXCLUDE, SEQUENCER_DIR,
};
pub use oid::{Oid, SHORT_SHA_LENGTH, SHA256_HEX_LENGTH};
pub use repository::RemoteCommandOutput;
pub use run_hook::RunHook;

use anyhow::Result;
use gpui::actions;

// `actions!` 宏必须展开在 crate 根或某个模块里，把 `git` 命名空间下的所有
// 动作挂到当前模块。这里放在 `actions.rs` 里展开，然后通过 `pub use actions::*;`
// 把生成的类型（如 `git::ToggleStaged`、`git::StageAll` 等）re-export 出来。
pub use actions::*;
