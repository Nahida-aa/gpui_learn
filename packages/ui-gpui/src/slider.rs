//! 滑块的可视元素与交互绑定。
//!
//! 采用 gpui-component 的「`Entity<SliderState>`（持久状态） + `Slider`（一次性
//! 元素）」双层架构。本文件是元素那一半：负责三层渲染（track / fill / thumb）
//! 与所有交互事件（鼠标按下、拖动、松开、hover、键盘）。值的换算与事件发射
//! 都在 `SliderState` 里，元素只调用它。

use crate::slider_state::SliderState;
use gpui::{
    Axis, Background, Bounds, DragMoveEvent, Entity, EntityId, Focusable, IntoElement,
    MouseButton, MouseDownEvent, Pixels, RenderOnce, Window, div, prelude::*, px, relative, rgb,
};

/// 拖动时挂到 gpui 全局 `cx.active_drag` 上的零视觉标记，携带「哪个 slider」。
///
/// 与 gpui-component `slider.rs:12-28` 同构：拖动事件按 TypeId 匹配，
/// 同页面多个 slider 会互相收到事件，回调里必须校验 EntityId。
#[derive(Clone)]
pub struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// 滑块对外发出的事件。
///
/// - `Change(v)`：值变化过程中连续发出（拖动/点击/键盘都发）。
/// - `Release(v)`：一次交互结束时发一次（松手 / 键盘提交 / Esc 取消）。
/// 外部用 `cx.subscribe(&slider, …)` 订阅，读 `state.read(cx).value()`。
#[derive(Clone, Debug)]
pub enum SliderEvent {
    Change(f32),
    Release(f32),
}

/// 滑块元素（一次性）。每帧由 `Slider::new(&state)` 重新构建。
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
}

impl Slider {
    /// 从一个 `Entity<SliderState>` 构建元素。
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let percentage = state.percentage();
        let axis = state.get_axis();
        let horizontal = matches!(axis, Axis::Horizontal);
        let hovered = state.is_hovered();
        let focused = state.is_focused();
        let disabled = state.is_disabled();
        let thumb_size = if hovered || focused { px(18.0) } else { px(16.0) };

        // 颜色：尽量不依赖主题 token（本仓库没有 gpui-component 的主题系统）。
        // 轨道色要比常见深色主背景亮一档，否则和背景糊成一片。
        let track_bg: Background = if disabled {
            rgb(0x3a_3a_3a).into()
        } else {
            rgb(0x55_55_55).into()
        };
        let fill_bg: Background = rgb(0x4c_8b_f5).into();
        let thumb_bg: Background = if hovered || focused {
            rgb(0x9e_c4_ff).into()
        } else {
            rgb(0xff_ff_ff).into()
        };

        let thumb_pct = percentage * 100.0; // 用于 relative() 定位
        let slider_state = self.state.clone();

        // thumb（手柄）。拖动能力挂在 thumb 上：因为 thumb 的 on_mouse_down 会
        // stop_propagation，若把 on_drag 挂在父级，thumb 的按下不会冒泡，drag 永远
        // 启动不了。所以 on_drag + on_drag_move 必须和 on_mouse_down 同在 thumb 上，
        // 与 gpui-component slider.rs:508-535 同构。
        let thumb = div()
            .absolute()
            .id(("thumb", entity_id))
            .when(horizontal, |th| th.left(relative(thumb_pct / 100.0)).top(px(0.0)))
            .when(!horizontal, |th| th.bottom(relative(thumb_pct / 100.0)).left(px(0.0)))
            .size(thumb_size)
            .rounded_full()
            .bg(thumb_bg)
            // thumb 上按下：阻止冒泡到外层（避免外层 on_mouse_down 重复跳值），
            // 同时本元素启动自己的 drag。
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_drag(
                DragSlider(entity_id),
                |drag, _, _, cx| cx.new(|_| drag.clone()),
            )
            .on_drag_move({
                let slider_state = slider_state.clone();
                move |e: &DragMoveEvent<DragSlider>, _window, cx| {
                    if let DragSlider(id) = e.drag(cx)
                        && *id == entity_id
                    {
                        slider_state.update(cx, |s, cx| {
                            s.update_value_by_position(e.event.position, cx);
                        });
                    }
                }
            });

