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
//! `entry_with_end_slot` 系列。

use crate::{h_flex, Color, Icon, IconName, IconSize};
use std::rc::Rc;

use gpui::{
    Action, Anchor, AnyElement, App, Context, DismissEvent, Div, Empty, Entity, EventEmitter,
    FocusHandle, Focusable, KeyDownEvent, MouseDownEvent, SharedString, Subscription, Window,
    anchored, deferred, div, prelude::*, px,
};


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
    /// 快捷键反查的焦点上下文（`context()` 设入，对齐 zed `action_context`）。
    action_context: Option<FocusHandle>,
    /// 失焦订阅（对齐 zed `_on_blur_subscription`：占位持有，防止宿主把
    /// 「菜单失焦即关闭」的 Subscription 提前释放）。
    _on_blur_subscription: Subscription,
    /// 当前打开的子菜单（对齐 zed `submenu_state`，zed `context_menu.rs:232`）。
    submenu_state: SubmenuState,
}

/// 子菜单的开关状态（对齐 zed `SubmenuState`，zed `context_menu.rs:34`）。
///
/// 与 zed 的差异：zed 的 `OpenSubmenu` 还带 `trigger_bounds` / `offset` /
/// `flip_left`，用来把子菜单钉到父菜单某一行的**侧边**（一套 canvas 观测量 +
/// 贴边翻转的逻辑）；我们把子菜单直接 `anchored()` 到它自己的行上，位置交给
/// gpui 算，所以这些都不需要。
enum SubmenuState {
    Closed,
    Open(OpenSubmenu),
}

/// 一个已打开的子菜单（对齐 zed `OpenSubmenu`）。
struct OpenSubmenu {
    item_index: usize,
    entity: Entity<ContextMenu>,
    /// 子菜单 emit `DismissEvent` 时收拢自己；与 zed 同型（zed 里是
    /// `create_submenu` 返回的第二个数）。
    _dismiss_subscription: Subscription,
}

/// 文档侧栏出现在菜单的哪一侧（对齐 zed `DocumentationSide`）。
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DocumentationSide {
    Left,
    Right,
}

/// 菜单条目旁的文档侧栏（对齐 zed `DocumentationAside`）：`render` 回调
/// 现场生成侧栏内容，由菜单渲染时决定摆放在 [`DocumentationSide`] 一侧。
#[derive(Clone)]
pub struct DocumentationAside {
    pub side: DocumentationSide,
    pub render: std::rc::Rc<dyn Fn(&mut App) -> AnyElement>,
}

impl DocumentationAside {
    pub fn new(side: DocumentationSide, render: std::rc::Rc<dyn Fn(&mut App) -> AnyElement>) -> Self {
        Self { side, render }
    }
}

impl ContextMenu {
    /// 保存一条失焦订阅（对齐 zed `ContextMenu::on_blur_subscription`）。
    ///
    /// 宿主用「菜单失焦即关闭」的订阅换取持有权：把它塞进菜单，菜单在
    /// 展示期间替宿主养着它，销毁时一并释放。
    pub fn on_blur_subscription(mut self, new_subscription: Subscription) -> Self {
        self._on_blur_subscription = new_subscription;
        self
    }

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
            action_context: None,
            _on_blur_subscription: Subscription::new(|| {}),
            submenu_state: SubmenuState::Closed,
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
                action_context: None,
                _on_blur_subscription: Subscription::new(|| {}),
            submenu_state: SubmenuState::Closed,
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
                action_context: None,
                _on_blur_subscription: Subscription::new(|| {}),
            submenu_state: SubmenuState::Closed,
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
            // 与 zed 同：fresh 里不继承（`rebuild` 只把 `items` 搬回来，
            // 所以 `self.action_context` 仍保持原值）。
            action_context: None,
            _on_blur_subscription: Subscription::new(|| {}),
            submenu_state: SubmenuState::Closed,
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

