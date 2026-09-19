//! 滑块的值类型：单值或区间（Range，双 thumb）。
//!
//! 与 gpui-component `SliderValue`（`slider.rs:44-137`）同构。Single 用于
//! 进度条/音量；Range 用于需要"起止区间"的场合（如时间轴选段）。

use std::ops::Range;

/// 滑块的值。
///
/// - `Single(v)`：单值滑块（进度条、音量条），只渲染一个 thumb。
/// - `Range(a, b)`：区间滑块（双 thumb），`a` 是起点、`b` 是终点，`a <= b`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range(f32, f32),
}

impl SliderValue {
    /// 区间的起点（Single 时就是值本身）。
    pub fn start(&self) -> f32 {
        match self {
            Self::Single(v) | Self::Range(v, _) => *v,
        }
    }

    /// 区间的终点（Single 时就是值本身）。
    pub fn end(&self) -> f32 {
        match self {
            Self::Single(v) | Self::Range(_, v) => *v,
        }
    }

    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }

    pub fn is_range(&self) -> bool {
        matches!(self, Self::Range(_, _))
    }

    /// 夹紧到 [min, max]，并保证 Range 的 a <= b。
    pub fn clamp(self, min: f32, max: f32) -> Self {
        match self {
            Self::Single(v) => Self::Single(v.clamp(min, max)),
            Self::Range(a, b) => {
                let a = a.clamp(min, max);
                let b = b.clamp(min, max);
                if a <= b {
                    Self::Range(a, b)
                } else {
                    Self::Range(b, a)
                }
            }
        }
    }

    /// 把区间中的某个端点替换成新值（`is_start` 选起点/终点），并保证顺序：
    /// 起点不越过终点（≤b），终点不越过起点（≥a）。Single 时直接替换整个值。
    pub fn with_thumb(self, is_start: bool, value: f32) -> Self {
        match self {
            Self::Single(_) => Self::Single(value),
            Self::Range(a, b) => {
                if is_start {
                    // 起点 = value，但不许越过终点。
                    Self::Range(value.min(b), b)
                } else {
                    // 终点 = value，但不许越过起点。
                    Self::Range(a, value.max(a))
                }
            }
        }
    }
}

impl Default for SliderValue {
    fn default() -> Self {
        Self::Single(0.0)
    }
}

impl From<f32> for SliderValue {
    fn from(v: f32) -> Self {
        Self::Single(v)
    }
}

impl From<(f32, f32)> for SliderValue {
    fn from((a, b): (f32, f32)) -> Self {
        Self::Range(a, b)
    }
}

impl From<Range<f32>> for SliderValue {
    fn from(r: Range<f32>) -> Self {
        Self::Range(r.start, r.end)
    }
}

impl std::fmt::Display for SliderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(v) => write!(f, "{v:.2}"),
            Self::Range(a, b) => write!(f, "({a:.2}, {b:.2})"),
        }
    }
}
