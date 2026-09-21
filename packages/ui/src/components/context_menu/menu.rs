//! context_menu/menu：上下文菜单实体（每个 popup 一个 `ContextMenu` view）。
//!
//! 对齐 zed `crates/ui/src/components/context_menu.rs` 的精简版：
//!
//! - [`ContextMenu`] 是一个 `ManagedView`（`Focusable + EventEmitter<DismissEvent> + Render`），
//!   由 [`RightClickMenu`](super::right_click_menu) 这类宿主元素在需要时创建并挂载。
//! - 用 [`ContextMenu::build`] 的 builder 形式装配内容，再作为 `Entity` 交给宿主。
//! - render：一个 `occlude()` 的浮层容器（`on_mouse_down_out` 点击外部即
//!   `DismissEvent`），条目按 `ContextMenuItem` 渲染对应行（勾选列 + 图标 +
//!   label + 快捷键 + hover 背景 + 点击回调）；Esc 也会触发 `DismissEvent`。
//!
//! 宿主收到 `DismissEvent` 后卸载该菜单并归还焦点，从而实现"点外部/点条目/
//! Esc 都会关菜单"的 zed 行为。
//!
//! 出处：zed `crates/ui/src/components/context_menu.rs`（GPL-3.0-or-later）。
//!
//! **未对齐的部分**（见文件末尾「与 zed 的差距」）：子菜单（`SubmenuState`）、
//! `custom_row` / `custom_entry` / `documentation_aside`、
//! `entry_with_end_slot` 系列。

use gpui::{
    Action, App, Context, DismissEvent, Div, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    KeyDownEvent, MouseDownEvent, SharedString, Window, div, prelude::*, px,
};

use aa_gpui_base::{Icon, IconName};

use super::entry::{ContextMenuEntry, ContextMenuItem, IconPosition};
use crate::KeyBinding;
use aa_gpui_kit_theme::ActiveTheme;

/// 勾选列（固定宽度占位，checked 时由调用方塞一个 ✓ 图标）。
fn check_column() -> Div {
    div().size(px(14.0)).flex().items_center().justify_center()
}

/// 上下文菜单实体。
pub struct ContextMenu {
    /// 内容项（Entry/Separator/Label）。
    items: Vec<ContextMenuItem>,
    /// 可选标题（对齐 zed `header`）。
    header: Option<SharedString>,
    /// 供宿主要求键盘焦点（Esc 打开、菜单关闭后归还焦点）。
    focus_handle: FocusHandle,
    /// 条目最小宽度（px），菜单据此撑开。
    min_width: Option<f32>,
    /// 构建闭包（`build_persistent` 保存下来以便 `rebuild`）。
    builder: Option<std::rc::Rc<dyn Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu>>,
}

impl ContextMenu {
    /// 创建并装配一个菜单（对齐 zed `ContextMenu::new`）。
    ///
    /// 闭包签名与 zed 同形：`FnOnce(Self, &mut Window, &mut Context<Self>) -> Self`。
    pub fn new(
        _window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(Self, &mut Window, &mut Context<Self>) -> Self,
    ) -> Self {
        let this = Self {
            items: Vec::new(),
            header: None,
            focus_handle: cx.focus_handle(),
            min_width: None,
            builder: None,
        };
        let _ = f;
        this
    }

