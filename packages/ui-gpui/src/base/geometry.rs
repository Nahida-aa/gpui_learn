//! 滑块的几何与刻度换算。
//!
//! 把「像素位置 ↔ 百分比(0..1) ↔ 实际值」三者的转换隔离在这一层，
//! 上层（拖动、点击、键盘）只跟「值」打交道，不关心刻度是线性还是对数、
//! 轴是水平还是垂直。这是抄 gpui-component `slider.rs` 时提炼出的关键抽象：
//! 它把线性/对数差异封在 `percentage_to_value` / `value_to_percentage` 两个函数里，
//! 其余代码无感（`slider.rs:310,325`）。

use gpui::{Axis, Bounds, Pixels, Point, Window};

/// 刻度模式。
///
/// - `Linear`：值沿轨道均匀分布（默认，进度条/通用滑块用）。
/// - `Log`：对数刻度（`value = min * (max/min)^p`），适合音量、频率等
///   感知非线性的量。要求 `min > 0`，否则对数无意义。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Scale {
    #[default]
    Linear,
    Log,
}

impl Scale {
    /// 百分比(0..1) → 实际值。
    pub fn percentage_to_value(&self, p: f32, min: f32, max: f32) -> f32 {
        let p = p.clamp(0.0, 1.0);
        match self {
            Scale::Linear => min + p * (max - min),
            // 对数：min*(max/min)^p。min<=0 时退化成线性，避免 NaN/无穷。
            Scale::Log if min > 0.0 && max > 0.0 => min * (max / min).powf(p),
            Scale::Log => min + p * (max - min),
        }
    }

    /// 实际值 → 百分比(0..1)（上面函数的逆）。
    pub fn value_to_percentage(&self, v: f32, min: f32, max: f32) -> f32 {
        if (max - min).abs() < f32::EPSILON {
            return 0.0;
        }
        let p = match self {
            Scale::Linear => (v - min) / (max - min),
            Scale::Log if min > 0.0 && max > 0.0 => (v / min).ln() / (max / min).ln(),
            Scale::Log => (v - min) / (max - min),
        };
        p.clamp(0.0, 1.0)
    }
}

/// 把「指针在窗口中的位置」换算成滑块「百分比(0..1)」。
///
/// 与 gpui-component `slider.rs:370-392` 同构：水平取 `x - left`，垂直要
/// 翻转 Y 轴（`bottom - y`），因为屏幕坐标 y 向下增长而我们的轨道百分比
/// 从「起点」往「终点」增长。结果 clamp 到 [0,1]。
///
/// `bounds` 来自 `on_prepaint` 回写的布局结果——没有它就没法从像素算值。
pub fn position_to_percentage(
    axis: Axis,
    position: Point<Pixels>,
    bounds: &Bounds<Pixels>,
) -> f32 {
    let size = match axis {
        Axis::Horizontal => bounds.size.width,
        Axis::Vertical => bounds.size.height,
    };
    if size == Pixels::ZERO {
        return 0.0;
    }
    let inner_pos = match axis {
        Axis::Horizontal => position.x - bounds.left(),
        // 垂直：轨道底部对应百分比 1，顶部对应 0，故 bottom - y。
        Axis::Vertical => bounds.bottom() - position.y,
    };
    (inner_pos / size).clamp(0.0, 1.0)
}

/// 在窗口里把指针位置换算成实际值：先百分比再按刻度转值，最后 step 取整 + 夹紧。
///
/// `step` 为 0 或负视为「不取整」。这是 click 与 drag 共用的唯一入口
/// （照抄 gpui-component `update_value_by_position` 的单一职责思路，`slider.rs:358`）。
pub fn position_to_value(
    axis: Axis,
    scale: Scale,
    position: Point<Pixels>,
    bounds: &Bounds<Pixels>,
    min: f32,
    max: f32,
    step: f32,
) -> f32 {
    let pct = position_to_percentage(axis, position, bounds);
    let raw = scale.percentage_to_value(pct, min, max);
    quantize(raw, min, max, step)
}

