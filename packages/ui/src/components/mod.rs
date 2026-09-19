//! components：复合组件层。
//!
//! [`crate::base`] 放"单个基础控件"（icon/button/input/slider）；本层放在
//! 基础控件之上、直接面向业务 UI 的组合件。与基础层的区别：
//!
//! - 基础层是一次性 `IntoElement`（无 `App`，只能硬编码色）；
//! - 复合层实现 `RenderOnce`，渲染阶段读 `cx.theme()` 取色。
//!
//! 当前包含：
//! - [`button`]：`ButtonStyle` + `IconButton`（对齐 zed 的 `IconButton`）。
//! - [`context_menu`]：上下文菜单（right-click / popup 菜单的通用实现）。
//! - [`tooltip`]：悬浮提示（[`Tooltip`] + [`TooltipHost`]，对齐 zed `Tooltip`）。
//! - [`popover_menu`]：点击触发的锚定浮层菜单（[`PopoverMenu`] +
//!   [`PopoverMenuHandle`]，对齐 zed `PopoverMenu`）。

pub mod button;
pub mod context_menu;
pub mod divider;
pub mod popover_menu;
pub mod stack;
pub mod tooltip;

pub use button::{ButtonRadius, ButtonStyle, IconButton, TintColor};
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem, RightClickMenu};
pub use divider::{Divider, DividerColor, DividerDirection};
pub use popover_menu::{PopoverMenu, PopoverMenuHandle, PopoverTrigger};
pub use stack::{h_flex, v_flex};
pub use tooltip::{Tooltip, TooltipHost, tooltip_host};
