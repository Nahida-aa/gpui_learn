//! # editor —— 自研 GPUI 编辑器内核（架构对齐 zed `crates/editor` 的精简版）
//!
//! 2026-09 从 `ui-gpui/src/base/input` 拆出（原 `base::input::{engine,editor}`）。
//! 拆分边界：**内核**（Rope 引擎 + 多行编辑器）在这里；单行 `Input` /
//! 多行 `Textarea` 的薄 facade 留在 ui-gpui（它们是控件库的门面控件）。
//!
//! ## 分层（域内结构不变，参照 gpui-component、概念对齐 zed）
//!
//! - [`engine`]——自研 Rope 文本引擎（sum_tree 底座）：文本存储、
//!   字节/UTF-16/Point 三系坐标
//! - [`editor`]——多行编辑器：选区、undo、display_map、movement、
//!   渲染元素、动作
//!
//! ## 依赖方向
//!
//! `editor → aa-gpui-kit-theme`（取选区/光标色）。编辑器内核不隶属控件库，
//! 也不依赖任何其他自研包。
//!
//! ## 用法
//!
//! 一般不直接用本包——ui-gpui 的 `InputState`（单行）/ `Textarea`（多行）
//! facade 是常规入口。直接用内核的场景（自建编辑器组件、Android 后端）：
//!
//! ```ignore
//! use editor::{Editor, EditorMode, bind_editor_keys, EDITOR_KEY_CONTEXT};
//!
//! let editor = cx.new(|cx| Editor::with_mode(EditorMode::MultiLine { rows: 8 }, cx));
//! ```

pub mod editor;
pub mod engine;

pub use editor::{
    EDITOR_KEY_CONTEXT, INPUT_KEY_CONTEXT, Editor, EditorElement, EditorEvent, EditorMode,
    ErasedEditorImpl, InputEvent, InputState, Textarea, TextareaState, bind_editor_keys,
    bind_input_keys, register_erased_editor_factory,
};
