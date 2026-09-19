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
//!
//! 与 zed 的差异:zed 的 `ButtonCommon` / `SelectableButton` / `ButtonLike`
//! (通用按钮渲染器)依赖 tooltip 回调与 ElevationIndex 等我们尚未建立的
//! 机制,待浮层体系(roadmap 阶段 4)就位后再补。

pub mod animation_ext;
pub mod clickable;
pub mod disableable;
pub mod styled_ext;
pub mod toggleable;
pub mod transformable;

pub use animation_ext::CommonAnimationExt;
pub use clickable::Clickable;
pub use disableable::Disableable;
pub use styled_ext::StyledExt;
pub use toggleable::{ToggleState, Toggleable};
pub use transformable::Transformable;
