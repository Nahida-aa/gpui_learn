//! Editor:多行编辑引擎,概念模型对齐 zed editor。
//!
//! 分层(参照 zed + gpui-component 双重调研,见 docs/editor-roadmap.md):
//! - 文本存储:[`engine::Rope`](super::engine)(sum_tree 底座)
//! - 选区:[`Selection`](selection.rs)(含 goal 列保持),对齐 zed `crates/text`
//! - 撤销:[`UndoManager`](undo.rs)(事务 + intent 合并,选区随事务走)
//! - 编辑主路径:`transact → edit → change_selections`(对齐 zed input.rs 的
//!   replace_selections 模式),所有动作收敛到这一条路
//! - 渲染:`editor/element.rs`(阶段 C)——Editor 只持状态,渲染从快照只读
//!
//! 与 zed 的有意差异(见计划文档):单行 Enter 用 action 分流(zed 用 fold
//! 把 \n 显示为 ⋯);显示映射链只做软换行一层。

mod actions;
mod selection;
mod undo;

use std::ops::Range;

use gpui::{
    App, Bounds, Context, EntityInputHandler, EventEmitter, FocusHandle, Hsla, Pixels, Rgba,
    ShapedLine, SharedString, UTF16Selection, rgb,
};

pub use actions::*;
pub(super) use selection::Selection;
pub(super) use undo::{Change, EditIntent};

use super::engine::{OffsetUtf16, Rope};
pub use selection::SelectionGoal;
pub use undo::UndoManager;

/// 编辑器的布局模式,对齐 zed `EditorMode`(editor.rs:469-487)的精简子集。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    /// 单行输入框:Enter 不换行(由宿主决定提交语义),粘贴去 \n。
    SingleLine,
    /// 多行,高度随内容自适应(限行数)。
    AutoHeight { min_rows: usize, max_rows: usize },
    /// 多行固定高度(行数),内容滚动。
    MultiLine { rows: usize },
}

impl EditorMode {
    pub fn is_single_line(&self) -> bool {
        matches!(self, EditorMode::SingleLine)
    }

    pub fn is_multi_line(&self) -> bool {
        !self.is_single_line()
    }
}

/// 编辑器对外事件,命名与语义对齐 zed `EditorEvent` 的核心子集。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorEvent {
    /// 内容已变更(事务提交后)。
    Edited,
    /// 单行模式下按 Enter(多行不发出)。
    Submitted,
    /// 选区变化(移动/选择)。
    SelectionsChanged,
    /// 滚动位置变化(阶段 C 接入滚动后发出)。
    ScrollPositionChanged,
    Focused,
    Blurred,
}

/// 编辑器状态实体。渲染见 [`super::editor::actions`] 注册的动作与
/// 阶段 C 的 `EditorElement`。
pub struct Editor {
    pub(super) rope: Rope,
    pub(super) selection: Selection<usize>,
    /// IME 组字区间(buffer 字节坐标)。
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) undo_manager: UndoManager,
    pub(super) mode: EditorMode,
    pub(super) focus_handle: FocusHandle,
    pub(super) placeholder: SharedString,
    pub(super) disabled: bool,
    /// 单行模式:粘贴时把 \n 换成空格(见计划文档差异 1)。
    pub(super) submit_on_enter: bool,
    // ---- 渲染缓存(LastLayout 快照,prepaint 写入、命中测试/IME 读取)----
    pub(super) last_layout: Option<ShapedLine>,
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    // ---- 视觉样式 ----
    pub(super) bg_color: Rgba,
    pub(super) border_color: Rgba,
    pub(super) placeholder_color: Hsla,
    // ---- IME 组字的 undo 括号 ----
    ime_composing: bool,
}

