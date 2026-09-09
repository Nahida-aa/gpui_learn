//! `Editor` —— 多行文本的「引擎」实体：拥有光标、闪烁、焦点、键盘与文本渲染。
//!
//! 文本本身存在一个共享的 `Entity<String>` 里，方便内外读写。
//! 本模块**平台无关**：既不依赖 Android，也不依赖桌面，IME/键盘输入都通过
//! gpui 标准的 `EntityInputHandler` + `window.handle_input` 接入，因此在桌面
//! 端也能直接用硬键盘（含回车换行）测试多行逻辑。
//!
//! （移植自 zed crates/gpui/examples/view_example/example_editor.rs）

use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, Context, Element, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, IntoElement, LayoutId, PaintQuad, Pixels, Point, ShapedLine,
    SharedString, Subscription, Task, TextRun, UTF16Selection, Window, actions, fill, hsla,
    point, prelude::*, px, relative, size,
};
use unicode_segmentation::*;

// ── actions（与 zed view_example_main.rs 一致）──────────────────────────────
actions!(
    view_example,
    [Backspace, Delete, Left, Right, Home, End, Enter, Quit, Copy, Cut, Paste, SelectAll]
);

pub struct Editor {
    pub value: Entity<String>,
    pub focus_handle: FocusHandle,
    /// 选区（UTF-8 字节范围）。空区间（start==end）即「折叠光标」。
    /// 这是编辑器唯一的位置真相来源，替代旧的标量 `cursor`。
    pub selection: Range<usize>,
    /// 选区是否反向：true 表示活动端（光标）在 `selection.start`，
    /// false 表示活动端在 `selection.end`。方向键/拖拽据此扩展或移动。
    pub selection_reversed: bool,
    /// 拖拽选区进行中（鼠标/触摸按住并移动）。由 TextArea 的鼠标事件维护。
    pub is_selecting: bool,
    pub cursor_visible: bool,
    _blink_task: Task<()>,
    _subscriptions: Vec<Subscription>,
    /// 诊断用：把每次 IME 实际送进来的文本记到这里，方便在屏幕上看到
    /// 软键盘到底有没有把回车当 \n 提交（区别于硬键盘的按键事件）。
    debug_log: Option<Entity<String>>,
    /// 最近一次 paint 的几何信息，供点击定位光标用。
    /// `prepaint` 阶段把文本框边界、行高、每行起始字节偏移、以及各行
    /// `ShapedLine` 写进来；点击时（桌面/移动端都走 TextArea::on_mouse_down）
    /// 用它们把点击坐标换算成字符字节偏移。
    last_bounds: Option<Bounds<Pixels>>,
    last_line_height: Pixels,
    last_line_starts: Vec<usize>,
    last_lines: Vec<ShapedLine>,
    /// 最近一次 paint 的文本总长度（字节），供 `selection_bounds` 计算末行
    /// 选区结束位置（prepaint 的 `line_starts` 不包含末尾项）。
    last_content_len: usize,
}

impl Editor {
    /// An editor that owns its own string internally, seeded with `text`.
    pub fn new(text: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let value = cx.new(|_| text.into());
        Self::over(value, window, cx)
    }

    /// An editor over a string *you* own, so the value is shared in and out.
    pub fn over(value: Entity<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::over_with_log(value, None, window, cx)
    }

