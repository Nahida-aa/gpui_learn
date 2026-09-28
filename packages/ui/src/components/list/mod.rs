//! 列表组件层（对齐 zed `crates/ui/src/components/list.rs`）。
//!
//! 目前只搬了 `ListItem`、`ListBulletItem` 与 `ListSubHeader` ——
//! `ListBulletItem` 渲染时要用 `ListItem`，`AlertModal` 的 preview 又要用
//! `ListBulletItem`，所以这两个是一起搬进来的。`ListSubHeader` 是 editor 的
//! code_context_menus 分组标题要用的。`List` 是 context_menu 整包照搬 zed 时
//! 带进来的（用 `List::new().children(..)` 装条目）。`ListHeader` 是 collab_panel 的分组标题要用才搬进来的（带 toggle /
//! end_hover_slot / dock，`ListSubHeader` 是它的简化版）。`List` 里 zed 那个
//! `header` 槽位仍未搬，见 list/list.rs 的说明。

pub mod list;
pub mod list_bullet_item;
pub mod list_header;
pub mod list_item;
pub mod list_separator;
pub mod list_sub_header;

pub use list::*;
pub use list_bullet_item::*;
pub use list_header::*;
pub use list_item::*;
pub use list_separator::*;
pub use list_sub_header::*;
