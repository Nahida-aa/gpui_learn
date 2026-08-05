//! # ui-gpui —— 自研 GPUI 控件库（教学）
//!
//! 当前提供通用 [`Slider`]：单值（进度条/音量）与区间（Range 双 thumb）、
//! 线性/对数刻度、min/max/step、reverse 反向填充、无障碍 role/aria。
//! 借鉴自 `gpui-component` 的 `slider.rs`（Entity 状态 + 一次性元素双层架构），
//! 并补齐了原库缺失的交互：键盘分级微调、Esc 拖动取消、hover/聚焦视觉态。
//!
//! 进度条只是它的一个用法：外部定时 `set_value` + `disabled(true)` 即只读进度条。
//!
//! 值的变更通过 [`SliderEvent`]（`Change` / `Release`）用 `cx.subscribe` 订阅；
//! 拖动状态可用 [`SliderState::is_dragging`] 查询（如播放器拖动 seek 时静音）。

mod geometry;
mod slider;
mod slider_state;
mod slider_value;

pub use geometry::{quantize, Scale};
pub use slider::{DragSlider, Slider, SliderEvent};
pub use slider_state::SliderState;
pub use slider_value::SliderValue;

/// 复用 gpui 的轴方向类型，方便调用方设置 `SliderState::axis`。
pub use gpui::Axis;
