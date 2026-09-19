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

use gpui::{ClipboardItem, Context, Window, actions};
use unicode_segmentation::UnicodeSegmentation;

use super::selection::{Selection, SelectionGoal};
use super::undo::EditIntent;
use super::{DisplayPoint, Editor, movement};
use crate::engine::Point as BufferPoint;
use std::ops::Range;

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
        // ---- 多光标(对齐 zed editor::AddSelectionAbove/Below 与 Secondary-d 系列)----
        AddSelectionAbove,
        AddSelectionBelow,
        /// 把下一处相同文本加成新光标(语义对齐 zed `editor::SelectNext`,
        /// 也就是 VS Code 的 cmd-d)。
        SelectNextOccurrence,
        /// 收拢为单个主光标(对齐 zed `editor::Cancel`,绑 escape)。
        Cancel,
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

    /// 左移:有选区则收拢到选区起点,否则退一个字素。**对每个光标生效**。
    pub(crate) fn move_left_core(&mut self, cx: &mut Context<Self>) {
        self.collapse_heads_to(|this, selection| {
            if !selection.is_empty() {
                selection.range().start
            } else {
                this.previous_boundary(selection.head())
            }
        });
        self.change_selections(cx);
    }

    pub fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.move_right_core(cx);
    }

    /// 右移:有选区则收拢到选区终点,否则进一个字素。**对每个光标生效**。
    pub(crate) fn move_right_core(&mut self, cx: &mut Context<Self>) {
        self.collapse_heads_to(|this, selection| {
            if !selection.is_empty() {
                selection.range().end
            } else {
                this.next_boundary(selection.head())
            }
        });
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
    /// 每个光标独立计算(各自保留自己的 goal);display_map 尚未同步
    /// (首帧)时退化为按 buffer 行移动。
    pub(crate) fn move_vertical_core(&mut self, direction: i32, cx: &mut Context<Self>) {
        for index in 0..self.selections.len() {
            let selection = self.selections[index];
            let Some(target) = self.vertical_target(selection, direction) else {
                continue;
            };
            let (target, goal) = target;
            self.selections[index].collapse_to(target, goal);
        }
        self.normalize_selections();
        self.change_selections(cx);
    }

    /// 某个光标上/下移动的目标字节位置与 goal。
    ///
    /// display_map 未就绪时走 buffer 行的退化路径。
    fn vertical_target(
        &self,
        selection: Selection<usize>,
        direction: i32,
    ) -> Option<(usize, SelectionGoal)> {
        let head = selection.head();
        if self.display_map.display_rows() > 0 {
            let point = self.display_point_for_offset(head);
            Some(if direction < 0 {
                let (point, goal) =
                    movement::up(&self.display_map, &self.rope, point, selection.goal);
                (self.offset_for_display_point(point), goal)
            } else {
                let (point, goal) =
                    movement::down(&self.display_map, &self.rope, point, selection.goal);
                (self.offset_for_display_point(point), goal)
            })
        } else {
            Some((
                self.buffer_vertical_target(head, direction)?,
                SelectionGoal::None,
            ))
        }
    }

    /// display 点 → buffer 字节偏移(走 display_map 逆映射)。
    fn offset_for_display_point(&self, point: DisplayPoint) -> usize {
        let buffer_point = self.display_map.display_point_to_buffer_point(point);
        self.rope.point_to_offset(buffer_point)
    }

    /// 按 buffer 行上下移动的目标偏移(已在边界则返回 None)。
    fn buffer_vertical_target(&self, head: usize, direction: i32) -> Option<usize> {
        let row = self.rope.offset_to_point(head).row;
        let max_row = self.rope.summary().lines.row;
        let target_row = row as i64 + direction as i64;
        // 越界靠但不动——对齐 zed 的语义:首行再上移到文档头、末行再下移到文档尾
        if target_row < 0 {
            return Some(0);
        }
        if target_row as u32 > max_row {
            return Some(self.rope.len());
        }
        let target_row = target_row as u32;
        let column = self.rope.offset_to_point(head).column;
        let target_start = self.rope.point_to_offset(BufferPoint::new(target_row, 0));
        let target_end = if target_row == max_row {
            self.rope.len()
        } else {
            self.rope
                .point_to_offset(BufferPoint::new(target_row + 1, 0))
                - 1
        };
        Some((target_start + column as usize).min(target_end))
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.collapse_heads_to(|_, _| 0);
        self.change_selections(cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.collapse_heads_to(|this, _| this.rope.len());
        self.change_selections(cx);
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        let selection = Selection::new(self.rope.len(), 0);
        self.set_selection(selection);
        self.change_selections(cx);
    }

    // ---- 选择(移动 + set_head)----

    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.set_heads_to(|this, selection| this.previous_boundary(selection.head()));
        self.change_selections(cx);
    }

    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.set_heads_to(|this, selection| this.next_boundary(selection.head()));
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
        for index in 0..self.selections.len() {
            let selection = self.selections[index];
            let Some((target, goal)) = self.vertical_target(selection, direction) else {
                continue;
            };
            self.selections[index].set_head(target, goal);
        }
        self.normalize_selections();
        self.change_selections(cx);
    }

    // ---- 编辑 ----

    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.backspace_core(cx) {
            window.play_system_bell();
        }
    }

    /// 删除选区或光标前一个字素(每个光标各删一处)。返回 true 表示所有
    /// 光标都已在文本边界(无事发生,调用方决定是否响铃)。
    pub(crate) fn backspace_core(&mut self, cx: &mut Context<Self>) -> bool {
        let mut at_boundary = true;
        let selections = self.selections.clone();
        for selection in selections.iter() {
            if !selection.is_empty() {
                at_boundary = false;
                continue;
            }
            let prev = self.previous_boundary(selection.head());
            if prev == selection.head() {
                continue;
            }
            at_boundary = false;
        }
        if at_boundary {
            return true;
        }
        // 空光标往后扩展一格;有选区的保持原样 —— 替换 "" 即删除
        self.set_heads_to(|this, selection| {
            if selection.is_empty() {
                this.previous_boundary(selection.head())
            } else {
                selection.head()
            }
        });
        self.replace_selections("", EditIntent::Backspace, cx);
        false
    }

    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.delete_core(cx) {
            window.play_system_bell();
        }
    }

    /// 删除选区或光标后一个字素(每个光标各删一处)。返回 true 表示所有
    /// 光标都已在文本边界。
    pub(crate) fn delete_core(&mut self, cx: &mut Context<Self>) -> bool {
        let selections = self.selections.clone();
        // 空光标先向前扩展到下一字素(reversed,同 zed 的 delete forward)
        let mut extended = Vec::with_capacity(selections.len());
        let mut any_change = false;
        for selection in &selections {
            if selection.is_empty() {
                let head = selection.head();
                let next = self.next_boundary(head);
                if next != head {
                    any_change = true;
                    extended.push(Selection::new(next, head));
                    continue;
                }
            }
            any_change |= !selection.is_empty();
            extended.push(*selection);
        }
        if !any_change {
            return true;
        }
        self.selections = extended;
        self.normalize_selections();
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

    pub fn newline_core(&mut self, cx: &mut Context<Self>) {
        let texts: Vec<String> = self
            .selections
            .iter()
            .map(|selection| {
                // 缩进继承:当前行行首的空白
                let head = selection.head();
                let row_start = self
                    .rope
                    .point_to_offset(BufferPoint::new(self.rope.offset_to_point(head).row, 0));
                let line_head = self.rope.text_in_range(row_start..head);
                let indent: String = line_head
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                format!("\n{indent}")
            })
            .collect();
        self.replace_all_selections(texts, EditIntent::Atomic, cx);
    }

    /// 粘贴(对齐 zed:按吩咐与光标数相同行数时按行分发,否则每处贴整段)。
    pub fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let text = if self.mode.is_single_line() {
            text.replace('\n', " ")
        } else {
            text
        };
        // 多光标 + 剪贴板行数正好 = 光标数 → 一行一个光标
        if self.selections.len() > 1 {
            let lines: Vec<String> = text.split('\n').map(|line| line.to_string()).collect();
            if lines.len() == self.selections.len() {
                self.replace_all_selections(lines, EditIntent::Atomic, cx);
                return;
            }
        }
        self.replace_selections(&text, EditIntent::Atomic, cx);
    }

    /// 复制:多个选区用换行连起来(同 VS Code / zed 的习惯)。
    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let selected: Vec<String> = self
            .selections
            .iter()
            .map(|selection| selection.range())
            .filter(|range| !range.is_empty())
            .map(|range| self.rope.text_in_range(range))
            .collect();
        if !selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(selected.join("\n")));
        }
    }

    pub fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        let any_non_empty = self
            .selections
            .iter()
            .any(|selection| !selection.is_empty());
        if any_non_empty {
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
        self.collapse_heads_to(|this, selection| this.previous_word_start_offset(selection.head()));
        self.change_selections(cx);
    }

    pub fn move_to_next_word_end(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.collapse_heads_to(|this, selection| this.next_word_end_offset(selection.head()));
        self.change_selections(cx);
    }

    pub fn delete_to_previous_word_start(
        &mut self,
        _: &DeleteToPreviousWordStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.selections.clone();
        self.set_heads_to(|this, selection| this.previous_word_start_offset(selection.head()));
        if self.selections == before {
            return; // 全都在文档头:没有可删的内容
        }
        self.replace_selections("", EditIntent::Atomic, cx);
    }

    pub fn delete_to_next_word_end(
        &mut self,
        _: &DeleteToNextWordEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selections = self.selections.clone();
        let extended: Vec<Selection<usize>> = selections
            .iter()
            .map(|selection| {
                if selection.is_empty() {
                    Selection::new(
                        self.next_word_end_offset(selection.head()),
                        selection.head(),
                    )
                } else {
                    *selection
                }
            })
            .collect();
        if extended == selections {
            return;
        }
        self.selections = extended;
        self.normalize_selections();
        self.replace_selections("", EditIntent::Atomic, cx);
    }

    // ---- 多光标(对齐 zed selection.rs 的 add_selection / select_next)----

    /// 在上方加一个光标(zed `editor::AddSelectionAbove`)。
    pub fn add_selection_above(
        &mut self,
        _: &AddSelectionAbove,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_selection_vertical(true, cx);
    }

    /// 在下方加一个光标(zed `editor::AddSelectionBelow`)。
    pub fn add_selection_below(
        &mut self,
        _: &AddSelectionBelow,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_selection_vertical(false, cx);
    }

    /// 为**每个**现有光标在上/下同位加一个光标(同列、保持 goal 列)。
    ///
    /// 与 zed 的差异:zed 用像素位找同列的新光标(`x_for_display_point`),
    /// 这里用 display 列([`movement`] 的 goal 语义)——一是避免为未渲染的
    /// 行做排版,二是尚无 x 度量缓存。到边界(首行再上、末行再下)不再加。
    pub(crate) fn add_selection_vertical(&mut self, above: bool, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            return;
        }
        let direction = if above { -1 } else { 1 };
        let selections = self.selections.clone();
        let mut additions: Vec<Selection<usize>> = Vec::new();
        for selection in &selections {
            let Some((target, goal)) = self.vertical_target(*selection, direction) else {
                continue;
            };
            // 已经走到文档边界 → 该光标不再扩展
            if self.display_row_for_offset(target) == self.display_row_for_offset(selection.head())
            {
                continue;
            }
            let mut added = Selection::new(target, target);
            added.goal = goal; // 继承 goal 列:连续加光标时列不漂
            additions.push(added);
        }
        if additions.is_empty() {
            return;
        }
        for added in additions {
            // 不去重:与已有光标重合时由 normalize 合并(zed 的 add_selection
            // 也是合并相交者),因此连续下压能一路往下加,而不是互相撤销
            self.push_selection(added);
        }
        self.change_selections(cx);
    }

    /// 把下一处相同文本加成新光标(VS Code 的 cmd-d)。
    ///
    /// 主光标为空 → 先选中光标下的词,再把它的下一处加成新光标。
    pub fn select_next_occurrence(
        &mut self,
        _: &SelectNextOccurrence,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_next_occurrence_core(cx);
    }

    pub(crate) fn select_next_occurrence_core(&mut self, cx: &mut Context<Self>) {
        let primary = self.selection();
        let range = if primary.is_empty() {
            self.word_range_at(primary.head())
        } else {
            primary.range()
        };
        if range.is_empty() {
            return;
        }
        let query = self.rope.text_in_range(range.clone());
        if query.is_empty() {
            return;
        }
        // 全串搜索:这个动作才用一次,可接受(zed 走 rope 的 find 流式检索)
        let text = self.rope.to_string();
        let from = range.end.min(text.len());
        let Some(hit) = text[from..].find(&query) else {
            return; // 没有下一处 → 什么都不做(同 VS Code)
        };
        let found_start = from + hit;
        let found = Selection::new(found_start + query.len(), found_start);
        if primary.is_empty() {
            // 主光标还是空 → 补成“选词 + 下一处”,一处给当前、一处给下一处
            let first = Selection::new(range.end, range.start);
            let mut rest = self.selections.clone();
            rest.pop(); // 去掉刚才那个空的主光标
            rest.push(first);
            rest.push(found);
            self.set_selections(rest);
        } else {
            self.add_selection(found);
        }
        self.change_selections(cx);
    }

    /// Escape:收拢成单个主光标(zed `editor::Cancel` 在有其一职责)。
    pub fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        self.cancel_core(cx);
    }

    pub(crate) fn cancel_core(&mut self, cx: &mut Context<Self>) {
        if self.selections.len() <= 1 {
            return;
        }
        let primary = self.selection();
        self.set_selection(Selection::new(primary.head(), primary.head()));
        self.change_selections(cx);
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
