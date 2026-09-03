//! input：单行文本输入框。
//!
//! 采用与 [`crate::base::slider`] 一致的「`Entity<InputState>`（持久状态） +
//! 一次性元素」架构，但渲染入口直接放在 `InputState` 的 [`gpui::Render`] 实现上
//! ——因为输入框要用 `cx.listener(...)` 绑 action，而 `RenderOnce` 只拿到
//! `&mut App` 取不到 listener。视觉定制走 `InputState` 上的 setter。
//!
//! 与教学示例 `apps/04_input` 相比，本组件补齐/修正了：
//! - 对外发 [`InputEvent`]（`Change` / `Submit`），外部用 `cx.subscribe` 订阅；
//! - `character_index_for_point` 不再 `assert_eq!(line.text, content)`：内容为空时
//!   渲染的是 placeholder，原断言会 panic；同时改用 `closest_index_for_x`
//!   （与 `index_for_mouse_position` 保持一致）；
//! - 按键绑定收进 [`bind_input_keys`]，并用 key_context 限定作用域，
//!   不再像示例那样全局（None）绑定。

pub mod element;
pub mod input_state;

pub use element::TextElement;
pub use input_state::{
    Backspace, Copy, Cut, Delete, End, Enter, Home, INPUT_KEY_CONTEXT, InputEvent, InputState, Left,
    Paste, Right, SelectAll, SelectLeft, SelectRight, ShowCharacterPalette, bind_input_keys,
};
