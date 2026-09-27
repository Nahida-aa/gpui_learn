//! context_menu：上下文菜单（右键菜单 / 弹出菜单的通用实现）。
//!
//! [`menu`](`crate::components::context_menu::menu`) 里那份 [`ContextMenu`]
//! 是**整文件照搬 zed** 的（详见该文件的模块头注释），所以键盘导航、子菜单、
//! 文档侧栏这些 zed 有的一应俱全。本模块自己提供的只有宿主元素：
//!
//! - [`right_click_menu`] / [`RightClickMenu`]：包裹任意 trigger，右键时现场
//!   创建菜单实体，用 `anchored` + `deferred(priority=1)` 渲染为覆盖浮层。
//!   宿主要是 [`ContextMenu`]，也可以是任何实现
//!   `ManagedView + Focusable + EventEmitter<DismissEvent>` 的 view。
//!
//! 数据层 [`ContextMenuItem`] / [`ContextMenuEntry`] 也在 `menu` 里（与 zed
//! 同：一个文件装完）。
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

pub mod menu;
pub mod right_click_menu;

pub use menu::{ContextMenu, ContextMenuEntry, ContextMenuItem, DocumentationAside, DocumentationSide};
// zed 的 `IconPosition` 定义在 `components/button/button_like.rs`（不是
// context_menu），菜单条目复用同一个枚举；这里转出只为兼容旧导入路径。
pub use crate::IconPosition;
pub use right_click_menu::{RightClickMenu, right_click_menu};
