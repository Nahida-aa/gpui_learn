use crate::fake_git_repo;
use crate::fake_git_repo::{
    FakeBlobReadGate, FakeCommitDataEntry, FakeGitRepository, FakeGitRepositoryState,
};
use crate::file_handle::FileHandle;
use crate::fs_watcher;
use crate::jobs::{JobEventReceiver, JobEventSender};
use crate::metadata::{MTime, Metadata};
use crate::options::{CopyOptions, CreateOptions, RemoveOptions, RenameOptions};
use crate::read_dir_items;
use crate::traits::Fs;
use crate::trash::{TrashId, TrashRestoreError, TrashedEntry};
use crate::watcher::{PathEvent, PathEventKind, Watcher};
use anyhow::{Context as _, Result};
use async_tar::Archive;
use collections::{BTreeMap, btree_map};
use futures::StreamExt;
use futures::{AsyncRead, Stream};
#[cfg(feature = "test-support")]
use git::repository::GitRepository;
use git::{
    repository::{CommitData, InitialGraphCommitData, RepoPath, Worktree, repo_path},
    status::{FileStatus, StatusCode, TrackedStatus, UnmergedStatus},
};
use gpui::BackgroundExecutor;
use parking_lot::Mutex;
use path::normalize_path;
use rope::Rope;
use slotmap::SlotMap;
use smol::io::AsyncReadExt;
use std::ffi::OsStr;
use std::{
    io::{self, Write},
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use text::LineEnding; // 只是示意；实际按需导入

// ...（FakeFs / FakeFsEntry / FakeWatches / FakeWatchBackend / FakeHandle
//     以及所有 `impl Fs for FakeFs` 的代码，从原文件按原样搬迁即可）

#[cfg(feature = "test-support")]
pub struct FakeFs {
    this: std::sync::Weak<Self>,
    // Use an unfair lock to ensure tests are deterministic.
    state: Arc<Mutex<FakeFsState>>,
    executor: gpui::BackgroundExecutor,
    native_watcher: Arc<fs_watcher::OsWatcher>,
    poll_watcher: Arc<fs_watcher::OsWatcher>,
}

#[cfg(feature = "test-support")]
struct FakeFsState {
    root: FakeFsEntry,
    next_inode: u64,
    next_mtime: SystemTime,
    git_event_tx: async_channel::Sender<PathBuf>,
    watch_roots: Vec<(PathBuf, std::sync::Weak<dyn Watcher>)>,
    watches: FakeWatches,
    events_paused: bool,
    buffered_events: Vec<PathEvent>,
    metadata_call_count: usize,
    read_dir_call_count: usize,
    path_write_counts: std::collections::HashMap<PathBuf, usize>,
    job_event_subscribers: Arc<Mutex<Vec<JobEventSender>>>,
    trash: Mutex<SlotMap<TrashId, (TrashedEntry, FakeFsEntry)>>,
    remove_dir_errors: std::collections::HashMap<PathBuf, String>,
    case_sensitive: bool,
}

/// The kernel's side of file watching, as far as the real watcher code above
/// notify can tell: the paths the backend has registered and the callback that
/// delivers events for them into the native `OsWatcher`.
#[cfg(feature = "test-support")]
#[derive(Default)]
struct FakeWatches {
    registered_paths: Vec<PathBuf>,
    watch_calls: Vec<PathBuf>,
    event_sink: Option<Box<dyn Fn(notify::Result<notify::Event>) + Send + Sync>>,
}

#[cfg(feature = "test-support")]
#[derive(Clone, Debug)]
pub(crate) enum FakeFsEntry {
    File {
        inode: u64,
        mtime: MTime,
        len: u64,
        content: Vec<u8>,
        // The path to the repository state directory, if this is a gitfile.
        git_dir_path: Option<PathBuf>,
    },
    Dir {
        inode: u64,
        mtime: MTime,
        len: u64,
        entries: BTreeMap<String, FakeFsEntry>,
        git_repo_state: Option<Arc<Mutex<FakeGitRepositoryState>>>,
    },
    Symlink {
        target: PathBuf,
    },
}

#[cfg(feature = "test-support")]
impl PartialEq for FakeFsEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::File {
                    inode: l_inode,
                    mtime: l_mtime,
                    len: l_len,
                    content: l_content,
                    git_dir_path: l_git_dir_path,
                },
                Self::File {
                    inode: r_inode,
                    mtime: r_mtime,
                    len: r_len,
                    content: r_content,
                    git_dir_path: r_git_dir_path,
                },
            ) => {
                l_inode == r_inode
                    && l_mtime == r_mtime
                    && l_len == r_len
                    && l_content == r_content
                    && l_git_dir_path == r_git_dir_path
            }
            (
                Self::Dir {
                    inode: l_inode,
                    mtime: l_mtime,
                    len: l_len,
                    entries: l_entries,
                    git_repo_state: l_git_repo_state,
                },
                Self::Dir {
                    inode: r_inode,
                    mtime: r_mtime,
                    len: r_len,
                    entries: r_entries,
                    git_repo_state: r_git_repo_state,
                },
            ) => {
                let same_repo_state = match (l_git_repo_state.as_ref(), r_git_repo_state.as_ref()) {
                    (Some(l), Some(r)) => Arc::ptr_eq(l, r),
                    (None, None) => true,
                    _ => false,
                };
                l_inode == r_inode
                    && l_mtime == r_mtime
                    && l_len == r_len
                    && l_entries == r_entries
                    && same_repo_state
            }
            (Self::Symlink { target: l_target }, Self::Symlink { target: r_target }) => {
                l_target == r_target
            }
            _ => false,
        }
    }
}

#[cfg(feature = "test-support")]
impl FakeFsState {
    fn get_and_increment_mtime(&mut self) -> MTime {
        let mtime = self.next_mtime;
        self.next_mtime += FakeFs::SYSTEMTIME_INTERVAL;
        MTime(mtime)
    }

    fn get_and_increment_inode(&mut self) -> u64 {
        let inode = self.next_inode;
        self.next_inode += 1;
        inode
    }

    fn canonicalize(&self, target: &Path, follow_symlink: bool) -> Option<PathBuf> {
        let mut canonical_path = PathBuf::new();
        let mut path = target.to_path_buf();
        let mut entry_stack = Vec::new();
        'outer: loop {
            let mut path_components = path.components().peekable();
            let mut prefix = None;
            while let Some(component) = path_components.next() {
                match component {
                    Component::Prefix(prefix_component) => prefix = Some(prefix_component),
                    Component::RootDir => {
                        entry_stack.clear();
                        entry_stack.push(&self.root);
                        canonical_path.clear();
                        match prefix {
                            Some(prefix_component) => {
                                canonical_path = PathBuf::from(prefix_component.as_os_str());
                                // Prefixes like `C:\\` are represented without their trailing slash, so we have to re-add it.
                                canonical_path.push(std::path::MAIN_SEPARATOR_STR);
                            }
                            None => canonical_path = PathBuf::from(std::path::MAIN_SEPARATOR_STR),
                        }
                    }
                    Component::CurDir => {}
                    Component::ParentDir => {
                        entry_stack.pop()?;
                        canonical_path.pop();
                    }
                    Component::Normal(name) => {
                        let current_entry = *entry_stack.last()?;
                        if let FakeFsEntry::Dir { entries, .. } = current_entry {
                            let name_str = name.to_str().unwrap();
                            let (canonical_name, entry) = match entries.get(name_str) {
                                Some(entry) => (name_str, entry),
                                None => {
                                    if !self.case_sensitive {
                                        entries
                                            .iter()
                                            .find(|(key, _)| key.eq_ignore_ascii_case(name_str))
                                            .map(|(key, entry)| (key.as_str(), entry))?
                                    } else {
                                        return None;
                                    }
                                }
                            };
                            if (path_components.peek().is_some() || follow_symlink)
                                && let FakeFsEntry::Symlink { target, .. } = entry
                            {
                                let mut target = target.clone();
                                target.extend(path_components);
                                path = target;
                                continue 'outer;
                            }
                            entry_stack.push(entry);
                            canonical_path = canonical_path.join(canonical_name);
                        } else {
                            return None;
                        }
                    }
                }
            }
            break;
        }

