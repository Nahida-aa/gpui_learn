use crate::components::button::ButtonStyle;
use crate::traits::Toggleable;

/// A trait for buttons that can be selected, and can have a distinct style when selected.
///
/// 对齐 zed `crates/ui/src/traits/selectable_button.rs`：`Button` / `IconButton`
/// 这类可切换的按钮用 `selected_style(..)` 指定「选中时」的样式。
pub trait SelectableButton: Toggleable {
    /// Set the style the button will use when it is selected.
    fn selected_style(self, style: ButtonStyle) -> Self;
}
