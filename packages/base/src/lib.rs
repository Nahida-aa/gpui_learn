//! # aa_gpui_base —— 自研基础控件层
//!
//! ## 它和 `aa_gpui_kit_ui` 的分工
//!
//! | 包 | 定位 | 和 zed 的关系 |
//! |---|---|---|
//! | `aa_gpui_kit_ui` | 复合控件库 | **对齐 zed `crates/ui` 的 API 面** |
//! | `aa_gpui_base`（本包） | 基础控件 + 通用数学 | **zed 没有对应物**，自己实现 |
//!
//! 拆开的目的：让 `ui` 能专心做「与 zed 逐字段对齐」，不必迁就我们自研件的形状；
//! 反过来 `base` 也能自由演化，不受 zed API 的约束。
//!
//! ## 依赖方向
//!
//! ```text
//!   aa_gpui_base          ← 本包：geometry / icon / slider（只依赖 gpui + assets）
//!        ▲
//!        │
//!   aa_gpui_kit_ui        ← 复合件；用本包的 Icon / IconSize / Slider
//! ```
//!
//! **单向**：本包不依赖 `ui`，也不依赖 `theme`（slider 的颜色是调用方传入或
//! 内置常量，见 `slider/element.rs`）。`button.rs` 没有搬进来 —— 它需要
//! `ui::traits::{Clickable, Disableable}`，说明它是复合件而非基础件，
//! 且定位与 `ui::ButtonLike` 重叠，留在 `ui` 里。
//!
//! ## 目录
//!
//! - [`geometry`]：刻度换算 / 像素↔值 / step 取整等纯数学，不绑定具体控件
//! - [`icon`]：图标控件（[`Icon`] + [`IconName`] + [`IconSize`]），渲染内嵌 SVG
//! - [`slider`]：滑块（单值 / Range 双 thumb，[`SliderState`] + [`Slider`]）

pub mod geometry;
pub mod icon;
pub mod slider;
pub mod transformable;

pub use geometry::{Scale, position_to_value, quantize, value_to_percentage};
pub use icon::{Icon, IconName, IconSize};
pub use slider::element::{DragSlider, Slider, SliderEvent};
pub use slider::slider_state::{SliderState, ThumbMode};
pub use slider::slider_value::SliderValue;
pub use transformable::Transformable;
