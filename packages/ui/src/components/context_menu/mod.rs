//! context_menu：上下文菜单（右键菜单 / 弹出菜单的通用实现）。
//!
//! 对齐 zed `crates/ui/src/components/context_menu.rs` 与 `right_click_menu.rs`
//! 的精简版，由三块组成：
//!
//! - [`right_click_menu`] / [`RightClickMenu`]：宿主元素。包裹任意 trigger，
//!   右键时现场创建菜单实体，并用 `anchored` + `deferred(priority=1)`
//!   渲染为覆盖浮层（无 overlay 层也能工作）。
//! - [`ContextMenu`]：菜单实体（`ManagedView`），`build` builder 装配内容项，
//!   render 时画成"勾选列 + label + hover 背景"的条目列表；点击条目执行回调并
//!   关闭，点击外部 / Esc 触发 `DismissEvent` 归还焦点。
//! - [`ContextMenuItem`]/[`ContextMenuEntry`]：菜单内容的数据层。
//!
//! 典型用法（状态栏 dock 面板按钮的右键菜单）：
//!
//! ```ignore
//! right_click_menu::<ContextMenu>(ElementId::from(("dock-panel", kind)))
//!     .trigger(move |_is_active, _window, cx| {
//!         IconButton::new(("dock", kind), kind.icon()).into_any_element()
//!     })
//!     .menu(move |_window, cx| {
//!         ContextMenu::build(window, cx, |menu, _window, _cx| {
//!             menu.item(ContextMenuEntry::new("Dock Left").checked(true))
//!         })
//!     })
//! ```

pub mod entry;
pub mod menu;
pub mod right_click_menu;

pub use entry::{ContextMenuEntry, ContextMenuItem, IconPosition};
pub use menu::ContextMenu;
pub use right_click_menu::{RightClickMenu, right_click_menu};
