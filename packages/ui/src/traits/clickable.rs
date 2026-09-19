use gpui::{ClickEvent, CursorStyle, Window, App};

/// A trait for elements that can be clicked. Enables the use of the `on_click` method.
///
/// 对齐 zed `crates/ui/src/traits/clickable.rs`：把「可点击」抽象成统一的
/// builder 接口，Button / IconButton / 未来的菜单项等都 impl 它。
pub trait Clickable {
    /// Sets the click handler that will fire whenever the element is clicked.
    fn on_click(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self;
    /// Sets the cursor style when hovering over the element.
    fn cursor_style(self, cursor_style: CursorStyle) -> Self;
}