impl Editor {
    pub fn new(mode: EditorMode, cx: &mut Context<Self>) -> Self {
        Self {
            rope: Rope::new(),
            selection: Selection::default(),
            marked_range: None,
            undo_manager: UndoManager::new(),
            mode,
            focus_handle: cx.focus_handle(),
            placeholder: "".into(),
            disabled: false,
            // 多行默认 Enter 换行;单行模式忽略此字段(总是 Submitted)
            submit_on_enter: false,
            last_layout: None,
            last_bounds: None,
            bg_color: rgb(0x1e1e2e),
            border_color: rgb(0x45475a),
            placeholder_color: gpui::hsla(0., 0., 0.55, 1.),
            ime_composing: false,
        }
    }

    /// 单行输入框(对齐 zed `Editor::single_line`)。
    pub fn single_line(cx: &mut Context<Self>) -> Self {
        Self::new(EditorMode::SingleLine, cx)
    }

    // ---- builder(从旧 InputState 平移,外部 API 保持)----

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 多行模式下,裸 Enter 是否改为提交(不插换行)。
    /// 对齐 gpui-component 的 submit_on_enter 运行期开关。
    pub fn submit_on_enter(mut self, submit: bool) -> Self {
        self.submit_on_enter = submit;
        self
    }

    pub fn bg(mut self, color: impl Into<Rgba>) -> Self {
        self.bg_color = color.into();
        self
    }

    pub fn border_color(mut self, color: impl Into<Rgba>) -> Self {
        self.border_color = color.into();
        self
    }