        if entry_stack.is_empty() {
            None
        } else {
            Some(canonical_path)
        }
    }

    fn try_entry(
        &mut self,
        target: &Path,
        follow_symlink: bool,
    ) -> Option<(&mut FakeFsEntry, PathBuf)> {
        let canonical_path = self.canonicalize(target, follow_symlink)?;

        let mut components = canonical_path
            .components()
            .skip_while(|component| matches!(component, Component::Prefix(_)));
        let Some(Component::RootDir) = components.next() else {
            panic!(
                "the path {:?} was not canonicalized properly {:?}",
                target, canonical_path
            )
        };

        let mut entry = &mut self.root;
        for component in components {
            match component {
                Component::Normal(name) => {
                    if let FakeFsEntry::Dir { entries, .. } = entry {
                        entry = entries.get_mut(name.to_str().unwrap())?;
                    } else {
                        return None;
                    }
                }
                _ => {
                    panic!(
                        "the path {:?} was not canonicalized properly {:?}",
                        target, canonical_path
                    )
                }
            }
        }

        Some((entry, canonical_path))
    }

    fn entry(&mut self, target: &Path) -> Result<&mut FakeFsEntry> {
        Ok(self
            .try_entry(target, true)
            .ok_or_else(|| {
                anyhow::anyhow!(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("not found: {target:?}")
                ))
            })?
            .0)
    }

    fn write_path<Fn, T>(&mut self, path: &Path, callback: Fn) -> Result<T>
    where
        Fn: FnOnce(btree_map::Entry<String, FakeFsEntry>) -> Result<T>,
    {
        let path = normalize_path(path);
        let filename = path.file_name().context("cannot overwrite the root")?;
        let parent_path = path.parent().unwrap();

        let parent = self.entry(parent_path)?;
        let new_entry = parent
            .dir_entries(parent_path)?
            .entry(filename.to_str().unwrap().into());
        callback(new_entry)
    }

    fn emit_event<I, T>(&mut self, paths: I)
    where
        I: IntoIterator<Item = (T, Option<PathEventKind>)>,
        T: Into<PathBuf>,
    {
        self.buffered_events
            .extend(paths.into_iter().map(|(path, kind)| PathEvent {
                path: path.into(),
                kind,
            }));

        if !self.events_paused {
            self.flush_events(self.buffered_events.len());
        }
    }

    fn flush_events(&mut self, mut count: usize) {
        use notify::event::{CreateKind, Flag, ModifyKind, RemoveKind};

        count = count.min(self.buffered_events.len());
        let events = self.buffered_events.drain(0..count).collect::<Vec<_>>();
        let Some(event_sink) = &self.watches.event_sink else {
            return;
        };
        for event in events {
            let is_registered = self.watches.registered_paths.iter().any(|registered_path| {
                if self.case_sensitive {
                    event.path.starts_with(registered_path)
                } else {
                    let event_path = event.path.to_string_lossy().to_lowercase();
                    let registered_path = registered_path.to_string_lossy().to_lowercase();
                    Path::new(&event_path).starts_with(Path::new(&registered_path))
                }
            });
            if !is_registered {
                continue;
            }
            let notify_event = match event.kind {
                Some(PathEventKind::Created) => {
                    notify::Event::new(notify::EventKind::Create(CreateKind::Any))
                }
                Some(PathEventKind::Changed) => {
                    notify::Event::new(notify::EventKind::Modify(ModifyKind::Any))
                }
                Some(PathEventKind::Removed) => {
                    notify::Event::new(notify::EventKind::Remove(RemoveKind::Any))
                }
                Some(PathEventKind::Rescan) => {
                    notify::Event::new(notify::EventKind::Other).set_flag(Flag::Rescan)
                }
                None => notify::Event::new(notify::EventKind::Any),
            };
            event_sink(Ok(notify_event.add_path(event.path)));
        }
    }
}

/// Stands in for notify at the boundary the real watcher code talks to: the
/// fake filesystem's own mutations are the "kernel" events.
#[cfg(feature = "test-support")]
struct FakeWatchBackend {
    state: Arc<Mutex<FakeFsState>>,
}

#[cfg(feature = "test-support")]
impl fs_watcher::WatchBackend for FakeWatchBackend {
    fn watch(&mut self, path: &Path, _mode: notify::RecursiveMode) -> notify::Result<()> {
        let path = normalize_path(path);
        let mut state = self.state.try_lock().expect(
            "fake filesystem state is locked; this execution would have caused a test hang",
        );
        state.watches.watch_calls.push(path.clone());
        state.watches.registered_paths.push(path);
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
        let path = normalize_path(path);
        self.state
            .lock()
            .watches
            .registered_paths
            .retain(|registered_path| *registered_path != path);
        Ok(())
    }
}

#[cfg(feature = "test-support")]
pub static FS_DOT_GIT: std::sync::LazyLock<&'static OsStr> =
    std::sync::LazyLock::new(|| OsStr::new(".git"));

#[cfg(feature = "test-support")]
impl FakeFs {
    /// We need to use something large enough for Windows and Unix to consider this a new file.
    /// https://doc.rust-lang.org/nightly/std/time/struct.SystemTime.html#platform-specific-behavior
    const SYSTEMTIME_INTERVAL: Duration = Duration::from_nanos(100);

    pub fn new(executor: gpui::BackgroundExecutor) -> Arc<Self> {
        let (tx, rx) = async_channel::bounded::<PathBuf>(10);

        let state = Arc::new(Mutex::new(FakeFsState {
            root: FakeFsEntry::Dir {
                inode: 0,
                mtime: MTime(UNIX_EPOCH),
                len: 0,
                entries: Default::default(),
                git_repo_state: None,
            },
            git_event_tx: tx,
            next_mtime: UNIX_EPOCH + Self::SYSTEMTIME_INTERVAL,
            next_inode: 1,
            watch_roots: Vec::new(),
            watches: FakeWatches::default(),
            buffered_events: Vec::new(),
            events_paused: false,
            read_dir_call_count: 0,
            metadata_call_count: 0,
            path_write_counts: Default::default(),
            job_event_subscribers: Arc::new(Mutex::new(Vec::new())),
            trash: Mutex::new(SlotMap::with_key()),
            remove_dir_errors: Default::default(),
            case_sensitive: true,
        }));
        let native_watcher = fs_watcher::OsWatcher::with_backend(
            fs_watcher::OsWatcherKind::Native,
            executor.clone(),
            Some(Box::new(FakeWatchBackend {
                state: state.clone(),
            })),
        );
        state.lock().watches.event_sink = Some(Box::new(native_watcher.event_sink()));
        let poll_watcher =
            fs_watcher::OsWatcher::new(fs_watcher::OsWatcherKind::Poll, executor.clone());

        let this = Arc::new_cyclic(|this| Self {
            this: this.clone(),
            executor: executor.clone(),
            state,
            native_watcher,
            poll_watcher,
        });

        executor.spawn({
            let this = this.clone();
            async move {
                while let Ok(git_event) = rx.recv().await {
                    if let Some(mut state) = this.state.try_lock() {
                        state.emit_event([(git_event, Some(PathEventKind::Changed))]);
                    } else {
                        panic!("Failed to lock file system state, this execution would have caused a test hang");
                    }
                }
            }
        }).detach();

        this
    }

    /// Configures whether the fake filesystem reports as case-sensitive.
    pub fn set_case_sensitive(&self, case_sensitive: bool) {
        self.state.lock().case_sensitive = case_sensitive;
    }

    pub fn set_next_mtime(&self, next_mtime: SystemTime) {
        let mut state = self.state.lock();
        state.next_mtime = next_mtime;
    }

    pub fn get_and_increment_mtime(&self) -> MTime {
        let mut state = self.state.lock();
        state.get_and_increment_mtime()
    }

