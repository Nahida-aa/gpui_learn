//! # 10_input_button —— ui-gpui 的 Input 与 Button 演示
//!
//! 验证清单（手动过一遍）：
//! - 输入、退格、删除、左右移动、Shift 选区、Cmd+A/C/V/X；
//! - 中文 IME 组字（应有下划线）与上屏；
//! - Enter 触发 [`InputEvent::Submit`]；
//! - 两个按钮的点击 / hover / 禁用态；
//! - 程序化 `set_value` 不应触发 `Change` 事件。
//!
//! 运行：`cargo run -p input_button_10`

use gpui::{
    App, Bounds, Context, Entity, Focusable, Render, SharedString, Subscription, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use ui_gpui::{Button, InputEvent, InputState, bind_input_keys};

struct InputDemo {
    input: Entity<InputState>,
    last_change: SharedString,
    last_submit: SharedString,
    /// 订阅必须随 entity 存活，否则回调会被提前释放。
    _subscriptions: Vec<Subscription>,
}

impl InputDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(cx)
                .placeholder("随便输入点什么（试试中文输入法）…")
                .default_value("")
        });

        let subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| match event {
            InputEvent::Change(value) => {
                this.last_change = value.clone();
                cx.notify();
            }
            InputEvent::Submit(value) => {
                this.last_submit = value.clone();
                cx.notify();
            }
        });

        let demo = Self {
            input,
            last_change: "（尚无）".into(),
            last_submit: "（尚无）".into(),
            _subscriptions: vec![subscription],
        };

        // 打开就把焦点放进输入框，省得先点一下。
        // 注意走 Focusable 的 `focus_handle(cx)`：`Entity` 上没有 `focus`。
        demo.input.focus_handle(cx).focus(window, cx);
        demo
    }

    fn clear_input(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.clear(cx));
        self.last_change = "（已清空，注意不应产生 Change 事件）".into();
        cx.notify();
    }
}

impl Render for InputDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.input.read(cx).value();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .bg(rgb(0x11111b))
            .text_color(rgb(0xcdd6f4))
            .text_size(px(14.))
            .child(div().text_size(px(18.)).child("10_input_button"))
            .child(self.input.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("submit")
                            .label("提交（Enter 也行）")
                            .primary()
                            .on_click({
                                let input = self.input.clone();
                                move |_, _, cx| {
                                    let v = input.read(cx).value();
                                    eprintln!("[submit] {v}");
                                }
                            }),
                    )
                    .child(
                        Button::new("clear")
                            .label("清空")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear_input(window, cx)
                            })),
                    )
                    .child(Button::new("disabled").label("禁用示例").danger().disabled(true)),
            )
            .child(div().child(format!("当前值：{value}")))
            .child(div().child(format!("最后一次 Change：{}", self.last_change)))
            .child(div().child(format!("最后一次 Submit：{}", self.last_submit)))
    }
}

fn main() {
    application().run(|cx: &mut App| {
        // 输入框的按键绑定，必须在开窗口前装上。
        bind_input_keys(cx);

        let bounds = Bounds::centered(None, size(px(560.0), px(320.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| InputDemo::new(window, cx)),
            )
            .expect("打开窗口失败");

        window
            .update(cx, |_, _, cx| cx.activate(true))
            .expect("更新窗口失败");
    });
}
