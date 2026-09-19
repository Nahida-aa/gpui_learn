//! context_menu/menu：上下文菜单实体（每个 popup 一个 `ContextMenu` view）。
//!
//! 对齐 zed `crates/ui/src/components/context_menu.rs` 的精简版：
//!
//! - [`ContextMenu`] 是一个 `ManagedView`（`Focusable + EventEmitter<DismissEvent> + Render`），
//!   由 [`RightClickMenu`](super::right_click_menu) 这类宿主元素在需要时创建并挂载。
//! - 用 [`ContextMenu::build`] 的 builder 形式装配内容，再作为 `Entity` 交给宿主。
//! - render：一个 `occlude()` 的浮层容器（`on_mouse_down_out` 点击外部即
//!   `DismissEvent`），条目按 `ContextMenuItem` 渲染对应行（勾选列 + label +
//!   hover 背景 + 点击回调）；Esc 也会触发 `DismissEvent`。
//!
//! 宿主收到 `DismissEvent` 后卸载该菜单并归还焦点，从而实现"点外部/点条目
//! Esc 都会关菜单"的 zed 行为。

use gpui::{
    App, Context, DismissEvent, Div, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    KeyDownEvent, MouseDownEvent, SharedString, Window, div, prelude::*, px,
};

use crate::base::icon::{Icon, IconName};
use aa_gpui_kit_theme::ActiveTheme;

use super::entry::ContextMenuItem;

/// 勾选列（固定宽度占位，checked 时由调用方塞一个 ✓ 图标）。
fn check_column() -> Div {
    div().size(px(14.0)).flex().items_center().justify_center()
}

/// 上下文菜单实体。
pub struct ContextMenu {
    /// 内容项（Entry/Separator/Label）。
    items: Vec<ContextMenuItem>,
    /// 供宿主要求键盘焦点（Esc 打开、菜单关闭后归还焦点）。
    focus_handle: FocusHandle,
    /// 条目最小宽度（px），菜单据此撑开。
    min_width: f32,
}

impl ContextMenu {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            items: Vec::new(),
            focus_handle: cx.focus_handle(),
            min_width: 180.0,
        }
    }

    /// 创建并装配一个菜单（对齐 zed `ContextMenu::build`）：
    ///
    /// ```ignore
    /// ContextMenu::build(cx, |menu, _| {
    ///     menu.item(ContextMenuEntry::new("Dock Left").checked(true))
    /// })
    /// ```
    pub fn build(
        cx: &mut App,
        f: impl FnOnce(ContextMenu, &mut Context<Self>) -> ContextMenu,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let menu = Self::new(cx);
            f(menu, cx)
        })
    }

    /// 条目最小宽度（默认 180px）。
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// 追加一项（`Into<ContextMenuItem>`：`ContextMenuEntry` / `&str`）。
    pub fn item(mut self, item: impl Into<ContextMenuItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// 分隔线。
    pub fn separator(self) -> Self {
        self.item(ContextMenuItem::Separator)
    }

    /// 纯文本标签（muted 色）。
    pub fn label(self, text: impl Into<SharedString>) -> Self {
        self.item(ContextMenuItem::Label(text.into()))
    }

    /// 点击某条目：执行回调并关闭（`DismissEvent`）。
    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        match &self.items[index] {
            ContextMenuItem::Entry(entry) => {
                entry.activate(window, cx);
                cx.emit(DismissEvent);
            }
            _ => {}
        }
    }

    /// 关闭菜单（点击条目外部 / Esc）。
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for ContextMenu {}

impl Render for ContextMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors().clone();
        let min_width = px(self.min_width);
        let font_size = px(12.0);

        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| match item {
                ContextMenuItem::Entry(entry) => {
                    let ix = ix;
                    let label = entry.label.clone();
                    let checked = entry.checked;
                    let disabled = entry.disabled;
                    let text_color = if disabled {
                        colors.text_disabled
                    } else {
                        colors.text
                    };
                    let check = if checked {
                        Icon::new(IconName::Check)
                            .size(font_size)
                            .color(colors.text_accent)
                            .into_any_element()
                    } else {
                        Empty.into_any_element()
                    };

                    div()
                        .id(("context-menu-item", ix))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .px_1p5()
                        .py_1()
                        .min_w(min_width)
                        .text_size(font_size)
                        .cursor_pointer()
                        .child(check_column().child(check))
                        .child(div().text_color(text_color).child(label))
                        .when(!disabled, |el| {
                            el.hover(|style| style.bg(colors.ghost_element_hover))
                        })
                        .on_click(cx.listener(move |this: &mut ContextMenu, _, window, cx| {
                            this.activate(ix, window, cx);
                        }))
                        .into_any_element()
                }
                ContextMenuItem::Label(label) => div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px_1p5()
                    .py_1()
                    .min_w(min_width)
                    .text_size(font_size)
                    .child(check_column())
                    .child(div().text_color(colors.text_muted).child(label.clone()))
                    .into_any_element(),
                ContextMenuItem::Separator => div()
                    .h(px(1.0))
                    .my_1()
                    .mx_1()
                    .bg(colors.border_variant)
                    .into_any_element(),
            })
            .collect::<Vec<_>>();

        div()
            .id("context-menu")
            .occlude()
            .on_mouse_down_out(
                cx.listener(|this: &mut ContextMenu, _: &MouseDownEvent, _, cx| {
                    this.dismiss(cx);
                }),
            )
            .on_key_down(
                cx.listener(|this: &mut ContextMenu, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        this.dismiss(cx);
                    }
                }),
            )
            .p_0p5()
            .flex()
            .flex_col()
            .bg(colors.elevated_surface_background)
            .rounded_md()
            .border_1()
            .border_color(colors.border)
            .children(items)
    }
}