    /// 同 `over`，但额外把 IME 收到的文本记到 `debug_log`（仅用于 07 诊断）。
    pub fn over_with_log(
        value: Entity<String>,
        debug_log: Option<Entity<String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let id = cx.entity().entity_id();

        let focus_sub = cx.on_focus(&focus_handle, window, {
            let id = id;
            move |this, _window, cx| {
                log::info!("[editor] FOCUS id={:?}", id);
                this.start_blink(cx);
            }
        });
        let blur_sub = cx.on_blur(&focus_handle, window, {
            let id = id;
            move |this, _window, cx| {
                log::info!("[editor] BLUR id={:?}", id);
                this.stop_blink(cx);
            }
        });

        // 外部写 value 时把选区夹回字符边界，并通知重渲染。
        let value_sub = cx.observe(&value, |this, value, cx| {
            let content = value.read(cx);
            let len = content.len();
            let mut start = this.selection.start.min(len);
            let mut end = this.selection.end.min(len);
            while start > 0 && !content.is_char_boundary(start) {
                start -= 1;
            }
            while end > 0 && !content.is_char_boundary(end) {
                end -= 1;
            }
            this.selection = start..end;
            cx.notify();
        });

        Self {
            value,
            focus_handle,
            selection: 0..0,
            selection_reversed: false,
            is_selecting: false,
            cursor_visible: false,
            _blink_task: Task::ready(()),
            _subscriptions: vec![focus_sub, blur_sub, value_sub],
            debug_log,
            last_bounds: None,
            last_line_height: px(0.),
            last_line_starts: Vec::new(),
            last_lines: Vec::new(),
            last_content_len: 0,
        }
    }

    /// The current text. Read this from anywhere to get the value out.
    pub fn text(&self, cx: &App) -> String {
        self.value.read(cx).clone()
    }

    /// 活动端（光标）字节偏移。
    pub fn cursor(&self) -> usize {
        if self.selection_reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    /// 把选区收拢成 `offset` 处的折叠光标（无选区）。
    pub fn collapse_to(&mut self, offset: usize) {
        self.selection = offset..offset;
        self.selection_reversed = false;
    }

    /// 移动活动端到 `offset`，保留锚点（另一端）不动 —— 用于 Shift+方向键 / 拖拽扩展。
    pub fn move_active_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let anchor = if self.selection_reversed {
            self.selection.end
        } else {
            self.selection.start
        };
        self.selection_reversed = offset < anchor;
        self.selection = anchor.min(offset)..anchor.max(offset);
        self.reset_blink(cx);
        cx.notify();
    }

