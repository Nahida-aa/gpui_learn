use anyhow::Result;
use std::path::PathBuf;

use crate::ResultExt as _;

/// Parses the contents of an `os-release` file (as found at `/etc/os-release`
/// and described by the systemd spec) into a human-readable string such as
/// `"ubuntu 24.04"`, combining the `ID` and `VERSION_ID` fields.
///
/// Returns `None` if no `ID` field is present. When `VERSION_ID` is absent
/// (e.g. on rolling releases), only the `ID` is returned.
pub fn parse_os_release(content: &str) -> Option<String> {
    let mut id = None;
    let mut version_id = None;
    for line in content.lines() {
        match line.split_once('=') {
            Some(("ID", value)) => id = Some(value.trim_matches('"')),
            Some(("VERSION_ID", value)) => version_id = Some(value.trim_matches('"')),
            _ => {}
        }
    }
    let id = id?;
    Some(match version_id {
        Some(version) => format!("{id} {version}"),
        None => id.to_string(),
    })
}

/// Raises the soft limit on open file descriptors without changing the hard limit.
///
/// Call during startup, before spawning children that will inherit the limit.
#[cfg(unix)]
pub fn increase_open_file_limit() -> Result<()> {
    use anyhow::Context as _;
    use nix::sys::resource::{Resource::RLIMIT_NOFILE, getrlimit, setrlimit};

    let (soft_limit, hard_limit) = getrlimit(RLIMIT_NOFILE).context("getrlimit(RLIMIT_NOFILE)")?;
    // These are startup targets, not OS ceilings. Preserve higher inherited limits.
    let target = if cfg!(target_os = "macos") {
        10_240
    } else {
        65_536
    };
    let mut requested_limit = hard_limit.min(target);

    while requested_limit > soft_limit {
        let Err(error) = setrlimit(RLIMIT_NOFILE, requested_limit, hard_limit) else {
            log::info!("raised open file soft limit from {soft_limit} to {requested_limit}");
            return Ok(());
        };

        // Some systems enforce a ceiling below the reported hard limit.
        if error != nix::errno::Errno::EINVAL || requested_limit == soft_limit + 1 {
            return Err(error).context("setrlimit(RLIMIT_NOFILE)");
        }
        requested_limit = soft_limit + (requested_limit - soft_limit) / 2;
    }

    Ok(())
}

/// Configures the process to start a new session, to prevent interactive shells from taking control
/// of the terminal.
///
/// For more details: <https://registerspill.thorstenball.com/p/how-to-lose-control-of-your-shell>
pub fn set_pre_exec_to_start_new_session(
    command: &mut std::process::Command,
) -> &mut std::process::Command {
    // safety: code in pre_exec should be signal safe.
    // https://man7.org/linux/man-pages/man7/signal-safety.7.html
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    };
    command
}