    pub async fn touch_path(&self, path: impl AsRef<Path>) {
        let mut state = self.state.lock();
        let path = path.as_ref();
        let new_mtime = state.get_and_increment_mtime();
        let new_inode = state.get_and_increment_inode();
        state
            .write_path(path, move |entry| {
                match entry {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(FakeFsEntry::File {
                            inode: new_inode,
                            mtime: new_mtime,
                            content: Vec::new(),
                            len: 0,
                            git_dir_path: None,
                        });
                    }
                    btree_map::Entry::Occupied(mut e) => match &mut *e.get_mut() {
                        FakeFsEntry::File { mtime, .. } => *mtime = new_mtime,
                        FakeFsEntry::Dir { mtime, .. } => *mtime = new_mtime,
                        FakeFsEntry::Symlink { .. } => {}
                    },
                }
                Ok(())
            })
            .unwrap();
        state.emit_event([(path.to_path_buf(), Some(PathEventKind::Changed))]);
    }

    pub async fn insert_file(&self, path: impl AsRef<Path>, content: Vec<u8>) {
        self.write_file_internal(path, content, true).unwrap()
    }

    pub async fn insert_symlink(&self, path: impl AsRef<Path>, target: PathBuf) {
        let mut state = self.state.lock();
        let path = path.as_ref();
        let file = FakeFsEntry::Symlink { target };
        state
            .write_path(path.as_ref(), move |e| match e {
                btree_map::Entry::Vacant(e) => {
                    e.insert(file);
                    Ok(())
                }
                btree_map::Entry::Occupied(mut e) => {
                    *e.get_mut() = file;
                    Ok(())
                }
            })
            .unwrap();
        state.emit_event([(path, Some(PathEventKind::Created))]);
    }

    pub(crate) fn write_file_internal(
        &self,
        path: impl AsRef<Path>,
        new_content: Vec<u8>,
        recreate_inode: bool,
    ) -> Result<()> {
        fn inner(
            this: &FakeFs,
            path: &Path,
            new_content: Vec<u8>,
            recreate_inode: bool,
        ) -> Result<()> {
            let mut state = this.state.lock();
            let path_buf = path.to_path_buf();
            *state.path_write_counts.entry(path_buf).or_insert(0) += 1;
            let new_inode = state.get_and_increment_inode();
            let new_mtime = state.get_and_increment_mtime();
            let new_len = new_content.len() as u64;
            let mut kind = None;
            state.write_path(path, |entry| {
                match entry {
                    btree_map::Entry::Vacant(e) => {
                        kind = Some(PathEventKind::Created);
                        e.insert(FakeFsEntry::File {
                            inode: new_inode,
                            mtime: new_mtime,
                            len: new_len,
                            content: new_content,
                            git_dir_path: None,
                        });
                    }
                    btree_map::Entry::Occupied(mut e) => {
                        kind = Some(PathEventKind::Changed);
                        if let FakeFsEntry::File {
                            inode,
                            mtime,
                            len,
                            content,
                            ..
                        } = e.get_mut()
                        {
                            *mtime = new_mtime;
                            *content = new_content;
                            *len = new_len;
                            if recreate_inode {
                                *inode = new_inode;
                            }
                        } else {
                            anyhow::bail!("not a file")
                        }
                    }
                }
                Ok(())
            })?;
            state.emit_event([(path, kind)]);
            Ok(())
        }
        inner(self, path.as_ref(), new_content, recreate_inode)
    }

    pub fn read_file_sync(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let path = normalize_path(path);
        let mut state = self.state.lock();
        let entry = state.entry(&path)?;
        entry.file_content(&path).cloned()
    }

    async fn load_internal(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let path = normalize_path(path);
        self.simulate_random_delay().await;
        let mut state = self.state.lock();
        let entry = state.entry(&path)?;
        entry.file_content(&path).cloned()
    }

    pub fn pause_events(&self) { self.state.lock().events_paused = true; }

    pub fn unpause_events_and_flush(&self) {
        self.state.lock().events_paused = false;
        self.flush_events(usize::MAX);
    }

    pub fn buffered_event_count(&self) -> usize { self.state.lock().buffered_events.len() }

    pub fn clear_buffered_events(&self) { self.state.lock().buffered_events.clear(); }

    /// Simulates the kernel's watch queue overflowing: all buffered
    /// (undelivered) events are lost, and the watcher reports only a single
    /// `Rescan` event for `root`, mirroring how the native backends report
    /// lost sync (FSEvents `kFSEventStreamEventFlagMustScanSubDirs`, inotify
    /// `IN_Q_OVERFLOW`, Windows `ERROR_NOTIFY_ENUM_DIR`).
    ///
    /// Note that the fake file system's state is unaffected; like a real
    /// overflow, only the notifications are lost, not the changes themselves.
    pub fn simulate_watcher_overflow(&self, root: impl Into<PathBuf>) {
        let mut state = self.state.lock();
        state.buffered_events.clear();
        state.emit_event([(root, Some(PathEventKind::Rescan))]);
    }

    /// Every path the watcher backend has been asked to watch, in order,
    /// including paths that were later unwatched.
    pub fn watch_calls(&self) -> Vec<PathBuf> { self.state.lock().watches.watch_calls.clone() }

    pub fn flush_events(&self, count: usize) { self.state.lock().flush_events(count); }

    pub(crate) fn entry(&self, target: &Path) -> Result<FakeFsEntry> {
        self.state.lock().entry(target).cloned()
    }

    pub(crate) fn insert_entry(&self, target: &Path, new_entry: FakeFsEntry) -> Result<()> {
        let mut state = self.state.lock();
        state.write_path(target, |entry| {
            match entry {
                btree_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(new_entry);
                }
                btree_map::Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.insert(new_entry);
                }
            }
            Ok(())
        })
    }

    #[must_use]
    pub fn insert_tree<'a>(
        &'a self,
        path: impl 'a + AsRef<Path> + Send,
        tree: serde_json::Value,
    ) -> futures::future::BoxFuture<'a, ()> {
        use futures::FutureExt as _;
        use serde_json::Value::*;

        fn inner<'a>(
            this: &'a FakeFs,
            path: Arc<Path>,
            tree: serde_json::Value,
        ) -> futures::future::BoxFuture<'a, ()> {
            async move {
                match tree {
                    Object(map) => {
                        this.create_dir(&path).await.unwrap();
                        for (name, contents) in map {
                            let mut path = PathBuf::from(path.as_ref());
                            path.push(name);
                            this.insert_tree(&path, contents).await;
                        }
                    }
                    Null => {
                        this.create_dir(&path).await.unwrap();
                    }
                    String(contents) => {
                        this.insert_file(&path, contents.into_bytes()).await;
                    }
                    _ => {
                        panic!("JSON object must contain only objects, strings, or null");
                    }
                }
            }
            .boxed()
        }
        inner(self, Arc::from(path.as_ref()), tree)
    }

    pub fn insert_tree_from_real_fs<'a>(
        &'a self,
        path: impl 'a + AsRef<Path> + Send,
        src_path: impl 'a + AsRef<Path> + Send,
    ) -> futures::future::BoxFuture<'a, ()> {
        use futures::FutureExt as _;

        async move {
            let path = path.as_ref();
            if std::fs::metadata(&src_path).unwrap().is_file() {
                let contents = std::fs::read(src_path).unwrap();
                self.insert_file(path, contents).await;
            } else {
                self.create_dir(path).await.unwrap();
                for entry in std::fs::read_dir(&src_path).unwrap() {
                    let entry = entry.unwrap();
                    self.insert_tree_from_real_fs(path.join(entry.file_name()), entry.path())
                        .await;
                }
            }
        }
        .boxed()
    }

    pub fn with_git_state_and_paths<T, F>(
        &self,
        dot_git: &Path,
        emit_git_event: bool,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&mut FakeGitRepositoryState, &Path, &Path) -> T,
    {
        let mut state = self.state.lock();
        let git_event_tx = state.git_event_tx.clone();
        let entry = state.entry(dot_git).context("open .git")?;

        if let FakeFsEntry::Dir { git_repo_state, .. } = entry {
            let repo_state = git_repo_state.get_or_insert_with(|| {
                log::debug!("insert git state for {dot_git:?}");
                Arc::new(Mutex::new(FakeGitRepositoryState::new(git_event_tx)))
            });
            let mut repo_state = repo_state.lock();

            let result = f(&mut repo_state, dot_git, dot_git);

            drop(repo_state);
            if emit_git_event {
                state.emit_event([(
                    dot_git.join("fake_git_repo_event"),
                    Some(PathEventKind::Changed),
                )]);
            }

            Ok(result)
        } else if let FakeFsEntry::File {
            content,
            git_dir_path,
            ..
        } = &mut *entry
        {
            let path = match git_dir_path {
                Some(path) => path,
                None => {
                    let path = std::str::from_utf8(content)
                        .ok()
                        .and_then(|content| content.strip_prefix("gitdir:"))
                        .context("not a valid gitfile")?
                        .trim();
                    git_dir_path.insert(normalize_path(&dot_git.parent().unwrap().join(path)))
                }
            }
            .clone();
            let Some((git_dir_entry, canonical_path)) = state.try_entry(&path, true) else {
                anyhow::bail!("pointed-to git dir {path:?} not found")
            };
            let FakeFsEntry::Dir {
                git_repo_state,
                entries,
                ..
            } = git_dir_entry
            else {
                anyhow::bail!("gitfile points to a non-directory")
            };
            let common_dir = if let Some(child) = entries.get("commondir") {
                let raw = std::str::from_utf8(child.file_content("commondir".as_ref())?)
                    .context("commondir content")?
                    .trim();
                let raw_path = Path::new(raw);
                if raw_path.is_relative() {
                    normalize_path(&canonical_path.join(raw_path))
                } else {
                    raw_path.to_owned()
                }
            } else {
                canonical_path.clone()
            };
            let repo_state = git_repo_state.get_or_insert_with(|| {
                Arc::new(Mutex::new(FakeGitRepositoryState::new(git_event_tx)))
            });
            let mut repo_state = repo_state.lock();

            let result = f(&mut repo_state, &canonical_path, &common_dir);

            if emit_git_event {
                drop(repo_state);
                state.emit_event([(
                    canonical_path.join("fake_git_repo_event"),
                    Some(PathEventKind::Changed),
                )]);
            }

            Ok(result)
        } else {
            anyhow::bail!("not a valid git repository");
        }
    }

    pub fn with_git_state<T, F>(&self, dot_git: &Path, emit_git_event: bool, f: F) -> Result<T>
    where
        F: FnOnce(&mut FakeGitRepositoryState) -> T,
    {
        self.with_git_state_and_paths(dot_git, emit_git_event, |state, _, _| f(state))
    }

    pub fn set_branch_name(&self, dot_git: &Path, branch: Option<impl Into<String>>) {
        self.with_git_state(dot_git, true, |state| {
            let branch = branch.map(Into::into);
            state.branches.extend(branch.clone());
            state.current_branch_name = branch
        })
        .unwrap();
    }

    pub fn set_remote_for_repo(
        &self,
        dot_git: &Path,
        name: impl Into<String>,
        url: impl Into<String>,
    ) {
        self.with_git_state(dot_git, true, |state| {
            state.remotes.insert(name.into(), url.into());
        })
        .unwrap();
    }

    pub fn insert_branches(&self, dot_git: &Path, branches: &[&str]) {
        self.with_git_state(dot_git, true, |state| {
            if let Some(first) = branches.first()
                && state.current_branch_name.is_none()
            {
                state.current_branch_name = Some(first.to_string())
            }
            state
                .branches
                .extend(branches.iter().map(ToString::to_string));
        })
        .unwrap();
    }

    pub async fn add_linked_worktree_for_repo(
        &self,
        dot_git: &Path,
        emit_git_event: bool,
        worktree: Worktree,
    ) {
        let ref_name = worktree
            .ref_name
            .as_ref()
            .expect("linked worktree must have a ref_name");
        let branch_name = ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(ref_name.as_ref());

        // Create ref in git state.
        self.with_git_state(dot_git, false, |state| {
            state
                .refs
                .insert(ref_name.to_string(), worktree.sha.to_string());
        })
        .unwrap();

        // Create .git/worktrees/<name>/ directory with HEAD, commondir, and gitdir.
        let worktrees_entry_dir = dot_git.join("worktrees").join(branch_name);
        self.create_dir(&worktrees_entry_dir).await.unwrap();

        self.write_file_internal(
            worktrees_entry_dir.join("HEAD"),
            format!("ref: {ref_name}").into_bytes(),
            false,
        )
        .unwrap();

        self.write_file_internal(
            worktrees_entry_dir.join("commondir"),
            dot_git.to_string_lossy().into_owned().into_bytes(),
            false,
        )
        .unwrap();

        let worktree_dot_git = worktree.path.join(".git");
        self.write_file_internal(
            worktrees_entry_dir.join("gitdir"),
            worktree_dot_git.to_string_lossy().into_owned().into_bytes(),
            false,
        )
        .unwrap();

        // Create the worktree checkout directory with a .git file pointing back.
        self.create_dir(&worktree.path).await.unwrap();

        self.write_file_internal(
            &worktree_dot_git,
            format!("gitdir: {}", worktrees_entry_dir.display()).into_bytes(),
            false,
        )
        .unwrap();

        if emit_git_event {
            self.with_git_state(dot_git, true, |_| {}).unwrap();
        }
    }

    pub async fn remove_worktree_for_repo(
        &self,
        dot_git: &Path,
        emit_git_event: bool,
        ref_name: &str,
    ) {
        let branch_name = ref_name.strip_prefix("refs/heads/").unwrap_or(ref_name);
        let worktrees_entry_dir = dot_git.join("worktrees").join(branch_name);

        // Read gitdir to find the worktree checkout path.
        let gitdir_content = self
            .load_internal(worktrees_entry_dir.join("gitdir"))
            .await
            .unwrap();
        let gitdir_str = String::from_utf8(gitdir_content).unwrap();
        let worktree_path = PathBuf::from(gitdir_str.trim())
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default();

        // Remove the worktree checkout directory.
        self.remove_dir(
            &worktree_path,
            RemoveOptions {
                recursive: true,
                ignore_if_not_exists: true,
            },
        )
        .await
        .unwrap();

        // Remove the .git/worktrees/<name>/ directory.
        self.remove_dir(
            &worktrees_entry_dir,
            RemoveOptions {
                recursive: true,
                ignore_if_not_exists: false,
            },
        )
        .await
        .unwrap();

        if emit_git_event {
            self.with_git_state(dot_git, true, |_| {}).unwrap();
        }
    }

    pub fn set_unmerged_paths_for_repo(
        &self,
        dot_git: &Path,
        unmerged_state: &[(RepoPath, UnmergedStatus)],
    ) {
        self.with_git_state(dot_git, true, |state| {
            state.unmerged_paths.clear();
            state.unmerged_paths.extend(
                unmerged_state
                    .iter()
                    .map(|(path, content)| (path.clone(), *content)),
            );
        })
        .unwrap();
    }

    pub fn set_index_for_repo(&self, dot_git: &Path, index_state: &[(&str, String)]) {
        self.with_git_state(dot_git, true, |state| {
            state.index_contents.clear();
            state.index_contents.extend(
                index_state
                    .iter()
                    .map(|(path, content)| (repo_path(path), content.as_bytes().to_vec())),
            );
        })
        .unwrap();
    }

    pub fn set_head_for_repo(
        &self,
        dot_git: &Path,
        head_state: &[(&str, String)],
        sha: impl Into<String>,
    ) {
        self.with_git_state(dot_git, true, |state| {
            state.head_contents.clear();
            state.head_contents.extend(
                head_state
                    .iter()
                    .map(|(path, content)| (repo_path(path), content.as_bytes().to_vec())),
            );
            state.refs.insert("HEAD".into(), sha.into());
        })
        .unwrap();
    }

    pub fn set_head_and_index_for_repo(&self, dot_git: &Path, contents_by_path: &[(&str, String)]) {
        self.with_git_state(dot_git, true, |state| {
            state.head_contents.clear();
            state.head_contents.extend(
                contents_by_path
                    .iter()
                    .map(|(path, contents)| (repo_path(path), contents.as_bytes().to_vec())),
            );
            state.index_contents = state.head_contents.clone();
        })
        .unwrap();
    }

    pub fn set_merge_base_content_for_repo(
        &self,
        dot_git: &Path,
        contents_by_path: &[(&str, String)],
    ) -> Vec<git::Oid> {
        self.with_git_state(dot_git, true, |state| {
            use git::Oid;

            state.merge_base_contents.clear();
            let mut assigned = Vec::with_capacity(contents_by_path.len());
            for (index, (path, content)) in contents_by_path.iter().enumerate() {
                let mut bytes = [0u8; 20];
                bytes[..4].copy_from_slice(&((index as u32) + 1).to_be_bytes());
                let oid = Oid::from_bytes(&bytes).unwrap();
                state.merge_base_contents.insert(repo_path(path), oid);
                state.oids.insert(oid, content.as_bytes().to_vec());
                assigned.push(oid);
            }
            assigned
        })
        .unwrap()
    }

    pub fn install_blob_read_gate_for_repo(&self, dot_git: &Path) -> FakeBlobReadGate {
        let gate = FakeBlobReadGate::default();
        self.with_git_state(dot_git, false, |state| {
            state.blob_read_gate = Some(gate.clone());
        })
        .unwrap();
        gate
    }

    pub fn set_blame_for_repo(&self, dot_git: &Path, blames: Vec<(RepoPath, git::blame::Blame)>) {
        self.with_git_state(dot_git, true, |state| {
            state.blames.clear();
            state.blames.extend(blames);
        })
        .unwrap();
    }

    pub fn set_graph_commits(&self, dot_git: &Path, commits: Vec<Arc<InitialGraphCommitData>>) {
        self.with_git_state(dot_git, true, |state| {
            state.graph_commits = commits;
        })
        .unwrap();
    }

    pub fn set_graph_error(&self, dot_git: &Path, error: Option<String>) {
        self.with_git_state(dot_git, true, |state| {
            state.simulated_graph_error = error;
        })
        .unwrap();
    }

    pub fn set_commit_data(
        &self,
        dot_git: &Path,
        commit_data: impl IntoIterator<Item = (CommitData, bool)>,
    ) {
        self.with_git_state(dot_git, true, |state| {
            state.commit_data = commit_data
                .into_iter()
                .map(|(data, should_fail)| {
                    (
                        data.sha,
                        if should_fail {
                            FakeCommitDataEntry::Fail(data)
                        } else {
                            FakeCommitDataEntry::Success(data)
                        },
                    )
                })
                .collect();
        })
        .unwrap();
    }

    /// Put the given git repository into a state with the given status,
    /// by mutating the head, index, and unmerged state.
    pub fn set_status_for_repo(&self, dot_git: &Path, statuses: &[(&str, FileStatus)]) {
        let workdir_path = dot_git.parent().unwrap();
        let workdir_contents = self.files_with_contents(workdir_path);
        self.with_git_state(dot_git, true, |state| {
            state.index_contents.clear();
            state.head_contents.clear();
            state.unmerged_paths.clear();
            for (path, content) in workdir_contents {
                use util::{paths::PathStyle, rel_path::RelPath};

                let repo_path = RelPath::new(path.strip_prefix(&workdir_path).unwrap(), PathStyle::local()).unwrap();
                let repo_path = RepoPath::from_rel_path(&repo_path);
                let status = statuses
                    .iter()
                    .find_map(|(p, status)| (*p == repo_path.as_unix_str()).then_some(status));
                let mut content = String::from_utf8_lossy(&content).to_string();

                let mut index_content = None;
                let mut head_content = None;
                match status {
                    None => {
                        index_content = Some(content.clone());
                        head_content = Some(content);
                    }
                    Some(FileStatus::Untracked | FileStatus::Ignored) => {}
                    Some(FileStatus::Unmerged(unmerged_status)) => {
                        state
                            .unmerged_paths
                            .insert(repo_path.clone(), *unmerged_status);
                        content.push_str(" (unmerged)");
                        index_content = Some(content.clone());
                        head_content = Some(content);
                    }
                    Some(FileStatus::Tracked(TrackedStatus {
                        index_status,
                        worktree_status,
                    })) => {
                        match worktree_status {
                            StatusCode::Modified => {
                                let mut content = content.clone();
                                content.push_str(" (modified in working copy)");
                                index_content = Some(content);
                            }
                            StatusCode::TypeChanged | StatusCode::Unmodified => {
                                index_content = Some(content.clone());
                            }
                            StatusCode::Added => {}
                            StatusCode::Deleted | StatusCode::Renamed | StatusCode::Copied => {
                                panic!("cannot create these statuses for an existing file");
                            }
                        };
                        match index_status {
                            StatusCode::Modified => {
                                let mut content = index_content.clone().expect(
                                    "file cannot be both modified in index and created in working copy",
                                );
                                content.push_str(" (modified in index)");
                                head_content = Some(content);
                            }
                            StatusCode::TypeChanged | StatusCode::Unmodified => {
                                head_content = Some(index_content.clone().expect("file cannot be both unmodified in index and created in working copy"));
                            }
                            StatusCode::Added => {}
                            StatusCode::Deleted  => {
                                head_content = Some("".into());
                            }
                            StatusCode::Renamed | StatusCode::Copied => {
                                panic!("cannot create these statuses for an existing file");
                            }
                        };
                    }
                };

                if let Some(content) = index_content {
                    state
                        .index_contents
                        .insert(repo_path.clone(), content.into_bytes());
                }
                if let Some(content) = head_content {
                    state
                        .head_contents
                        .insert(repo_path.clone(), content.into_bytes());
                }
            }
        }).unwrap();
    }

    pub fn set_error_message_for_index_write(&self, dot_git: &Path, message: Option<String>) {
        self.with_git_state(dot_git, true, |state| {
            state.simulated_index_write_error_message = message;
        })
        .unwrap();
    }

    pub fn set_create_worktree_error(&self, dot_git: &Path, message: Option<String>) {
        self.with_git_state(dot_git, true, |state| {
            state.simulated_create_worktree_error = message;
        })
        .unwrap();
    }

    /// Makes subsequent `remove_dir` calls for `path` fail with `message`.
    pub fn set_remove_dir_error(&self, path: impl AsRef<Path>, message: String) {
        self.state
            .lock()
            .remove_dir_errors
            .insert(Self::remove_dir_error_key(path.as_ref()), message);
    }

    pub fn clear_remove_dir_error(&self, path: impl AsRef<Path>) {
        self.state
            .lock()
            .remove_dir_errors
            .remove(&Self::remove_dir_error_key(path.as_ref()));
    }

    /// Entry resolution in `try_entry` ignores drive prefixes, so the error
    /// injection map must too.
    /// Otherwise, on Windows, a key like `C:\workspace\dir` would never match a
    /// lookup for `\workspace\dir`.
    fn remove_dir_error_key(path: &Path) -> PathBuf {
        normalize_path(path)
            .components()
            .skip_while(|component| matches!(component, Component::Prefix(_)))
            .collect()
    }

    pub fn paths(&self, include_dot_git: bool) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut queue = collections::VecDeque::new();
        let state = &*self.state.lock();
        queue.push_back((PathBuf::from(util::path!("/")), &state.root));
        while let Some((path, entry)) = queue.pop_front() {
            if let FakeFsEntry::Dir { entries, .. } = entry {
                for (name, entry) in entries {
                    queue.push_back((path.join(name), entry));
                }
            }
            if include_dot_git
                || !path
                    .components()
                    .any(|component| component.as_os_str() == *FS_DOT_GIT)
            {
                result.push(path);
            }
        }
        result
    }

    pub fn directories(&self, include_dot_git: bool) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut queue = collections::VecDeque::new();
        let state = &*self.state.lock();
        queue.push_back((PathBuf::from(util::path!("/")), &state.root));
        while let Some((path, entry)) = queue.pop_front() {
            if let FakeFsEntry::Dir { entries, .. } = entry {
                for (name, entry) in entries {
                    queue.push_back((path.join(name), entry));
                }
                if include_dot_git
                    || !path
                        .components()
                        .any(|component| component.as_os_str() == *FS_DOT_GIT)
                {
                    result.push(path);
                }
            }
        }
        result
    }

    pub fn files(&self) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut queue = collections::VecDeque::new();
        let state = &*self.state.lock();
        queue.push_back((PathBuf::from(util::path!("/")), &state.root));
        while let Some((path, entry)) = queue.pop_front() {
            match entry {
                FakeFsEntry::File { .. } => result.push(path),
                FakeFsEntry::Dir { entries, .. } => {
                    for (name, entry) in entries {
                        queue.push_back((path.join(name), entry));
                    }
                }
                FakeFsEntry::Symlink { .. } => {}
            }
        }
        result
    }

    pub fn files_with_contents(&self, prefix: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut result = Vec::new();
        let mut queue = collections::VecDeque::new();
        let state = &*self.state.lock();
        queue.push_back((PathBuf::from(util::path!("/")), &state.root));
        while let Some((path, entry)) = queue.pop_front() {
            match entry {
                FakeFsEntry::File { content, .. } => {
                    if path.starts_with(prefix) {
                        result.push((path, content.clone()));
                    }
                }
                FakeFsEntry::Dir { entries, .. } => {
                    for (name, entry) in entries {
                        queue.push_back((path.join(name), entry));
                    }
                }
                FakeFsEntry::Symlink { .. } => {}
            }
        }
        result
    }

    /// How many `read_dir` calls have been issued.
    pub fn read_dir_call_count(&self) -> usize { self.state.lock().read_dir_call_count }

    /// The roots passed to `Fs::watch` whose watchers are still alive.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let state = self.state.lock();
        state
            .watch_roots
            .iter()
            .filter_map(|(path, watcher)| (watcher.strong_count() > 0).then_some(path.clone()))
            .collect()
    }

    /// How many `metadata` calls have been issued.
    pub fn metadata_call_count(&self) -> usize { self.state.lock().metadata_call_count }

    /// How many write operations have been issued for a specific path.
    pub fn write_count_for_path(&self, path: impl AsRef<Path>) -> usize {
        let path = path.as_ref().to_path_buf();
        self.state
            .lock()
            .path_write_counts
            .get(&path)
            .copied()
            .unwrap_or(0)
    }

    pub fn emit_fs_event(&self, path: impl Into<PathBuf>, event: Option<PathEventKind>) {
        self.state.lock().emit_event(std::iter::once((path, event)));
    }

    fn simulate_random_delay(&self) -> impl futures::Future<Output = ()> {
        self.executor.simulate_random_delay()
    }

    async fn remove_dir_inner(
        &self,
        path: &Path,
        options: RemoveOptions,
    ) -> Result<Option<FakeFsEntry>> {
        self.simulate_random_delay().await;

        let path = normalize_path(path);
        if let Some(message) = self
            .state
            .lock()
            .remove_dir_errors
            .get(&Self::remove_dir_error_key(&path))
        {
            anyhow::bail!("{message}");
        }
        let parent_path = path.parent().context("cannot remove the root")?;
        let base_name = path.file_name().context("cannot remove the root")?;

        let mut state = self.state.lock();
        let parent_entry = state.entry(parent_path)?;
        let entry = parent_entry
            .dir_entries(parent_path)?
            .entry(base_name.to_str().unwrap().into());

        let removed = match entry {
            btree_map::Entry::Vacant(_) => {
                if !options.ignore_if_not_exists {
                    anyhow::bail!("{path:?} does not exist");
                }

                None
            }
            btree_map::Entry::Occupied(mut entry) => {
                {
                    let children = entry.get_mut().dir_entries(&path)?;
                    if !options.recursive && !children.is_empty() {
                        anyhow::bail!("{path:?} is not empty");
                    }
                }

                Some(entry.remove())
            }
        };

        state.emit_event([(path, Some(PathEventKind::Removed))]);
        Ok(removed)
    }

    async fn remove_file_inner(
        &self,
        path: &Path,
        options: RemoveOptions,
    ) -> Result<Option<FakeFsEntry>> {
        self.simulate_random_delay().await;

        let path = normalize_path(path);
        let parent_path = path.parent().context("cannot remove the root")?;
        let base_name = path.file_name().unwrap();
        let mut state = self.state.lock();
        let parent_entry = state.entry(parent_path)?;
        let entry = parent_entry
            .dir_entries(parent_path)?
            .entry(base_name.to_str().unwrap().into());
        let removed = match entry {
            btree_map::Entry::Vacant(_) => {
                if !options.ignore_if_not_exists {
                    anyhow::bail!("{path:?} does not exist");
                }

                None
            }
            btree_map::Entry::Occupied(mut entry) => {
                entry.get_mut().file_content(&path)?;
                Some(entry.remove())
            }
        };

        state.emit_event([(path, Some(PathEventKind::Removed))]);
        Ok(removed)
    }

    pub fn trashed_paths(&self) -> Vec<PathBuf> {
        self.state
            .lock()
            .trash
            .lock()
            .values()
            .map(|(trashed_entry, _fake_entry)| {
                PathBuf::new()
                    .join(trashed_entry.original_parent.clone())
                    .join(trashed_entry.name.clone())
            })
            .collect::<Vec<PathBuf>>()
    }
}

