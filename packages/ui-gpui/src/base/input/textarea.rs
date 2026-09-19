//! Textarea:多行文本域 = `editor::Editor` 的多行模式。
//!
//! 对齐 gpui-component 的 facade 形态:`TextareaState` 是类型别名,
//! `Textarea` 直接复用 Editor 的 Render(状态/渲染分离已在 editor 层完成,
//! facade 无需再包一层)。Enter 换行、Shift 组合、垂直滚动、undo 全部
//! 由引擎统一提供。
//!
//! 内核已独立成包(`packages/editor`),本文件只是控件库的门面薄壳。
//! `textarea()` / `auto_height()` 两个便捷构造也随类型住在内核
//! (orphan rule:外部 crate 不能给类型别名写 inherent impl)。

pub use editor::EditorMode;

pub type TextareaState = editor::Editor;
pub type Textarea = editor::Editor;
