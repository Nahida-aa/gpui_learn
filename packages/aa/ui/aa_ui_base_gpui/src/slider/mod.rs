//! slider：滑块控件（单值 / Range 双 thumb）。
//!
//! 采用「`Entity<SliderState>`（持久状态） + `Slider`（一次性元素）」双层架构，
//! 值的换算与刻度逻辑在 [`crate::geometry`]，交互与渲染在 [`element`]。
//!
//! 颜色是调用方传入或内置常量（`element.rs` 里的硬编码 `rgb(..)`），
//! **本包不依赖 theme** —— 这是它和 `ui` 复合件的关键区别。

pub mod element;
pub mod slider_state;
pub mod slider_value;