#[cfg(feature = "test-support")]
impl FakeFsEntry {
    fn is_file(&self) -> bool { matches!(self, Self::File { .. }) }

    fn is_symlink(&self) -> bool { matches!(self, Self::Symlink { .. }) }

    fn file_content(&self, path: &Path) -> Result<&Vec<u8>> {
        if let Self::File { content, .. } = self {
            Ok(content)
        } else {
            anyhow::bail!("not a file: {path:?}");
        }
    }

    fn dir_entries(&mut self, path: &Path) -> Result<&mut BTreeMap<String, FakeFsEntry>> {
        if let Self::Dir { entries, .. } = self {
            Ok(entries)
        } else {
            anyhow::bail!("not a directory: {path:?}");
        }
    }
}

#[cfg(feature = "test-support")]
#[derive(Debug)]
struct FakeHandle {
    inode: u64,
}

#[cfg(feature = "test-support")]
impl FileHandle for FakeHandle {
    fn current_path(&self, fs: &Arc<dyn Fs>) -> Result<PathBuf> {
        let fs = fs.as_fake();
        let state = fs.state.lock();
        let mut queue = collections::VecDeque::new();
        queue.push_back((PathBuf::from(util::path!("/")), &state.root));
        while let Some((path, entry)) = queue.pop_front() {
            match entry {
                FakeFsEntry::File { inode, .. } | FakeFsEntry::Dir { inode, .. }
                    if *inode == self.inode =>
                {
                    return Ok(path);
                }
                FakeFsEntry::Dir { entries, .. } => {
                    for (name, entry) in entries {
                        queue.push_back((path.join(name), entry));
                    }
                }
                _ => {}
            }
        }
        anyhow::bail!("fake fd target not found")
    }
}

