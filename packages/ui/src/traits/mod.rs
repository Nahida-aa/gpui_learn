//! 交互组件的公共 trait 体系,对齐 zed `crates/ui/src/traits/`。
//!
//! 目标:Button / IconButton / 未来的菜单项、SplitButton 等都 impl 同一套
//! 接口,让「任何像按钮的东西」可以被统一消费(菜单、工具栏不再关心具体
//! 是哪个组件)。
//!
//! | trait | 职责 | zed 对应 |
//! |---|---|---|
//! | [`Clickable`] | on_click / cursor_style | 同名 |
//! | [`Disableable`] | disabled | 同名 |
//! | [`Toggleable`] + [`ToggleState`] | toggle 语义(含三态) | 同名 |
//! | [`Transformable`] | transform | 同名 |
//! | [`CommonAnimationExt`] | 常用动画(旋转等) | 同名 |
//! | [`SelectableButton`] | selected_style | 同名 |
//!
//! 与 zed 的差异:zed 的 `ButtonCommon` 还要求 `size` / `tab_index` /
//! `layer` / `track_focus`,那些依赖我们尚未建立的按钮尺寸档位(`ButtonSize`)
//! 与浮层体系,待其就位后补齐。

pub mod animation_ext;
pub mod clickable;
pub mod disableable;
pub mod fixed;
pub mod selectable_button;
pub mod styled_ext;
pub mod toggleable;
pub mod transformable;
pub mod visible_on_hover;

pub use animation_ext::CommonAnimationExt;
pub use clickable::Clickable;
pub use disableable::Disableable;
pub use fixed::FixedWidth;
pub use selectable_button::SelectableButton;
pub use styled_ext::StyledExt;
pub use toggleable::{ToggleState, Toggleable};
pub use transformable::Transformable;
pub use visible_on_hover::VisibleOnHover;
