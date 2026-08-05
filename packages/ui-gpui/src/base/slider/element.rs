//! 滑块的可视元素与交互绑定。
//!
//! 采用 gpui-component 的「`Entity<SliderState>`（持久状态） + `Slider`（一次性
//! 元素）」双层架构。本文件是元素那一半：负责三层渲染（track / fill / thumb）
//! 与所有交互事件（鼠标按下、拖动、松开、hover、键盘、无障碍）。值的换算与
//! 事件发射都在 `SliderState` 里，元素只调用它。

use crate::base::slider::slider_state::SliderState;
use crate::base::slider::slider_value::SliderValue;
use gpui::{
    AccessibleAction, Axis, Background, Bounds, DragMoveEvent, Entity, EntityId, Focusable,
    IntoElement, MouseButton, MouseDownEvent, Orientation, Pixels, RenderOnce, Rgba, Role, Window,
    div, prelude::*, px, relative, rgb, rgba,
};

/// 拖动外层轨道时挂到 `cx.active_drag` 的标记，携带「哪个 slider」。
///
/// 用于单值滑块的"点轨道任意处拖动跟手"。与 gpui-component `slider.rs:22-28` 同构。
#[derive(Clone)]
pub struct DragSlider(EntityId);

impl Render for DragSlider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// 拖动某个 thumb 时挂到 `cx.active_drag` 的标记，携带「哪个 slider + 是否起点端」。
///
/// Range 双滑块需要区分在拖起点 thumb 还是终点 thumb。
#[derive(Clone)]
pub struct DragThumb(EntityId, bool);

impl Render for DragThumb {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// 滑块对外发出的事件。
///
/// - `Change(v)`：值变化过程中连续发出（拖动/点击/键盘都发）。
/// - `Release(v)`：一次交互结束时发一次（松手 / 键盘提交 / Esc 取消）。
///
/// 外部用 `cx.subscribe(&slider, …)` 订阅，读事件里的 [`SliderValue`]。
#[derive(Clone, Debug)]
pub enum SliderEvent {
    Change(SliderValue),
    Release(SliderValue),
}

/// 滑块元素（一次性）。每帧由 `Slider::new(&state)` 重新构建。
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    /// 轨道底色；None 用内置默认。
    track_color: Option<Rgba>,
    /// 填充色；None 用内置默认。
    fill_color: Option<Rgba>,
    /// thumb 圆点基础色；None 用内置默认。hover/拖动时会向白提亮。
    thumb_color: Option<Rgba>,
    /// thumb 圆点基础大小（普通态）；None 用内置默认 16px，hover/拖动时 +2px。
    thumb_size: Option<Pixels>,
}

