//! Tab 组件层（对齐 zed `crates/ui/src/components/tab.rs`）。
//!
//! 目前只搬了 `TabCloseSide` —— workspace 的 `pane/tab_bar.rs` 用它决定
//! 关闭按钮在标签的哪一侧。`Tab` 组件本体（含 `TabPosition` 与 slot 渲染）
//! 尚未搬。

/// 关闭按钮画在标签的哪一侧。
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TabCloseSide {
    Start,
    End,
}
