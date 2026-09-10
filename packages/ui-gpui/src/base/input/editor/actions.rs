//! 编辑动作:定义与处理。
//!
//! 动作名对齐 zed editor 的 action 集合(move_left/select_left/backspace/
//! newline/undo…);处理方法统一走 [`Editor::replace_selections`]
//! (移动选区 + 替换)或 [`Editor::change_selections`](纯移动)。
//!
//! 分层:每个动作是薄包装(接收 window,处理响铃/字符面板),核心逻辑在
//! 无 window 的 `*_core` 方法里——动作方法可以用 `cx.listener` 注册,
//! core 方法可以在无窗口测试里直接调用。
//!
//! 阶段 A 移动在 buffer 字节空间;display_map 落地后(阶段 B)切到
//! display 空间并启用 goal 列保持——core 方法签名不变。

use gpui::{actions, ClipboardItem, Context, Window};
use unicode_segmentation::UnicodeSegmentation;

use super::selection::{Selection, SelectionGoal};
use std::ops::Range;
use super::undo::EditIntent;
use super::{Editor, movement};
use crate::base::input::engine::Point as BufferPoint;

actions!(
    editor,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        MoveToBeginning,
        MoveToEnd,
        Newline,
        Paste,
        Copy,
        Cut,
        Undo,
        Redo,
        ShowCharacterPalette,
        WordLeft,
        WordRight,
        DeleteToPreviousWordStart,
        DeleteToNextWordEnd,
    ]
);

// ---- 字素边界(与旧 InputState 同语义;字素移动下沉 engine 属阶段 B)----

impl Editor {
    /// 上一个字素边界(避免切断 emoji / 组合字符)。
    pub fn previous_boundary(&self, offset: usize) -> usize {
        let head = self.rope.text_in_range(0..offset);
        head.grapheme_indices(true)
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// 下一个字素边界。
    pub fn next_boundary(&self, offset: usize) -> usize {
        let text = self.rope.to_string();
        text.grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(text.len())
    }

    // ---- 纯移动 ----

    pub fn move_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.move_left_core(cx);
    }

    pub(crate) fn move_left_core(&mut self, cx: &mut Context<Self>) {
        if !self.selection.is_empty() {
            self.selection
                .collapse_to(self.selection.range().start, SelectionGoal::None);
        } else {
            let head = self.selection.head();
            self.selection
                .collapse_to(self.previous_boundary(head), SelectionGoal::None);
        }
        self.change_selections(cx);
    }

