//! # ui-gpui —— 自研 GPUI 控件库（教学）
//!
//! 当前提供通用 [`Slider`]：线性/对数刻度、min/max/step、单值。
//! 借鉴自 `gpui-component` 的 `slider.rs`（Entity 状态 + 一次性元素双层架构），
//! 并补齐了原库缺失的交互：键盘分级微调、Esc 拖动取消、hover/聚焦视觉态。
//!
//! 进度条只是它的一个用法：外部定时 `set_value` + `disabled(true)` 即只读进度条。
//!
//! 值的变更通过 [`SliderEvent`]（`Change` / `Release`）用 `cx.subscribe` 订阅。

mod geometry;
mod slider;
mod slider_state;

pub use geometry::{quantize, Scale};
pub use slider::{DragSlider, Slider, SliderEvent};
pub use slider_state::SliderState;

/// 复用 gpui 的轴方向类型，方便调用方设置 `SliderState::axis`。
pub use gpui::Axis;
