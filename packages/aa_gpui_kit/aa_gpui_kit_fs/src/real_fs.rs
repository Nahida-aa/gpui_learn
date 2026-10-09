use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use futures::stream::iter;
use futures::{AsyncRead, Stream};
use gpui::{BackgroundExecutor, SharedString};
use parking_lot::Mutex;
use slotmap::SlotMap;
use tempfile::TempDir;
use util::maybe;

use crate::fs_watcher;
use crate::git_clone_progress;
use crate::jobs::{JobEventSender, JobInfo, JobTracker};
use crate::metadata::{MTime, Metadata};
use crate::options::{CopyOptions, CreateOptions, RemoveOptions, RenameOptions};
use crate::traits::Fs;
use crate::trash::{TrashId, TrashRestoreError, TrashedEntry};
use crate::watcher::{PathEvent, Watcher};

use git::repository::{GitRepository, RealGitRepository};
use rope::Rope;
use text::LineEnding;

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

#[cfg(windows)]
use is_executable::IsExecutable;

pub struct RealFs {
    this: std::sync::Weak<Self>,
    bundled_git_binary_path: Option<PathBuf>,
    executor: BackgroundExecutor,
    native_watcher: Arc<fs_watcher::OsWatcher>,
    poll_watcher: Arc<fs_watcher::OsWatcher>,
    next_job_id: Arc<AtomicUsize>,
    job_event_subscribers: Arc<Mutex<Vec<JobEventSender>>>,
    trash: Arc<Mutex<SlotMap<TrashId, TrashedEntry>>>,
    is_case_sensitive: AtomicU8,
}

pub struct RealWatcher {}

impl RealFs {
    pub fn new(git_binary_path: Option<PathBuf>, executor: BackgroundExecutor) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            bundled_git_binary_path: git_binary_path,
            native_watcher: fs_watcher::OsWatcher::new(
                fs_watcher::OsWatcherKind::Native,
                executor.clone(),
            ),
            poll_watcher: fs_watcher::OsWatcher::new(
                fs_watcher::OsWatcherKind::Poll,
                executor.clone(),
            ),
            executor,
            next_job_id: Arc::new(AtomicUsize::new(0)),
            job_event_subscribers: Arc::new(Mutex::new(Vec::new())),
            trash: Arc::new(Mutex::new(SlotMap::with_key())),
            is_case_sensitive: Default::default(),
        })
    }

    #[cfg(target_os = "windows")]
    fn canonicalize(path: &Path) -> Result<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Storage::FileSystem::GetVolumePathNameW;
        use windows::core::HSTRING;

        // std::fs::canonicalize resolves mapped network paths to UNC paths, which can
        // confuse some software. To mitigate this, we canonicalize the input, then rebase
        // the result onto the input's original volume root if both paths are on the same
        // volume. This keeps the same drive letter or mount point the caller used.

        let abs_path = if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.to_path_buf()
        };

        let path_hstring = HSTRING::from(abs_path.as_os_str());
        let mut vol_buf = vec![0u16; abs_path.as_os_str().len() + 2];
        unsafe { GetVolumePathNameW(&path_hstring, &mut vol_buf)? };
        let volume_root = {
            let len = vol_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(vol_buf.len());
            PathBuf::from(OsString::from_wide(&vol_buf[..len]))
        };

        let resolved_path = dunce::canonicalize(&abs_path)?;
        let resolved_root = dunce::canonicalize(&volume_root)?;

        if let Ok(relative) = resolved_path.strip_prefix(&resolved_root) {
            let mut result = volume_root;
            result.push(relative);
            Ok(result)
        } else {
            Ok(resolved_path)
        }
    }
}

// ---------- platform helpers ----------

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_without_replace(source: &Path, target: &Path) -> io::Result<()> {
    let source = path_to_c_string(source)?;
    let target = path_to_c_string(target)?;

    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(source.as_ptr(), target.as_ptr(), libc::RENAME_EXCL) };

    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "windows")]