/// 值 → 百分比（渲染 fill/thumb 用），转发到 `Scale`。
pub fn value_to_percentage(v: f32, scale: Scale, min: f32, max: f32) -> f32 {
    scale.value_to_percentage(v, min, max)
}

/// step 取整 + 夹紧到 [min, max]。
///
/// gpui-component 只在最后做 `(value/step).round()*step` 且**不再夹回**，
/// step 不整除 range 时会略微越界（`slider.rs:392`）。我们补一次 clamp，
/// 因为越界值在进度条/音量场景里会画出轨道外的 fill。step<=0 视为不取整。
pub fn quantize(v: f32, min: f32, max: f32, step: f32) -> f32 {
    let v = if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    };
    v.clamp(min, max)
}

/// 让 `Window` 在类型标注时少写一点（仅文档用途，避免未用告警）。
#[allow(dead_code)]
pub(crate) fn _assert_window(_: &Window) {}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn linear_roundtrip() {
        let s = Scale::Linear;
        // 0.5 → 中点
        assert!((s.percentage_to_value(0.5, 0.0, 100.0) - 50.0).abs() < 1e-6);
        // 50 → 0.5
        assert!((s.value_to_percentage(50.0, 0.0, 100.0) - 0.5).abs() < 1e-6);
        // 越界百分比被 clamp
        assert!((s.percentage_to_value(1.5, 0.0, 100.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn log_roundtrip() {
        let s = Scale::Log;
        // 中点附近：min=1,max=100，p=0.5 → sqrt(100)=10
        assert!((s.percentage_to_value(0.5, 1.0, 100.0) - 10.0).abs() < 1e-4);
        assert!((s.value_to_percentage(10.0, 1.0, 100.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn log_falls_back_when_nonpositive() {
        // min<=0 时对数量无意义，应退化成线性而非产生 NaN
        let s = Scale::Log;
        assert!(s.percentage_to_value(0.5, 0.0, 100.0).is_finite());
        assert!((s.percentage_to_value(0.5, 0.0, 100.0) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn quantize_rounds_and_clamps() {
        // step=10：37 → 40
        assert_eq!(quantize(37.0, 0.0, 100.0, 10.0), 40.0);
        // step=10：99 → 四舍五入到 100，超过 max=95 → 被 clamp 回 95（gpui-component 不会 clamp，
        // 会越界成 100 画出轨道外；我们补上 clamp）。
        assert_eq!(quantize(99.0, 0.0, 95.0, 10.0), 95.0);
        // step<=0：原样
        assert_eq!(quantize(37.3, 0.0, 100.0, 0.0), 37.3);
    }

    #[test]
    fn position_to_percentage_horizontal() {
        let bounds = Bounds::new(
            gpui::point(px(100.0), px(0.0)),
            gpui::size(px(200.0), px(10.0)),
        );
        // 中点 x=200 → 0.5
        let p = position_to_percentage(
            Axis::Horizontal,
            gpui::point(px(200.0), px(5.0)),
            &bounds,
        );
        assert!((p - 0.5).abs() < 1e-6);
        // 超出左边界 → 0
        let p = position_to_percentage(
            Axis::Horizontal,
            gpui::point(px(0.0), px(5.0)),
            &bounds,
        );
        assert_eq!(p, 0.0);
    }

    #[test]
    fn position_to_percentage_vertical_flips_y() {
        let bounds = Bounds::new(
            gpui::point(px(0.0), px(0.0)),
            gpui::size(px(10.0), px(200.0)),
        );
        // 垂直：y=bottom(200) 对应 1.0，y=top(0) 对应 0.0。取中点 y=100 → 0.5
        let p = position_to_percentage(
            Axis::Vertical,
            gpui::point(px(5.0), px(100.0)),
            &bounds,
        );
        assert!((p - 0.5).abs() < 1e-6);
    }
}