    pub fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.move_right_core(cx);
    }

    pub(crate) fn move_right_core(&mut self, cx: &mut Context<Self>) {
        if !self.selection.is_empty() {
            self.selection
                .collapse_to(self.selection.range().end, SelectionGoal::None);
        } else {
            let head = self.selection.head();
            self.selection
                .collapse_to(self.next_boundary(head), SelectionGoal::None);
        }
        self.change_selections(cx);
    }

    pub fn move_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            return; // 对齐 zed:单行纵向移动交回宿主(propagate 在注册时表达)
        }
        self.move_vertical_core(-1, cx);
    }

    pub fn move_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            return;
        }
        self.move_vertical_core(1, cx);
    }

    /// 上下移动:在 **display(视觉)空间**进行(软换行下一条 buffer 行可能
    /// 占多条视觉行),带 goal 列保持——对齐 zed movement::up/down。
    ///
    /// display_map 尚未同步(首帧)时退化为按 buffer 行移动。
    pub(crate) fn move_vertical_core(&mut self, direction: i32, cx: &mut Context<Self>) {
        if self.display_map.display_rows() > 0 {
            let head = self.selection.head();
            let point = self.display_point_for_offset(head);
            let (new_point, goal) = if direction < 0 {
                movement::up(&self.display_map, &self.rope, point, self.selection.goal)
            } else {
                movement::down(&self.display_map, &self.rope, point, self.selection.goal)
            };
            let buffer_point = self
                .display_map
                .display_point_to_buffer_point(new_point);
            let target = self.rope.point_to_offset(buffer_point);
            self.selection.collapse_to(target, goal);
            self.change_selections(cx);
            return;
        }
        self.move_vertical_buffer(direction, cx);
    }

    /// 按 buffer 行移动(display_map 未就绪时的退化路径)。
    fn move_vertical_buffer(&mut self, direction: i32, cx: &mut Context<Self>) {
        let head = self.selection.head();
        let (row, column) = self.rope.offset_to_point(head).into_parts();
        let target_row = row as i64 + direction as i64;
        if target_row < 0 {
            self.selection.collapse_to(0, SelectionGoal::None);
            return self.change_selections(cx);
        }
        let target_row = target_row as u32;
        let max_row = self.rope.summary().lines.row;
        if target_row > max_row {
            let end = self.rope.len();
            self.selection.collapse_to(end, SelectionGoal::None);
            return self.change_selections(cx);
        }
        // 目标行的合法偏移(行首..下一行首-1)
        let target_start = self.rope.point_to_offset(BufferPoint::new(target_row, 0));
        let target_end = if target_row == max_row {
            self.rope.len()
        } else {
            self.rope.point_to_offset(BufferPoint::new(target_row + 1, 0)) - 1
        };
        let target = (target_start + column as usize).min(target_end);
        self.selection.collapse_to(target, SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.selection.collapse_to(0, SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let end = self.rope.len();
        self.selection.collapse_to(end, SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selection = Selection::new(self.rope.len(), 0);
        self.change_selections(cx);
    }

    // ---- 选择(移动 + set_head)----

    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let head = self.selection.head();
        self.selection
            .set_head(self.previous_boundary(head), SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let head = self.selection.head();
        self.selection
            .set_head(self.next_boundary(head), SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            return;
        }
        self.select_vertical_core(-1, cx);
    }

    pub fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            return;
        }
        self.select_vertical_core(1, cx);
    }

    /// 与 [`Self::move_vertical_core`] 同逻辑,但作用于 head(set_head 保持 tail)。
    fn select_vertical_core(&mut self, direction: i32, cx: &mut Context<Self>) {
        if self.display_map.display_rows() > 0 {
            let head = self.selection.head();
            let point = self.display_point_for_offset(head);
            let (new_point, goal) = if direction < 0 {
                movement::up(&self.display_map, &self.rope, point, self.selection.goal)
            } else {
                movement::down(&self.display_map, &self.rope, point, self.selection.goal)
            };
            let buffer_point = self
                .display_map
                .display_point_to_buffer_point(new_point);
            let target = self.rope.point_to_offset(buffer_point);
            self.selection.set_head(target, goal);
            self.change_selections(cx);
            return;
        }

        let head = self.selection.head();
        let (row, column) = self.rope.offset_to_point(head).into_parts();
        let target_row = (row as i64 + direction as i64).max(0) as u32;
        let max_row = self.rope.summary().lines.row;
        let target_start = self
            .rope
            .point_to_offset(BufferPoint::new(target_row.min(max_row + 1), 0));
        let target_end = if target_row >= max_row {
            self.rope.len()
        } else {
            self.rope.point_to_offset(BufferPoint::new(target_row + 1, 0)) - 1
        };
        let target = (target_start + column as usize).min(target_end);
        self.selection.set_head(target, SelectionGoal::None);
        self.change_selections(cx);
    }

    // ---- 编辑 ----

    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.backspace_core(cx) {
            window.play_system_bell();
        }
    }

    /// 删除选区或光标前一个字素。返回 true 表示已到文本边界(无事发生,
    /// 调用方决定是否响铃)。
    pub(crate) fn backspace_core(&mut self, cx: &mut Context<Self>) -> bool {
        if self.selection.is_empty() {
            let head = self.selection.head();
            let prev = self.previous_boundary(head);
            if prev == head {
                return true;
            }
            self.selection.set_head(prev, SelectionGoal::None);
        }
        self.replace_selections("", EditIntent::Backspace, cx);
        false
    }

    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.delete_core(cx) {
            window.play_system_bell();
        }
    }

    /// 删除选区或光标后一个字素。返回 true 表示已到文本边界。
    pub(crate) fn delete_core(&mut self, cx: &mut Context<Self>) -> bool {
        if self.selection.is_empty() {
            let head = self.selection.head();
            let next = self.next_boundary(head);
            if next == head {
                return true;
            }
            self.selection = Selection::new(next, head);
        }
        self.replace_selections("", EditIntent::DeleteForward, cx);
        false
    }

    /// 回车分流(对齐 gpui-component state.rs 的 enter()):
    /// 单行 → 总是 Submitted;多行 → 默认插换行(继承行首缩进),
    /// 开了 submit_on_enter 才提交。
    pub fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        if self.mode.is_single_line() || self.submit_on_enter {
            cx.emit(super::EditorEvent::Submitted);
            return;
        }
        self.newline_core(cx);
    }

    pub(crate) fn newline_core(&mut self, cx: &mut Context<Self>) {
        // 缩进继承:当前行行首的空白
        let head = self.selection.head();
        let row_start = self
            .rope
            .point_to_offset(BufferPoint::new(self.rope.offset_to_point(head).row, 0));
        let line_head = self.rope.text_in_range(row_start..head);
        let indent: String = line_head.chars().take_while(|c| c.is_whitespace()).collect();
        let text = format!("\n{indent}");
        self.replace_selections(&text, EditIntent::Atomic, cx);
    }

    pub fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.mode.is_single_line() {
                text.replace('\n', " ")
            } else {
                text
            };
            self.replace_selections(&text, EditIntent::Atomic, cx);
        }
    }

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let range = self.selection.range();
        if !range.is_empty() {
            let selected = self.rope.text_in_range(range);
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
        }
    }

    pub fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        let range = self.selection.range();
        if !range.is_empty() {
            let selected = self.rope.text_in_range(range.clone());
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
            self.replace_selections("", EditIntent::Atomic, cx);
        }
    }

    // ---- 词级移动 / 词删除(对齐 zed 的 word movement)----

    /// 上一个词首(buffer 字节坐标)。
    ///
    /// 词边界用 unicode-segmentation 的词段划分(中文整段一词、英文按词、
    /// URL 保持整体),空白段不计为词。直接作用在 buffer 上,不依赖 display_map。
    pub(crate) fn previous_word_start_offset(&self, offset: usize) -> usize {
        let text = self.rope.to_string();
        let mut best = 0;
        for (idx, word) in text.split_word_bound_indices() {
            if idx >= offset {
                break;
            }
            if !word.trim().is_empty() {
                best = idx;
            }
        }
        best
    }

    /// 下一个词尾。
    pub(crate) fn next_word_end_offset(&self, offset: usize) -> usize {
        let text = self.rope.to_string();
        for (idx, word) in text.split_word_bound_indices() {
            let end = idx + word.len();
            if end > offset && !word.trim().is_empty() {
                return end;
            }
        }
        text.len()
    }

    pub fn move_to_previous_word_start(
        &mut self,
        _: &WordLeft,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let head = self.selection.head();
        self.selection
            .collapse_to(self.previous_word_start_offset(head), SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn move_to_next_word_end(
        &mut self,
        _: &WordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let head = self.selection.head();
        self.selection
            .collapse_to(self.next_word_end_offset(head), SelectionGoal::None);
        self.change_selections(cx);
    }

    pub fn delete_to_previous_word_start(
        &mut self,
        _: &DeleteToPreviousWordStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let head = self.selection.head();
        let target = self.previous_word_start_offset(head);
        if target == head {
            return;
        }
        self.selection.set_head(target, SelectionGoal::None);
        self.replace_selections("", EditIntent::Atomic, cx);
    }

    pub fn delete_to_next_word_end(
        &mut self,
        _: &DeleteToNextWordEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let head = self.selection.head();
        let target = self.next_word_end_offset(head);
        if target == head {
            return;
        }
        self.selection = Selection::new(target, head);
        self.replace_selections("", EditIntent::Atomic, cx);
    }

    /// 双击选词:返回光标所在的词区间。
    pub(crate) fn word_range_at(&self, offset: usize) -> Range<usize> {
        let text = self.rope.to_string();
        for (idx, word) in text.split_word_bound_indices() {
            let end = idx + word.len();
            if idx <= offset && offset <= end && !word.trim().is_empty() {
                return idx..end;
            }
        }
        offset..offset
    }

    pub fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }
}

/// engine::Point 的解构辅助(上下移动取行列)。
trait IntoParts {
    fn into_parts(self) -> (u32, u32);
}

impl IntoParts for BufferPoint {
    fn into_parts(self) -> (u32, u32) {
        (self.row, self.column)
    }
}