        // 轨道 = 全区域点击/拖动热区，同时也是 on_children_prepainted 的参考 bounds。
        // 内部放一根「细条 bar」作为视觉轨道（thin bar），fill/thumb 挂在 bar 上，
        // 而不是把整个热区涂成轨道色（否则滑轨和主背景糊成一片）。
        let track = div()
            .id("track")
            .relative()
            .when(horizontal, |t| t.h_full().w_full().flex().items_center())
            .when(!horizontal, |t| t.w_full().h_full().flex().flex_col().justify_center())
            // 细条 bar：水平 → 高 6px 铺满宽；垂直 → 宽 6px 铺满高。
            .child(
                div()
                    .relative()
                    .when(horizontal, |b| b.w_full().h(px(6.0)).flex().items_center())
                    .when(!horizontal, |b| b.h_full().w(px(6.0)).flex().flex_col().justify_center())
                    .bg(track_bg)
                    .rounded_full()
                    // fill（已填充部分）
                    .child(
                        div()
                            .absolute()
                            .when(horizontal, |f| {
                                f.left(px(0.0)).top(px(0.0)).bottom(px(0.0))
                                    .w(relative(thumb_pct / 100.0))
                            })
                            .when(!horizontal, |f| {
                                f.left(px(0.0)).right(px(0.0)).bottom(px(0.0))
                                    .h(relative(thumb_pct / 100.0))
                            })
                            .bg(fill_bg)
                            .rounded_full(),
                    )
                    // thumb（手柄）。拖动能力挂在 thumb 上：因为 thumb 的 on_mouse_down
                    // 会 stop_propagation，若把 on_drag 挂在父级，thumb 的按下不会冒泡，
                    // drag 永远启动不了（这正是之前"拖不动"的根因）。所以 on_drag +
                    // on_drag_move 必须和 on_mouse_down 同在 thumb 这一个元素上。
                    .child(thumb),
            );
        // 外层容器：track 作为第一个直接子元素，便于 on_children_prepainted 取 bounds。
        // 注意调用顺序：on_children_prepainted 只定义在 Div 上（本 rev 没有 on_prepaint），
        // 而 .id() 会把它变成 Stateful<Div>，之后就只能用 on_mouse_down 等 Stateful 方法。
        // 所以「捕获 track bounds」必须在 .id() 之前、还在 Div 上时调用。
        div()
            .flex()
            // 撑满调用方给定的盒子：水平滑块需要宽度、垂直滑块需要高度。
            .size_full()
            .cursor(if horizontal {
                gpui::CursorStyle::ResizeLeftRight
            } else {
                gpui::CursorStyle::ResizeUpDown
            })
            // 捕获 track 的布局 bounds，供像素→值换算（替代新版 gpui 的 on_prepaint）。
            // children.first() 就是唯一直接子元素 track 的 bounds。
            .on_children_prepainted({
                let slider_state = slider_state.clone();
                move |children: Vec<Bounds<Pixels>>, _window, cx| {
                    if let Some(b) = children.first() {
                        slider_state.update(cx, |s, _cx| s.set_bounds_no_notify(*b));
                    }
                }
            })
            .id(("slider", entity_id))
            .on_mouse_down(MouseButton::Left, {
                let slider_state = slider_state.clone();
                move |e: &MouseDownEvent, window, cx| {
                    // 点击 track：聚焦 + 开始拖动 + 跳到点击位置。
                    let fh = slider_state.read(cx).focus_handle(cx);
                    window.focus(&fh, cx);
                    slider_state.update(cx, |s, cx| {
                        s.set_focused(true, cx);
                        s.begin_drag(e.position, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let slider_state = slider_state.clone();
                move |_, _, cx| {
                    slider_state.update(cx, |s, cx| s.end_drag(cx));
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let slider_state = slider_state.clone();
                move |_, _, cx| {
                    slider_state.update(cx, |s, cx| s.end_drag(cx));
                }
            })
            .on_hover({
                let slider_state = slider_state.clone();
                move |hovered, _window, cx| {
                    slider_state.update(cx, |s, cx| s.set_hovered(*hovered, cx));
                }
            })
            .on_key_down({
                let slider_state = slider_state.clone();
                move |e: &gpui::KeyDownEvent, _window, cx| {
                    let step = slider_state.read(cx).step_value();
                    // 分级：Shift ×10，Ctrl/Alt ÷10（gpui-component 没有，我们加上）。
                    let mult = if e.keystroke.modifiers.shift { 10.0 } else { 1.0 }
                        / if e.keystroke.modifiers.control || e.keystroke.modifiers.alt {
                            10.0
                        } else {
                            1.0
                        };
                    let delta = step * mult;
                    match e.keystroke.key.as_str() {
                        "left" | "down" => {
                            slider_state.update(cx, |s, cx| s.nudge(-delta, cx))
                        }
                        "right" | "up" => {
                            slider_state.update(cx, |s, cx| s.nudge(delta, cx))
                        }
                        "home" => slider_state.update(cx, |s, cx| s.jump(s.min_value(), cx)),
                        "end" => slider_state.update(cx, |s, cx| s.jump(s.max_value(), cx)),
                        "escape" => {
                            let dragging = slider_state.read(cx).dragging;
                            if dragging {
                                slider_state.update(cx, |s, cx| s.cancel_drag(cx));
                            }
                        }
                        _ => {}
                    }
                }
            })
            // 轨道整条可拖：点 track 任意处按住即可拖动跟手（不只 thumb）。
            // on_drag_move 会在「任一」元素上以 Capture 阶段派发（div.rs:342），
            // 所以外层与 thumb 都注册无妨——update_value_by_position 幂等，不会重复发 Change。
            .on_drag(
                DragSlider(entity_id),
                |drag, _, _, cx| cx.new(|_| drag.clone()),
            )
            .on_drag_move({
                let slider_state = slider_state.clone();
                move |e: &DragMoveEvent<DragSlider>, _window, cx| {
                    if let DragSlider(id) = e.drag(cx)
                        && *id == entity_id
                    {
                        slider_state.update(cx, |s, cx| {
                            s.update_value_by_position(e.event.position, cx);
                        });
                    }
                }
            })
            .child(track)
    }
}