#[cfg(feature = "test-support")]
#[async_trait::async_trait]
impl Fs for FakeFs {
    async fn create_dir(&self, path: &Path) -> Result<()> {
        self.simulate_random_delay().await;

        let mut created_dirs = Vec::new();
        let mut cur_path = PathBuf::new();
        for component in path.components() {
            let should_skip = matches!(component, Component::Prefix(..) | Component::RootDir);
            cur_path.push(component);
            if should_skip {
                continue;
            }
            let mut state = self.state.lock();

            let inode = state.get_and_increment_inode();
            let mtime = state.get_and_increment_mtime();
            state.write_path(&cur_path, |entry| {
                entry.or_insert_with(|| {
                    created_dirs.push((cur_path.clone(), Some(PathEventKind::Created)));
                    FakeFsEntry::Dir {
                        inode,
                        mtime,
                        len: 0,
                        entries: Default::default(),
                        git_repo_state: None,
                    }
                });
                Ok(())
            })?
        }

        self.state.lock().emit_event(created_dirs);
        Ok(())
    }

    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()> {
        self.simulate_random_delay().await;
        let mut state = self.state.lock();
        let inode = state.get_and_increment_inode();
        let mtime = state.get_and_increment_mtime();
        let file = FakeFsEntry::File {
            inode,
            mtime,
            len: 0,
            content: Vec::new(),
            git_dir_path: None,
        };
        let mut kind = Some(PathEventKind::Created);
        state.write_path(path, |entry| {
            match entry {
                btree_map::Entry::Occupied(mut e) => {
                    if options.overwrite {
                        kind = Some(PathEventKind::Changed);
                        *e.get_mut() = file;
                    } else if !options.ignore_if_exists {
                        anyhow::bail!("path already exists: {path:?}");
                    }
                }
                btree_map::Entry::Vacant(e) => {
                    e.insert(file);
                }
            }
            Ok(())
        })?;
        state.emit_event([(path, kind)]);
        Ok(())
    }

