use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;

use crate::Fs;

pub trait FileHandle: Send + Sync + std::fmt::Debug {
    fn current_path(&self, fs: &Arc<dyn Fs>) -> Result<PathBuf>;
}

impl FileHandle for std::fs::File {
    #[cfg(target_os = "macos")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::ffi::{CStr, OsStr};
        use std::mem::MaybeUninit;
        use std::os::fd::{AsFd, AsRawFd};
        use std::os::unix::ffi::OsStrExt;

        let fd = self.as_fd();
        let mut path_buf = MaybeUninit::<[u8; libc::PATH_MAX as usize]>::uninit();

        let result = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETPATH, path_buf.as_mut_ptr()) };
        anyhow::ensure!(result != -1, "fcntl returned -1");

        // SAFETY: `fcntl` will initialize the path buffer.
        let c_str = unsafe { CStr::from_ptr(path_buf.as_ptr().cast()) };
        anyhow::ensure!(!c_str.is_empty(), "Could find a path for the file handle");
        let path = PathBuf::from(OsStr::from_bytes(c_str.to_bytes()));
        Ok(path)
    }

    // Android 复用 linux 实现：Android 同为 Linux 内核，`/proc/self/fd/` 与
    // " (deleted)" 后缀判定都成立。这里**不是**权宜之计 —— 与 linux 同源，
    // 不是「暂时没适配」。
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::os::fd::{AsFd, AsRawFd};

        let fd = self.as_fd();
        let fd_path = format!("/proc/self/fd/{}", fd.as_raw_fd());
        let new_path = std::fs::read_link(fd_path)?;
        if new_path
            .file_name()
            .is_some_and(|f| f.to_string_lossy().ends_with(" (deleted)"))
        {
            anyhow::bail!("file was deleted")
        }

        Ok(new_path)
    }

    #[cfg(target_os = "freebsd")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::ffi::{CStr, OsStr};
        use std::mem::MaybeUninit;
        use std::os::fd::{AsFd, AsRawFd};
        use std::os::unix::ffi::OsStrExt;

        let fd = self.as_fd();
        let mut kif = MaybeUninit::<libc::kinfo_file>::uninit();
        kif.kf_structsize = libc::KINFO_FILE_SIZE;

        let result = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_KINFO, kif.as_mut_ptr()) };
        anyhow::ensure!(result != -1, "fcntl returned -1");

        // SAFETY: `fcntl` will initialize the kif.
        let c_str = unsafe { CStr::from_ptr(kif.assume_init().kf_path.as_ptr()) };
        anyhow::ensure!(!c_str.is_empty(), "Could find a path for the file handle");
        let path = PathBuf::from(OsStr::from_bytes(c_str.to_bytes()));
        Ok(path)
    }

    #[cfg(target_os = "windows")]
    fn current_path(&self, _: &Arc<dyn Fs>) -> Result<PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::io::AsRawHandle;

        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Storage::FileSystem::{
            FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW,
        };

        let handle = HANDLE(self.as_raw_handle() as _);

        // Query required buffer size (in wide chars)
        let required_len =
            unsafe { GetFinalPathNameByHandleW(handle, &mut [], FILE_NAME_NORMALIZED) };
        anyhow::ensure!(
            required_len != 0,
            "GetFinalPathNameByHandleW returned 0 length"
        );

        // Allocate buffer and retrieve the path
        let mut buf: Vec<u16> = vec![0u16; required_len as usize + 1];
        let written = unsafe { GetFinalPathNameByHandleW(handle, &mut buf, FILE_NAME_NORMALIZED) };
        anyhow::ensure!(
            written != 0,
            "GetFinalPathNameByHandleW failed to write path"
        );

        let os_str: OsString = OsString::from_wide(&buf[..written as usize]);
        anyhow::ensure!(!os_str.is_empty(), "Could find a path for the file handle");
        Ok(PathBuf::from(os_str))
    }
}
