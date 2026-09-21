//! # aa_gpui_kit_ui —— GPUI 控件库，**API 面逐字段对齐 zed `crates/ui`**
//!
//! 这里放的是「zed 也有的那些控件」：Button / Icon / Label / KeyBinding /
//! Avatar / Checkbox / Switch / Tooltip / PopoverMenu …。目标是外部代码能
//! **原样 import** zed 的写法（见 `packages/ui/tests/prelude.rs`）。
//!
//! ## 与 `aa_gpui_base` 的分工（2026-09 拆出）
//!
//! | 包 | 定位 |
//! |---|---|
//! | `aa_gpui_kit_ui`（本包） | 对齐 zed，API 面以 zed 为准 |
//! | `aa_gpui_base` | **zed 没有对应物**的自研基础件（geometry / icon / slider） |
//!
//! 依赖方向是**单向** `ui → base`：base 只管几何与图元，不认识主题、不认识
//! 复合控件；ui 在它之上搭 zed 那套。这样两边互不干扰 —— base 可以自由演化，
//! ui 也能专心对齐。拆分的完整理由见 `docs/zed/ui-input-analysis.md` 同级的分析。
//!
//! ## 相关 workspace 包（不在本 crate 内）
//!
//! - `aa_gpui_base`（`packages/base`）：图标 / 滑块 / 几何数学
//! - `aa_gpui_kit_theme`（`packages/theme`）：主题系统，组件用它取色
//! - `aa_gpui_kit_assets`（`packages/assets`）：图标等资源内嵌
//! - `aa_gpui_kit_ui_input`（`packages/ui_input`）：需要编辑器的表单件

pub mod component_prelude;
pub mod components;
pub mod prelude;
pub mod utils;

// 布局 helper 与 rem 换算（typography/color 等模块经 crate:: 根路径引用）。
pub use components::stack::{h_flex, v_flex};
pub use components::label::{Label, LabelCommon, LabelLike, LabelSize, LineHeightStyle};
pub use styles::units::{vh, vw, BASE_REM_SIZE_IN_PX, rems_from_px};
mod styles;
pub mod traits;
/// 图标等资源内嵌在仓库根 `assets/` 下，由工作区共享的 `assets` crate 统一加载。
/// aa_gpui_kit_ui 复用它，不自带资源目录。
pub use aa_gpui_kit_assets::Assets;

// ---- 基础控件层：来自 aa_gpui_base（原 packages/ui/src/base，2026-09 拆出）----
pub use aa_gpui_base::{
    DragSlider, Icon, IconName, IconSize, Scale, Slider, SliderEvent, SliderState, SliderValue,
    ThumbMode, position_to_value, quantize, value_to_percentage,
};
// 自研的普通按钮（`Button` + `ButtonVariant`）。注意它与对齐 zed 的
// `ButtonLike` / `IconButton` 是两套东西：后者是 zed 的面，这个是我们的。
pub use components::button::plain::{Button, ButtonVariant};
pub use styles::*;
// 主题系统在独立包 `aa-gpui-kit-theme`（原 `base/theme`）：组件从那里取色，
// 调用方也用 `theme_settings::init`（装配在 theme-settings 包）。这里**不做**别名 re-export——
// 「主题不隶属控件库」这件事在代码里应当可见。
pub use components::button::{
    ButtonCommon, ButtonLike, ButtonRadius, ButtonStyle, IconButton, SplitButton, SplitButtonKind,
    SplitButtonStyle, TintColor,
};
pub use components::context_menu::{
    ContextMenu, ContextMenuEntry, ContextMenuItem, RightClickMenu, right_click_menu,
};
pub use components::avatar::{
    AudioStatus, Avatar, AvatarAudioStatusIndicator, AvatarAvailabilityIndicator,
    CollaboratorAvailability,
};
pub use components::divider::{Divider, DividerColor, DividerDirection};
pub use components::facepile::{EXAMPLE_FACES, Facepile};
pub use components::icon::{DecoratedIcon, IconDecoration, IconDecorationKind, KnockoutIconName};
pub use components::image::{Vector, VectorName};
pub use components::indicator::Indicator;
pub use components::keybinding::{
    Key, KeyBinding, KeyBindingStyle, KeyIcon, render_keybinding_keystroke, render_modifiers,
    text_for_action, text_for_keystroke, text_for_keystrokes, text_for_keybinding_keystrokes,
};
pub use components::keybinding_hint::KeybindingHint;
pub use components::popover_menu::{PopoverMenu, PopoverMenuHandle, PopoverTrigger};
pub use components::tooltip::{Tooltip, TooltipHost, tooltip_host};
pub use components::toggle::{
    Checkbox, Switch, SwitchColor, SwitchLabelPosition, ToggleStyle, checkbox, switch,
};
pub use traits::{
    Clickable, CommonAnimationExt, Disableable, StyledExt, ToggleState, Toggleable, Transformable,
};

/// 复用 gpui 的轴方向类型，方便调用方设置 `SliderState::axis`。
pub use gpui::Axis;
