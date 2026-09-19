//! prelude 集成测试：从**外部调用方**视角验证一行
//! `use aa_gpui_kit_ui::prelude::*;` 即可获得常用 API。
//!
//! 缺符号会直接编译失败 —— 这份文件同时充当 API 面的调用方示例。

use aa_gpui_kit_ui::prelude::*;

#[test]
fn layout_helpers_from_prelude() {
    let _row = h_flex();
    let _col = v_flex();
}

#[test]
fn button_apis_from_prelude() {
    // ButtonLike：组件方法 + trait 方法（Clickable/Disableable）混合使用
    let _button = ButtonLike::new("save")
        .label("保存")
        .icon(IconName::Check)
        .on_click(|_, _, _| {})
        .disabled(false);

    // IconButton + trait（Toggleable）
    let _icon = IconButton::new("menu", IconName::Menu).toggle_state(true);

    // SplitButton
    let _split = SplitButton::new(
        IconButton::new("primary", IconName::ChevronDown),
        div().into_any_element(),
    )
    .style(SplitButtonStyle::Outlined);
}

#[test]
fn interactive_elements_from_prelude() {
    // div 上的交互方法（gpui::prelude）+ 我们的 StyledExt 布局方法
    let _bar = div()
        .id("bar")
        .h_flex()
        .gap_2()
        .cursor_pointer()
        .on_click(|_: &ClickEvent, _, _| {});
}
