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

pub mod avatar;
pub mod banner;
pub mod button;
pub mod chip;
pub mod circular_progress;
pub mod context_menu;
pub mod count_badge;
pub mod disclosure;
pub mod divider;
pub mod facepile;
pub mod icon;
pub mod image;
pub mod indicator;
pub mod keybinding;
pub mod keybinding_hint;
pub mod label;
pub mod list;
pub mod modal;
pub mod navigable;
pub mod notification;
pub mod popover;
pub mod popover_menu;
pub mod scrollbar;
pub mod diff_stat;
pub mod stack;
pub mod tab;
pub mod tab_bar;
pub mod toggle;
pub mod update_button;
pub mod tooltip;

pub use banner::Banner;
pub use avatar::{
    AudioStatus, Avatar, AvatarAudioStatusIndicator, AvatarAvailabilityIndicator,
    CollaboratorAvailability,
};
pub use button::{
    ButtonCommon, ButtonRadius, ButtonStyle, IconButton, IconButtonShape, IconPosition, TintColor,
};
pub use chip::Chip;
pub use circular_progress::CircularProgress;
pub use context_menu::{ContextMenu, ContextMenuEntry, ContextMenuItem, DocumentationAside, DocumentationSide, RightClickMenu};
pub use count_badge::CountBadge;
pub use disclosure::Disclosure;
pub use divider::{Divider, DividerColor, DividerDirection};
pub use facepile::{EXAMPLE_FACES, Facepile};
pub use icon::{DecoratedIcon, IconDecoration, IconDecorationKind, KnockoutIconName};
pub use image::{Vector, VectorName};
pub use indicator::Indicator;
pub use keybinding::{
    Key, KeyBinding, KeyBindingStyle, KeyIcon, render_keybinding_keystroke, render_modifiers,
    text_for_action, text_for_keystroke, text_for_keystrokes, text_for_keybinding_keystrokes,
};
pub use keybinding_hint::KeybindingHint;
pub use label::{Label, LabelCommon, LabelSize, LineHeightStyle};
pub use list::list::{EmptyMessage, List};
pub use list::list_bullet_item::ListBulletItem;
pub use list::list_item::{ListItem, ListItemSpacing};
pub use list::list_separator::ListSeparator;
pub use list::list_sub_header::ListSubHeader;
pub use diff_stat::DiffStat;
pub use modal::{Modal, ModalFooter, ModalHeader, Section, SectionHeader};
pub use navigable::{Navigable, NavigableEntry};
pub use notification::alert_modal::AlertModal;
pub use popover::{POPOVER_Y_PADDING, Popover};
pub use popover_menu::{PopoverMenu, PopoverMenuHandle, PopoverTrigger};
pub use scrollbar::{
    EDITOR_SCROLLBAR_WIDTH, ReservedSpace, ScrollAxes, ScrollbarRevealPolicy, ScrollbarStyle,
    ScrollableHandle, Scrollbars, ShowBehavior, WithScrollbar, scrollbars,
};
pub use stack::{h_flex, v_flex};
pub use tab::{Tab, TabCloseSide, TabPosition};
pub use tab_bar::TabBar;
pub use toggle::{
    Checkbox, Switch, SwitchColor, SwitchLabelPosition, ToggleStyle, checkbox, switch,
};
pub use update_button::UpdateButton;
pub use tooltip::{Tooltip, TooltipHost, tooltip_container, tooltip_host};
