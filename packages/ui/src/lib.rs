//! # aa-gpui-kit-ui —— 自研 GPUI 控件库（教学）
//!
//! 当前提供通用 [`Slider`]：单值（进度条/音量）与区间（Range 双 thumb）、
//! 线性/对数刻度、min/max/step、reverse 反向填充、无障碍 role/aria。
//! 借鉴自 `gpui-component` 的 `slider.rs`（Entity 状态 + 一次性元素双层架构），
//! 并补齐了原库缺失的交互：键盘分级微调、Esc 拖动取消、hover 视觉态。
//!
//! 进度条只是它的一个用法：外部定时 `set_value` + `disabled(true)` 即只读进度条。
//!
//! 值的变更通过 [`SliderEvent`]（`Change` / `Release`）用 `cx.subscribe` 订阅；
//! 拖动状态可用 [`SliderState::is_dragging`] 查询（如播放器拖动 seek 时静音）。
//!
//! 目录结构：
//! - [`base`]：通用基础控件层（`geometry` 数学换算 + `slider` 滑块）。
//!   未来其他组件（button/input 等）可并排放在 `base/` 下。
//!
//! 相关 workspace 包（不在本 crate 内）：
//! - `aa-gpui-kit-theme`（`packages/theme`）：主题系统，组件用它取色。
//! - `aa-gpui-kit-assets`（`packages/assets`）：图标等资源内嵌。

pub mod base;
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
/// aa-gpui-kit-ui 复用它，不自带资源目录。
pub use aa_gpui_kit_assets::Assets;
pub use base::button::{Button, ButtonVariant};
pub use base::geometry::{Scale, quantize};
pub use base::icon::{Icon, IconName};
pub use base::input::bind_input_keys;
pub use base::input::input::{InputEvent, InputState};
pub use base::slider::element::{DragSlider, Slider, SliderEvent};
pub use base::slider::slider_state::{SliderState, ThumbMode};
pub use base::slider::slider_value::SliderValue;
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
pub use components::divider::{Divider, DividerColor, DividerDirection};
pub use components::keybinding::{
    Key, KeyBinding, KeyBindingStyle, KeyIcon, render_keybinding_keystroke, render_modifiers,
    text_for_action, text_for_keystroke, text_for_keystrokes, text_for_keybinding_keystrokes,
};
pub use components::popover_menu::{PopoverMenu, PopoverMenuHandle, PopoverTrigger};
pub use components::tooltip::{Tooltip, TooltipHost, tooltip_host};
pub use traits::{
    Clickable, CommonAnimationExt, Disableable, StyledExt, ToggleState, Toggleable, Transformable,
};

/// 复用 gpui 的轴方向类型，方便调用方设置 `SliderState::axis`。
pub use gpui::Axis;
