use std::path::PathBuf;

fn dev_repo_root() -> Option<&'static std::path::Path> {
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

fn main() {
    println!("current_exe: {:?}", std::env::current_exe());
    println!("canonicalize: {:?}", std::env::current_exe().and_then(|e| e.canonicalize()).ok());
    println!("current_dir: {:?}", std::env::current_dir());
    println!("dev_repo_root: {:?}", dev_repo_root());
}