    /// 把活动端移动到 `offset` 并收拢（无 Shift 的方向键 / 单击定位）。
    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let content = self.text(cx);
        let mut cursor = offset.min(content.len());
        while cursor > 0 && !content.is_char_boundary(cursor) {
            cursor -= 1;
        }
        if cursor != self.cursor() {
            log::info!(
                "[editor] move_to id={:?} from={} to={}",
                cx.entity().entity_id(),
                self.cursor(),
                cursor
            );
            self.collapse_to(cursor);
            self.reset_blink(cx);
            cx.notify();
        }
    }

    /// 选区是否为空（折叠光标）。
    pub fn is_empty_selection(&self) -> bool {
        self.selection.is_empty()
    }

    /// 选区在编辑器内的像素包围盒（window-local，相对编辑器文本元素）。
    ///
    /// 基于 `prepaint` 写回的几何缓存（`last_bounds` / `last_line_starts` /
    /// `last_lines`）计算，与选区高亮用同一套逐行求交逻辑，因此坐标空间
    /// 与选区背景完全一致。供「选中文字上方浮动工具条」（方式 A）定位使用。
    ///
    /// 折叠光标或尚未 paint 过时返回 `None`。
    pub fn selection_bounds(&self) -> Option<Bounds<Pixels>> {
        if self.selection.is_empty() {
            return None;
        }
        let bounds = self.last_bounds?;
        let line_height = self.last_line_height;
        let line_starts = &self.last_line_starts;
        let lines = &self.last_lines;
        // prepaint 写回的 `line_starts` 长度 = 行数（末尾不另存 content.len()），
        // 末行结束位置用 `last_content_len`。
        if lines.is_empty() || line_starts.len() != lines.len() {
            return None;
        }
        let content_len = self.last_content_len;
        let mut min_x = Pixels::MAX;
        let mut min_y = Pixels::MAX;
        let mut max_x = Pixels::MIN;
        let mut max_y = Pixels::MIN;
        let mut any = false;
        for (line_idx, line) in lines.iter().enumerate() {
            let line_start = line_starts[line_idx];
            let line_end = if line_idx + 1 < line_starts.len() {
                line_starts[line_idx + 1] - '\n'.len_utf8()
            } else {
                content_len
            };
            let seg_start = self.selection.start.max(line_start).min(line_end);
            let seg_end = self.selection.end.max(line_start).min(line_end);
            if seg_end <= seg_start {
                continue;
            }
            let x_start = line.x_for_index(seg_start - line_start);
            let x_end = line.x_for_index(seg_end - line_start);
            let y = line_height * line_idx as f32;
            min_x = min_x.min(bounds.left() + x_start);
            max_x = max_x.max(bounds.left() + x_end);
            min_y = min_y.min(bounds.top() + y);
            max_y = max_y.max(bounds.top() + y + line_height);
            any = true;
        }
        if !any {
            return None;
        }
        Some(Bounds::new(
            point(min_x, min_y),
            size(max_x - min_x, max_y - min_y),
        ))
    }

    /// 编辑器文本元素最近一次 prepaint 的窗口原点，供「选区上方浮动工具条」
    /// （方式 A）把选区窗口坐标换算成相对 TextArea 的坐标。
    pub fn last_bounds_origin(&self) -> Option<Point<Pixels>> {
        self.last_bounds.map(|b| b.origin)
    }

    /// 选中光标所在的「词」（以空白/换行分隔），并进入选择态。
    /// 用于移动端长按选词。词边界夹到字符边界。
    pub fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let content = self.text(cx);
        if content.is_empty() {
            return;
        }
        let offset = offset.min(content.len()).max(0);
        let is_word_char = |c: char| !c.is_whitespace() && c != '\n';
        // 起点：向左找到第一个非词字符之后。
        let mut start = offset;
        while start > 0 {
            let prev = previous_boundary(&content, start);
            if !is_word_char(content[prev..offset].chars().next().unwrap_or(' ')) {
                break;
            }
            start = prev;
        }
        // 终点：向右找到第一个非词字符之前。
        let mut end = offset;
        while end < content.len() {
            let next = next_boundary(&content, end);
            if !is_word_char(content[end..next].chars().next().unwrap_or(' ')) {
                break;
            }
            end = next;
        }
        while start > 0 && !content.is_char_boundary(start) {
            start -= 1;
        }
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        let (start, end) = if start < end { (start, end) } else { (offset, offset) };
        log::info!(
            "[editor] select_word_at id={:?} offset={} word=[{}..{}] {:?}",
            cx.entity().entity_id(),
            offset,
            start,
            end,
            &content[start..end]
        );
        self.selection = start..end;
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify();
    }

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_visible = true;
        self._blink_task = Self::spawn_blink_task(cx);
    }

    fn stop_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_visible = false;
        self._blink_task = Task::ready(());
        cx.notify();
    }

    fn spawn_blink_task(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let result = this.update(cx, |editor, cx| {
                    editor.cursor_visible = !editor.cursor_visible;
                    cx.notify();
                });
                if result.is_err() {
                    break;
                }
            }
        })
    }

    fn reset_blink(&mut self, cx: &mut Context<Self>) {
        self.cursor_visible = true;
        self._blink_task = Self::spawn_blink_task(cx);
    }

    /// 把点击坐标换算成内容里的字符字节偏移。
    /// 用 `prepaint` 阶段存下来的行几何信息（边界/行高/每行起始偏移/各 `ShapedLine`）。
    /// 多行：先按 y 落在第几行，再在该行的 `ShapedLine` 上按 x 找最近字符边界。
    pub fn index_for_point(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), lines) = (self.last_bounds.as_ref(), &self.last_lines) else {
            log::info!("[editor] index_for_point: NO geometry cached, pos={:?}", position);
            return 0;
        };
        if lines.is_empty() {
            return 0;
        }
        let line_height = self.last_line_height;
        if line_height == px(0.) {
            return 0;
        }

        // 纵向：落在第几行（点在框外则夹到首/尾行）。
        let rel_y = position.y - bounds.top();
        let mut line_idx = (rel_y / line_height).floor() as usize;
        if line_idx >= lines.len() {
            line_idx = lines.len() - 1;
        }
        // 横向：与行左边缘的相对 x（点在中线左侧也按 0 处理）。
        let x = position.x - bounds.left();

        let idx_in_line = lines[line_idx].closest_index_for_x(x);
        let line_start = self.last_line_starts.get(line_idx).copied().unwrap_or(0);
        let result = line_start + idx_in_line;
        log::info!(
            "[editor] index_for_point: pos={:?} bounds.top={} bounds.left={} line_h={} line_idx={} x={} idx_in_line={} line_start={} -> {}",
            position, bounds.top(), bounds.left(), line_height, line_idx, x, idx_in_line, line_start, result
        );
        result
    }

    pub fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.text(cx);
        let target = if self.cursor() > 0 {
            previous_boundary(&content, self.cursor())
        } else {
            0
        };
        if window.modifiers().shift {
            self.move_active_to(target, cx);
        } else {
            self.move_to(target, cx);
        }
    }

    pub fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.text(cx);
        let target = if self.cursor() < content.len() {
            next_boundary(&content, self.cursor())
        } else {
            content.len()
        };
        if window.modifiers().shift {
            self.move_active_to(target, cx);
        } else {
            self.move_to(target, cx);
        }
    }

    pub fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        if window.modifiers().shift {
            self.move_active_to(0, cx);
        } else {
            self.move_to(0, cx);
        }
    }

    pub fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.text(cx).len();
        if window.modifiers().shift {
            self.move_active_to(len, cx);
        } else {
            self.move_to(len, cx);
        }
    }

    pub fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        // 有选区：整段删掉，光标落到选区起点。
        if !self.is_empty_selection() {
            let start = self.selection.start;
            self.value.update(cx, |s, cx| {
                s.drain(self.selection.clone());
                cx.notify();
            });
            self.collapse_to(start);
            self.reset_blink(cx);
            cx.notify();
            return;
        }
        let content = self.text(cx);
        if self.cursor() > 0 {
            let prev = previous_boundary(&content, self.cursor());
            let cursor = self.cursor();
            self.value.update(cx, |s, cx| {
                s.drain(prev..cursor);
                cx.notify();
            });
            self.collapse_to(prev);
        }
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        // 有选区：整段删掉，光标落到选区起点。
        if !self.is_empty_selection() {
            let start = self.selection.start;
            self.value.update(cx, |s, cx| {
                s.drain(self.selection.clone());
                cx.notify();
            });
            self.collapse_to(start);
            self.reset_blink(cx);
            cx.notify();
            return;
        }
        let content = self.text(cx);
        if self.cursor() < content.len() {
            let next = next_boundary(&content, self.cursor());
            let cursor = self.cursor();
            self.value.update(cx, |s, cx| {
                s.drain(cursor..next);
                cx.notify();
            });
        }
        self.reset_blink(cx);
        cx.notify();
    }

    /// 插入换行：软键盘/硬键盘的 Enter 都走这里，实现多行换行。
    /// 若当前有选区，先删掉选区再插入换行（与主流编辑器一致）。
    pub fn insert_newline(&mut self, cx: &mut Context<Self>) {
        let cursor = if !self.is_empty_selection() {
            let start = self.selection.start;
            self.value.update(cx, |s, cx| {
                s.drain(self.selection.clone());
                cx.notify();
            });
            self.collapse_to(start);
            start
        } else {
            self.cursor()
        };
        self.value.update(cx, |s, cx| {
            s.insert(cursor, '\n');
            cx.notify();
        });
        self.collapse_to(cursor + 1);
        // 诊断：确认 Enter action 已触发、\n 已插入（与 IME 提交路径分开记录）。
        log::info!(
            "[editor] insert_newline id={:?} cursor={} text={:?}",
            cx.entity().entity_id(),
            cursor,
            self.text(cx)
        );
        // 同时在 UI 诊断框里显示回车（⏎），与 IME 字符走同一 debug_log，
        // 这样屏幕上也能看到回车是否触发，无需看终端日志。
        if let Some(log) = self.debug_log.as_ref() {
            let _ = log.update(cx, |s, cx| {
                if s.len() > 200 {
                    s.clear();
                }
                s.push_str("IME→\"⏎\" ");
                cx.notify();
            });
        }
        self.reset_blink(cx);
        cx.notify();
    }

    /// 删除整段选区（有选区时）；否则无操作。供 IME 替换前清理选区用。
    pub fn delete_selection_if_any(&mut self, cx: &mut Context<Self>) -> bool {
        if self.is_empty_selection() {
            return false;
        }
        let start = self.selection.start;
        self.value.update(cx, |s, cx| {
            s.drain(self.selection.clone());
            cx.notify();
        });
        self.collapse_to(start);
        self.reset_blink(cx);
        cx.notify();
        true
    }

    pub fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_empty_selection() {
            return;
        }
        let content = self.text(cx);
        let text = content[self.selection.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    pub fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_empty_selection() {
            return;
        }
        let content = self.text(cx);
        let text = content[self.selection.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.delete_selection_if_any(cx);
    }

    pub fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                // 有选区先删选区，再插入（与主流编辑器一致）。
                self.delete_selection_if_any(cx);
                let cursor = self.cursor();
                self.value.update(cx, |s, cx| {
                    s.insert_str(cursor, &text);
                    cx.notify();
                });
                self.collapse_to(cursor + text.len());
                self.reset_blink(cx);
                cx.notify();
            }
        }
    }

    /// 全选：把选区扩展到整段文本。供系统 ActionMode「全选」与桌面 Ctrl/Cmd+A 复用。
    pub fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        let len = self.text(cx).len();
        self.selection = 0..len;
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify();
    }
}

