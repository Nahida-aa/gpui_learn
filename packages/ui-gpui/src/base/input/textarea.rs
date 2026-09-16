//! Textarea:多行文本域 = [`Editor`] 的多行模式。
//!
//! 对齐 gpui-component 的 facade 形态:`TextareaState` 是类型别名,
//! `Textarea` 直接复用 Editor 的 Render(状态/渲染分离已在 editor 层完成,
//! facade 无需再包一层)。Enter 换行、Shift 组合、垂直滚动、undo 全部
//! 由引擎统一提供。

pub type TextareaState = crate::base::input::editor::Editor;
pub type Textarea = crate::base::input::editor::Editor;

use crate::base::input::editor::EditorMode;

impl Textarea {
    /// 固定行数的多行文本域(内容超出时滚动)。
    pub fn textarea(rows: usize, cx: &mut gpui::Context<Self>) -> Self {
        Self::with_mode(EditorMode::MultiLine { rows }, cx)
    }

    /// 高度随内容自适应(限行数)。
    pub fn auto_height(min_rows: usize, max_rows: usize, cx: &mut gpui::Context<Self>) -> Self {
        Self::with_mode(EditorMode::AutoHeight { min_rows, max_rows }, cx)
    }
}
