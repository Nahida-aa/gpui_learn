//! context_menu/entry：菜单项的数据层（对齐 zed `ContextMenuItem` / `ContextMenuEntry`）。
//!
//! 数据与渲染分离：本文件只描述"菜单里有什么"（标签、图标、勾选、快捷键、
//! 禁用、回调），具体画成什么样由 [`super::menu`] 的 `Render` 决定。
//!
//! 出处：zed `crates/ui/src/components/context_menu.rs`
//! （GPL-3.0-or-later）。与 zed 的差异在注释里逐条标出。

use std::rc::Rc;

use gpui::{Action, App, SharedString, Window};

use aa_gpui_base::{Icon, IconName, IconSize};
use crate::{Color, KeyBinding};

/// 图标在条目的哪一侧（对齐 zed `IconPosition`）。
///
/// 也用于 `toggleable`：勾选标记画在首列还是尾列。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum IconPosition {
    /// 图标 / 勾选在文字之前（默认）。
    #[default]
    Start,
    /// 图标 / 勾选在文字之后。
    End,
}

/// 单个可点击菜单项。
///
/// 用法（与 zed 同构）：
/// ```ignore
/// ContextMenuEntry::new("Dock Left")
///     .icon(IconName::PanelLeft)
///     .checked(side == DockSide::Left)
///     .action(Box::new(editor::ToggleLeftDock))
/// ```
pub struct ContextMenuEntry {
    pub(crate) label: SharedString,
    /// 左侧勾选列是否打勾（当前 dock 位置等）。
    pub(crate) checked: bool,
    /// 禁用态：置灰且点击无效。
    pub(crate) disabled: bool,
    /// 条目前的图标（对齐 zed `icon`）。
    pub(crate) icon: Option<IconName>,
    /// 图标位置（对齐 zed `icon_position`）。
    pub(crate) icon_position: IconPosition,
    /// 图标尺寸（对齐 zed `icon_size`）。
    pub(crate) icon_size: IconSize,
    /// 图标颜色（对齐 zed `icon_color`）。
    pub(crate) icon_color: Option<Color>,
    /// 勾选标记的位置（`toggleable` 设入；`None` 表示不在"切换"语义里）。
    pub(crate) toggle_position: Option<IconPosition>,
    /// 点击时要派发的 action（对齐 zed `action`）。
    ///
    /// 与 `handler` 二选一：action 走 gpui 的动作派发（可被 keymap 命中、
    /// 有 `Action::name` 可用于显示快捷键），handler 是裸回调。
    pub(crate) action: Option<Box<dyn Action>>,
    /// 点击回调（对齐 zed `handler`）。
    pub(crate) on_click: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    /// 显示在条目右侧的快捷键提示（对齐 zed 用 action 反查 keymap 的做法，
    /// 我们允许显式传入 —— 没有 keymap 的调用方也能显示）。
    pub(crate) key_binding: Option<KeyBinding>,
    /// 是否可被"选中"（选中态会画高亮背景，对齐 zed `selectable`）。
    pub(crate) selectable: bool,
}

// `Box<dyn Action>` 不是 `Clone`，所以手写（action 自己提供 `boxed_clone`）。
impl Clone for ContextMenuEntry {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            checked: self.checked,
            disabled: self.disabled,
            icon: self.icon,
            icon_position: self.icon_position,
            icon_size: self.icon_size,
            icon_color: self.icon_color,
            toggle_position: self.toggle_position,
            action: self.action.as_ref().map(|action| action.boxed_clone()),
            on_click: self.on_click.clone(),
            key_binding: self.key_binding.clone(),
            selectable: self.selectable,
        }
    }
}

impl ContextMenuEntry {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            disabled: false,
            icon: None,
            icon_position: IconPosition::default(),
            icon_size: IconSize::Small,
            icon_color: None,
            toggle_position: None,
            action: None,
            on_click: None,
            key_binding: None,
            selectable: true,
        }
    }

    /// 勾选列打勾（如当前 dock 位置高亮）。
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// 置为禁用：置灰 + 点击无效。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 条目前的图标（对齐 zed `icon`）。
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 图标位置（对齐 zed `icon_position`）。
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// 图标尺寸（对齐 zed `icon_size`）。
    pub fn icon_size(mut self, icon_size: IconSize) -> Self {
        self.icon_size = icon_size;
        self
    }

    /// 图标颜色（对齐 zed `icon_color`）。
    pub fn icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = Some(icon_color);
        self
    }

    /// 标记这是个"可切换"条目：勾选标记画在 `toggle_position` 一侧。
    ///
    /// 对齐 zed `ContextMenuEntry::toggleable`。
    pub fn toggleable(mut self, toggle_position: IconPosition, toggled: bool) -> Self {
        self.toggle_position = Some(toggle_position);
        self.checked = toggled;
        self
    }

    /// `toggleable` 的别名（zed 两个名字都有，语义相同）。
    pub fn toggle(self, toggle_position: IconPosition, toggled: bool) -> Self {
        self.toggleable(toggle_position, toggled)
    }

    /// 点击时派发的 action（对齐 zed `action`）。
    pub fn action(mut self, action: Box<dyn Action>) -> Self {
        self.action = Some(action);
        self
    }

    /// 设置点击回调。trigger 阶段只收集，由 `activate` 统一触发。
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// `on_click` 的别名（对齐 zed `handler`）。
    pub fn handler(self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click(handler)
    }

    /// 条目右侧的快捷键提示（对齐 zed 的"显示绑定"效果，但由调用方显式给）。
    pub fn key_binding(mut self, key_binding: impl Into<Option<KeyBinding>>) -> Self {
        self.key_binding = key_binding.into();
        self
    }

    /// 是否可被选中（选中态画高亮背景，对齐 zed `selectable`）。
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// 点击后要做的事：禁用直接忽略；有 action 就派发 action，否则跑回调。
    ///
    /// 与 zed 的差异：zed 在 `ContextMenu::activate_entry` 里做这件事
    /// （要处理 submenu 分支）；我们没有子菜单，所以收在条目自己身上。
    pub(crate) fn activate(&self, window: &mut Window, cx: &mut App) {
        if self.disabled {
            return;
        }
        if let Some(action) = &self.action {
            window.dispatch_action(action.boxed_clone(), cx);
            return;
        }
        if let Some(handler) = &self.on_click {
            handler(window, cx);
        }
    }

    /// 渲染用的图标（颜色已按禁用态展开）。
    ///
    /// 需要 `Window`：`IconSize` 是 rem 单位，换算成 px 要用 `window.rem_size()`。
    pub(crate) fn render_icon(&self, window: &Window, cx: &App) -> Option<Icon> {
        let icon = self.icon?;
        let color = if self.disabled {
            Color::Disabled
        } else {
            self.icon_color.unwrap_or(Color::Muted)
        };
        Some(
            Icon::new(icon)
                .size(self.icon_size.rems() * window.rem_size())
                .color(color.color(cx)),
        )
    }
}

/// 一次显示在菜单里的内容项（对齐 zed `ContextMenuItem` 的精简子集）。
pub enum ContextMenuItem {
    /// 可交互项（勾选/图标/禁用）。
    Entry(ContextMenuEntry),
    /// 分隔线。
    Separator,
    /// 纯文本（muted 色，不可交互）。
    Label(SharedString),
}

impl From<ContextMenuEntry> for ContextMenuItem {
    fn from(entry: ContextMenuEntry) -> Self {
        ContextMenuItem::Entry(entry)
    }
}

impl From<&str> for ContextMenuItem {
    fn from(label: &str) -> Self {
        ContextMenuItem::Label(label.into())
    }
}