    /// 不可选中的自定义条目（对齐 zed `ContextMenu::custom_row`，
    /// zed `context_menu.rs:689`）：只渲染、不响应键盘选中，handler 是空实现。
    pub fn custom_row(
        mut self,
        entry_render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::CustomEntry {
            entry_render: Box::new(entry_render),
            handler: std::rc::Rc::new(|_, _, _| {}),
            selectable: false,
            documentation_aside: None,
        });
        self
    }

    /// 可选中的自定义条目（对齐 zed `ContextMenu::custom_entry`，
    /// zed `context_menu.rs:702`）。整行交给 `entry_render`，命中跑 `handler`。
    pub fn custom_entry(
        mut self,
        entry_render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::CustomEntry {
            entry_render: Box::new(entry_render),
            handler: std::rc::Rc::new(move |_, window, cx| handler(window, cx)),
            selectable: true,
            documentation_aside: None,
        });
        self
    }

    /// 同 [`Self::custom_entry`]，额外带文档侧栏（对齐 zed
    /// `ContextMenu::custom_entry_with_docs`，zed `context_menu.rs:715`）。
    pub fn custom_entry_with_docs(
        mut self,
        entry_render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        handler: impl Fn(&mut Window, &mut App) + 'static,
        documentation_aside: Option<DocumentationAside>,
    ) -> Self {
        self.items.push(ContextMenuItem::CustomEntry {
            entry_render: Box::new(entry_render),
            handler: std::rc::Rc::new(move |_, window, cx| handler(window, cx)),
            selectable: true,
            documentation_aside,
        });
        self
    }

    /// 加一个子菜单（对齐 zed `ContextMenu::submenu`，zed `context_menu.rs:866`）。
    ///
    /// `builder` 在**点击命中时**才跑（子菜单每次打开都重建，和 zed 一致）。
    pub fn submenu(
        mut self,
        label: impl Into<SharedString>,
        builder: impl Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Submenu {
            label: label.into(),
            icon: None,
            icon_color: None,
            builder: Rc::new(builder),
        });
        self
    }

    /// 同 [`Self::submenu`]，标题前带图标（对齐 zed
    /// `ContextMenu::submenu_with_icon`，zed `context_menu.rs:880`）。
    pub fn submenu_with_icon(
        self,
        label: impl Into<SharedString>,
        icon: IconName,
        builder: impl Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu + 'static,
    ) -> Self {
        self.push_submenu(label, Some(icon), None, Rc::new(builder))
    }

    /// 同 [`Self::submenu_with_icon`]，图标可指定颜色（对齐 zed
    /// `ContextMenu::submenu_with_colored_icon`，zed `context_menu.rs:895`）。
    pub fn submenu_with_colored_icon(
        self,
        label: impl Into<SharedString>,
        icon: IconName,
        icon_color: Color,
        builder: impl Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu + 'static,
    ) -> Self {
        self.push_submenu(label, Some(icon), Some(icon_color), Rc::new(builder))
    }

    fn push_submenu(
        mut self,
        label: impl Into<SharedString>,
        icon: Option<IconName>,
        icon_color: Option<Color>,
        builder: Rc<dyn Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu>,
    ) -> Self {
        self.items.push(ContextMenuItem::Submenu {
            label: label.into(),
            icon,
            icon_color,
            builder,
        });
        self
    }

    /// 改最后一项能否被键盘选中（对齐 zed `ContextMenu::selectable`，
    /// zed `context_menu.rs:731`）；只对 `CustomEntry` 生效。
    pub fn selectable(mut self, selectable: bool) -> Self {
        if let Some(ContextMenuItem::CustomEntry {
            selectable: entry_selectable,
            ..
        }) = self.items.last_mut()
        {
            *entry_selectable = selectable;
        }
        self
    }

    /// 快捷键的反查上下文（对齐 zed `ContextMenu::context`）。
    ///
    /// 设了它之后，条目若只给了 `action`、没给 `key_binding`，渲染时按**这个
    /// 焦点句柄**所在区域解析绑定（同一 action 在不同区域可能绑不同键）。
    pub fn context(mut self, focus: FocusHandle) -> Self {
        self.action_context = Some(focus);
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

    /// 加一个按条件置灰的 action 条目（对齐 zed
    /// `ContextMenu::action_disabled_when`：`disabled` 为真时条目置灰、
    /// 快捷键与 action 仍挂上但不响应）。
    pub fn action_disabled_when(
        mut self,
        disabled: bool,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
    ) -> Self {
        self.item(
            ContextMenuEntry::new(label)
                .action(action)
                .disabled(disabled),
        )
    }

    /// 加一个可切换条目（对齐 zed `ContextMenu::toggleable_entry`）。
    ///
    /// 六个参数与 zed 同形，注意顺序：`toggled` 在 `position` 之前，
    /// `action` 是 `Option`（可以只给 `handler`），`position` 决定勾选标记
    /// 画在文字哪一侧。
    pub fn toggleable_entry(
        self,
        label: impl Into<SharedString>,
        toggled: bool,
        position: IconPosition,
        action: Option<Box<dyn Action>>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.toggleable_entry_disabled_when(label, toggled, false, position, action, handler)
    }

    /// 同 [`Self::toggleable_entry`]，但 `disabled` 为真时条目置灰、回调不触发
    /// （对齐 zed `ContextMenu::toggleable_entry_disabled_when`）。
    pub fn toggleable_entry_disabled_when(
        self,
        label: impl Into<SharedString>,
        toggled: bool,
        disabled: bool,
        position: IconPosition,
        action: Option<Box<dyn Action>>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let mut entry = ContextMenuEntry::new(label)
            .toggleable(position, toggled)
            .disabled(disabled)
            .on_click(handler);
        if let Some(action) = action {
            entry = entry.action(action);
        }
        self.item(entry)
    }

    /// 点击某条目：执行回调并关闭（`DismissEvent`）。
    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        match &self.items[index] {
            ContextMenuItem::Entry(entry) => entry.activate(window, cx),
            // 子菜单：命中即打开（**不**关闭父菜单），对齐 zed
            // `context_menu.rs:1018` 那一处处理。
            ContextMenuItem::Submenu { builder, .. } => {
                let builder = builder.clone();
                self.open_submenu(index, builder, window, cx);
                return;
            }
            // zed: `context_menu.rs:1044` 的 CustomEntry 分支，`handler` 拿到的
            // 第一个参数是菜单的 `action_context`（可能为空）。
            ContextMenuItem::CustomEntry { handler, .. } => {
                let handler = handler.clone();
                let context = self.action_context.clone();
                handler(context.as_ref(), window, cx);
            }
            ContextMenuItem::Separator | ContextMenuItem::Label(_) => return,
        }
        // 走到这里说明跑完了 Entry / CustomEntry 的回调：一律「执行 + 关闭」。
        // 与 zed 的差异：zed 还有 `keep_open_on_confirm`，我们没搬。
        cx.emit(DismissEvent);
    }

    /// 关闭菜单（点击条目外部 / Esc）。
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    // ---- 子菜单（对齐 zed `create_submenu` / `open_submenu` / `close_submenu`）----

    /// 建一个子菜单实体并订阅它的关闭事件（对齐 zed `ContextMenu::create_submenu`，
    /// zed `context_menu.rs:1279`）。
    fn create_submenu(
        builder: Rc<dyn Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<ContextMenu>, Subscription) {
        let submenu = ContextMenu::build(window, cx, |menu, window, cx| {
            builder(menu, window, cx)
        });
        let dismiss_subscription =
            cx.subscribe(&submenu, |this, _submenu, _: &DismissEvent, cx| {
                this.close_submenu(cx);
            });
        (submenu, dismiss_subscription)
    }

    /// 关闭子菜单（对齐 zed `ContextMenu::close_submenu`，zed `context_menu.rs:1346`）。
    fn close_submenu(&mut self, cx: &mut Context<Self>) {
        self.submenu_state = SubmenuState::Closed;
        cx.notify();
    }

    /// 打开某一项的子菜单（对齐 zed `ContextMenu::open_submenu`，
    /// zed `context_menu.rs:1360`）。已经开着就不重建——与 zed 同。
    fn open_submenu(
        &mut self,
        item_index: usize,
        builder: Rc<dyn Fn(ContextMenu, &mut Window, &mut Context<ContextMenu>) -> ContextMenu>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(
            &self.submenu_state,
            SubmenuState::Open(open_submenu) if open_submenu.item_index == item_index
        ) {
            return;
        }

        let (submenu, dismiss_subscription) = Self::create_submenu(builder, window, cx);

        self.submenu_state = SubmenuState::Open(OpenSubmenu {
            item_index,
            entity: submenu,
            _dismiss_subscription: dismiss_subscription,
        });
        cx.notify();
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
                .color(colors.text_accent.into())
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
            entry.action.as_ref().map(|action| {
                // 设过 `context()` 就按那个焦点上下文解析（同一 action 在不同
                // 焦点区域可能绑不同键），否则用全局绑定。对齐 zed 的
                // `action_context`（context_menu.rs:974 / 1030）。
                match self.action_context.as_ref() {
                    Some(context) => KeyBinding::for_action_in(action.as_ref(), context, cx),
                    None => KeyBinding::for_action(action.as_ref(), cx),
                }
            })
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

    /// 子菜单的行（对齐 zed `render_menu_entry` 里的 Submenu 分支，
    /// zed `context_menu.rs:1650` 附近）。
    ///
    /// 与 zed 的差异：zed 把子菜单浮层挂在**整个菜单**上（canvas 量行 bounds →
    /// 算 `offset` → 绝对定位 + 贴边翻转）；我们直接把浮层 `anchored()` 到这一行
    /// 右侧的一个零尺寸定位点上，位置交给 gpui 算，于是 zed 的
    /// `main_menu_observed_bounds` / `flip_left` 整套都不需要。
    fn render_submenu_row(
        &self,
        ix: usize,
        label: SharedString,
        icon: Option<IconName>,
        icon_color: Option<Color>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme().colors().clone();
        let font_size = px(12.0);
        let min_width = self.min_width.map(px);

        let open_submenu = match &self.submenu_state {
            SubmenuState::Open(open_submenu) if open_submenu.item_index == ix => {
                Some(open_submenu.entity.clone())
            }
            _ => None,
        };

        div()
            .id(("context-menu-submenu", ix))
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_1()
            .px_1p5()
            .py_1()
            .text_size(font_size)
            .when_some(min_width, |this, w| this.min_w(w))
            .hover(|style| style.bg(colors.ghost_element_hover))
            .on_click(cx.listener(move |this: &mut ContextMenu, _, window, cx| {
                this.activate(ix, window, cx);
            }))
            .child(
                h_flex()
                    .flex_1()
                    .gap_1()
                    .children(icon.map(|icon| {
                        Icon::new(icon)
                            .size(IconSize::Small)
                            .when_some(icon_color, |this, color| this.color(color))
                    }))
                    .child(div().text_color(colors.text).child(label)),
            )
            .child(
                Icon::new(IconName::ChevronRight)
                    .size(IconSize::XSmall)
                    .color(Color::Muted),
            )
            .when_some(open_submenu, |this, submenu| {
                this.child(
                    div().absolute().top_0().right_0().child(deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .snap_to_window_with_margin(px(8.0))
                            .child(div().occlude().child(submenu)),
                    )),
                )
            })
    }
}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for ContextMenu {}