impl Slider {
    /// 从一个 `Entity<SliderState>` 构建元素。
    ///
    /// 配置（min/max/step/scale/axis/reverse/disabled/默认值）都在
    /// `SliderState` 上设置；这里只引用它的状态，并可选覆盖颜色。
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            state: state.clone(),
            track_color: None,
            fill_color: None,
            thumb_color: None,
            thumb_size: None,
        }
    }

    /// 轨道底色（未填充部分的灰条）。
    pub fn track_color(mut self, color: impl Into<Rgba>) -> Self {
        self.track_color = Some(color.into());
        self
    }

    /// 填充色（已填充的蓝色条）。
    pub fn fill_color(mut self, color: impl Into<Rgba>) -> Self {
        self.fill_color = Some(color.into());
        self
    }

    /// thumb 圆点基础色；hover/拖动时自动向白提亮，disabled 时置灰。
    pub fn thumb_color(mut self, color: impl Into<Rgba>) -> Self {
        self.thumb_color = Some(color.into());
        self
    }

    /// thumb 圆点基础大小（普通态）。默认 16px，hover/拖动时自动 +2px。
    pub fn thumb_size(mut self, size: impl Into<Pixels>) -> Self {
        self.thumb_size = Some(size.into());
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let entity_id = self.state.entity_id();
        let state = self.state.read(cx);
        let percentage = state.percentage();
        let axis = state.get_axis();
        let horizontal = matches!(axis, Axis::Horizontal);
        let disabled = state.is_disabled();
        let is_range = state.value().is_range();

        // 颜色：支持调用方覆盖（track/fill/thumb），未设时用内置默认。
        // disabled 时一律置灰。
        let track_bg: Background = self
            .track_color
            .unwrap_or(if disabled { rgb(0x3a_3a_3a) } else { rgb(0x55_55_55) })
            .into();
        let fill_bg: Background = self
            .fill_color
            .unwrap_or(if disabled { rgb(0x55_55_55) } else { rgb(0x4c_8b_f5) })
            .into();
        // thumb 普通态颜色；hover/拖动时向白提亮 50%，disabled 置灰。
        let thumb_normal = self.thumb_color.unwrap_or(rgb(0xff_ff_ff));
        let thumb_highlight = thumb_normal.blend(rgba(0xff_ff_ff_80));
        let thumb_bg = |is_start: bool| -> Background {
            if disabled {
                rgb(0x88_88_88).into()
            } else if state.thumb_highlighted(is_start) {
                thumb_highlight.into()
            } else {
                thumb_normal.into()
            }
        };
        // thumb 大小：普通态用基础值（默认 16px，可定制），hover/拖动时 ×1.125
        // （16→18px 的比例），保持普通态到高亮态的相对放大。
        let thumb_base = self.thumb_size.unwrap_or(px(16.0));
        let thumb_highlight_size = thumb_base * 1.125;
        let thumb_size = |is_start: bool| -> Pixels {
            if state.thumb_highlighted(is_start) {
                thumb_highlight_size
            } else {
                thumb_base
            }
        };

        // fill 的起止百分比（0..1）。
        // Single：fill 从 min 端(0)到当前值，reverse 时从当前值到 max 端(1)。
        // Range：fill 夹在两个 thumb 之间（start..end）。
        let (bar_start, bar_end) = if is_range {
            (percentage.start, 1.0 - percentage.end)
        } else if state.is_reverse() {
            (percentage.end, 0.0)
        } else {
            (0.0, 1.0 - percentage.end)
        };

        let slider_state = self.state.clone();

        // 起点 thumb（Range 才有），终点 thumb（Single 也用）。
        let start_thumb = is_range.then(|| {
            render_thumb(
                &slider_state,
                entity_id,
                horizontal,
                thumb_size(true),
                thumb_bg(true),
                disabled,
                true,
                percentage.start,
            )
        });
        let end_thumb = render_thumb(
            &slider_state,
            entity_id,
            horizontal,
            thumb_size(false),
            thumb_bg(false),
            disabled,
            false,
            percentage.end,
        );

        // 轨道 = 全区域点击/拖动热区，也是 on_children_prepainted 的参考 bounds。
        // 内部放一根「细条 bar」作为视觉轨道，fill/thumb 挂在 bar 上。
        let mut track = div()
            .id("track")
            .relative()
            .when(horizontal, |t| t.h_full().w_full().flex().items_center())
            .when(!horizontal, |t| t.w_full().h_full().flex().flex_col().justify_center())
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
                                f.top(px(0.0)).bottom(px(0.0)).left(relative(bar_start))
                                    .right(relative(bar_end))
                            })
                            .when(!horizontal, |f| {
                                f.left(px(0.0)).right(px(0.0)).bottom(relative(bar_start))
                                    .top(relative(bar_end))
                            })
                            .bg(fill_bg)
                            .rounded_full(),
                    )
                    // thumb(s)：Range 两个，Single 一个。
                    .when_some(start_thumb, |b, t| b.child(t))
                    .child(end_thumb),
            );
        if !disabled {
            // 单值滑块：点轨道任意处按住即可拖动跟手（不只 thumb）。
            // on_drag_move 只在 active_drag 类型匹配时才派发（div.rs:344），
            // 所以 DragSlider（外层）与 DragThumb（thumb）互不干扰。
            track = track.on_drag(
                DragSlider(entity_id),
                |drag, _, _, cx| cx.new(|_| drag.clone()),
            );
        }

        // 外层容器：track 作为第一个直接子元素，便于 on_children_prepainted 取 bounds。
        // on_children_prepainted 只定义在 Div 上（本 rev 无 on_prepaint），必须在 .id()
        // 之前、还在 Div 上时调用（.id() 会把它变成 Stateful<Div>）。
        let mut outer = div()
            .flex()
            .size_full()
            .cursor(if horizontal {
                gpui::CursorStyle::ResizeLeftRight
            } else {
                gpui::CursorStyle::ResizeUpDown
            })
            // 捕获 track 的布局 bounds，供像素→值换算。
            .on_children_prepainted({
                let slider_state = slider_state.clone();
                move |children: Vec<Bounds<Pixels>>, _window, cx| {
                    if let Some(b) = children.first() {
                        slider_state.update(cx, |s, _cx| s.set_bounds_no_notify(*b));
                    }
                }
            })
            .id(("slider", entity_id))
            // 无障碍：暴露 role/aria + 键盘/读屏的增减动作。
            .role(Role::Slider)
            .aria_numeric_value(percentage.end as f64)
            .aria_min_numeric_value(state.min_value() as f64)
            .aria_max_numeric_value(state.max_value() as f64)
            .aria_orientation(if horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            })
            .on_a11y_action(AccessibleAction::Increment, {
                let st = slider_state.clone();
                move |_, _window, cx| st.update(cx, |s, cx| s.increment(cx))
            })
            .on_a11y_action(AccessibleAction::Decrement, {
                let st = slider_state.clone();
                move |_, _window, cx| st.update(cx, |s, cx| s.decrement(cx))
            });

        if !disabled {
            outer = outer
                .on_mouse_down(MouseButton::Left, {
                    let st = slider_state.clone();
                    move |e: &MouseDownEvent, window, cx| {
                        // 点击 track：聚焦 + 开始拖动 + 跳到点击位置。
                        // Range 时按点击位置 vs 范围中点决定移动起点还是终点 thumb。
                        let fh = st.read(cx).focus_handle(cx);
                        window.focus(&fh, cx);
                        st.update(cx, |s, cx| {
                            let is_start = s.value().is_range()
                                && pick_is_start(s, e.position);
                            s.begin_drag(e.position, is_start, cx);
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let st = slider_state.clone();
                    move |_, _, cx| st.update(cx, |s, cx| s.end_drag(cx))
                })
                .on_mouse_up_out(MouseButton::Left, {
                    let st = slider_state.clone();
                    move |_, _, cx| st.update(cx, |s, cx| s.end_drag(cx))
                })
                .on_drag_move({
                    let st = slider_state.clone();
                    move |e: &DragMoveEvent<DragSlider>, _window, cx| {
                        // 该 handler 仅在 active_drag 类型为 DragSlider 时才被派发，
                        // 故 e.drag(cx) 必然成功（div.rs:344 的类型守卫）。
                        // 先拷出 id 再 update，避免 cx 借用冲突。
                        let DragSlider(drag_id) = *e.drag(cx);
                        if drag_id == entity_id {
                            st.update(cx, |s, cx| {
                                // 用点击时 pick 出的 active_thumb 决定移动哪个端，
                                // 否则点左边拖动会错误地移动终点 thumb。
                                let is_start = s.active_thumb.unwrap_or(false);
                                s.update_value_by_position(e.event.position, is_start, cx);
                            });
                        }
                    }
                });
        }

        outer = outer
            .on_key_down({
                let st = slider_state.clone();
                move |e: &gpui::KeyDownEvent, _window, cx| {
                    let step = st.read(cx).step_value();
                    // 分级：Shift ×10，Ctrl/Alt ÷10（gpui-component 没有，我们加上）。
                    let mult = if e.keystroke.modifiers.shift { 10.0 } else { 1.0 }
                        / if e.keystroke.modifiers.control || e.keystroke.modifiers.alt {
                            10.0
                        } else {
                            1.0
                        };
                    let delta = step * mult;
                    match e.keystroke.key.as_str() {
                        "left" | "down" => st.update(cx, |s, cx| s.nudge(-delta, false, cx)),
                        "right" | "up" => st.update(cx, |s, cx| s.nudge(delta, false, cx)),
                        "home" => st.update(cx, |s, cx| s.jump(s.min_value(), cx)),
                        "end" => st.update(cx, |s, cx| s.jump(s.max_value(), cx)),
                        "escape" if st.read(cx).is_dragging() => {
                            st.update(cx, |s, cx| s.cancel_drag(cx));
                        }
                        _ => {}
                    }
                }
            })
            .child(track);
        outer
    }
}

/// 渲染一个 thumb（手柄）。`is_start` 决定它代表起点端还是终点端，并进入
/// `DragThumb` 标记，让拖动时知道在拖哪个端。
///
/// 拖动能力必须和 `on_mouse_down` 同在 thumb 上：thumb 的 on_mouse_down 会
/// `stop_propagation`，若把 on_drag 挂在父级，按下不会冒泡，drag 永远启动不了。
#[allow(clippy::too_many_arguments)]
fn render_thumb(
    slider_state: &Entity<SliderState>,
    entity_id: EntityId,
    horizontal: bool,
    thumb_size: Pixels,
    thumb_bg: Background,
    disabled: bool,
    is_start: bool,
    pct: f32,
) -> impl IntoElement {
    let mut th = div()
        .absolute()
        .id(format!("thumb-{entity_id}-{is_start}"))
        .when(horizontal, |t| {
            t.top(px(-5.0)).left(relative(pct)).ml(-px(8.0))
        })
        .when(!horizontal, |t| {
            t.bottom(relative(pct)).left(px(-5.0)).mb(-px(8.0))
        })
        .size(thumb_size)
        .rounded_full()
        .bg(thumb_bg)
        // 每个 thumb 独立记录悬停，Range 两个圆点不会同时变色。
        .on_hover({
            let st = slider_state.clone();
            move |hovered, _window, cx| {
                st.update(cx, |s, cx| s.set_thumb_hovered(is_start, *hovered, cx));
            }
        });
    if !disabled {
        // thumb 上按下：阻止冒泡到外层（避免外层 on_mouse_down 重复跳值），
        // 并立刻把该 thumb 置 active，避免「悬停→按下→拖动开始」的间隙闪回原色。
        let st = slider_state.clone();
        let press_st = slider_state.clone();
        th = th
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                press_st.update(cx, |s, cx| s.begin_thumb_press(is_start, cx));
                cx.stop_propagation();
            })
            .on_drag(
                DragThumb(entity_id, is_start),
                |drag, _, _, cx| cx.new(|_| drag.clone()),
            )
            .on_drag_move(move |e: &DragMoveEvent<DragThumb>, _window, cx| {
                // 该 handler 仅在 active_drag 类型为 DragThumb 时才被派发（div.rs:344）。
                // 先拷出 (id, is_start) 再 update，避免 cx 借用冲突。
                let DragThumb(drag_id, is_start) = *e.drag(cx);
                if drag_id == entity_id {
                    st.update(cx, |s, cx| {
                        s.update_value_by_position(e.event.position, is_start, cx);
                    });
                }
            });
    }
    th
}

