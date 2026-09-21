//! 平台风格抽象（对齐 zed `crates/ui/src/styles/platform.rs`）。
//!
//! `KeyBinding` 之类的控件需要按宿主平台决定「⌘ / Ctrl / Win」怎么显示，
//! 但又不能在渲染期做 `cfg!` 分支散落各处 —— 于是抽出一个可显式覆盖的
//! `PlatformStyle` 值类型。

/// Expands to a literal so platform-specific hints can use `concat!` without allocating.
#[cfg(target_os = "macos")]
#[macro_export]
macro_rules! alt_key_name {
    () => {
        "option"
    };
}

/// Expands to a literal so platform-specific hints can use `concat!` without allocating.
#[cfg(not(target_os = "macos"))]
#[macro_export]
macro_rules! alt_key_name {
    () => {
        "alt"
    };
}

/// The platform style to use when rendering UI.
///
/// This can be used to abstract over platform differences.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum PlatformStyle {
    /// Display in macOS style.
    Mac,
    /// Display in Linux style.
    Linux,
    /// Display in Windows style.
    Windows,
}

impl PlatformStyle {
    /// Returns the [`PlatformStyle`] for the current platform.
    pub const fn platform() -> Self {
        if cfg!(any(target_os = "linux", target_os = "freebsd")) {
            Self::Linux
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Mac
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_matches_host() {
        // 我们不交叉编译，所以 platform() 必须与编译期 cfg 一致。
        let expected = if cfg!(any(target_os = "linux", target_os = "freebsd")) {
            PlatformStyle::Linux
        } else if cfg!(target_os = "windows") {
            PlatformStyle::Windows
        } else {
            PlatformStyle::Mac
        };
        assert_eq!(PlatformStyle::platform(), expected);
    }

    #[test]
    fn ordering_is_stable() {
        assert!(PlatformStyle::Mac < PlatformStyle::Linux);
        assert!(PlatformStyle::Linux < PlatformStyle::Windows);
    }
}
