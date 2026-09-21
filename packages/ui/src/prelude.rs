//! The prelude of this crate. When building UI you almost always want to import this.
//!
//! 对齐 zed `crates/ui/src/prelude.rs`：把最常用的 trait、组件与 gpui 原语
//! 聚合到一个 `use aa_gpui_kit_ui::prelude::*;` 里。只收拢**高频**项，
//! 低频 API（如 `Slider` 内部类型）仍按需从包根导入。
//!
//! ```ignore
//! use aa_gpui_kit_ui::prelude::*;
//!
//! // trait 方法、组件、布局 helper 一次到位
//! let bar = h_flex().child(
//!     IconButton::new("menu", IconName::Menu).on_click(|_, _, _| {}),
//! );
//! ```

pub use gpui::prelude::*;
pub use gpui::{
    AbsoluteLength, Anchor, AnyElement, App, BoxShadow, ClickEvent, Context, CursorStyle,
    DefiniteLength, DismissEvent, Div, Element, ElementId, Entity, Hsla, InteractiveElement,
    IntoElement, ManagedView, ParentElement, Pixels, Point, RenderOnce, SharedString, Stateful,
    Styled, Window, div, hsla, percentage, px, rgb, rgba,
};

pub use crate::traits::animation_ext::*;
pub use crate::traits::clickable::*;
pub use crate::traits::disableable::*;
pub use crate::traits::styled_ext::*;
pub use crate::traits::toggleable::*;
pub use crate::traits::transformable::*;

pub use crate::components::avatar::{
    AudioStatus, Avatar, AvatarAudioStatusIndicator, AvatarAvailabilityIndicator,
    CollaboratorAvailability,
};
pub use crate::components::button::{
    ButtonCommon, ButtonLike, ButtonRadius, ButtonStyle, IconButton, SplitButton, SplitButtonKind,
    SplitButtonStyle, TintColor,
};
pub use crate::components::context_menu::{
    ContextMenu, ContextMenuEntry, ContextMenuItem, RightClickMenu, right_click_menu,
};
pub use crate::components::divider::{Divider, DividerColor, DividerDirection};
pub use crate::components::facepile::Facepile;
pub use crate::components::icon::{DecoratedIcon, IconDecoration, IconDecorationKind};
pub use crate::components::indicator::Indicator;
pub use crate::components::keybinding::{Key, KeyBinding, KeyBindingStyle, KeyIcon};
pub use crate::components::keybinding_hint::KeybindingHint;
pub use crate::components::popover_menu::{PopoverMenu, PopoverMenuHandle, PopoverTrigger};
pub use crate::components::stack::{h_flex, v_flex};
pub use crate::components::tooltip::{Tooltip, TooltipHost, tooltip_host};
pub use crate::components::toggle::{Checkbox, Switch, SwitchColor, SwitchLabelPosition};
pub use crate::components::label::{Label, LabelCommon, LabelLike, LabelSize, LineHeightStyle};
pub use crate::{Button, ButtonVariant, Icon, IconName, Slider, SliderEvent, SliderState};

pub use crate::styles::{Color, DynamicSpacing, StyledTypography, TextSize, UiDensity};

pub use crate::component_prelude::*;

pub use aa_gpui_kit_theme::ActiveTheme;