/// Range 时，判断点击位置更靠近起点 thumb 还是终点 thumb（返回是否起点端）。
///
/// 用像素空间中点比较（与 gpui-component `slider.rs:669-679` 同构）。
fn pick_is_start(state: &SliderState, position: gpui::Point<Pixels>) -> bool {
    let axis = state.get_axis();
    let bounds = state.bounds;
    let pct_start = state.percentage().start;
    let pct_end = state.percentage().end;
    let total = match axis {
        Axis::Horizontal => bounds.size.width,
        Axis::Vertical => bounds.size.height,
    };
    if total == Pixels::ZERO {
        return false;
    }
    let pos = match axis {
        Axis::Horizontal => position.x - bounds.left(),
        Axis::Vertical => bounds.bottom() - position.y,
    };
    // 两个 thumb 的像素中点 = (pct_start+pct_end)/2 * total。
    let center = (pct_end + pct_start) / 2.0 * total.as_f32();
    pos.as_f32() < center
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{TestAppContext, point, px, size};

    /// 水平轨道 bounds：x 从 100 到 300（宽 200），y 0..10。默认 Range(20,80)。
    fn range_state(cx: &mut TestAppContext) -> Entity<SliderState> {
        let s = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value((20.0, 80.0))
        });
        s.update(cx, |s, _| {
            s.set_bounds(Bounds::new(point(px(100.0), px(0.0)), size(px(200.0), px(10.0))))
        });
        s
    }

    #[gpui::test]
    fn pick_is_start_left_of_midpoint(cx: &mut TestAppContext) {
        let s = range_state(cx);
        // 范围 (20,80)，中点 = 50。点击 x=150（值 25，偏左）→ 应选起点端。
        let is_start = s.read_with(cx, |st, _| pick_is_start(st, point(px(150.0), px(5.0))));
        assert!(is_start);
        // 点击 x=250（值 75，偏右）→ 应选终点端。
        let is_start = s.read_with(cx, |st, _| pick_is_start(st, point(px(250.0), px(5.0))));
        assert!(!is_start);
        // 点击 x=200（值 50，正好中点）→ 选起点端（pos < center 为 false 当相等时）。
        let is_start = s.read_with(cx, |st, _| pick_is_start(st, point(px(200.0), px(5.0))));
        assert!(!is_start);
    }
}
