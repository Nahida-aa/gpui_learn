use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_tar::Archive;
use futures::{AsyncRead, Stream};
use git::repository::GitRepository;
use rope::Rope;
use text::LineEnding;

use crate::file_handle::FileHandle;
use crate::fs_watcher;
use crate::jobs::JobEventReceiver;
use crate::metadata::Metadata;
use crate::options::{CopyOptions, CreateOptions, RemoveOptions, RenameOptions};
use crate::trash::{TrashId, TrashRestoreError};
use crate::watcher::{PathEvent, Watcher};

#[cfg(feature = "test-support")]
use crate::fake_fs::FakeFs;

#[async_trait::async_trait]
pub trait Fs: Send + Sync {
    async fn create_dir(&self, path: &Path) -> Result<()>;
    async fn create_symlink(&self, path: &Path, target: PathBuf) -> Result<()>;
    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()>;
    async fn create_file_with(
        &self,
        path: &Path,
        content: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<()>;
    async fn extract_tar_file(
        &self,
        path: &Path,
        content: Archive<Pin<&mut (dyn AsyncRead + Send)>>,
    ) -> Result<()>;
    async fn copy_file(&self, source: &Path, target: &Path, options: CopyOptions) -> Result<()>;
    async fn rename(&self, source: &Path, target: &Path, options: RenameOptions) -> Result<()>;

    /// Removes a directory from the filesystem.
    /// There is no expectation that the directory will be preserved in the
    /// system trash.
    async fn remove_dir(&self, path: &Path, options: RemoveOptions) -> Result<()>;

    /// Moves a file or directory to the system trash.
    /// Returns a [`TrashedEntry`] that can be used to keep track of the
    /// location of the trashed item in the system's trash.
    async fn trash(&self, path: &Path, options: RemoveOptions) -> Result<TrashId>;

    /// Removes a file from the filesystem.
    /// There is no expectation that the file will be preserved in the system
    /// trash.
    async fn remove_file(&self, path: &Path, options: RemoveOptions) -> Result<()>;

    async fn open_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>>;
    async fn open_sync(&self, path: &Path) -> Result<Box<dyn io::Read + Send + Sync>>;
    async fn load(&self, path: &Path) -> Result<String> {
        Ok(String::from_utf8(self.load_bytes(path).await?)?)
    }
    async fn load_bytes(&self, path: &Path) -> Result<Vec<u8>>;
    async fn atomic_write(&self, path: PathBuf, text: String) -> Result<()>;
    async fn save(&self, path: &Path, text: &Rope, line_ending: LineEnding) -> Result<()>;
    async fn write(&self, path: &Path, content: &[u8]) -> Result<()>;
    async fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    async fn is_file(&self, path: &Path) -> bool;
    async fn is_dir(&self, path: &Path) -> bool;
    async fn metadata(&self, path: &Path) -> Result<Option<Metadata>>;
    async fn read_link(&self, path: &Path) -> Result<PathBuf>;
    async fn read_dir(
        &self,
        path: &Path,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PathBuf>>>>>;

    /// Creates the native file watcher now rather than on the first `watch`, so a
    /// failure to start it (e.g. inotify instance limits) can be reported at startup.
    fn start_native_watcher(&self) -> Result<()> { Ok(()) }

    /// Records raw local watcher notifications until the returned recording is dropped.
    fn record_watcher_diagnostics(&self) -> Option<fs_watcher::WatchRecording> { None }

    /// Whether `path` exists, without following a final symlink. Synchronous
    /// because watches are registered synchronously by the worktree scanner.
    fn path_exists(&self, path: &Path) -> bool;
    /// Whether the volume holding `path` compares file names case-sensitively.
    fn is_path_case_sensitive(&self, path: &Path) -> bool;
    /// Whether `path` sits on a filesystem where native file watching does not
    /// deliver events (network mounts, some FUSE and WSL mounts), so it must be polled.
    fn requires_poll_watcher(&self, path: &Path) -> bool;

    async fn watch(
        &self,
        path: &Path,
        latency: Duration,
    ) -> (
        Pin<Box<dyn Send + Stream<Item = Vec<PathEvent>>>>,
        Arc<dyn Watcher>,
    );

    fn open_repo(
        &self,
        abs_dot_git: &Path,
        system_git_binary_path: Option<&Path>,
    ) -> Result<Arc<dyn GitRepository>>;
    async fn git_init(&self, abs_work_directory: &Path, fallback_branch_name: String)
    -> Result<()>;
    async fn git_clone(&self, abs_work_directory: &Path, repo_url: &str) -> Result<()>;
    async fn git_config(&self, abs_work_directory: &Path, args: Vec<String>) -> Result<String>;
    fn is_fake(&self) -> bool;
    async fn is_case_sensitive(&self) -> bool;
    fn subscribe_to_jobs(&self) -> JobEventReceiver;

    /// Returns the original absolute path of the item identified by `trash_id`.
    fn original_path_for_trash_id(&self, trash_id: TrashId) -> Option<PathBuf>;

    /// Restores the item identified by `trash_id`, moving it from the system's
    /// trash back to its original path.
    async fn restore(&self, trash_id: TrashId) -> std::result::Result<PathBuf, TrashRestoreError>;

    #[cfg(feature = "test-support")]
    fn as_fake(&self) -> Arc<FakeFs> {
        panic!("called as_fake on a real fs");
    }
}