fn rename_without_replace(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
    use windows::core::PCWSTR;

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVE_FILE_FLAGS::default(),
        )
    }
    .map_err(|_| io::Error::last_os_error())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn path_to_c_string(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path contains interior NUL: {}", path.display()),
        )
    })
}

// On Unix targets, std::fs::ReadDir panics in its Drop implementation
// when an unexpected error is returned from closedir(2). We hit this
// condition in production; one cause seems to be macOS's FSEventStream
// incorrectly closing fds it doesn't own, resulting in closedir returning
// EBADF, see https://github.com/zed-industries/zed/issues/59952#issuecomment-5080178879.
//
// We also see occasional errors like ENXIO and ETIMEDOUT that seem to
// come from network or other exotic filesystems.
//
// To avoid crashing the app in this situation, we use the rustix analogue of
// ReadDir, which doesn't have this panic in drop.
#[cfg(unix)]
fn read_dir_entries(path: PathBuf) -> Result<impl Send + Iterator<Item = Result<PathBuf>>> {
    use rustix::fs::{Dir, Mode, OFlags};
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let directory_fd = rustix::fs::open(
        &path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .with_context(|| format!("failed to open directory {path:?}"))?;
    let directory =
        Dir::new(directory_fd).with_context(|| format!("failed to read directory {path:?}"))?;

    Ok(directory.filter_map(move |entry| {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return Some(Err(anyhow::Error::new(error)
                    .context(format!("failed to read directory entry in {path:?}"))));
            }
        };
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            return None;
        }
        Some(Ok(path.join(OsStr::from_bytes(name))))
    }))
}

#[cfg(not(unix))]
fn read_dir_entries(path: PathBuf) -> Result<impl Send + Iterator<Item = Result<PathBuf>>> {
    let entries =
        std::fs::read_dir(&path).with_context(|| format!("failed to open directory {path:?}"))?;
    Ok(entries.map(move |entry| {
        entry
            .map(|entry| entry.path())
            .with_context(|| format!("failed to read directory entry in {path:?}"))
    }))
}

// todo(windows)
// can we get file id not open the file twice?
// https://github.com/rust-lang/rust/issues/63010
#[cfg(target_os = "windows")]
async fn file_id(path: impl AsRef<Path>) -> Result<u64> {
    use std::os::windows::io::AsRawHandle;

    use smol::fs::windows::OpenOptionsExt;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, GetFileInformationByHandle,
        },
    };

    let file = smol::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(path)
        .await?;

    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    smol::unblock(move || {
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle() as _), &mut info)? };

        Ok(((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64))
    })
    .await
}

#[cfg(target_os = "windows")]
fn atomic_replace<P: AsRef<Path>>(
    replaced_file: P,
    replacement_file: P,
) -> windows::core::Result<()> {
    use windows::{
        Win32::Storage::FileSystem::{REPLACE_FILE_FLAGS, ReplaceFileW},
        core::HSTRING,
    };

    // If the file does not exist, create it.
    let _ = std::fs::File::create_new(replaced_file.as_ref());

    unsafe {
        ReplaceFileW(
            &HSTRING::from(replaced_file.as_ref().to_string_lossy().into_owned()),
            &HSTRING::from(replacement_file.as_ref().to_string_lossy().into_owned()),
            None,
            REPLACE_FILE_FLAGS::default(),
            None,
            None,
        )
    }
}

// ---------- Fs impl ----------

#[async_trait::async_trait]
impl Fs for RealFs {
    async fn create_dir(&self, path: &Path) -> Result<()> {
        Ok(smol::fs::create_dir_all(path).await?)
    }

