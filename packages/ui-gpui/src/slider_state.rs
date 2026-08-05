//! 滑块的持久状态。
//!
//! 与 gpui-component 一样采用「`Entity<SliderState>`（跨帧持久） + `Slider`
//! 一次性元素（每帧配置）」双层架构。本文件是状态那一半：它持有值、刻度、
//! 布局边界，并负责把「指针位置」换算成「值」以及向外发事件。
//!
//! 支持单值（`SliderValue::Single`，进度条/音量）与区间（`SliderValue::Range`，
//! 双 thumb，如时间轴选段）。

use crate::geometry::{position_to_value, quantize, value_to_percentage, Scale};
use crate::slider::SliderEvent;
use crate::slider_value::SliderValue;
use gpui::{
    Axis, Bounds, Context, EventEmitter, Focusable, FocusHandle, Pixels, Point, Render, Window,
    div, prelude::*,
};
use std::ops::Range;

/// 滑块的持久状态，由 `Entity<SliderState>` 持有。
pub struct SliderState {
    /// 值域下界。
    pub(crate) min: f32,
    /// 值域上界。
    pub(crate) max: f32,
    /// 步进；<=0 表示不取整。
    pub(crate) step: f32,
    /// 当前值（单值或区间）。
    pub(crate) value: SliderValue,
    /// 刻度模式（线性/对数）。
    scale: Scale,
    /// 轴方向（水平/垂直）。
    axis: Axis,
    /// fill 是否反向（从 max 端往 min 端填充，视觉用）。
    pub(crate) reverse: bool,
    /// 渲染后的布局边界，由 prepaint 回写。没有它没法从像素算值。
    pub(crate) bounds: Bounds<Pixels>,
    /// 当前值对应的百分比(0..1)缓存，渲染 fill/thumb 用。Single 时 start==end。
    pub(crate) percentage: Range<f32>,
    /// 是否正在拖动（用于只在真拖动后才发 Release，也供外部查 is_dragging）。
    pub(crate) dragging: bool,
    /// 按下时的初始值，拖动中按 Esc 取消回退用。
    pub(crate) start_value: SliderValue,
    /// 是否 hover（视觉态）。
    pub(crate) hovered: bool,
    /// 是否禁用（禁用则不响应交互）。
    pub(crate) disabled: bool,
}