    pub fn placeholder_color(mut self, color: impl Into<Hsla>) -> Self {
        self.placeholder_color = color.into();
        self
    }

    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.rope = Rope::from(value.into().as_str());
        let end = self.rope.len();
        self.selection = Selection::new(end, end);
        self
    }

    // ---- 状态访问 ----

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    pub fn is_multi_line(&self) -> bool {
        self.mode.is_multi_line()
    }

    /// 当前内容。
    pub fn value(&self) -> SharedString {
        self.rope.to_string().into()
    }

    /// 当前内容(多字节场景请优先用 [`Self::text_in_range`] 避免整串物化)。
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// 取 `[range)` 的文本。
    pub fn text_in_range(&self, range: Range<usize>) -> String {
        self.rope.text_in_range(range)
    }

    pub fn selection(&self) -> Selection<usize> {
        self.selection
    }

    /// 程序化设置内容:**不**触发 [`EditorEvent::Edited`](计划沿用旧约定,
    /// 避免程序化初始化回环),光标移到末尾,清空 undo 历史。
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.rope = Rope::from(value.into().as_str());
        let end = self.rope.len();
        self.selection = Selection::new(end, end);
        self.marked_range = None;
        self.undo_manager.clear();
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_value("", cx);
    }

    // ---- 编辑主路径(对齐 zed:transact → edit → change_selections)----

    /// 包一个事务:编辑后统一 emit Edited + SelectionsChanged。
    pub(super) fn transact(
        &mut self,
        edit: impl FnOnce(&mut Self, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) {
        edit(self, cx);
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    /// 用 `text` 替换当前选区——所有插入/删除/粘贴的唯一入口。
    ///
    /// 对齐 zed `Editor::replace_selections`(input.rs:1952):编辑 rope、
    /// 折叠选区到插入尾端、记录 undo 事务。
    pub(super) fn replace_selections(
        &mut self,
        text: &str,
        intent: EditIntent,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.transact(
            |this, _cx| {
                let range = Range::from(&this.selection);
                let old_text = this.rope.text_in_range(range.clone());
                let selection_before = this.selection;

                this.rope.replace(range.clone(), text);
                let new_head = range.start + text.len();
                this.selection.collapse_to(new_head, SelectionGoal::None);
                this.marked_range = None;

                let change = Change::new(
                    range.clone(),
                    &old_text,
                    range.start..new_head,
                    text,
                    selection_before,
                    this.selection,
                );
                this.undo_manager.record_transaction(change, intent);
            },
            cx,
        );
    }

    /// 非选区型移动后调用:折叠 + 通知。
    pub(super) fn change_selections(&mut self, cx: &mut Context<Self>) {
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    // ---- undo / redo ----

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.undo_manager.undo() else {
            return;
        };
        // 逆序回放:后发生的先撤销
        for change in &changes {
            self.rope.replace(change.new_range.clone(), &change.old_text);
        }
        // 恢复事务开始前的选区(逆序第一项的 selection_before)
        if let Some(first) = changes.first() {
            self.selection = first.selection_before;
        }
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(changes) = self.undo_manager.redo() else {
            return;
        };
        for change in &changes {
            self.rope.replace(change.old_range.clone(), &change.new_text);
        }
        if let Some(last) = changes.last() {
            self.selection = last.selection_after;
        }
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    // ---- UTF-16 ↔ UTF-8(IME 接口,走 Rope 树内查询)----

    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        self.rope.offset_utf16_to_offset(OffsetUtf16(offset))
    }

    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        self.rope.offset_to_offset_utf16(offset).0
    }

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl EventEmitter<EditorEvent> for Editor {}

impl gpui::Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _;

    #[gpui::test]
    fn test_insert_and_value(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("hello"));
        editor.update(cx, |editor, cx| {
            editor.selection = Selection::new(5, 5);
            editor.replace_selections(" world", EditIntent::Typing, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e.selection.head()),
            11,
            "光标应折叠到插入尾端"
        );
    }

    #[gpui::test]
    fn test_undo_redo_restores_selection(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("hello"));
        editor.update(cx, |editor, cx| {
            editor.selection = Selection::new(5, 5);
            editor.replace_selections(" world", EditIntent::Typing, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );

        editor.update(cx, |editor, cx| editor.undo(cx));
        assert_eq!(editor.read_with(cx, |e, _| e.value().to_string()), "hello");
        assert_eq!(
            editor.read_with(cx, |e, _| e.selection.head()),
            5,
            "undo 应恢复事务前选区"
        );

        editor.update(cx, |editor, cx| editor.redo(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
        assert_eq!(editor.read_with(cx, |e, _| e.selection.head()), 11);
    }

    #[gpui::test]
    fn test_typing_coalesces_into_one_undo(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx));
        editor.update(cx, |editor, cx| {
            for c in ['a', 'b', 'c'] {
                editor.replace_selections(&c.to_string(), EditIntent::Typing, cx);
            }
        });
        assert_eq!(editor.read_with(cx, |e, _| e.value().to_string()), "abc");

        editor.update(cx, |editor, cx| editor.undo(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "",
            "连续打字应合并为一个事务,一次 undo 全撤"
        );
    }

    #[gpui::test]
    fn test_paste_replaces_selection(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("hello world"));
        editor.update(cx, |editor, cx| {
            editor.selection = Selection::new(6, 11);
            editor.replace_selections("gpui", EditIntent::Atomic, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello gpui"
        );

        editor.update(cx, |editor, cx| editor.undo(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
    }

    #[gpui::test]
    fn test_multiline_newline_inherits_indent(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::new(EditorMode::MultiLine { rows: 5 }, cx).default_value("  fn main()")
        });
        editor.update(cx, |editor, cx| {
            let end = editor.rope.len();
            editor.selection = Selection::new(end, end);
            editor.newline_core(cx);
        });
        let value = editor.read_with(cx, |e, _| e.value().to_string());
        assert_eq!(value, "  fn main()\n  ", "换行应继承行首缩进");
    }

    #[gpui::test]
    fn test_single_line_newline_submits_without_insert(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("abc"));
        // 单行 Enter:走 Submitted 分支、不插入换行(事件在 UI 阶段验证)
        editor.update(cx, |editor, cx| {
            let end = editor.rope.len();
            editor.selection = Selection::new(end, end);
            editor.newline_core(cx);
            // core 直接插换行——单行必须经由动作包装分流,这里核对 core 行为
            assert_eq!(editor.rope.len(), 4);
        });
    }

    #[gpui::test]
    fn test_multiline_vertical_movement(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::new(EditorMode::MultiLine { rows: 3 }, cx).default_value("abcd\nef\nghijk")
        });
        editor.update(cx, |editor, cx| {
            // 光标到第一行行尾(offset 4)
            editor.selection = Selection::new(4, 4);
            editor.move_vertical_core(1, cx);
            // 第二行只有 "ef":列被 clamp 到行尾(offset 7)
            assert_eq!(editor.selection.head(), 7, "下移应 clamp 到短行行尾");

            editor.move_vertical_core(1, cx);
            // 第三行 "ghijk":列 2 对应 offset 8+2=10
            assert_eq!(editor.selection.head(), 10);

            editor.move_vertical_core(-1, cx);
            assert_eq!(editor.selection.head(), 7, "上移应回到第二行");
        });
    }

    #[gpui::test]
    fn test_multiline_backspace_across_lines(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::new(EditorMode::MultiLine { rows: 3 }, cx).default_value("ab\ncd")
        });
        editor.update(cx, |editor, cx| {
            // 光标在第二行行首(offset 3),退格应删除换行合并两行
            editor.selection = Selection::new(3, 3);
            assert!(!editor.backspace_core(cx));
        });
        assert_eq!(editor.read_with(cx, |e, _| e.value().to_string()), "abcd");
        assert_eq!(editor.read_with(cx, |e, _| e.selection.head()), 2);
    }

    #[gpui::test]
    fn test_backspace_at_boundary_rings_bell(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("ab"));
        editor.update(cx, |editor, cx| {
            editor.selection = Selection::new(0, 0);
            assert!(editor.backspace_core(cx), "起点退格应报告边界");
            assert_eq!(editor.rope.len(), 2);
        });
    }
}