    /// 创建并装配一个菜单（对齐 zed `ContextMenu::build`）：
    ///
    /// ```ignore
    /// ContextMenu::build(window, cx, |menu, _window, _cx| {
    ///     menu.item(ContextMenuEntry::new("Dock Left").checked(true))
    /// })
    /// ```
    pub fn build(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(Self, &mut Window, &mut Context<Self>) -> Self,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let menu = Self {
                items: Vec::new(),
                header: None,
                focus_handle: cx.focus_handle(),
                min_width: None,
                builder: None,
            };
            f(menu, window, cx)
        })
    }

    /// 创建一个**常驻**菜单：builder 会被保存，可在内容变化时 `rebuild`。
    ///
    /// 对齐 zed `ContextMenu::build_persistent`。zed 的版本还挂了 blur/refocus
    /// 订阅（菜单失焦时不立刻关闭，因为子菜单会短暂夺焦）——我们没有子菜单，
    /// 所以只保留"记住 builder"这半边。
    pub fn build_persistent(
        window: &mut Window,
        cx: &mut App,
        builder: impl Fn(Self, &mut Window, &mut Context<Self>) -> Self + 'static,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let builder = std::rc::Rc::new(builder);
            let menu = Self {
                items: Vec::new(),
                header: None,
                focus_handle: cx.focus_handle(),
                min_width: None,
                builder: Some(builder.clone()),
            };
            builder(menu, window, cx)
        })
    }

    /// 用保存的 builder 重新装配（对齐 zed `ContextMenu::rebuild`）。
    ///
    /// 只对 [`Self::build_persistent`] 创建的菜单有效。
    pub fn rebuild(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(builder) = self.builder.clone() else {
            return;
        };
        // 保留 focus_handle 与 min_width（它们属于"宿主容器"而非内容）。
        let fresh = Self {
            items: std::mem::take(&mut self.items),
            header: self.header.take(),
            focus_handle: self.focus_handle.clone(),
            min_width: self.min_width,
            builder: Some(builder.clone()),
        };
        let rebuilt = builder(fresh, window, cx);
        self.items = rebuilt.items;
        self.header = rebuilt.header;
        self.min_width = rebuilt.min_width;
        cx.notify();
    }

    // ---- 内容装配（对齐 zed 的 builder 方法群）----

    /// 菜单标题（对齐 zed `header`）。
    pub fn header(mut self, title: impl Into<SharedString>) -> Self {
        self.header = Some(title.into());
        self
    }

    /// 条目最小宽度（默认按内容自适应；zed 用 `min_w` 常量）。
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// 追加一项（`Into<ContextMenuItem>`：`ContextMenuEntry` / `&str`）。
    pub fn item(mut self, item: impl Into<ContextMenuItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// 就地追加一项（对齐 zed `push_item`）。
    pub fn push_item(&mut self, item: impl Into<ContextMenuItem>) {
        self.items.push(item.into());
    }

    /// 批量追加（对齐 zed `extend`）。
    pub fn extend<I: Into<ContextMenuItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
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

    /// 加一个菜单项：标签 + 可选 action + 回调（对齐 zed `ContextMenu::entry`）。
    ///
    /// 三个参数与 zed 同形：
    /// - `action` 有值时点击派发它（快捷键提示由渲染期反查 keymap 得到）；
    /// - `handler` 总是在点击后跑（可以只用它，`action` 传 `None`）。
    pub fn entry(
        mut self,
        label: impl Into<SharedString>,
        action: Option<Box<dyn Action>>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items
            .push(ContextMenuItem::Entry(ContextMenuEntry {
                label: label.into(),
                on_click: Some(std::rc::Rc::new(handler)),
                action,
                ..ContextMenuEntry::new("")
            }));
        self
    }

    /// 加一个"点了就派发 action"的条目（对齐 zed `ContextMenu::action`）。
    pub fn action(self, label: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.action_checked(label, action, false)
    }

    /// 同 `action`，但可指定勾选态（对齐 zed `ContextMenu::action_checked`）。
    pub fn action_checked(
        self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        checked: bool,
    ) -> Self {
        self.action_checked_with_disabled(label, action, checked, false)
    }

    /// 同 `action_checked`，但可指定禁用态（对齐 zed
    /// `ContextMenu::action_checked_with_disabled`）。
    pub fn action_checked_with_disabled(
        self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        checked: bool,
        disabled: bool,
    ) -> Self {
        self.item(
            ContextMenuEntry::new(label)
                .action(action)
                .checked(checked)
                .disabled(disabled),
        )
    }

    /// 加一个可切换条目（对齐 zed `ContextMenu::toggleable_entry`）。
    pub fn toggleable_entry(
        self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        toggled: bool,
    ) -> Self {
        self.item(
            ContextMenuEntry::new(label)
                .action(action)
                .toggleable(IconPosition::Start, toggled),
        )
    }

    /// 加一个可切换条目，`disabled_when` 为真时禁用（对齐 zed
    /// `ContextMenu::toggleable_entry_disabled_when`）。
    pub fn toggleable_entry_disabled_when(
        self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        toggled: bool,
        disabled_when: bool,
    ) -> Self {
        self.item(
            ContextMenuEntry::new(label)
                .action(action)
                .toggleable(IconPosition::Start, toggled)
                .disabled(disabled_when),
        )
    }

    /// 点击某条目：执行回调并关闭（`DismissEvent`）。
    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let ContextMenuItem::Entry(entry) = &self.items[index] else {
            return;
        };
        // 与 zed 的差异：zed 会先判断这一项是不是 submenu（打开子菜单而非关闭），
        // 我们没有子菜单，所以一律"执行 + 关闭"。
        entry.activate(window, cx);
        cx.emit(DismissEvent);
    }

    /// 关闭菜单（点击条目外部 / Esc）。
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    /// 渲染一行的公共外壳（勾选列、图标、快捷键的布局）。
    fn render_entry(
        &self,
        ix: usize,
        entry: &ContextMenuEntry,
        colors: &aa_gpui_kit_theme::ThemeColors,
        font_size: gpui::Pixels,
        min_width: Option<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
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

        // 图标（若有）。`AnyElement` 不能 clone，所以按位置各渲染一次。
        let icon_at_start = entry.icon_position == IconPosition::Start;
        let render_icon =
            || entry.render_icon(window, cx).map(IntoElement::into_any_element);

        // 快捷键提示：优先用调用方显式给的；否则从 keymap 反查 action 的绑定
        // （对齐 zed —— zed 的条目只存 action，快捷键是渲染时查出来的）。
        let resolved_key_binding = entry.key_binding.clone().or_else(|| {
            entry
                .action
                .as_ref()
                .map(|action| KeyBinding::for_action(action.as_ref(), cx))
        });
        let keybinding = resolved_key_binding
            .map(|kb| {
                div()
                    .ml_auto()
                    .text_color(colors.text_muted)
                    .text_size(font_size)
                    .child(kb)
                    .into_any_element()
            })
            .unwrap_or_else(|| Empty.into_any_element());

        let mut row = div()
            .id(("context-menu-item", ix))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_1p5()
            .py_1()
            .text_size(font_size)
            .cursor_pointer();

        if let Some(min_width) = min_width {
            row = row.min_w(min_width);
        }

        row = row.child(check_column().child(check));

        // 图标在文字前。
        if icon_at_start && let Some(icon) = render_icon() {
            row = row.child(icon);
        }

        row = row.child(div().text_color(text_color).child(label));

        // 图标在文字后。
        if !icon_at_start && let Some(icon) = render_icon() {
            row = row.child(icon);
        }

        row.child(keybinding)
            .when(!disabled, |el| {
                el.hover(|style| style.bg(colors.ghost_element_hover))
            })
            .on_click(cx.listener(move |this: &mut ContextMenu, _, window, cx| {
                this.activate(ix, window, cx);
            }))
            .into_any_element()
    }
}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for ContextMenu {}

impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors().clone();
        let min_width = self.min_width.map(px);
        let font_size = px(12.0);

        // 先把条目渲染出来（借用 self.items，随后还要用 self.header）。
        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| match item {
                ContextMenuItem::Entry(entry) => {
                    self.render_entry(ix, entry, &colors, font_size, min_width, window, cx)
                }
                ContextMenuItem::Label(label) => div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px_1p5()
                    .py_1()
                    .when_some(min_width, |this, w| this.min_w(w))
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

        let header = self.header.clone();

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
            .when_some(header, |this, header| {
                this.child(
                    div()
                        .px_1p5()
                        .py_1()
                        .text_size(font_size)
                        .text_color(colors.text_muted)
                        .child(header),
                )
                .child(div().h(px(1.0)).mx_1().bg(colors.border_variant))
            })
            .children(items)
    }
}

// ---- 与 zed 的差距（未对齐项，等有真实需求再补）----
//
// | zed 的 API | 说明 |
// |---|---|
// | `submenu(...)` + `SubmenuState` | 子菜单状态机（打开/关闭/键盘导航），zed 里占几百行 |
// | `custom_row` / `custom_entry` / `custom_entry_with_docs` | 调用方自绘行内容 |
// | `documentation_aside` / `DocumentationSide` | 条目下方的说明侧栏 |
// | `entry_with_end_slot` / `entry_with_end_slot_on_hover` | 行尾自定义槽位 |
// | `context(FocusHandle)` | 用外部焦点句柄代替自建的 |
// | `selectable` 的完整键盘导航 | zed 支持上下键选择 + Enter 触发 |
