//! 临时探针：ButtonLike + 自定义 child + size/None + Transparent + tooltip。
use aa_gpui_kit_ui::prelude::*;
use aa_gpui_kit_ui::{ButtonLike, ButtonSize, ButtonStyle, Tooltip};

#[allow(dead_code)]
fn probe(name: gpui::SharedString, color: gpui::Hsla, cx: &mut gpui::App) {
    let _ = ButtonLike::new("swatch")
        .child(
            div()
                .size_8()
                .bg(color)
                .border_1()
                .border_color(cx.theme().colors().border)
                .overflow_hidden(),
        )
        .size(ButtonSize::None)
        .style(ButtonStyle::Transparent)
        .tooltip(move |window, cx| {
            let name = name.clone();
            Tooltip::with_meta(name, None, format!("{:?}", color), cx)
        });
}