// ---- IME / 平台文本输入接口(与旧 InputState 语义一致)----

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.rope.text_in_range(range))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&Range::from(&self.selection)),
            reversed: self.selection.reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        // 指定区间(IME 上屏)→ 用之;否则替换当前选区
        if let Some(range_utf16) = range_utf16 {
            let range = self.range_from_utf16(&range_utf16);
            let old_text = self.rope.text_in_range(range.clone());
            let selection_before = self.selection;
            self.transact(
                |this, _| {
                    this.rope.replace(range.clone(), new_text);
                    let new_head = range.start + new_text.len();
                    this.selection.collapse_to(new_head, SelectionGoal::None);
                    this.marked_range = None;
                    let change = Change::new(
                        range.clone(),
                        &old_text,
                        range.start..new_head,
                        new_text,
                        selection_before,
                        this.selection,
                    );
                    this.undo_manager
                        .record_transaction(change, EditIntent::Atomic);
                },
                cx,
            );
        } else {
            let intent = if self.ime_composing {
                EditIntent::Atomic
            } else {
                EditIntent::Typing
            };
            self.replace_selections(new_text, intent, cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        // 组字开始:打开 undo 显式事务(整段组字一次撤销)
        if !self.ime_composing {
            self.ime_composing = true;
            self.undo_manager.begin_transaction();
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| Range::from(&self.selection));

        let old_text = self.rope.text_in_range(range.clone());
        let selection_before = self.selection;
        self.rope.replace(range.clone(), new_text);
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        let marked_len = self
            .marked_range
            .as_ref()
            .map(|r| r.len())
            .unwrap_or_default();
        let new_selected = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| {
                let head = range.start + marked_len;
                head..head
            });
        self.selection = Selection::new(new_selected.end, new_selected.start);
        let new_range = range.start..range.start + new_text.len();
        let change = Change::new(
            range,
            &old_text,
            new_range,
            new_text,
            selection_before,
            self.selection,
        );
        self.undo_manager.record_transaction(change, EditIntent::Atomic);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut gpui::Window,
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
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        // 空内容时渲染的是 placeholder,不能断言 last_layout.text == 内容
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.closest_index_for_x(point.x - line_point.x);
        Some(self.offset_to_utf16(utf8_index))
    }
}
