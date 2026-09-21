use gpui::DefiniteLength;

/// A trait for elements that can have a fixed width.
///
/// 对齐 zed `crates/ui/src/traits/fixed.rs`：按钮设固定宽 / 撑满容器宽。
pub trait FixedWidth {
    /// Sets the width of the element.
    fn width(self, width: impl Into<DefiniteLength>) -> Self;

    /// Sets the element's width to the full width of its container.
    fn full_width(self) -> Self;
}
