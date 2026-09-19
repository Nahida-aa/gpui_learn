//! `Styled` trait 扩展 —— 提供 Zed 风格的 `h_flex()` / `v_flex()` 快捷方法。

use gpui::Styled;

/// Extends [`gpui::Styled`] with layout helpers.
pub trait StyledExt: Styled + Sized {
    /// Horizontally stacks elements.
    ///
    /// Sets `flex()`, `flex_row()`, `items_center()`
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Vertically stacks elements.
    ///
    /// Sets `flex()`, `flex_col()`
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }
}

impl<E: Styled> StyledExt for E {}
