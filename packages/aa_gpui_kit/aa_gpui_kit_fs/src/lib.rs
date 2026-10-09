pub mod fs_watcher;
mod git_clone_progress;

mod file_handle;
mod jobs;
mod metadata;
mod options;
mod real_fs;
mod traits;
mod trash;
mod util;
mod watcher;

#[cfg(feature = "test-support")]
mod fake_fs;
#[cfg(feature = "test-support")]
mod fake_git_repo;

use std::sync::Arc;

use gpui::{App, Global, ReadGlobal as _};

pub use file_handle::FileHandle;
pub use jobs::{JobEvent, JobEventReceiver, JobEventSender, JobId, JobInfo};
pub use metadata::{MTime, Metadata};
pub use options::{CopyOptions, CreateOptions, RemoveOptions, RenameOptions};
pub use real_fs::{RealFs, RealWatcher};
pub use traits::Fs;
pub use trash::{TrashId, TrashRestoreError};
pub use util::{copy_recursive, read_dir_items};
pub use watcher::{PathEvent, PathEventKind, Watcher};

#[cfg(feature = "test-support")]
pub use fake_fs::FakeFs;
#[cfg(feature = "test-support")]
pub use fake_git_repo::FakeBlobReadGate;

struct GlobalFs(Arc<dyn Fs>);

impl Global for GlobalFs {}

impl dyn Fs {
    /// Returns the global [`Fs`].
    pub fn global(cx: &App) -> Arc<Self> { GlobalFs::global(cx).0.clone() }

    /// Sets the global [`Fs`].
    pub fn set_global(fs: Arc<Self>, cx: &mut App) { cx.set_global(GlobalFs(fs)); }
}