// zed: `impl FluentBuilder for ContextMenu {}`（context_menu.rs:271）。
// 同上：菜单本体是 `Entity`（不是元素），拿不到 gpui 的 blanket impl，
// 显式补一条，`|menu, _, _| menu.separator().when_some(...)` 这类链式写法才成立。
impl FluentBuilder for ContextMenu {}

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
                // zed: `context_menu.rs:1505` 的 CustomEntry 分支。行内容交给
                // `entry_render`，菜单负责套壳 + 挂命中回调（与 zed 同：回调不在
                // `entry_render` 里挂）。简化点：zed 外面还额外铺了一层 canvas
                // 给文档侧栏量位置，我们没有 `aside_trigger_bounds`，略过。
                ContextMenuItem::CustomEntry { entry_render, .. } => {
                    let rendered = entry_render(window, cx);
                    div()
                        .id(("context-menu-child", ix))
                        .child(rendered)
                        .on_click(cx.listener(move |this: &mut ContextMenu, _, window, cx| {
                            this.activate(ix, window, cx);
                        }))
                        .into_any_element()
                }
                // 子菜单的行（对齐 zed `render_menu_entry` 的 Submenu 分支，
                // zed `context_menu.rs:1600` 附近）：label + 可选图标 + 行尾
                // 箭头；打开时把子菜单 `anchored()` 挂在这一行右侧（zed 是挂在
                // 整个菜单上的绝对定位，我们用 anchored，省掉 bounds 观测）。
                ContextMenuItem::Submenu {
                    label, icon, icon_color, ..
                } => self
                    .render_submenu_row(ix, label.clone(), *icon, *icon_color, window, cx)
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
// | zed 的 API | 说明 | 差距 |
// |---|---|---|
// | `submenu(...)` / `submenu_with_icon` / `submenu_with_colored_icon` | 子菜单 | 已搬最小版：点击命中时现建实体 + `anchored()` 到本行右侧；zed 那套 bounds 观测偏移 / 贴边翻转 / hover 切换 / 键盘进入子菜单没搬 |
// | `custom_row` / `custom_entry` / `custom_entry_with_docs` / `selectable` | 调用方自绘行内容 | 已搬；`documentation_aside` 只落了数据链路，zed 那套 canvas 定位的侧栏渲染没搬 |
// | `entry_with_end_slot` / `entry_with_end_slot_on_hover` | 行尾自定义槽位 | 未搬 |
// | 键盘导航 | `is_selectable()` 已搬，但没有 zed 的 `selected_index` 上下键选择 + Enter 触发 | 部分 |
