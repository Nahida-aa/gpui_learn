//! 滑块的持久状态。
//!
//! 与 gpui-component 一样采用「`Entity<SliderState>`（跨帧持久） + `Slider`
//! 一次性元素（每帧配置）」双层架构。本文件是状态那一半：它持有值、刻度、
//! 布局边界，并负责把「指针位置」换算成「值」以及向外发事件。
//!
//! 我们只做单值滑块（进度条/音量条都是单值）；区间滑块不在范围内。

use crate::geometry::{position_to_value, quantize, value_to_percentage, Scale};
use crate::slider::SliderEvent;
use gpui::{
    Axis, Bounds, Context, EventEmitter, Focusable, FocusHandle, Pixels, Point, Render, Window,
    div, prelude::*,
};

/// 滑块的持久状态，由 `Entity<SliderState>` 持有。
pub struct SliderState {
    /// 值域下界。
    pub(crate) min: f32,
    /// 值域上界。
    pub(crate) max: f32,
    /// 步进；<=0 表示不取整。
    pub(crate) step: f32,
    /// 当前值。
    pub(crate) value: f32,
    /// 刻度模式（线性/对数）。
    scale: Scale,
    /// 轴方向（水平/垂直）。
    axis: Axis,
    /// 渲染后的布局边界，由 `on_prepaint` 回写。没有它没法从像素算值。
    pub(crate) bounds: Bounds<Pixels>,
    /// 当前值对应的百分比(0..1)缓存，渲染 fill/thumb 用，所有写值后更新。
    pub(crate) percentage: f32,
    /// 是否正在拖动（用于只在真拖动后才发 Release）。
    pub(crate) dragging: bool,
    /// 按下时的初始值，拖动中按 Esc 取消回退用。
    pub(crate) start_value: f32,
    /// 是否 hover（视觉态）。
    pub(crate) hovered: bool,
    /// 是否聚焦（视觉态 + 键盘交互前提）。
    pub(crate) focused: bool,
    /// 是否禁用（禁用则不响应交互）。
    pub(crate) disabled: bool,
}

impl SliderState {
    /// 新建，默认 [0,100]、step 1、线性、水平。
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: 0.0,
            scale: Scale::Linear,
            axis: Axis::Horizontal,
            bounds: Bounds::default(),
            percentage: 0.0,
            dragging: false,
            start_value: 0.0,
            hovered: false,
            focused: false,
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    // ----- 运行时读写 -----

    /// 当前值。
    pub fn value(&self) -> f32 {
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

    pub fn percentage(&self) -> f32 {
        self.percentage
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn get_axis(&self) -> Axis {
        self.axis
    }

    /// 写值（夹紧 + step 取整），刷新百分比缓存并通知重绘。不发事件——
    /// 事件由具体的交互入口（点击/拖动/键盘）决定发不发 `Change`/`Release`。
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        self.value = quantize(value, self.min, self.max, self.step);
        self.refresh();
        cx.notify();
    }

    /// 由 `on_prepaint` 回写布局边界。
    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }

    /// 回写布局边界但不触发重绘（prepaint 阶段每帧调用，notify 会死循环）。
    pub(crate) fn set_bounds_no_notify(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }

    /// 重算百分比缓存（值在 `min..max` 中的归一化位置）。
    fn refresh(&mut self) {
        self.percentage = value_to_percentage(self.value, self.scale, self.min, self.max);
    }

