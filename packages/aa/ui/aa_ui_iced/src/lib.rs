//! iced 侧的共享 UI 组件库 —— 对应 Zed 的 `crates/ui`。
//!
//! 与 `aa_ui_core`（框架无关的颜色算法、APCA 对比度）同一分工：本 crate 是**依赖
//! iced 的那一层**，gpui 侧对应的是 `aa_gpui_kit_ui`。两边组件**同名、同结构**，
//! 只是把 gpui 的 `Div`/`Entity` 换成 iced 的 `Element`/消息。
//!
//! 目前只有线程侧边栏需要的 [`ThreadItem`]。其余组件（`IconButton`、`List`、
//! `Divider`……）等真有消费方再搬 —— 提前搬只会留下一堆没有调用方的空壳。
#![warn(missing_docs)]

pub mod components;

pub use components::ai::{AgentThreadStatus, ThreadItem};