impl SliderState {
    /// 新建，默认 [0,100]、step 1、线性、水平、单值 0。
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: SliderValue::Single(0.0),
            scale: Scale::Linear,
            axis: Axis::Horizontal,
            reverse: false,
            bounds: Bounds::default(),
            percentage: 0.0..0.0,
            dragging: false,
            start_value: SliderValue::Single(0.0),
            hovered: false,
            disabled: false,
        }
    }

    // ----- 消费式 builder（构造期配置） -----

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self.refresh();
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self.refresh();
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self.refresh();
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// 初始值（单值或区间）。
    pub fn default_value(mut self, value: impl Into<SliderValue>) -> Self {
        self.value = value.into().clamp(self.min, self.max);
        self.refresh();
        self
    }

    /// fill 反向（从 max 端往 min 端填充）。
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    // ----- 运行时读写 -----

    /// 当前值（单值或区间）。
    pub fn value(&self) -> SliderValue {
        self.value
    }

    pub fn min_value(&self) -> f32 {
        self.min
    }

    pub fn max_value(&self) -> f32 {
        self.max
    }

    pub fn step_value(&self) -> f32 {
        self.step
    }

    /// 当前百分比区间（Single 时 start==end），渲染 fill/thumb 用。
    pub fn percentage(&self) -> Range<f32> {
        self.percentage.clone()
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_reverse(&self) -> bool {
        self.reverse
    }

    /// 是否正在拖动。aa-player 拖动 seek 时用它触发 MuteAudio 等副作用。
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn get_axis(&self) -> Axis {
        self.axis
    }

    /// 外部驱动写值：夹紧 + step 取整 + 刷新 + 通知，**不发事件**。
    /// 事件由具体交互入口（点击/拖动/键盘）决定发不发 `Change`/`Release`。
    /// 单值传 `f32`，区间传 `(f32, f32)` 或 `Range<f32>`。
    pub fn set_value(&mut self, value: impl Into<SliderValue>, cx: &mut Context<Self>) {
        self.value = quantize_value(value.into(), self.min, self.max, self.step);
        self.refresh();
        cx.notify();
    }

    /// 由 prepaint 回写布局边界。
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }

    /// 回写布局边界但不触发重绘（prepaint 阶段每帧调用，notify 会死循环）。
    pub(crate) fn set_bounds_no_notify(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }

    /// 重算百分比缓存（值在 `min..max` 中的归一化位置）。
    fn refresh(&mut self) {
        let start = value_to_percentage(self.value.start(), self.scale, self.min, self.max);
        let end = value_to_percentage(self.value.end(), self.scale, self.min, self.max);
        self.percentage = start..end;
    }

    /// 把指针窗口坐标换算成值并写入，发 `Change`。click 与 drag 共用。
    ///
    /// `is_start`：Range 时选择移动起点 thumb（true）还是终点 thumb（false）；
    /// Single 时忽略（移动唯一值）。各端 clamp：起点不越过终点，终点不越过起点。
    ///
    /// 与 gpui-component `update_value_by_position` 同构（`slider.rs:358`）。
    /// 注意：会把 `dragging` 置 true（gpui-component 同款语义），松手后
    /// `end_drag` 才能发 `Release`。返回是否真的变化了值（避免无意义事件）。
    pub fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
        is_start: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.disabled {
            return false;
        }
        self.dragging = true;
        let new = position_to_value(
            self.axis,
            self.scale,
            position,
            &self.bounds,
            self.min,
            self.max,
            self.step,
        );
        // Range 时按端点 clamp；Single 时直接替换。
        let next = if self.value.is_range() {
            self.value.with_thumb(is_start, new)
        } else {
            SliderValue::Single(new)
        };
        let changed = value_differs(self.value, next);
        if changed {
            self.value = next;
            self.refresh();
            cx.notify();
            cx.emit(SliderEvent::Change(self.value));
        }
        changed
    }

    /// 按下：记录起点值（Esc 取消用），置 dragging，跳到指针位置并发 Change。
    /// `is_start` 语义同 `update_value_by_position`。
    pub fn begin_drag(&mut self, position: Point<Pixels>, is_start: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.start_value = self.value;
        self.dragging = true;
        self.update_value_by_position(position, is_start, cx);
    }

    /// 松手：若真在拖动，发一次 `Release`，清 dragging。drag 与 click 都走这。
    pub fn end_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging {
            self.dragging = false;
            cx.emit(SliderEvent::Release(self.value));
        }
    }

    /// 键盘/无障碍改值：写值 + 发 Change + 发 Release（键盘视为一次提交）。
    /// `is_start` 语义同 `update_value_by_position`。
    pub fn nudge(&mut self, delta: f32, is_start: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let next = if self.value.is_range() {
            let cur = if is_start {
                self.value.start()
            } else {
                self.value.end()
            };
            self.value
                .with_thumb(is_start, quantize(cur + delta, self.min, self.max, self.step))
        } else {
            SliderValue::Single(quantize(
                self.value.end() + delta,
                self.min,
                self.max,
                self.step,
            ))
        };
        if value_differs(self.value, next) {
            self.value = next;
            self.refresh();
            cx.notify();
            cx.emit(SliderEvent::Change(self.value));
        }
        cx.emit(SliderEvent::Release(self.value));
    }

    /// 键盘跳转（Home/End）。
    pub fn jump(&mut self, to: f32, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.value = quantize_value(SliderValue::Single(to), self.min, self.max, self.step);
        self.refresh();
        cx.notify();
        cx.emit(SliderEvent::Change(self.value));
        cx.emit(SliderEvent::Release(self.value));
    }

    /// 拖动中取消：回退到按下时的值，发 Release，结束拖动。
    pub fn cancel_drag(&mut self, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.value = self.start_value;
        self.refresh();
        self.dragging = false;
        cx.notify();
        cx.emit(SliderEvent::Release(self.value));
    }

    /// 无障碍/键盘 Increment：终点值 +step 并夹紧（gpui-component `slider.rs:620`）。
    pub fn increment(&mut self, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let new = quantize(
            self.value.end() + self.step,
            self.min,
            self.max,
            self.step,
        );
        self.set_value(new, cx);
        cx.emit(SliderEvent::Change(self.value));
        cx.emit(SliderEvent::Release(self.value));
    }

    /// 无障碍/键盘 Decrement：终点值 -step 并夹紧。
    pub fn decrement(&mut self, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let new = quantize(
            self.value.end() - self.step,
            self.min,
            self.max,
            self.step,
        );
        self.set_value(new, cx);
        cx.emit(SliderEvent::Change(self.value));
        cx.emit(SliderEvent::Release(self.value));
    }

    pub fn set_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.notify();
        }
    }
}

