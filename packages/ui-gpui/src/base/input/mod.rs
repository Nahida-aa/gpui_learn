//! 输入组件模块:editor 引擎 + Input(单行)/ Textarea(多行)facade。
//!
//! 分层(参照 gpui-component,概念对齐 zed):
//! - [`engine`]——自研 Rope 文本引擎(sum_tree 底座,阶段 0)
//! - [`editor`]——多行编辑引擎:选区、undo、display_map、movement、渲染
//! - [`input`]——单行输入 facade(= Editor 的 SingleLine 模式,兼容旧 API)
//! - [`textarea`]——多行文本域 facade(= Editor 的 MultiLine/AutoHeight 模式)
//!
//! 外部 API:单行用 [`InputState`] / [`InputEvent`] / [`bind_input_keys`];
//! 多行用 [`Editor`](或 [`Textarea`]) / [`EditorEvent`] / [`bind_editor_keys`]。

pub mod editor;
pub mod engine;
pub mod input;
pub mod textarea;

pub use editor::{Editor, EditorElement, EditorEvent, EditorMode, bind_editor_keys};
pub use input::{INPUT_KEY_CONTEXT, InputEvent, InputState, bind_input_keys};
pub use textarea::{Textarea, TextareaState};
