#![cfg_attr(target_family = "wasm", no_main)]

use gpui::{
    App, Axis, Bounds, Context, Entity, FocusHandle, Focusable, MouseButton, MouseUpEvent, Render,
    SharedString, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use ui_gpui::{Scale, Slider, SliderEvent, SliderState};

/// 一行：标签 + 滑块 + 当前值文本。按滑块轴方向给合适尺寸：
/// 水平滑块 → 高条横向铺满；垂直滑块 → 细长的竖条。
fn slider_row(
    label: SharedString,
    slider: &Entity<SliderState>,
    cx: &mut Context<SliderDemo>,
) -> impl IntoElement {
    let value = slider.read(cx).value();
    let min = slider.read(cx).min_value();
    let max = slider.read(cx).max_value();
    let vertical = matches!(slider.read(cx).get_axis(), Axis::Vertical);
    let dragging = slider.read(cx).is_dragging();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .h(if vertical { px(180.0) } else { px(48.0) })
        .w_full()
        .child(div().w(px(96.0)).text_size(px(14.0)).child(label.clone()))
        // 滑块盒子：垂直 → 窄而高；水平 → 高而宽。Slider 会 size_full 填满它。
        .child(
            div()
                .when(vertical, |d| d.w(px(32.0)).h(px(160.0)))
                .when(!vertical, |d| d.flex_1().h(px(32.0)))
                .child(Slider::new(slider)),
        )
        .child(
            div()
                .w(px(170.0))
                .text_size(px(13.0))
                .child(format!("v={value} [{min:.1},{max:.1}] drag={dragging}")),
        )
}

/// 同 `slider_row`，但给 Slider 元素应用自定义颜色（演示颜色覆盖）。
fn slider_row_colored(
    label: SharedString,
    slider: &Entity<SliderState>,
    cx: &mut Context<SliderDemo>,
) -> impl IntoElement {
    let value = slider.read(cx).value();
    let min = slider.read(cx).min_value();
    let max = slider.read(cx).max_value();
    let dragging = slider.read(cx).is_dragging();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .h(px(48.0))
        .w_full()
        .child(div().w(px(96.0)).text_size(px(14.0)).child(label.clone()))
        .child(
            div()
                .flex_1()
                .h(px(32.0))
                .child(
                    Slider::new(slider)
                        .track_color(rgb(0x2a_2a_2a))
                        .fill_color(rgb(0xff_a0_40))
                        .thumb_color(rgb(0xff_80_20))
                        .thumb_size(px(24.0))
                        .track_size(px(10.0)),
                ),
        )
        .child(
            div()
                .w(px(170.0))
                .text_size(px(13.0))
                .child(format!("v={value} [{min:.1},{max:.1}] drag={dragging}")),
        )
}

struct SliderDemo {
    /// 通用值滑块（水平、线性、step=1）。
    basic: Entity<SliderState>,
    /// 只读进度条：disabled，由外部按钮 set_value 推进。
    progress: Entity<SliderState>,
    /// 音量：对数刻度（0.1..1.0），模拟听感。
    volume: Entity<SliderState>,
    /// 垂直滑块。
    vertical: Entity<SliderState>,
    /// 步进滑块（step=10）。
    stepped: Entity<SliderState>,
    /// 区间双滑块（Range）。
    range: Entity<SliderState>,
    /// 反向填充滑块（reverse）。
    reversed: Entity<SliderState>,
    /// 自定义颜色滑块（演示 track/fill/thumb 颜色覆盖）。
    colored: Entity<SliderState>,
    /// 事件日志（最新在前，最多 6 条）。
    log: Vec<String>,
    focus_handle: FocusHandle,
}

impl SliderDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let basic = cx.new(|_| SliderState::new());
        let progress = cx.new(|_| SliderState::new().disabled(true));
        let volume = cx.new(|_| SliderState::new().min(0.1).max(1.0).step(0.01).scale(Scale::Log));
        let vertical = cx.new(|_| SliderState::new().axis(Axis::Vertical));
        let stepped = cx.new(|_| SliderState::new().min(0.0).max(100.0).step(10.0));
        let range = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value((20.0, 80.0))
        });
        let reversed = cx.new(|_| SliderState::new().min(0.0).max(100.0).reverse(true));
        let colored = cx.new(|_| SliderState::new().min(0.0).max(100.0));

        let demo = Self {
            basic,
            progress,
            volume,
            vertical,
            stepped,
            range,
            reversed,
            colored,
            log: vec![],
            focus_handle: cx.focus_handle(),
        };

        // 订阅滑块事件，写入日志。SliderEvent::{Change,Release} 携带 SliderValue。
        for slider in [
            &demo.basic,
            &demo.progress,
            &demo.volume,
            &demo.vertical,
            &demo.stepped,
            &demo.range,
            &demo.reversed,
            &demo.colored,
        ] {
            cx.subscribe(slider, |view, _slider, event, cx| {
                let msg = match event {
                    SliderEvent::Change(v) => format!("Change({v})"),
                    SliderEvent::Release(v) => format!("Release({v})"),
                };
                view.log.insert(0, msg);
                view.log.truncate(6);
                cx.notify();
            })
            .detach();
        }

        demo
    }

    /// 演示「只读进度条由外部驱动」：每次 +10，走到 100 回到 0。
    fn on_advance_progress(&mut self, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let cur = self.progress.read(cx).value().end();
        let next = (cur + 10.0) % 101.0;
        self.progress.update(cx, |s, cx| s.set_value(next, cx));
    }
}

impl Focusable for SliderDemo {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SliderDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(rgb(0x222222))
            .text_color(rgb(0xeeeeee))
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .child(div().text_size(px(20.0)).child("ui-gpui Slider 演示"))
            .child(slider_row("通用".into(), &self.basic, cx))
            .child(slider_row("音量(对数)".into(), &self.volume, cx))
            .child(slider_row("step=10".into(), &self.stepped, cx))
            .child(slider_row("区间[20,80]".into(), &self.range, cx))
            .child(slider_row("反向填充".into(), &self.reversed, cx))
            .child(slider_row_colored("自定义颜色".into(), &self.colored, cx))
            .child(slider_row("垂直".into(), &self.vertical, cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .h(px(48.0))
                    .w_full()
                    .child(
                        div()
                            .w(px(96.0))
                            .text_size(px(14.0))
                            .child("只读进度")
                    )
                    .child(div().flex_1().h(px(32.0)).child(Slider::new(&self.progress)))
                    .child(
                        div()
                            .border_1()
                            .border_color(rgb(0x666666))
                            .px_2()
                            .py_1()
                            .child("+10")
                            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_advance_progress)),
                    ),
            )
            .child(div().text_size(px(12.0)).text_color(rgb(0x88aa88)).child("事件日志："))
            .children(
                self.log
                    .iter()
                    .map(|l| div().text_size(px(12.0)).text_color(rgb(0x88aa88)).child(l.clone())),
            )
    }
}

fn run_example() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(640.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(SliderDemo::new),
        )
        .unwrap();
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