    async fn create_symlink(&self, path: &Path, target: PathBuf) -> Result<()> {
        #[cfg(unix)]
        smol::fs::unix::symlink(target, path).await?;

        #[cfg(windows)]
        if smol::fs::metadata(&target).await?.is_dir() {
            let status = util::command::new_command("cmd")
                .args(["/C", "mklink", "/J"])
                .args([path, target.as_path()])
                .status()
                .await?;

            if !status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to create junction from {:?} to {:?}",
                    path,
                    target
                ));
            }
        } else {
            smol::fs::windows::symlink_file(target, path).await?
        }

        Ok(())
    }

    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()> {
        let mut open_options = smol::fs::OpenOptions::new();
        open_options.write(true).create(true);
        if options.overwrite {
            open_options.truncate(true);
        } else if !options.ignore_if_exists {
            open_options.create_new(true);
        }
        open_options
            .open(path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        Ok(())
    }

    async fn create_file_with(
        &self,
        path: &Path,
        content: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<()> {
        let mut file = smol::fs::File::create(&path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        futures::io::copy(content, &mut file).await?;
        Ok(())
    }

    async fn extract_tar_file(
        &self,
        path: &Path,
        content: async_tar::Archive<Pin<&mut (dyn AsyncRead + Send)>>,
    ) -> Result<()> {
        content.unpack(path).await?;
        Ok(())
    }

    async fn copy_file(&self, source: &Path, target: &Path, options: CopyOptions) -> Result<()> {
        if !options.overwrite && smol::fs::metadata(target).await.is_ok() {
            if options.ignore_if_exists {
                return Ok(());
            } else {
                anyhow::bail!("{target:?} already exists");
            }
        }

        smol::fs::copy(source, target).await?;
        Ok(())
    }

    async fn rename(&self, source: &Path, target: &Path, options: RenameOptions) -> Result<()> {
        if options.create_parents {
            if let Some(parent) = target.parent() {
                self.create_dir(parent).await?;
            }
        }

        if options.overwrite {
            smol::fs::rename(source, target).await?;
            return Ok(());
        }

        let use_metadata_fallback = {
            #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
            {
                let source = source.to_path_buf();
                let target = target.to_path_buf();
                match self
                    .executor
                    .spawn(async move { rename_without_replace(&source, &target) })
                    .await
                {
                    Ok(()) => return Ok(()),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                        if options.ignore_if_exists {
                            return Ok(());
                        }
                        return Err(error.into());
                    }
                    Err(error)
                        if error.raw_os_error().is_some_and(|code| {
                            code == libc::ENOSYS
                                || code == libc::ENOTSUP
                                || code == libc::EOPNOTSUPP
                                || code == libc::EINVAL
                        }) =>
                    {
                        // For case when filesystem or kernel does not support atomic no-overwrite rename.
                        // EINVAL is returned by FUSE-based filesystems (e.g. NTFS via ntfs-3g)
                        // that don't support RENAME_NOREPLACE.
                        true
                    }
                    Err(error) => return Err(error.into()),
                }
            }

            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                // For platforms which do not have an atomic no-overwrite rename yet.
                true
            }
        };

        if use_metadata_fallback && smol::fs::metadata(target).await.is_ok() {
            if options.ignore_if_exists {
                return Ok(());
            } else {
                anyhow::bail!("{target:?} already exists");
            }
        }

        smol::fs::rename(source, target).await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        let result = if options.recursive {
            smol::fs::remove_dir_all(path).await
        } else {
            smol::fs::remove_dir(path).await
        };
        match result {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound && options.ignore_if_not_exists => {
                Ok(())
            }
            Err(err) => Err(err)?,
        }
    }

    async fn remove_file(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        #[cfg(windows)]
        if let Ok(Some(metadata)) = self.metadata(path).await
            && metadata.is_symlink
            && metadata.is_dir
        {
            self.remove_dir(
                path,
                RemoveOptions {
                    recursive: false,
                    ignore_if_not_exists: true,
                },
            )
            .await?;
            return Ok(());
        }

        match smol::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound && options.ignore_if_not_exists => {
                Ok(())
            }
            Err(err) => Err(err)?,
        }
    }

    async fn trash(&self, path: &Path, _options: RemoveOptions) -> Result<TrashId> {
        // We must make the path absolute or trash will make a weird abomination
        // of the zed working directory (not usually the worktree) and whatever
        // the path variable holds.
        // We deliberately use `std::path::absolute` instead of `canonicalize`
        // to avoid resolving symlinks. Otherwise trashing a symlink would trash
        // its target and leave the link behind.
        let path = std::path::absolute(path).context("Could not make the path absolute")?;

        // trash-rs 无 android 后端（见 Cargo.toml 里该依赖的注释）。android 上
        // 「移到回收站」退化为返回不支持，其余删除语义不变。
        // —— 权宜之计，替换 trash-rs 后移除本分支。——
        //
        // 用 #[cfg] 而非 cfg!()：后者是运行时分支，`trash::delete_with_info`
        // 仍会被编译，android 上依旧找不到 trash crate。
        #[cfg(target_os = "android")]
        {
            anyhow::bail!("Moving to the system trash is not supported on Android (yet)");
        }

        #[cfg(not(target_os = "android"))]
        {
            let entry = smol::unblock(move || trash::delete_with_info(path))
                .await
                .context("Could not trash file or dir")?
                .into();

            Ok(self.trash.lock().insert(entry))
        }
    }

    async fn open_sync(&self, path: &Path) -> Result<Box<dyn io::Read + Send + Sync>> {
        Ok(Box::new(std::fs::File::open(path)?))
    }

    async fn open_handle(&self, path: &Path) -> Result<Arc<dyn crate::FileHandle>> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(windows::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS.0);
        }
        Ok(Arc::new(options.open(path)?))
    }

    async fn load(&self, path: &Path) -> Result<String> {
        let path = path.to_path_buf();
        self.executor
            .spawn(async move {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read file {}", path.display()))
            })
            .await
    }

    async fn load_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let path = path.to_path_buf();
        let bytes = self
            .executor
            .spawn(async move { std::fs::read(path) })
            .await?;
        Ok(bytes)
    }

    #[cfg(not(target_os = "windows"))]
    async fn atomic_write(&self, path: PathBuf, data: String) -> Result<()> {
        smol::unblock(move || {
            // Use the directory of the destination as temp dir to avoid
            // invalid cross-device link error, and XDG_CACHE_DIR for fallback.
            // See https://github.com/zed-industries/zed/pull/8437 for more details.
            let mut tmp_file =
                tempfile::NamedTempFile::new_in(path.parent().unwrap_or(paths::temp_dir()))?;
            tmp_file.write_all(data.as_bytes())?;
            tmp_file.persist(path)?;
            anyhow::Ok(())
        })
        .await?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn atomic_write(&self, path: PathBuf, data: String) -> Result<()> {
        smol::unblock(move || {
            // If temp dir is set to a different drive than the destination,
            // we receive error:
            //
            // failed to persist temporary file:
            // The system cannot move the file to a different disk drive. (os error 17)
            //
            // This is because `ReplaceFileW` does not support cross volume moves.
            // See the remark section: "The backup file, replaced file, and replacement file must all reside on the same volume."
            // https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew#remarks
            //
            // So we use the directory of the destination as a temp dir to avoid it.
            // https://github.com/zed-industries/zed/issues/16571
            let temp_dir = TempDir::new_in(path.parent().unwrap_or(paths::temp_dir()))?;
            let temp_file = {
                let temp_file_path = temp_dir.path().join("temp_file");
                let mut file = std::fs::File::create_new(&temp_file_path)?;
                file.write_all(data.as_bytes())?;
                temp_file_path
            };
            atomic_replace(path.as_path(), temp_file.as_path())?;
            anyhow::Ok(())
        })
        .await?;
        Ok(())
    }

    async fn save(&self, path: &Path, text: &Rope, line_ending: LineEnding) -> Result<()> {
        use smol::io::AsyncWriteExt as _;

        let buffer_size = text.summary().len.min(10 * 1024);
        if let Some(path) = path.parent() {
            self.create_dir(path)
                .await
                .with_context(|| format!("Failed to create directory at {:?}", path))?;
        }
        let file = smol::fs::File::create(path)
            .await
            .with_context(|| format!("Failed to create file at {:?}", path))?;
        let mut writer = smol::io::BufWriter::with_capacity(buffer_size, file);
        for chunk in text::chunks_with_line_ending(text, line_ending) {
            writer.write_all(chunk.as_bytes()).await?;
        }
        writer.flush().await?;
        Ok(())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        if let Some(path) = path.parent() {
            self.create_dir(path)
                .await
                .with_context(|| format!("Failed to create directory at {:?}", path))?;
        }
        let path = path.to_owned();
        let contents = content.to_owned();
        self.executor
            .spawn(async move {
                std::fs::write(path, contents)?;
                Ok(())
            })
            .await
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let path = path.to_owned();
        self.executor
            .spawn(async move {
                #[cfg(target_os = "windows")]
                let result = Self::canonicalize(&path);

                #[cfg(not(target_os = "windows"))]
                let result = std::fs::canonicalize(&path);

                result.with_context(|| format!("canonicalizing {path:?}"))
            })
            .await
    }

    async fn is_file(&self, path: &Path) -> bool {
        let path = path.to_owned();
        self.executor
            .spawn(async move { std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) })
            .await
    }

    async fn is_dir(&self, path: &Path) -> bool {
        let path = path.to_owned();
        self.executor
            .spawn(async move { std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir()) })
            .await
    }

    async fn metadata(&self, path: &Path) -> Result<Option<Metadata>> {
        let path_buf = path.to_owned();
        let symlink_metadata = match self
            .executor
            .spawn(async move { std::fs::symlink_metadata(&path_buf) })
            .await
        {
            Ok(metadata) => metadata,
            Err(err) => {
                return match err.kind() {
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Ok(None),
                    _ => Err(anyhow::Error::new(err)),
                };
            }
        };

        let is_symlink = symlink_metadata.file_type().is_symlink();
        let metadata = if is_symlink {
            let path_buf = path.to_path_buf();
            // Read target metadata, if the target exists
            match self
                .executor
                .spawn(async move { std::fs::metadata(path_buf) })
                .await
            {
                Ok(target_metadata) => target_metadata,
                Err(err) => {
                    if err.kind() != io::ErrorKind::NotFound {
                        // TODO: Also FilesystemLoop when that's stable
                        log::warn!(
                            "Failed to read symlink target metadata for path {path:?}: {err}"
                        );
                    }
                    // For a broken or recursive symlink, return the symlink metadata. (Or
                    // as edge cases, a symlink into a directory we can't read, which is hard
                    // to distinguish from just being broken.)
                    symlink_metadata
                }
            }
        } else {
            symlink_metadata
        };

        #[cfg(unix)]
        let inode = metadata.ino();

        #[cfg(windows)]
        let inode = file_id(path).await?;

        #[cfg(windows)]
        let is_fifo = false;

        #[cfg(unix)]
        let is_fifo = metadata.file_type().is_fifo();

        #[cfg(unix)]
        let is_executable = metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;

        #[cfg(windows)]
        let path_buf = path.to_path_buf();
        #[cfg(windows)]
        let is_executable = self
            .executor
            .spawn(async move { path_buf.is_executable() })
            .await;

        Ok(Some(Metadata {
            inode,
            mtime: MTime(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
            len: metadata.len(),
            is_symlink,
            is_dir: metadata.file_type().is_dir(),
            is_fifo,
            is_executable,
            is_writable: !metadata.permissions().readonly(),
        }))
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf> {
        let path = path.to_owned();
        let path = self
            .executor
            .spawn(async move { std::fs::read_link(&path) })
            .await?;
        Ok(path)
    }

    async fn read_dir(
        &self,
        path: &Path,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PathBuf>>>>> {
        let path = path.to_owned();
        let entries = self
            .executor
            .spawn(async move { read_dir_entries(path) })
            .await?;
        Ok(Box::pin(iter(entries)))
    }

    fn start_native_watcher(&self) -> Result<()> { self.native_watcher.ensure_backend() }

    fn record_watcher_diagnostics(&self) -> Option<fs_watcher::WatchRecording> {
        Some(fs_watcher::WatchRecording::new([
            self.native_watcher.clone(),
            self.poll_watcher.clone(),
        ]))
    }

    fn path_exists(&self, path: &Path) -> bool { std::fs::symlink_metadata(path).is_ok() }

    fn is_path_case_sensitive(&self, path: &Path) -> bool {
        !fs_watcher::case_insensitive_path(path)
    }

    fn requires_poll_watcher(&self, path: &Path) -> bool { fs_watcher::requires_poll_watcher(path) }

    async fn watch(
        &self,
        path: &Path,
        latency: Duration,
    ) -> (
        Pin<Box<dyn Send + Stream<Item = Vec<PathEvent>>>>,
        Arc<dyn Watcher>,
    ) {
        let this = self
            .this
            .upgrade()
            .expect("RealFs is only constructed inside an Arc");
        fs_watcher::watch(
            this,
            self.native_watcher.clone(),
            self.poll_watcher.clone(),
            self.executor.clone(),
            path,
            latency,
        )
        .await
    }

    fn open_repo(
        &self,
        dotgit_path: &Path,
        system_git_binary_path: Option<&Path>,
    ) -> Result<Arc<dyn GitRepository>> {
        Ok(Arc::new(RealGitRepository::new(
            dotgit_path,
            self.bundled_git_binary_path.clone(),
            system_git_binary_path.map(|path| path.to_path_buf()),
            self.executor.clone(),
        )?))
    }

    async fn git_init(
        &self,
        abs_work_directory_path: &Path,
        fallback_branch_name: String,
    ) -> Result<()> {
        use util::command::new_command;

        let result = new_command("git")
            .current_dir(abs_work_directory_path)
            .args(&["config", "--global", "--get", "init.defaultBranch"])
            .output()
            .await;

        // In case the `git config` command fails, which would be the case if
        // the user doesn't have an `init.defaultBranch` value set, we'll just
        // default to the provided `fallback_branch_name`.
        let branch_name = match result {
            Ok(output) if !output.stdout.is_empty() => String::from_utf8(output.stdout)?,
            _ => fallback_branch_name,
        };

        new_command("git")
            .current_dir(abs_work_directory_path)
            .args(&["init", "-b"])
            .arg(branch_name.trim())
            .output()
            .await?;

        Ok(())
    }

    async fn git_clone(&self, abs_work_directory: &Path, repo_url: &str) -> Result<()> {
        use util::command::{Stdio, new_command};

        let job_id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
        let job_info = JobInfo {
            id: job_id,
            start: Instant::now(),
            message: SharedString::from(format!("Cloning {}", repo_url)),
        };

        let job_tracker = JobTracker::new(job_info, self.job_event_subscribers.clone());
        let mut child = new_command("git")
            .current_dir(abs_work_directory)
            .args(["clone", "--progress", repo_url])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stderr = child
            .stderr
            .take()
            .context("failed to read git clone progress")?;
        let stderr_output = git_clone_progress::read(stderr, |message| {
            job_tracker.update(message.into());
        })
        .await?;
        let status = child.status().await?;

        if !status.success() {
            anyhow::bail!(
                "git clone failed: {}",
                git_clone_progress::failure_message(&stderr_output)
            );
        }

        Ok(())
    }

    /// Runs `git config` with the given arguments.
    /// Will return `Ok` if the commands exit status is `0`, with the stdout
    /// contents. Otherwise returns `Err` with the stderr contents.
    async fn git_config(&self, abs_work_directory: &Path, args: Vec<String>) -> Result<String> {
        use util::command::new_command;

        let output = new_command("git")
            .current_dir(abs_work_directory)
            .args([String::from("config")].into_iter().chain(args))
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8(output.stderr)?;
            anyhow::bail!(err);
        }

        String::from_utf8(output.stdout).map_err(Into::into)
    }

    fn is_fake(&self) -> bool { false }

    fn subscribe_to_jobs(&self) -> crate::jobs::JobEventReceiver {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        self.job_event_subscribers.lock().push(sender);
        receiver
    }

    /// Checks whether the file system is case sensitive by attempting to create two files
    /// that have the same name except for the casing.
    ///
    /// It creates both files in a temporary directory it removes at the end.
    async fn is_case_sensitive(&self) -> bool {
        const UNINITIALIZED: u8 = 0;
        const CASE_SENSITIVE: u8 = 1;
        const NOT_CASE_SENSITIVE: u8 = 2;

        // Note we could CAS here, but really, if we race we do this work twice at worst which isn't a big deal.
        let load = self.is_case_sensitive.load(Ordering::Acquire);
        if load != UNINITIALIZED {
            return load == CASE_SENSITIVE;
        }
        let temp_dir = self.executor.spawn(async { TempDir::new() });
        let res = maybe!(async {
            let temp_dir = temp_dir.await?;
            let test_file_1 = temp_dir.path().join("case_sensitivity_test.tmp");
            let test_file_2 = temp_dir.path().join("CASE_SENSITIVITY_TEST.TMP");

            let create_opts = CreateOptions {
                overwrite: false,
                ignore_if_exists: false,
            };

            // Create file1
            self.create_file(&test_file_1, create_opts).await?;

            // Now check whether it's possible to create file2
            let case_sensitive = match self.create_file(&test_file_2, create_opts).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    if let Some(io_error) = e.downcast_ref::<io::Error>() {
                        if io_error.kind() == io::ErrorKind::AlreadyExists {
                            Ok(false)
                        } else {
                            Err(e)
                        }
                    } else {
                        Err(e)
                    }
                }
            };

            temp_dir.close()?;
            case_sensitive
        }).await.unwrap_or_else(|e| {
            log::error!(
                "Failed to determine whether filesystem is case sensitive (falling back to true) due to error: {e:#}"
            );
            true
        });
        self.is_case_sensitive.store(
            if res {
                CASE_SENSITIVE
            } else {
                NOT_CASE_SENSITIVE
            },
            Ordering::Release,
        );
        res
    }

    fn original_path_for_trash_id(&self, trash_id: TrashId) -> Option<PathBuf> {
        self.trash
            .lock()
            .get(trash_id)
            .map(|entry| entry.original_parent.join(&entry.name))
    }

    async fn restore(&self, trash_id: TrashId) -> std::result::Result<PathBuf, TrashRestoreError> {
        let trashed_entry = self
            .trash
            .lock()
            .get(trash_id)
            .cloned()
            .ok_or(TrashRestoreError::AlreadyRestored)?;

        let restored_item_path = trashed_entry.original_parent.join(&trashed_entry.name);

        // 同 trash()：trash-rs 无 android 后端，android 上恢复退化为返回不支持。
        // —— 权宜之计，替换 trash-rs 后移除本分支。——
        #[cfg(target_os = "android")]
        {
            return Err(TrashRestoreError::Unknown {
                description: "Restoring from the system trash is not supported on Android (yet)"
                    .into(),
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::Builder::new()
                .name("restore trashed item".to_string())
                .spawn(move || {
                    let res = trash::restore_all([trashed_entry.into_trash_item()]);
                    tx.send(res)
                })
                .expect("The OS can spawn a threads");

            rx.await.expect("Restore all never panics")?;
            self.trash.lock().remove(trash_id);
            Ok(restored_item_path)
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
impl Watcher for RealWatcher {
    fn add(&self, _: &Path) -> Result<()> { Ok(()) }

    fn remove(&self, _: &Path) -> Result<()> { Ok(()) }
}