    /// 把指针窗口坐标换算成值并写入，发 `Change`。click 与 drag 共用。
    ///
    /// 与 gpui-component `update_value_by_position` 同构（`slider.rs:358`）：
    /// 取布局 bounds → 像素百分比 → 按刻度转值 → step 取整 + 夹紧，
    /// 然后 `emit Change`。返回是否真的变化了值（用于避免无意义事件）。
    ///
    /// 注意：这里会把 `dragging` 置 true（gpui-component 同款语义）。这样 thumb
    /// 直接拖动（不经过 `begin_drag`）时，`dragging` 也会置位，松手后 `end_drag`
    /// 才能发 `Release`。`begin_drag` 已置 true，重复置无副作用。
    pub fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
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
        let changed = (new - self.value).abs() > f32::EPSILON;
        if changed {
            self.value = new;
            self.refresh();
            cx.notify();
            cx.emit(SliderEvent::Change(self.value));
        }
        changed
    }

    /// 按下：记录起点值（Esc 取消用），置 dragging，跳到指针位置并发 Change。
    pub fn begin_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.start_value = self.value;
        self.dragging = true;
        self.update_value_by_position(position, cx);
    }

    /// 松手：若真在拖动，发一次 `Release`，清 dragging。drag 与 click 都走这。
    pub fn end_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging {
            self.dragging = false;
            cx.emit(SliderEvent::Release(self.value));
        }
    }

    /// 键盘/无障碍改值：写值 + 发 Change + 发 Release（键盘视为一次提交）。
    pub fn nudge(&mut self, delta: f32, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let new = quantize(self.value + delta, self.min, self.max, self.step);
        if (new - self.value).abs() > f32::EPSILON {
            self.value = new;
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
        self.value = quantize(to, self.min, self.max, self.step);
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

    pub fn set_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.notify();
        }
    }

    pub fn set_focused(&mut self, focused: bool, cx: &mut Context<Self>) {
        if self.focused != focused {
            self.focused = focused;
            cx.notify();
        }
    }
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
        // 仅作为 `DragSlider` 拖拽标记的载体（gpui-component `slider.rs:12-28`）。
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

        // 左端 x=100 → 0；右端 x=300 → 100；中点 x=200 → 50。
        let v = slider.read_with(cx, |s, _| s.value());
        assert_eq!(v, 0.0);

        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(200.0), px(5.0)), cx);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 50.0);

        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(300.0), px(5.0)), cx);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 100.0);

        // 超出右边界 → clamp 到 100。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(999.0), px(5.0)), cx);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 100.0);
    }

    #[gpui::test]
    fn update_value_by_position_sets_dragging(cx: &mut TestAppContext) {
        let slider = new_slider(cx);
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        // update_value_by_position 应置 dragging=true（gpui-component 同款语义），
        // 这样 thumb 直接拖动后松手能发 Release。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(150.0), px(5.0)), cx);
        });
        assert!(slider.read_with(cx, |s, _| s.dragging));

        // 点击（begin_drag → end_drag）后 dragging 复位。
        slider.update(cx, |s, cx| s.begin_drag(point(px(150.0), px(5.0)), cx));
        assert!(slider.read_with(cx, |s, _| s.dragging));
        slider.update(cx, |s, cx| s.end_drag(cx));
        assert!(!slider.read_with(cx, |s, _| s.dragging));
    }

    #[gpui::test]
    fn nudge_respects_step_and_clamps(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(10.0));
        // 0 + 4 → 取整到 0（step=10），值不变。
        slider.update(cx, |s, cx| s.nudge(4.0, cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 0.0);
        // 0 + 16 → 16 → 取整到 20。
        slider.update(cx, |s, cx| s.nudge(16.0, cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 20.0);
        // 远超上限 → clamp 到 max。
        slider.update(cx, |s, cx| s.nudge(1000.0, cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 100.0);
    }

    #[gpui::test]
    fn cancel_drag_reverts_to_start_value(cx: &mut TestAppContext) {
        let slider = new_slider(cx);
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        slider.update(cx, |s, cx| s.set_value(20.0, cx));
        // 按下并拖到 80。
        slider.update(cx, |s, cx| s.begin_drag(point(px(200.0), px(5.0)), cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 50.0);
        // Esc 取消 → 回到按下时的 20。
        slider.update(cx, |s, cx| s.cancel_drag(cx));
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 20.0);
        assert!(!slider.read_with(cx, |s, _| s.dragging));
    }

    #[gpui::test]
    fn set_value_quantizes_and_clamps(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().min(0.0).max(95.0).step(10.0));
        slider.update(cx, |s, cx| s.set_value(99.0, cx)); // 99 → 取整 100 → clamp 95
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 95.0);
        slider.update(cx, |s, cx| s.set_value(-5.0, cx)); // 越下界 → clamp 0
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 0.0);
        slider.update(cx, |s, cx| s.set_value(37.0, cx)); // 37 → 取整 40
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 40.0);
    }

    #[gpui::test]
    fn disabled_slider_ignores_interaction(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().disabled(true));
        slider.update(cx, |s, _| s.set_bounds(h_bounds()));
        slider.update(cx, |s, cx| {
            let changed = s.update_value_by_position(point(px(200.0), px(5.0)), cx);
            assert!(!changed);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 0.0);
        assert!(!slider.read_with(cx, |s, _| s.dragging));
    }

    #[gpui::test]
    fn vertical_slider_flips_y(cx: &mut TestAppContext) {
        let slider = cx.new(|_| SliderState::new().axis(Axis::Vertical));
        // 垂直：bounds y 从 0 到 200，底(y=200)→1.0，顶(y=0)→0.0。
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(10.0), px(200.0)));
        slider.update(cx, |s, _| s.set_bounds(bounds));
        // y=100（中点）→ 0.5 → 值 50。
        slider.update(cx, |s, cx| {
            s.update_value_by_position(point(px(5.0), px(100.0)), cx);
        });
        assert_eq!(slider.read_with(cx, |s, _| s.value()), 50.0);
    }
}

