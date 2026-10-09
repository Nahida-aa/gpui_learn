/// The checkout that produced this binary, resolved at runtime by walking up to
/// the first ancestor that contains a `.git` entry (a directory in a normal
/// clone, a file in a git worktree or submodule). Cargo and corgi both place
/// built binaries under `<repo>/target/<profile>/`, so the repository root is
/// always an ancestor of the executable.
///
/// Dev-only affordances use this instead of baking a build-time path into the
/// artifact: such a path points at the wrong checkout from any other worktree
/// and, under corgi, is rejected because artifacts must be checkout-independent
/// to be shared across worktrees.
///
/// The executable's launch path is tried first, then its canonical form, then
/// the working directory. In CI, `target/` (or the checkout root) can be a
/// symlink onto another volume, so canonicalizing the executable alone can walk
/// off the checkout and miss `.git`; the launch path and the test runner's cwd
/// (a crate dir under the checkout) stay inside it.
pub fn dev_repo_root() -> Option<&'static std::path::Path> {
    use std::path::PathBuf;
    static ROOT: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let exe = std::env::current_exe().ok();
        let candidates = [
            exe.clone(),
            exe.and_then(|exe| exe.canonicalize().ok()),
            std::env::current_dir().ok(),
        ];
        candidates.into_iter().flatten().find_map(|start| {
            Some(
                start
                    .ancestors()
                    .find(|dir| dir.join(".git").exists())?
                    .to_path_buf(),
            )
        })
    })
    .as_deref()
}
