//! 图标控件族（对齐 zed `crates/ui/src/components/icon.rs` + `icon/`）。
//!
//! 基础 [`Icon`] 在本目录 `icon.rs`（zed 也放在这里）；衍生品：
//! [`DecoratedIcon`]（角上叠装饰）、[`IconDecoration`]（装饰本身）与
//! [`IconWithIndicator`]（右下角状态徽标）。
//!
//! `IconName` 来自 `aa_icons`（zed `crates/icons` 原样），全仓库唯一的枚举。

mod decorated_icon;
mod icon;
mod icon_decoration;
mod icon_with_indicator;

pub use decorated_icon::DecoratedIcon;
pub use icon::{AnyIcon, Icon, IconSize, git_hosting_provider_icon};
pub use aa_icons::IconName;
pub use icon_decoration::{IconDecoration, IconDecorationKind, KnockoutIconName};
pub use icon_with_indicator::IconWithIndicator;