/// 对整个 SliderValue 做 step 取整 + 夹紧，并保证 Range 的 a<=b。
fn quantize_value(v: SliderValue, min: f32, max: f32, step: f32) -> SliderValue {
    match v {
        SliderValue::Single(x) => SliderValue::Single(quantize(x, min, max, step)),
        SliderValue::Range(a, b) => SliderValue::Range(
            quantize(a, min, max, step).min(quantize(b, min, max, step)),
            quantize(a, min, max, step).max(quantize(b, min, max, step)),
        ),
    }
}

/// 两个 SliderValue 是否不同（比较端点）。
fn value_differs(a: SliderValue, b: SliderValue) -> bool {
    (a.start() - b.start()).abs() > f32::EPSILON || (a.end() - b.end()).abs() > f32::EPSILON
}

impl Default for SliderState {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter<SliderEvent> for SliderState {}

impl Focusable for SliderState {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        // 本 rev 的 FocusHandle 没有公开构造器，改用 App::focus_handle()
        // 返回与当前 entity 关联的句柄（gpui 按 entity 跟踪，多次调用一致）。
        cx.focus_handle()
    }
}

impl Render for SliderState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // SliderState 本身不渲染视觉（由 `Slider` 元素负责）。这里返回空，
        // 仅作为拖拽标记的载体。
        div()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{Entity, TestAppContext, point, px, size};

    /// 一块水平 track bounds：x 从 100 到 300（宽 200），y 0..10。
    fn h_bounds() -> Bounds<Pixels> {
        Bounds::new(point(px(100.0), px(0.0)), size(px(200.0), px(10.0)))
    }

    /// 新建一个默认水平线性 [0,100] step=1 的 slider。
    fn new_slider(cx: &mut TestAppContext) -> Entity<SliderState> {
        cx.new(|_| SliderState::new())
    }

    #[gpui::test]
    fn update_value_by_position_maps_pixels_to_value(cx: &mut TestAppContext) {
        let slider = new_slider(cx);
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(0.0));

        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(200.0), px(5.0)), false, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(50.0)
        );

        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(300.0), px(5.0)), false, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(100.0)
        );

        // 超出右边界 → clamp 到 100。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(999.0), px(5.0)), false, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(100.0)
        );
    }

    #[gpui::test]
    fn update_value_by_position_sets_dragging(cx: &mut TestAppContext) {
        let slider = new_slider(cx);
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(150.0), px(5.0)), false, cx);
        });
        assert!(slider.read_with(cx, |s, _| s.is_dragging()));

        slider.update(cx, |s, cx| s.begin_drag(point(px(150.0), px(5.0)), false, cx));
        assert!(slider.read_with(cx, |s, _| s.is_dragging()));
        slider.update(cx, |s, cx| s.end_drag(cx));
        assert!(!slider.read_with(cx, |s, _| s.is_dragging()));
    }

    #[gpui::test]
    fn range_move_start_and_end_clamp_each_other(cx: &mut TestAppContext) {
        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value((30.0, 70.0))
        });
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Range(30.0, 70.0)
        );

        // 把起点拖到 x=250（值 75）：起点 clamp 到不超过终点 70。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(250.0), px(5.0)), true, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Range(70.0, 70.0)
        );

        // 把终点拖到 x=120（值 10）：终点 clamp 到不小于起点 70，起点不动。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(120.0), px(5.0)), false, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Range(70.0, 70.0)
        );
    }

    #[gpui::test]
    fn nudge_respects_step_and_clamps(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(10.0));
        slider.update(cx, |s, cx| s.nudge(4.0, false, cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(0.0));
        slider.update(cx, |s, cx| s.nudge(16.0, false, cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(20.0)
        );
        slider.update(cx, |s, cx| s.nudge(1000.0, false, cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(100.0)
        );
    }

    #[gpui::test]
    fn cancel_drag_reverts_to_start_value(cx: &mut TestAppContext) {
        let slider = new_slider(cx);
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        slider.update(cx, |s, cx| s.set_value(20.0, cx));
        slider.update(cx, |s, cx| s.begin_drag(point(px(200.0), px(5.0)), false, cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(50.0)
        );
        slider.update(cx, |s, cx| s.cancel_drag(cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(20.0)
        );
        assert!(!slider.read_with(cx, |s, _| s.is_dragging()));
    }

    #[gpui::test]
    fn set_value_quantizes_and_clamps(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(95.0).step(10.0));
        slider.update(cx, |s, cx| s.set_value(99.0, cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(95.0)
        );
        slider.update(cx, |s, cx| s.set_value(-5.0, cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(0.0));
        slider.update(cx, |s, cx| s.set_value(37.0, cx));
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(40.0)
        );
    }

    #[gpui::test]
    fn disabled_slider_ignores_interaction(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().disabled(true));
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        slider.update(cx, |s, cx| {
            let changed = s.update_value_by_position(point(px(200.0), px(5.0)), false, cx);
            assert!(!changed);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(0.0));
        assert!(!slider.read_with(cx, |s, _| s.is_dragging()));
    }

    #[gpui::test]
    fn vertical_slider_flips_y(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().axis(Axis::Vertical));
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(10.0), px(200.0)));
        slider.update(cx, |s, _| s.set_bounds(bounds));
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(5.0), px(100.0)), false, cx);
        });
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(50.0)
        );
    }

    #[gpui::test]
    fn increment_decrement_clamp_to_bounds(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(10.0).step(1.0));
        slider.update(cx, |s, cx| s.decrement(cx)); // 0 - 1 → clamp 0
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(0.0));
        slider.update(cx, |s, cx| s.increment(cx)); // 0 + 1 → 1
        assert_eq!(slider.read_with(cx, |s, _| s.value()), SliderValue::Single(1.0));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx));
        slider.update(cx, |s, cx| s.increment(cx)); // 9→10
        slider.update(cx, |s, cx| s.increment(cx)); // 10+1 → clamp 10
        assert_eq!(
            slider.read_with(cx, |s, _| s.value()),
            SliderValue::Single(10.0)
        );
    }

    #[gpui::test]
    fn reverse_sets_percentage_direction(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(100.0).reverse(true));
        slider.update(cx, |s, cx| s.set_value(80.0, cx));
        // 值 80，百分比仍是 0.8（reverse 只影响 fill 视觉方向，不改百分比语义）。
        let pct = slider.read_with(cx, |s, _| s.percentage());
        assert!((pct.end - 0.8).abs() < 1e-6);
        assert!(slider.read_with(cx, |s, _| s.is_reverse()));
    }
}
