//! 列表组件层（对齐 zed `crates/ui/src/components/list.rs`）。
//!
//! 目前只搬了 `ListItem`、`ListBulletItem` 与 `ListSubHeader` ——
//! `ListBulletItem` 渲染时要用 `ListItem`，`AlertModal` 的 preview 又要用
//! `ListBulletItem`，所以这两个是一起搬进来的。`ListSubHeader` 是 editor 的
//! code_context_menus 分组标题要用的。`List` 是 context_menu 整包照搬 zed 时
//! 带进来的（用 `List::new().children(..)` 装条目）。`ListHeader` 尚未搬，
//! 故 `List` 里没有 zed 那个 `header` 槽位（见 list/list.rs 的说明）。

pub mod list_bullet_item;
pub mod list_item;
pub mod list_separator;
pub mod list_sub_header;

pub mod list;
pub use list::*;
pub use list_bullet_item::*;
pub use list_separator::*;
pub use list_item::*;
pub use list_sub_header::*;
