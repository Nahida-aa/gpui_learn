//! 图标相关的复合件（对齐 zed `crates/ui/src/components/icon.rs` + `icon/`）。
//!
//! 基础 [`Icon`](crate::Icon) 在 `base/icon.rs`；这里放它的衍生品：
//! [`DecoratedIcon`]（角上叠装饰）、[`IconDecoration`]（装饰本身）与
//! [`IconWithIndicator`]（右下角状态徽标）。

mod decorated_icon;
mod icon_decoration;
mod icon_with_indicator;

pub use decorated_icon::DecoratedIcon;
pub use icon_decoration::{IconDecoration, IconDecorationKind, KnockoutIconName};
pub use icon_with_indicator::IconWithIndicator;
