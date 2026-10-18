//! 严重级别（对齐 zed `crates/ui/src/styles/severity.rs`）。
//!
//! [`Banner`](crate::Banner) 这类"按级别换配色 + 换图标"的组件用它：
//! 级别既决定背景/边框色，也决定配哪个图标。

/// Severity levels that determine the style of the component.
/// Usually, it affects the background. Most of the time,
/// it also follows with an icon corresponding the severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}
