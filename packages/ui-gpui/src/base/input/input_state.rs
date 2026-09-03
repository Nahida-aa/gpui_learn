//! 输入框的持久状态。
//!
//! 与 [`crate::base::slider::SliderState`] 同一套双层架构：本文件是「状态」那一半，
//! 由 `Entity<InputState>` 持有，负责文本、选区、IME 组字区，并把指针位置换算成
//! 字符下标。文本的实际绘制在 [`crate::base::input::element::TextElement`]。
//!
//! 移植自教学示例 `apps/04_input`，修正见 [`crate::base::input`] 的模块文档。

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, Hsla, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Render, Rgba, ShapedLine, SharedString, UTF16Selection, Window, actions, div, hsla,
    prelude::*, px, rgb,
};
use unicode_segmentation::*;

use super::element::TextElement;

/// 输入框的 key_context 名，[`bind_input_keys`] 与渲染时的 `.key_context(..)` 共用。
pub const INPUT_KEY_CONTEXT: &str = "ui-gpui-input";

actions!(
    ui_gpui_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Enter,
    ]
);

/// 输入框对外发出的事件。外部用 `cx.subscribe(&input_state, …)` 订阅。
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// 内容发生变化（键入 / 粘贴 / 删除 / IME 上屏）。
    /// 程序调用 [`InputState::set_value`] **不**触发，避免回环。
    Change(SharedString),
    /// 用户在输入框聚焦时按下 Enter。
    Submit(SharedString),
}

/// 输入框的持久状态，由 `Entity<InputState>` 持有。
pub struct InputState {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) content: SharedString,
    pub(crate) placeholder: SharedString,
    pub(crate) selected_range: Range<usize>,
    pub(crate) selection_reversed: bool,
    /// IME 组字中的区间（未上屏）。
    pub(crate) marked_range: Option<Range<usize>>,
    /// 最近一帧的文本排版结果，命中测试（点击定位光标）要用。
    pub(crate) last_layout: Option<ShapedLine>,
    /// 最近一帧的文本区域，命中测试要用。
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    pub(crate) is_selecting: bool,
    pub(crate) disabled: bool,
    /// 背景色 / 边框色是 `Rgba`；文本色系必须是 `Hsla`（gpui 的 `TextStyle::color`）。
    pub(crate) bg_color: Rgba,
    pub(crate) border_color: Rgba,
    pub(crate) placeholder_color: Hsla,
}

impl InputState {
    /// 新建空输入框。需要 `Context` 来取 [`FocusHandle`]。
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            disabled: false,
            bg_color: rgb(0x1e1e2e),
            border_color: rgb(0x45475a),
            // 中灰：在深色底和浅色底上都能看清。
            placeholder_color: hsla(0., 0., 0.55, 1.),
        }
    }

    /// 占位提示文字。
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// 初始内容（仅在构造时设置，不会触发 [`InputEvent::Change`]）。
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.content = value.into();
        self
    }

    /// 禁用：不响应输入与鼠标，视觉置灰。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 背景色。
    pub fn bg(mut self, color: impl Into<Rgba>) -> Self {
        self.bg_color = color.into();
        self
    }

    /// 边框色。
    pub fn border_color(mut self, color: impl Into<Rgba>) -> Self {
        self.border_color = color.into();
        self
    }

    /// 占位文字颜色（文本色系用 `Hsla`）。
    pub fn placeholder_color(mut self, color: impl Into<Hsla>) -> Self {
        self.placeholder_color = color.into();
        self
    }

    /// 当前内容。
    pub fn value(&self) -> SharedString {
        self.content.clone()
    }

    /// 是否禁用。
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// 程序化设置内容：**不**触发 [`InputEvent::Change`]，光标移到末尾。
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = value.into();
        let end = self.content.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    /// 清空内容。
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_value("", cx);
    }
}

/// 把输入框需要的按键绑到 [`INPUT_KEY_CONTEXT`] 上。
///
/// 必须在应用启动回调里调用一次，否则光标移动/删除/粘贴等全部失效。
pub fn bind_input_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("delete", Delete, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("left", Left, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("right", Right, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("cmd-a", SelectAll, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("cmd-v", Paste, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("cmd-c", Copy, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("cmd-x", Cut, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("home", Home, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("end", End, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("enter", Enter, Some(INPUT_KEY_CONTEXT)),
    ]);
}

// ---- 光标 / 选区 ----

impl InputState {
    /// 光标所在字节下标（选区非空时取「活动端」）。
    pub(crate) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    /// 由鼠标位置换算字符下标（用上一帧的排版结果）。
    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    /// 上一个字素边界（按 grapheme，避免切断 emoji / 组合字符）。
    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    /// 下一个字素边界。
    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    // ---- UTF-16 ↔ UTF-8 下标换算（IME / 剪贴板接口都用 UTF-16 下标）----

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

// ---- action 处理 ----

impl InputState {
    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn enter(&mut self, _: &Enter, _window: &mut Window, cx: &mut Context<Self>) {
        let value = self.content.clone();
        cx.emit(InputEvent::Submit(value));
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace('\n', " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }
}

// ---- IME / 文本输入接口 ----

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..]).into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.notify();
        // 上屏（含键入 / 粘贴 / 删除）才算一次变更；组字中不发，避免高频噪音。
        let value = self.content.clone();
        cx.emit(InputEvent::Change(value));
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..]).into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            gpui::point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            gpui::point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        // 注意：不能断言 last_layout.text == self.content —— 内容为空时渲染的是
        // placeholder，两者不等（教学示例 apps/04_input 就是这么 panic 的）。
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.closest_index_for_x(point.x - line_point.x);
        Some(self.offset_to_utf16(utf8_index))
    }
}

impl EventEmitter<InputEvent> for InputState {}

impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .key_context(INPUT_KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .px_2()
            .py_1()
            .border_1()
            .rounded_md()
            .bg(self.bg_color)
            .border_color(self.border_color)
            .text_size(px(14.))
            .child(TextElement {
                input: cx.entity(),
            })
    }
}
