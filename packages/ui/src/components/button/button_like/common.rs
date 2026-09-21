//! button_like/common：所有"按钮类"元素共有的配置接口。
//!
//! 对齐 zed `button_like.rs` 顶部的 `ButtonCommon` trait（精简版）：
//! zed 的版本还要求 `size` / `tab_index` / `layer` / `track_focus`，
//! 那些依赖我们尚未建立的 UI 密度与浮层体系，先只收拢实际共有的三件。
//!
//! 有了它，`menu` / `toolbar` 这类容器就能接受"任何按钮"而不关心
//! 具体是 `Button`、`IconButton` 还是 `ButtonLike`。

use gpui::{App, ElementId, Entity, Window};

use crate::components::tooltip::Tooltip;

/// 所有按钮类元素共有的配置接口（builder 风格，方法消费 self 返回 Self）。
pub trait ButtonCommon {
    /// A unique element ID to identify the button.
    fn id(&self) -> &ElementId;
    /// The visual style of the button.
    fn style(self, style: crate::components::button::ButtonStyle) -> Self;
    /// Tab 键导航序号（对齐 zed `ButtonCommon::tab_index`）。
    fn tab_index(self, tab_index: impl Into<isize>) -> Self;
    /// The tooltip that shows when a user hovers over the button.
    ///
    /// Nearly all interactable elements should have a tooltip. Some example
    /// exceptions might be a scroll bar, or a slider.
    fn tooltip(
        self,
        tooltip: impl Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static,
    ) -> Self;
}
