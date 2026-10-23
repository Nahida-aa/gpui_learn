//! 身份命名：包名段规范化、identifier 推导、仓库名发现。
//!
//! 这些是「给应用起合法名字」的公共规则：桌面 identifier（反向域名）、
//! Android applicationId 与 crate/二进制名都过同一道 [`pkg_segment`]。

use std::path::Path;

/// 把一个字符串规范成「合法标识段」(`[A-Za-z][A-Za-z0-9_]*`)：
/// - 仅保留 ASCII 字母/数字/下划线，其余（含 `-`、空格、中文、点等）替换为 `_`；
/// - 整体转小写（反向域名惯例全小写）；
/// - **剔除开头所有非字母**（数字/下划线不能作段首，而不是给它们补 `_` ——
///   补 `_` 后仍是非法段）。如 `_09_a11y` → `a11y`、`3repo` → `repo`；
/// - 若剔除后不含任何字母（纯符号/中文名），兜底为单段 `x`，保证段非空且合法。
///
/// 仓库目录名可能含 `-`/大写，包名/二进制名应以同样规则约束，
/// 故它们都过这一道。Android applicationId 与桌面 identifier 都用它分段。
pub fn pkg_segment(s: &str) -> String {
    let mut seg: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while seg
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        seg.remove(0);
    }
    if seg.is_empty() {
        seg.push('x');
    }
    seg
}

/// 推导默认 identifier：`<仓库名>.<bin 名>`，两段都过 [`pkg_segment`] 规范化。
///
/// 第二段用 **bin 名**而非 package 名 —— `[[bin]].name` 完全可以与
/// `package.name` 不同，用包名就是假设用户从不设 bin 名。bin 名的取值规则：
/// - 有 `[[bin]]` 且唯一 → 该目标名
/// - 无 `[[bin]]` → 包名（cargo 的自动发现规则，不是猜）
/// - 多个 `[[bin]]` → 歧义，调用方应要求显式声明（返回 `None`）
///
/// `bin_name` 传 `None` 表示「多 bin，无法推断」。仓库名拿不到（不在 git 仓库内）
/// 同样返回 `None`，由调用方报错。
pub fn derive_identifier(repo_name: &str, bin_name: Option<&str>) -> Option<String> {
    let bin = bin_name?;
    Some(format!("{}.{}", pkg_segment(repo_name), pkg_segment(bin)))
}

/// 推导仓库名（用于默认包名 / identifier 的命名空间前缀）。
///
/// 优先用 `git rev-parse --show-toplevel` 拿仓库根目录名；若 git 不可用，
/// 则向上回溯目录树找第一个含 `.git` 的祖先，取其目录名。**两者都拿不到
/// （不在 git 仓库内）返回 None**——调用方据此报错，不兜底假名。
pub fn repo_name_from(project_dir: &Path) -> Option<String> {
    // 1) git 可用时直接用仓库根目录名（最可靠）。
    if let Some(name) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            Path::new(s.trim())
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
    {
        return Some(name);
    }

    // 2) git 不可用：向上找含 .git 的祖先目录，取其目录名。
    let mut dir = project_dir.canonicalize().ok()?;
    loop {
        if dir.join(".git").exists() {
            return dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .filter(|s| !s.is_empty());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkg_segment_normalizes_names() {
        // 大写 + 连字符 → 小写 + 下划线
        assert_eq!(pkg_segment("My-Repo2"), "my_repo2");
        // 以数字开头 → 剔除数字，不能前缀补 _
        assert_eq!(pkg_segment("3repo"), "repo");
        // 中文等非 ASCII → 下划线；剔光后兜底为单个合法段
        assert_eq!(pkg_segment("我的仓库"), "x");
        // 合法名不变
        assert_eq!(pkg_segment("uniform_list_07"), "uniform_list_07");
        assert_eq!(pkg_segment("_09_a11y"), "a11y");
        assert_eq!(pkg_segment("AaCode"), "aacode");
    }

    #[test]
    fn derive_identifier_uses_repo_and_bin() {
        // 第二段是 bin 名，不是 package 名
        assert_eq!(
            derive_identifier("gpui_learn", Some("AaCode")).as_deref(),
            Some("gpui_learn.aacode")
        );
        // 多 bin 无法推断 → None（调用方应要求显式声明）
        assert_eq!(derive_identifier("gpui_learn", None), None);
        // 仓库名同样要规范化
        assert_eq!(
            derive_identifier("My-Repo", Some("app_01")).as_deref(),
            Some("my_repo.app_01")
        );
    }
}
