//! 输入控件 facade:单行 `Input` / 多行 `Textarea` 的门面薄壳。
//!
//! 编辑器内核(Rope 引擎 + 多行编辑器)已独立成 `packages/editor` 包
//! (2026-09 拆出,原 `base::input::{engine,editor}`),依赖方向
//! `ui → editor`。这里只剩两个门面控件:
//!
//! - [`input`]——单行输入 facade(`InputState` = Editor 的 SingleLine 模式,
//!   兼容旧 API)
//! - [`textarea`]——多行文本域 facade(`Textarea` = Editor 的多行模式)
//!
//! 外部 API:单行用 [`InputState`] / [`InputEvent`] / [`bind_input_keys`];
//! 多行用 [`Textarea`]。直接用内核(`Editor` / `EditorEvent` /
//! `bind_editor_keys` 等)请依赖 `editor` 包。

pub mod input;
pub mod textarea;

pub use input::{INPUT_KEY_CONTEXT, InputEvent, InputState, bind_input_keys};
pub use textarea::{Textarea, TextareaState};
