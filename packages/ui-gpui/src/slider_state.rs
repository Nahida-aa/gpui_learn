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
    pub fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.disabled {
            return false;
        }
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