    async fn create_symlink(&self, path: &Path, target: PathBuf) -> Result<()> {
        let mut state = self.state.lock();
        let file = FakeFsEntry::Symlink { target };
        state
            .write_path(path.as_ref(), move |e| match e {
                btree_map::Entry::Vacant(e) => {
                    e.insert(file);
                    Ok(())
                }
                btree_map::Entry::Occupied(mut e) => {
                    *e.get_mut() = file;
                    Ok(())
                }
            })
            .unwrap();
        state.emit_event([(path, Some(PathEventKind::Created))]);

        Ok(())
    }

    async fn create_file_with(
        &self,
        path: &Path,
        mut content: Pin<&mut (dyn AsyncRead + Send)>,
    ) -> Result<()> {
        let mut bytes = Vec::new();
        content.read_to_end(&mut bytes).await?;
        self.write_file_internal(path, bytes, true)?;
        Ok(())
    }

    async fn extract_tar_file(
        &self,
        path: &Path,
        content: Archive<Pin<&mut (dyn AsyncRead + Send)>>,
    ) -> Result<()> {
        let mut entries = content.entries()?;
        while let Some(entry) = entries.next().await {
            let mut entry = entry?;
            if entry.header().entry_type().is_file() {
                let path = path.join(entry.path()?.as_ref());
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).await?;
                self.create_dir(path.parent().unwrap()).await?;
                self.write_file_internal(&path, bytes, true)?;
            }
        }
        Ok(())
    }

    async fn rename(&self, old_path: &Path, new_path: &Path, options: RenameOptions) -> Result<()> {
        self.simulate_random_delay().await;

        let old_path = normalize_path(old_path);
        let new_path = normalize_path(new_path);

        if options.create_parents {
            if let Some(parent) = new_path.parent() {
                self.create_dir(parent).await?;
            }
        }

        let mut state = self.state.lock();
        let moved_entry = state.write_path(&old_path, |e| {
            if let btree_map::Entry::Occupied(e) = e {
                Ok(e.get().clone())
            } else {
                anyhow::bail!("path does not exist: {old_path:?}")
            }
        })?;

        // POSIX `rename` succeeds without doing anything when both names resolve
        // to the same file. Falling through would assign the entry onto itself
        // and then remove it, destroying the file. The lookup above has already
        // reported a missing source, so only an existing one reaches here.
        if old_path == new_path {
            return Ok(());
        }

        let mut moved = true;
        state.write_path(&new_path, |e| {
            match e {
                btree_map::Entry::Occupied(mut e) => {
                    if options.overwrite {
                        *e.get_mut() = moved_entry;
                    } else if options.ignore_if_exists {
                        // `RealFs` reports success without moving anything here,
                        // leaving the source in place. Removing it instead would
                        // destroy a file the caller still expects to find.
                        moved = false;
                    } else {
                        anyhow::bail!("path already exists: {new_path:?}");
                    }
                }
                btree_map::Entry::Vacant(e) => {
                    e.insert(moved_entry);
                }
            }
            Ok(())
        })?;

        if !moved {
            return Ok(());
        }

        state
            .write_path(&old_path, |e| {
                if let btree_map::Entry::Occupied(e) = e {
                    Ok(e.remove())
                } else {
                    unreachable!()
                }
            })
            .unwrap();

        state.emit_event([
            (old_path, Some(PathEventKind::Removed)),
            (new_path, Some(PathEventKind::Created)),
        ]);
        Ok(())
    }

    async fn copy_file(&self, source: &Path, target: &Path, options: CopyOptions) -> Result<()> {
        self.simulate_random_delay().await;

        let source = normalize_path(source);
        let target = normalize_path(target);
        let mut state = self.state.lock();
        let mtime = state.get_and_increment_mtime();
        let inode = state.get_and_increment_inode();
        let source_entry = state.entry(&source)?;
        let content = source_entry.file_content(&source)?.clone();
        let mut kind = Some(PathEventKind::Created);
        state.write_path(&target, |e| match e {
            btree_map::Entry::Occupied(e) => {
                if options.overwrite {
                    kind = Some(PathEventKind::Changed);
                    Ok(Some(e.get().clone()))
                } else if !options.ignore_if_exists {
                    anyhow::bail!("{target:?} already exists");
                } else {
                    Ok(None)
                }
            }
            btree_map::Entry::Vacant(e) => Ok(Some(
                e.insert(FakeFsEntry::File {
                    inode,
                    mtime,
                    len: content.len() as u64,
                    content,
                    git_dir_path: None,
                })
                .clone(),
            )),
        })?;
        state.emit_event([(target, kind)]);
        Ok(())
    }

    async fn remove_dir(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        self.remove_dir_inner(path, options).await.map(|_| ())
    }

    async fn trash(&self, path: &Path, options: RemoveOptions) -> Result<TrashId> {
        let normalized_path = normalize_path(path);
        let parent_path = normalized_path.parent().context("cannot remove the root")?;
        let base_name = normalized_path.file_name().unwrap();
        let result = if self.is_dir(path).await {
            self.remove_dir_inner(path, options).await?
        } else {
            self.remove_file_inner(path, options).await?
        };

        match result {
            Some(fake_entry) => {
                let trashed_entry = TrashedEntry {
                    id: base_name.to_str().unwrap().into(),
                    name: base_name.to_str().unwrap().into(),
                    original_parent: parent_path.to_path_buf(),
                };

                let trash_id = self
                    .state
                    .lock()
                    .trash
                    .lock()
                    .insert((trashed_entry, fake_entry));

                Ok(trash_id)
            }
            None => anyhow::bail!("{normalized_path:?} does not exist"),
        }
    }

    async fn remove_file(&self, path: &Path, options: RemoveOptions) -> Result<()> {
        self.remove_file_inner(path, options).await.map(|_| ())
    }

    async fn open_sync(&self, path: &Path) -> Result<Box<dyn io::Read + Send + Sync>> {
        let bytes = self.load_internal(path).await?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    async fn open_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>> {
        self.simulate_random_delay().await;
        let mut state = self.state.lock();
        let inode = match state.entry(path)? {
            FakeFsEntry::File { inode, .. } => *inode,
            FakeFsEntry::Dir { inode, .. } => *inode,
            _ => unreachable!(),
        };
        Ok(Arc::new(FakeHandle { inode }))
    }

    async fn load(&self, path: &Path) -> Result<String> {
        let content = self.load_internal(path).await?;
        Ok(String::from_utf8(content)?)
    }

    async fn load_bytes(&self, path: &Path) -> Result<Vec<u8>> { self.load_internal(path).await }

    async fn atomic_write(&self, path: PathBuf, data: String) -> Result<()> {
        self.simulate_random_delay().await;
        let path = normalize_path(path.as_path());
        if let Some(path) = path.parent() {
            self.create_dir(path).await?;
        }
        self.write_file_internal(path, data.into_bytes(), true)?;
        Ok(())
    }

    async fn save(&self, path: &Path, text: &Rope, line_ending: LineEnding) -> Result<()> {
        self.simulate_random_delay().await;
        let path = normalize_path(path);
        let content = text::chunks_with_line_ending(text, line_ending).collect::<String>();
        if let Some(path) = path.parent() {
            self.create_dir(path).await?;
        }
        self.write_file_internal(path, content.into_bytes(), false)?;
        Ok(())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        self.simulate_random_delay().await;
        let path = normalize_path(path);
        if let Some(path) = path.parent() {
            self.create_dir(path).await?;
        }
        self.write_file_internal(path, content.to_vec(), false)?;
        Ok(())
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let path = normalize_path(path);
        self.simulate_random_delay().await;
        let state = self.state.lock();
        let canonical_path = state
            .canonicalize(&path, true)
            .with_context(|| format!("path does not exist: {path:?}"))?;
        Ok(canonical_path)
    }

    async fn is_file(&self, path: &Path) -> bool {
        let path = normalize_path(path);
        self.simulate_random_delay().await;
        let mut state = self.state.lock();
        if let Some((entry, _)) = state.try_entry(&path, true) {
            entry.is_file()
        } else {
            false
        }
    }

    async fn is_dir(&self, path: &Path) -> bool {
        self.metadata(path)
            .await
            .is_ok_and(|metadata| metadata.is_some_and(|metadata| metadata.is_dir))
    }

    async fn metadata(&self, path: &Path) -> Result<Option<Metadata>> {
        self.simulate_random_delay().await;
        let path = normalize_path(path);
        let mut state = self.state.lock();
        state.metadata_call_count += 1;
        if let Some((mut entry, _)) = state.try_entry(&path, false) {
            let is_symlink = entry.is_symlink();
            if is_symlink {
                if let Some(e) = state.try_entry(&path, true).map(|e| e.0) {
                    entry = e;
                } else {
                    return Ok(None);
                }
            }

            Ok(Some(match &*entry {
                FakeFsEntry::File {
                    inode, mtime, len, ..
                } => Metadata {
                    inode: *inode,
                    mtime: *mtime,
                    len: *len,
                    is_dir: false,
                    is_symlink,
                    is_fifo: false,
                    is_executable: false,
                    is_writable: true,
                },
                FakeFsEntry::Dir {
                    inode, mtime, len, ..
                } => Metadata {
                    inode: *inode,
                    mtime: *mtime,
                    len: *len,
                    is_dir: true,
                    is_symlink,
                    is_fifo: false,
                    is_executable: false,
                    is_writable: true,
                },
                FakeFsEntry::Symlink { .. } => unreachable!(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf> {
        self.simulate_random_delay().await;
        let path = normalize_path(path);
        let mut state = self.state.lock();
        let (entry, _) = state
            .try_entry(&path, false)
            .with_context(|| format!("path does not exist: {path:?}"))?;
        if let FakeFsEntry::Symlink { target } = entry {
            Ok(target.clone())
        } else {
            anyhow::bail!("not a symlink: {path:?}")
        }
    }

    async fn read_dir(
        &self,
        path: &Path,
    ) -> Result<Pin<Box<dyn Send + Stream<Item = Result<PathBuf>>>>> {
        self.simulate_random_delay().await;
        let path = normalize_path(path);
        let mut state = self.state.lock();
        state.read_dir_call_count += 1;
        let entry = state.entry(&path)?;
        let children = entry.dir_entries(&path)?;
        let paths = children
            .keys()
            .map(|file_name| Ok(path.join(file_name)))
            .collect::<Vec<_>>();
        Ok(Box::pin(futures::stream::iter(paths)))
    }

    async fn watch(
        &self,
        path: &Path,
        _: Duration,
    ) -> (
        Pin<Box<dyn Send + Stream<Item = Vec<PathEvent>>>>,
        Arc<dyn Watcher>,
    ) {
        self.simulate_random_delay().await;
        // Zero latency: the deterministic executor doesn't advance time on its
        // own, so a real debounce would stall every test until `advance_clock`.
        let (events, watcher) = fs_watcher::watch(
            self.this.upgrade().unwrap(),
            self.native_watcher.clone(),
            self.poll_watcher.clone(),
            self.executor.clone(),
            path,
            Duration::ZERO,
        )
        .await;
        self.state
            .lock()
            .watch_roots
            .push((normalize_path(path), Arc::downgrade(&watcher)));
        (events, watcher)
    }

    fn open_repo(
        &self,
        abs_dot_git: &Path,
        _system_git_binary: Option<&Path>,
    ) -> Result<Arc<dyn GitRepository>> {
        self.with_git_state_and_paths(
            abs_dot_git,
            false,
            |_, repository_dir_path, common_dir_path| {
                Arc::new(fake_git_repo::FakeGitRepository {
                    fs: self.this.upgrade().unwrap(),
                    executor: self.executor.clone(),
                    dot_git_path: abs_dot_git.to_path_buf(),
                    repository_dir_path: repository_dir_path.to_owned(),
                    common_dir_path: common_dir_path.to_owned(),
                    checkpoints: Arc::default(),
                    is_trusted: Arc::default(),
                }) as _
            },
        )
    }

    async fn git_init(
        &self,
        abs_work_directory_path: &Path,
        _fallback_branch_name: String,
    ) -> Result<()> {
        self.create_dir(&abs_work_directory_path.join(".git")).await
    }

    async fn git_clone(&self, _abs_work_directory: &Path, _repo_url: &str) -> Result<()> {
        anyhow::bail!("Git clone is not supported in fake Fs")
    }

    async fn git_config(&self, _abs_work_directory: &Path, _args: Vec<String>) -> Result<String> {
        anyhow::bail!("Git config is not supported in fake Fs")
    }

    fn record_watcher_diagnostics(&self) -> Option<fs_watcher::WatchRecording> {
        Some(fs_watcher::WatchRecording::new([
            self.native_watcher.clone(),
            self.poll_watcher.clone(),
        ]))
    }

    fn path_exists(&self, path: &Path) -> bool {
        self.state
            .lock()
            .try_entry(&normalize_path(path), false)
            .is_some()
    }

    fn is_path_case_sensitive(&self, _path: &Path) -> bool { self.state.lock().case_sensitive }

    fn requires_poll_watcher(&self, _path: &Path) -> bool { false }

    fn is_fake(&self) -> bool { true }

    async fn is_case_sensitive(&self) -> bool { self.state.lock().case_sensitive }

    fn subscribe_to_jobs(&self) -> JobEventReceiver {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        self.state.lock().job_event_subscribers.lock().push(sender);
        receiver
    }

    fn original_path_for_trash_id(&self, trash_id: TrashId) -> Option<PathBuf> {
        self.state
            .lock()
            .trash
            .lock()
            .get(trash_id)
            .map(|(entry, _)| entry.original_parent.join(&entry.name))
    }

    async fn restore(&self, trash_id: TrashId) -> Result<PathBuf, TrashRestoreError> {
        let mut state = self.state.lock();

        let Some((trashed_entry, fake_entry)) = state.trash.lock().get(trash_id).cloned() else {
            return Err(TrashRestoreError::AlreadyRestored);
        };

        let path = trashed_entry
            .original_parent
            .join(trashed_entry.name.clone());

        let result = state.write_path(&path, |entry| match entry {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(fake_entry);
                Ok(())
            }
            btree_map::Entry::Occupied(_) => {
                anyhow::bail!("Failed to restore {:?}", path);
            }
        });

        match result {
            Ok(_) => {
                state.trash.lock().remove(trash_id);
                state.emit_event([(path.clone(), Some(PathEventKind::Created))]);
                Ok(path)
            }
            Err(_) => {
                // For now we'll just assume that this failed because it was a
                // collision error, which I think that, for the time being, is
                // the only case where this could fail?
                Err(TrashRestoreError::Collision { path })
            }
        }
    }

    #[cfg(feature = "test-support")]
    fn as_fake(&self) -> Arc<FakeFs> { self.this.upgrade().unwrap() }
}