fn previous_boundary(content: &str, offset: usize) -> usize {
    content
        .grapheme_indices(true)
        .rev()
        .find_map(|(idx, _)| (idx < offset).then_some(idx))
        .unwrap_or(0)
}

fn next_boundary(content: &str, offset: usize) -> usize {
    content
        .grapheme_indices(true)
        .find_map(|(idx, _)| (idx > offset).then_some(idx))
        .unwrap_or(content.len())
}

fn offset_from_utf16(content: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in content.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

fn offset_to_utf16(content: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in content.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

fn range_to_utf16(content: &str, range: &Range<usize>) -> Range<usize> {
    offset_to_utf16(content, range.start)..offset_to_utf16(content, range.end)
}

fn range_from_utf16(content: &str, range_utf16: &Range<usize>) -> Range<usize> {
    offset_from_utf16(content, range_utf16.start)..offset_from_utf16(content, range_utf16.end)
}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let content = self.text(cx);
        let range = range_from_utf16(&content, &range_utf16);
        actual_range.replace(range_to_utf16(&content, &range));
        Some(content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let content = self.text(cx);
        let utf16_range = range_to_utf16(&content, &self.selection);
        Some(UTF16Selection {
            range: utf16_range,
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let content = self.text(cx);
        // IME 没给范围时：若有选区就替换选区，否则替换光标处。
        let range = range_utf16
            .as_ref()
            .map(|r| range_from_utf16(&content, r))
            .unwrap_or_else(|| {
                if self.is_empty_selection() {
                    self.cursor()..self.cursor()
                } else {
                    self.selection.clone()
                }
            });

        log::info!(
            "[editor] replace_text_in_range id={:?} new_text={:?}",
            cx.entity().entity_id(),
            new_text
        );

        let new_content = content[..range.start].to_owned() + new_text + &content[range.end..];
        let new_cursor = range.start + new_text.len();
        // 替换后收拢成光标（IME 提交会清掉选区）。
        self.selection = new_cursor..new_cursor;
        self.selection_reversed = false;
        self.value.update(cx, |s, cx| {
            *s = new_content;
            cx.notify();
        });

        // 诊断：把 IME 实际送进来的文本记下来（\n 显示为 ⏎ 便于肉眼辨认）。
        if let Some(log) = self.debug_log.as_ref() {
            let display: String = new_text
                .chars()
                .map(|c| if c == '\n' { '⏎' } else { c })
                .collect();
            let _ = log.update(cx, |s, cx| {
                if s.len() > 200 {
                    s.clear();
                }
                s.push_str(&format!("IME→{:?} ", display));
                cx.notify();
            });
        }

        self.reset_blink(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        // 用与点击一致的几何换算；返回 UTF-16 偏移（IME 要求）。
        let utf8 = self.index_for_point(point);
        let content = self.text(&*cx);
        Some(offset_to_utf16(&content, utf8))
    }
}

impl gpui::Render for Editor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Editor>) -> impl IntoElement {
        EditorText {
            editor: cx.entity(),
        }
    }
}

// ---------------------------------------------------------------------------
// EditorText —— 专用渲染器：逐行 shape 文本、逐行 paint、画光标。
// 多行实现的关键就在 split('\n') 逐行 shape_line，并按行号偏移 y。
// ---------------------------------------------------------------------------

struct EditorText {
    editor: Entity<Editor>,
}

struct EditorTextPrepaint {
    lines: Vec<ShapedLine>,
    cursor: Option<PaintQuad>,
    /// 选区高亮背景块（每行一段）。活动端光标单独用 `cursor` 画。
    selection_quads: Vec<PaintQuad>,
}

impl IntoElement for EditorText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorText {
    type RequestLayoutState = ();
    type PrepaintState = EditorTextPrepaint;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let editor = self.editor.read(cx);
        let content = editor.value.read(cx);
        let line_count = content.split('\n').count().max(1);
        let line_height = window.line_height();
        let mut style = gpui::Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (line_height * line_count as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let content = editor.value.read(cx).clone();
        let cursor_offset = editor.cursor();
        let selection = editor.selection.clone();
        let cursor_visible = editor.cursor_visible;
        let is_focused = editor.focus_handle.is_focused(window);

        let style = window.text_style();
        let text_color = style.color;
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();

        let is_placeholder = content.is_empty();

        let lines: Vec<ShapedLine> = if is_placeholder {
            let placeholder: SharedString = "Type here...".into();
            let run = TextRun {
                len: placeholder.len(),
                font: style.font(),
                color: hsla(0., 0., 0.5, 0.5),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            vec![window
                .text_system()
                .shape_line(placeholder, font_size, &[run], None)]
        } else {
            content
                .split('\n')
                .map(|line_str| {
                    let text: SharedString = SharedString::from(line_str.to_string());
                    let run = TextRun {
                        len: text.len(),
                        font: style.font(),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    window
                        .text_system()
                        .shape_line(text, font_size, &[run], None)
                })
                .collect()
        };

        let cursor = if is_focused && cursor_visible {
            let (cursor_line, offset_in_line) = cursor_line_and_offset(&content, cursor_offset);
            let cursor_line = cursor_line.min(lines.len().saturating_sub(1));
            let cursor_x = lines[cursor_line].x_for_index(offset_in_line);
            Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + cursor_x,
                        bounds.top() + line_height * cursor_line as f32,
                    ),
                    size(px(1.5), line_height),
                ),
                text_color,
            ))
        } else {
            None
        };

        // 每行起始字节偏移：按 \n 切分的各段在原始 content 里的起点。
        let line_starts: Vec<usize> = if is_placeholder {
            vec![0]
        } else {
            let mut starts = Vec::with_capacity(content.split('\n').count());
            let mut idx = 0;
            starts.push(0);
            for ch in content.chars() {
                if ch == '\n' {
                    idx += ch.len_utf8();
                    starts.push(idx);
                } else {
                    idx += ch.len_utf8();
                }
            }
            starts
        };

        // 选区高亮：把选区字节范围拆成「按行」的若干段，每段画一个背景块。
        let selection_quads: Vec<PaintQuad> = if is_focused && !selection.is_empty() {
            let sel_color = hsla(0.6, 0.4, 0.6, 0.3);
            let mut quads = Vec::new();
            for (line_idx, line) in lines.iter().enumerate() {
                let line_start = line_starts.get(line_idx).copied().unwrap_or(0);
                let line_end = if line_idx + 1 < line_starts.len() {
                    line_starts[line_idx + 1] - '\n'.len_utf8()
                } else {
                    content.len()
                };
                // 本行与选区交集。
                let seg_start = selection.start.max(line_start).min(line_end);
                let seg_end = selection.end.max(line_start).min(line_end);
                if seg_end <= seg_start {
                    continue;
                }
                let x_start = line.x_for_index(seg_start - line_start);
                let x_end = line.x_for_index(seg_end - line_start);
                quads.push(fill(
                    Bounds::new(
                        point(
                            bounds.left() + x_start,
                            bounds.top() + line_height * line_idx as f32,
                        ),
                        size(x_end - x_start, line_height),
                    ),
                    sel_color,
                ));
            }
            quads
        } else {
            Vec::new()
        };

        // 把本次 paint 的几何信息写回 Editor 实体，供点击定位光标用。
        self.editor.update(cx, |editor, _cx| {
            editor.last_bounds = Some(bounds);
            editor.last_line_height = line_height;
            editor.last_line_starts = line_starts.clone();
            editor.last_lines = lines.clone();
            editor.last_content_len = content.len();
        });

        EditorTextPrepaint {
            lines,
            cursor,
            selection_quads,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        // IME 接入：与 05 同源，经 gpui-android 的 nativeCommitText 桥接；
        // 桌面端则走 gpui 原生输入路径。两边都是标准 EntityInputHandler。
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        let line_height = window.line_height();
        // 先画选区高亮背景，再画文字，最后画光标（保证光标在文字之上）。
        for quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(quad);
        }
        for (i, line) in prepaint.lines.iter().enumerate() {
            let origin = point(bounds.left(), bounds.top() + line_height * i as f32);
            line.paint(origin, line_height, gpui::TextAlign::Left, None, window, cx)
                .unwrap();
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}

fn cursor_line_and_offset(content: &str, cursor: usize) -> (usize, usize) {
    let mut line_index = 0;
    let mut line_start = 0;
    for (i, ch) in content.char_indices() {
        if i >= cursor {
            break;
        }
        if ch == '\n' {
            line_index += 1;
            line_start = i + 1;
        }
    }
    (line_index, cursor - line_start)
}

/// 把标准编辑动作绑定到 Editor 实体上（Backspace/Delete/Left/Right/Home/End）。
pub fn standard_actions<E: InteractiveElement>(editor: Entity<Editor>) -> impl FnOnce(E) -> E {
    move |element| {
        element
            .on_action({
                let editor = editor.clone();
                move |a: &Left, window, cx| editor.update(cx, |e, cx| e.left(a, window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Right, window, cx| editor.update(cx, |e, cx| e.right(a, window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Home, window, cx| editor.update(cx, |e, cx| e.home(a, window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &End, window, cx| editor.update(cx, |e, cx| e.end(a, window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Backspace, window, cx| {
                    editor.update(cx, |e, cx| e.backspace(a, window, cx))
                }
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Delete, window, cx| editor.update(cx, |e, cx| e.delete(a, window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Copy, _window, cx| editor.update(cx, |e, cx| e.copy(a, _window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Cut, _window, cx| editor.update(cx, |e, cx| e.cut(a, _window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &Paste, _window, cx| editor.update(cx, |e, cx| e.paste(a, _window, cx))
            })
            .on_action({
                let editor = editor.clone();
                move |a: &SelectAll, _window, cx| {
                    editor.update(cx, |e, cx| e.select_all(a, _window, cx))
                }
            })
    }
}
