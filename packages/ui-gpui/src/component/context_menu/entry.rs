//! context_menu/entry：菜单项的数据层（对齐 zed `ContextMenuItem`/`ContextMenuEntry`）。
//!
//! 数据与渲染分离：本文件只描述"菜单里有什么"（标签、勾选、禁用、点击回调），
//! 具体画成什么样由 [`super::menu`] 的重渲染决定——对齐 zed 里 `ContextMenuEntry`
//! 是枚举数据、`ContextMenu::render` 负责绘制。
//!
//! [`ContextMenuEntry::activate`] 是统一的"点了一下"入口：禁用项直接忽略，否则
//! 调用回调；菜单实体随后负责关闭（`DismissEvent`）。

use gpui::{App, SharedString, Window};

use std::rc::Rc;

/// 单个可点击菜单项。
///
/// 用法（与 zed 同构）：
/// ```ignore
/// ContextMenuEntry::new("Dock Left")
///     .checked(side == DockSide::Left)
///     .on_click(|_, cx| { /* 点击后做什么 */ })
/// ```
#[derive(Clone)]
pub struct ContextMenuEntry {
    pub(crate) label: SharedString,
    /// 左侧勾选列是否打勾（当前 dock 位置等）。
    pub(crate) checked: bool,
    /// 禁用态：置灰且点击无效。
    pub(crate) disabled: bool,
    /// 点击回调（`&mut App` 里可 `entity.update(cx, ...)` 驱动业务）。
    on_click: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl ContextMenuEntry {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            disabled: false,
            on_click: None,
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

    /// 设置点击回调。trigger 阶段只收集，由 `activate` 统一触发。
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub(crate) fn activate(&self, window: &mut Window, cx: &mut App) {
        if self.disabled {
            return;
        }
        if let Some(handler) = &self.on_click {
            handler(window, cx);
        }
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
