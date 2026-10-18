use gpui::{InteractiveElement, SharedString, Styled};

/// 元素只在指定 group 被 hover 时可见（对齐 zed `ui::VisibleOnHover`）。
///
/// `group_name` 传 `""` 表示用全局 group。
pub trait VisibleOnHover {
    fn visible_on_hover(self, group_name: impl Into<SharedString>) -> Self;
}

impl<E: InteractiveElement + Styled> VisibleOnHover for E {
    fn visible_on_hover(self, group_name: impl Into<SharedString>) -> Self {
        self.invisible().group_hover(group_name, |style| style.visible())
    }
}
