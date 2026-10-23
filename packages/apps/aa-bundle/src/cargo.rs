//! cargo 构建环境查询：target 目录、profile 目录名、产物路径。
//!
//! 这里只**问** cargo / 拼路径，不执行构建——构建是项目自己的事
//! （aa-bundle 只打包）。

use std::path::{Path, PathBuf};

/// cargo 的 `target_directory`：`cargo metadata --no-deps` 问出。
///
/// **workspace 成员的 target 由 workspace 根决定**，不在 crate 目录下——
/// 把产物/输出猜在 `<crate>/target/` 是多 crate workspace 下的经典错误。
/// 这里问 cargo 而不是拼路径，顺带尊重 `CARGO_TARGET_DIR`；metadata 拿不到
/// （cargo 不在 PATH 等）时退回 `<project_dir>/target`，由调用方的存在性
/// 检查兜底。`--no-deps` 只读 manifest，不解析依赖、不联网。
pub fn resolve_target_dir(project_dir: &Path) -> PathBuf {
    std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("target_directory")?.as_str().map(PathBuf::from))
        .unwrap_or_else(|| project_dir.join("target"))
}

/// cargo profile 名 → target 下的产物目录名。`dev` 落 `debug`，其余同名
/// （`release` / `release-quick` / 自定义 profile）。
pub fn profile_dir(profile: &str) -> &str {
    if profile == "dev" { "debug" } else { profile }
}

/// cargo profile 名 → `cargo build` 的 flag。`release` 用简写（保持与旧输出
/// 一致），其余走 `--profile <name>`。
pub fn profile_flag(profile: &str) -> String {
    if profile == "release" {
        "--release".to_string()
    } else {
        format!("--profile {profile}")
    }
}

/// 二进制路径：`resolve_target_dir` 问出 workspace 根的 target/，拼
/// `<profile 目录>/<bin>`（也尊重 `CARGO_TARGET_DIR`）。
///
/// 各格式共用：pkgbuild 的源码模式（package() 拼路径）、prebuilt 模式（生成期
/// 定位）与 tar.gz（本来就只打包不构建）都要它。
pub fn resolve_binary_path(project_dir: &Path, bin: &str, profile: &str) -> PathBuf {
    resolve_target_dir(project_dir)
        .join(profile_dir(profile))
        .join(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_helpers_map_dev_and_release() {
        // dev → cargo 的 debug 目录；其余 profile 同名
        assert_eq!(profile_dir("dev"), "debug");
        assert_eq!(profile_dir("release"), "release");
        assert_eq!(profile_dir("release-quick"), "release-quick");
        // release 用简写 flag（历史输出兼容），其余 --profile
        assert_eq!(profile_flag("release"), "--release");
        assert_eq!(profile_flag("dev"), "--profile dev");
        assert_eq!(profile_flag("release-quick"), "--profile release-quick");
    }

    #[test]
    fn resolve_binary_path_uses_target_dir_and_profile_dir() {
        // 与既有 subprocess 测试同模式：临时目录里 cargo metadata 问出
        // target_directory（= <dir>/target），拼 <profile 目录>/<bin>。
        let dir = std::env::temp_dir().join(format!(
            "aa-bundle-cargo-{}-bin_path",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\n")
            .unwrap();
        let p = resolve_binary_path(&dir, "aacode", "release-quick");
        assert_eq!(
            p,
            dir.join("target").join("release-quick").join("aacode")
        );
        // dev 落 debug 目录
        let p = resolve_binary_path(&dir, "aacode", "dev");
        assert_eq!(p, dir.join("target").join("debug").join("aacode"));
    }
}
