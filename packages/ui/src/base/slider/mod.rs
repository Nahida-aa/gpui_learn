//! slider：滑块控件（单值 / Range 双 thumb）。
//!
//! 采用「`Entity<SliderState>`（持久状态） + `Slider`（一次性元素）」双层架构，
//! 值的换算与刻度逻辑在 [`geometry`]（base 层），交互与渲染在 [`element`]。

pub mod element;
pub mod slider_state;
pub mod slider_value;
