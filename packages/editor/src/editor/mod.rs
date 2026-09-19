//! Editor:多行编辑引擎,概念模型对齐 zed editor。
//!
//! 分层(参照 zed + gpui-component 双重调研,见 docs/editor-roadmap.md):
//! - 文本存储:[`engine::Rope`](crate::engine)(sum_tree 底座)
//! - 选区:[`Selection`](selection.rs)(含 goal 列保持),对齐 zed `crates/text`
//! - 撤销:[`UndoManager`](undo.rs)(事务 + intent 合并,选区随事务走)
//! - 编辑主路径:`transact → edit → change_selections`(对齐 zed input.rs 的
//!   replace_selections 模式),所有动作收敛到这一条路
//! - 渲染:`editor/element.rs`(阶段 C)——Editor 只持状态,渲染从快照只读
//!
//! 与 zed 的有意差异(见计划文档):单行 Enter 用 action 分流(zed 用 fold
//! 把 \n 显示为 ⋯);显示映射链只做软换行一层。

mod actions;
// 阶段 B 先落地 display_map/movement 纯函数库(带完整测试);
// Editor 动作切到 display 空间在阶段 C 渲染度量接入时完成。
#[allow(dead_code)]
mod display_map;
pub mod element;
#[allow(dead_code)]
mod movement;
mod selection;
mod undo;

use std::ops::Range;

use gpui::{
    div, prelude::*, px, App, Bounds, Context, CursorStyle, EntityInputHandler, EventEmitter,
    FocusHandle, Hsla, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Render, ScrollWheelEvent, ShapedLine, SharedString, UTF16Selection, Window,
};

pub use actions::*;
pub use display_map::{DisplayMap, DisplayPoint, DisplayRow};
pub use element::EditorElement;
pub(super) use selection::Selection;
pub(super) use undo::{Change, EditIntent};

use crate::engine::{OffsetUtf16, Rope};
use aa_gpui_kit_theme::ActiveTheme;
pub use selection::SelectionGoal;
pub use undo::UndoManager;

impl Editor {
    /// 单行模式下的旧 API 兼容事件(多行不发)。
    fn emit_input_change(&self, cx: &mut Context<Self>) {
        if self.mode.is_single_line() {
            cx.emit(InputEvent::Change(self.rope.to_string().into()));
        }
    }
}

/// 编辑器的 key_context 名,[`bind_editor_keys`] 与渲染时的
/// `.key_context(..)` 共用。
pub const EDITOR_KEY_CONTEXT: &str = "Editor";

/// 内容不足一屏时是否允许继续滚动,对齐 zed `ScrollBeyondLastLine`
/// (crates/settings_content/src/editor.rs)。
///
/// zed 默认为 `OnePage`:编辑区可以把最后一行滚到视口顶部,下方留空白——
/// 长文件里把当前行放在视口中上部更好读,这是编辑器的通行做法
/// (VS Code 的 scrollBeyondLastLine 同理)。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScrollBeyondLastLine {
    /// 不允许滚过最后一行:滚到底时最后一行在视口底部。
    Off,
    /// 允许滚到「最后一行贴视口顶」,下方留空白(zed 默认)。
    #[default]
    OnePage,
}

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

/// 单行输入框对外事件(旧 API 兼容,随引擎走:由引擎在 SingleLine 模式下
/// 自动发出;定义原先在 ui-gpui 的 facade,内核独立成包后随之搬入)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// 内容发生变化(程序化 `set_value` 不触发)。
    Change(SharedString),
    /// Enter 提交。
    Submit(SharedString),
}

impl gpui::EventEmitter<InputEvent> for Editor {}

/// 单行输入框的 key_context 名(引擎 Render 在 SingleLine 模式下使用;
/// ui-gpui 的 `bind_input_keys` 按 it 绑定单行键位)。
pub const INPUT_KEY_CONTEXT: &str = "ui-gpui-input";

/// 编辑器状态实体。渲染见 `actions` 模块注册的动作与
/// 阶段 C 的 `EditorElement`。
pub struct Editor {
    pub(super) rope: Rope,
    /// 光标组(至少一个)。
    ///
    /// 规范序:按起点升序、互不重叠/相交——对齐 zed `SelectionsSet`
    /// (crates/editor/src/selection.rs)的规则,相交的选区会被合并掉。
    /// 与 zed 的差异:zed 用 Anchor + id 记住插入顺序(newest/oldest),
    /// 这里用字节偏移,排序即身份,增删/随编辑平移由编辑主路径负责。
    ///
    /// 组内**最后一个**(视觉最下方)是「主光标」:[`Editor::selection`]、
    /// IME 上屏位置、自动滚动跟随都以它为准;单光标时就是唯一元素。
    pub(super) selections: Vec<Selection<usize>>,
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
    /// 单行模式的整行布局。
    pub(super) last_layout: Option<ShapedLine>,
    /// 单行模式的文本区域。
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    /// 软换行(buffer 行超出视口宽时折成多个视觉行);单行模式恒为 false。
    ///
    /// 对齐 zed 的 SoftWrap(默认按编辑器宽度折行)。关闭后长行不折,
    /// 只能靠水平滚动(尚未实现)。
    pub(super) soft_wrap: bool,
    /// 软换行映射:buffer 行 → 视觉行分段。渲染层在 prepaint 里更新。
    pub(super) display_map: DisplayMap,
    /// 多行模式:每个**可见**视觉行的排版结果(软换行后一行可能占多条)。
    pub(super) last_lines: Vec<ShapedLine>,
    /// 与 `last_lines` 对齐:每个可见视觉行覆盖的 buffer 字节区间。
    pub(super) last_line_ranges: Vec<Range<usize>>,
    /// `last_lines[0]` 对应的全局视觉行号(命中测试换算用)。
    pub(super) first_visible_display_row: u32,
    /// 文本版本号:每次编辑 +1,渲染层据此判断是否需要重新 wrap。
    pub(super) text_revision: u64,
    /// 渲染层缓存:已 wrap 的文本版本号(与 `text_revision` 不等即需重算)。
    pub(super) wrapped_revision: u64,
    /// 渲染层缓存:已 wrap 时使用的视口宽度。
    pub(super) wrapped_width: Pixels,
    /// 多行模式:整个内容区的 bounds(全高,滚动前的内容坐标)。
    pub(super) last_content_bounds: Option<Bounds<Pixels>>,
    /// 最近一帧的行高(命中测试用)。
    pub(super) last_line_height: Pixels,
    /// 多行模式:垂直滚动位置(内容坐标,向下滚增大)。
    ///
    /// 对齐 zed 的自绘滚动:editor 不用 div 的 overflow_scroll,而是
    /// prepaint 里算 `content_origin = frame.top - scroll_position` 自行平移,
    /// 滚轮事件在 `on_scroll_wheel` 里累加(见 zed editor.rs 的 ScrollManager,
    /// 这里是其单行精简版)。坐标完全自控,命中测试无歧义。
    pub(super) scroll_position: Pixels,
    /// 多行模式:视口高度(视口 = 滚动裁剪区域,prepaint 写入)。
    pub(super) viewport_height: Pixels,
    /// 内容不足一屏时是否允许继续滚(对齐 zed,默认 OnePage)。
    pub(super) scroll_beyond_last_line: ScrollBeyondLastLine,
    /// 下一帧 prepaint 时把光标滚入视口(编辑/移动选区后置位)。
    pub(super) needs_autoscroll: bool,
    /// 鼠标拖拽选区进行中。
    pub(super) is_selecting: bool,
    // ---- 视觉样式 ----
    pub(super) bg_color: Hsla,
    pub(super) border_color: Hsla,
    pub(super) placeholder_color: Hsla,
    // ---- IME 组字的 undo 括号 ----
    ime_composing: bool,
}

impl Editor {
    /// 兼容旧 `InputState::new(cx)` 的入口:单行输入框。
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::single_line(cx)
    }

    /// 按 mode 构造(原 `new`;改名以让位给上面的兼容入口)。
    pub fn with_mode(mode: EditorMode, cx: &mut Context<Self>) -> Self {
        Self {
            rope: Rope::new(),
            selections: vec![Selection::default()],
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
            soft_wrap: true,
            display_map: DisplayMap::default(),
            last_lines: Vec::new(),
            last_line_ranges: Vec::new(),
            first_visible_display_row: 0,
            text_revision: 0,
            wrapped_revision: u64::MAX,
            wrapped_width: Pixels::MAX,
            last_content_bounds: None,
            last_line_height: px(0.),
            scroll_position: px(0.),
            viewport_height: px(0.),
            scroll_beyond_last_line: ScrollBeyondLastLine::OnePage,
            needs_autoscroll: false,
            is_selecting: false,
            // 默认色取当前主题(对齐 zed:组件色一律来自 cx.theme())
            bg_color: cx.theme().colors().editor_background,
            border_color: cx.theme().colors().border,
            placeholder_color: cx.theme().colors().text_placeholder,
            ime_composing: false,
        }
    }

    /// 单行输入框(对齐 zed `Editor::single_line`)。
    pub fn single_line(cx: &mut Context<Self>) -> Self {
        Self::with_mode(EditorMode::SingleLine, cx)
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

    /// 软换行:buffer 行超出视口宽时折成多个视觉行(多行默认开)。
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = soft_wrap;
        self
    }

    /// 内容不足一屏时是否允许继续滚(默认 [`ScrollBeyondLastLine::OnePage`],
    /// 同 zed)。设为 `Off` 即普通滚动行为。
    pub fn scroll_beyond_last_line(mut self, mode: ScrollBeyondLastLine) -> Self {
        self.scroll_beyond_last_line = mode;
        self
    }

    /// 多行模式下,裸 Enter 是否改为提交(不插换行)。
    /// 对齐 gpui-component 的 submit_on_enter 运行期开关。
    pub fn submit_on_enter(mut self, submit: bool) -> Self {
        self.submit_on_enter = submit;
        self
    }

    pub fn bg(mut self, color: impl Into<Hsla>) -> Self {
        self.bg_color = color.into();
        self
    }

    pub fn border_color(mut self, color: impl Into<Hsla>) -> Self {
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
        self.selections = vec![Selection::new(end, end)];
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

    /// 全部光标(规范序:按起点升序、互不重叠)。
    pub fn selections(&self) -> &[Selection<usize>] {
        &self.selections
    }

    /// 光标数量。
    pub fn cursor_count(&self) -> usize {
        self.selections.len()
    }

    /// 主光标:组内最后一个(视觉最下方),单光标时就是唯一元素。
    ///
    /// 保留旧 API 名以免上层/Demo 改动——多光标场景请以
    /// [`Self::selections`] 为准。
    pub fn selection(&self) -> Selection<usize> {
        self.selections.last().copied().unwrap_or_default()
    }

    /// 折叠为单一光标。
    pub(crate) fn set_selection(&mut self, selection: Selection<usize>) {
        self.selections = vec![selection];
    }

    /// 主光标的可变借用(鼠标拖拽 / IME 只作用于主光标)。
    pub(crate) fn selection_mut(&mut self) -> &mut Selection<usize> {
        if self.selections.is_empty() {
            self.selections.push(Selection::default());
        }
        let last = self.selections.len() - 1;
        &mut self.selections[last]
    }

    /// 整组替换 + 规范化。空组回落到 0 处的单个光标。
    pub(crate) fn set_selections(&mut self, selections: Vec<Selection<usize>>) {
        self.selections = selections;
        self.normalize_selections();
    }

    /// 加入一个光标(相交/重合由 `normalize_selections` 合并为一个)。
    /// 键盘加光标(AddSelectionAbove/Below)走这里。
    pub(crate) fn push_selection(&mut self, selection: Selection<usize>) {
        self.selections.push(selection);
        self.normalize_selections();
    }

    /// 加入一个光标;与已有光标相交/重合则把那个光标撤掉(toggle 语义,
    /// 同 VS Code 的 alt+click),始终保持至少一个光标。
    pub(crate) fn add_selection(&mut self, selection: Selection<usize>) {
        let existing = self
            .selections
            .iter()
            .position(|other| Self::intersects(other, &selection));
        match existing {
            // 相交且还有别的剩余光标 → 撤掉那个光标(toggle)
            Some(index) if self.selections.len() > 1 => {
                self.selections.remove(index);
            }
            Some(_) => {}
            None => self.selections.push(selection),
        }
        self.normalize_selections();
    }

    /// 两个选区是否相交/重合(含端点相接,闭合区间语义)。
    fn intersects(a: &Selection<usize>, b: &Selection<usize>) -> bool {
        let (a_start, a_end) = (a.range().start, a.range().end);
        let (b_start, b_end) = (b.range().start, b.range().end);
        // 两个空光标在同一点也算重合(端点相接)
        a_start.max(b_start) <= a_end.min(b_end)
    }

    /// 规范化:丢弃空(不可达)项 → 按起点排序 → 合并相交者 → 至少留一个。
    pub(crate) fn normalize_selections(&mut self) {
        self.selections
            .sort_by_key(|selection| (selection.range().start, selection.range().end));
        let mut merged: Vec<Selection<usize>> = Vec::with_capacity(self.selections.len());
        for selection in self.selections.drain(..) {
            match merged.last_mut() {
                Some(previous) if Self::intersects(previous, &selection) => {
                    // 相交 → 取并集,保留原先的方向与 goal
                    let start = previous.range().start.min(selection.range().start);
                    let end = previous.range().end.max(selection.range().end);
                    previous.start = start;
                    previous.end = end;
                }
                _ => merged.push(selection),
            }
        }
        if merged.is_empty() {
            merged.push(Selection::default());
        }
        self.selections = merged;
    }

    /// 对所有光标套同一个变换:`collapse_to(f(this, selection))`。
    ///
    /// 水平移动会清掉 goal([`SelectionGoal::None`]);上下移动的 goal 处理
    /// 在 `move_vertical_core` 里单独做。
    pub(crate) fn collapse_heads_to(&mut self, f: impl Fn(&Self, Selection<usize>) -> usize) {
        for index in 0..self.selections.len() {
            let selection = self.selections[index];
            let target = f(self, selection);
            self.selections[index].collapse_to(target, SelectionGoal::None);
        }
        self.normalize_selections();
    }

    /// 同上,但用 `set_head`(Shift 扩展选区语义,保留 tail / 方向)。
    pub(crate) fn set_heads_to(&mut self, f: impl Fn(&Self, Selection<usize>) -> usize) {
        for index in 0..self.selections.len() {
            let selection = self.selections[index];
            let target = f(self, selection);
            self.selections[index].set_head(target, SelectionGoal::None);
        }
        self.normalize_selections();
    }

    /// 固定行数的多行文本域(内容超出时滚动)。
    ///
    /// 原是 ui-gpui 里 \`Textarea\` facade 的便捷构造;内核独立成包后随类型
    /// 住在这里(外部 crate 不能给类型别名写 inherent impl)。
    pub fn textarea(rows: usize, cx: &mut Context<Self>) -> Self {
        Self::with_mode(EditorMode::MultiLine { rows }, cx)
    }

    /// 高度随内容自适应(限行数)。
    pub fn auto_height(min_rows: usize, max_rows: usize, cx: &mut Context<Self>) -> Self {
        Self::with_mode(EditorMode::AutoHeight { min_rows, max_rows }, cx)
    }

    /// 程序化设置内容:**不**触发 [`EditorEvent::Edited`](计划沿用旧约定,
    /// 避免程序化初始化回环),光标移到末尾,清空 undo 历史。
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.rope = Rope::from(value.into().as_str());
        let end = self.rope.len();
        self.selections = vec![Selection::new(end, end)];
        self.marked_range = None;
        self.text_revision += 1;
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
        self.text_revision += 1;
        self.request_autoscroll();
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        self.emit_input_change(cx);
        cx.notify();
    }

    /// 用 `text` 替换**每个光标**的选区——所有插入/删除/粘贴的唯一入口。
    ///
    /// 对齐 zed `Editor::replace_selections`(input.rs:1952):编辑 rope、
    /// 折叠各选区到插入尾端、记录 undo 事务。
    ///
    /// 多光标做法(对齐 zed 的一次 buffer transaction):
    /// - 按起点**升序**依次应用,用 `shift` 累计前面几处的长度变化,
    ///   这样每次替换的区间都是当前 rope 里的真实坐标(可顺序回放);
    /// - 整组用一个显式 undo 事务装载(每处一个 [`Change`]),所以
    ///   一次 undo 把多处编辑一起撤回,光标组也整体恢复。
    pub(super) fn replace_selections(
        &mut self,
        text: &str,
        intent: EditIntent,
        cx: &mut Context<Self>,
    ) {
        let texts = vec![text.to_string(); self.selections.len()];
        self.replace_all_selections(texts, intent, cx);
    }

    /// 各光标插入**不同**文本(长度须等于光标数):换行继承缩进、
    /// 多光标粘贴按行分发走这里。
    pub(super) fn replace_all_selections(
        &mut self,
        texts: Vec<String>,
        intent: EditIntent,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        debug_assert_eq!(texts.len(), self.selections.len());
        self.transact(
            |this, _cx| {
                let selections_before = this.selections.clone();
                let multi_cursor = selections_before.len() > 1;
                if multi_cursor {
                    this.undo_manager
                        .begin_transaction(selections_before.clone());
                }

                // 规范序即升序;逐处替换并累计偏移
                let mut shift = 0i64;
                for (index, selection_before) in selections_before.iter().enumerate() {
                    let text = texts.get(index).map(String::as_str).unwrap_or_default();
                    let start = (selection_before.range().start as i64 + shift) as usize;
                    let end = (selection_before.range().end as i64 + shift) as usize;
                    let range = start..end;
                    let old_text = this.rope.text_in_range(range.clone());
                    let new_head = start + text.len();

                    this.rope.replace(range.clone(), text);
                    let selection_after = Selection::new(new_head, new_head);
                    this.selections[index] = selection_after;

                    let change = Change::new(
                        range.clone(),
                        &old_text,
                        start..new_head,
                        text,
                        *selection_before,
                        selection_after,
                    );
                    this.undo_manager.record_transaction(change, intent);

                    shift += text.len() as i64 - old_text.len() as i64;
                }
                this.marked_range = None;

                if multi_cursor {
                    this.normalize_selections();
                    this.undo_manager
                        .commit_transaction(this.selections.clone());
                }
            },
            cx,
        );
    }

    /// 非选区型移动后调用:折叠 + 通知。
    pub(super) fn change_selections(&mut self, cx: &mut Context<Self>) {
        self.request_autoscroll();
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    // ---- undo / redo ----

    pub fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_core(cx);
    }

    pub(crate) fn undo_core(&mut self, cx: &mut Context<Self>) {
        let Some(step) = self.undo_manager.undo() else {
            return;
        };
        // 逆序回放:后发生的先撤销
        for change in &step.changes {
            self.rope
                .replace(change.new_range.clone(), &change.old_text);
        }
        // 恢复事务开始前的整组光标(多光标时一次恢复所有)
        if !step.selections.is_empty() {
            self.selections = step.selections;
            self.normalize_selections();
        }
        self.text_revision += 1;
        self.text_revision += 1;
        self.request_autoscroll();
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        cx.notify();
    }

    pub fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.redo_core(cx);
    }

    pub(crate) fn redo_core(&mut self, cx: &mut Context<Self>) {
        let Some(step) = self.undo_manager.redo() else {
            return;
        };
        for change in &step.changes {
            self.rope
                .replace(change.old_range.clone(), &change.new_text);
        }
        if !step.selections.is_empty() {
            self.selections = step.selections;
            self.normalize_selections();
        }
        self.text_revision += 1;
        self.text_revision += 1;
        self.request_autoscroll();
        cx.emit(EditorEvent::Edited);
        cx.emit(EditorEvent::SelectionsChanged);
        self.emit_input_change(cx);
        cx.notify();
    }

    // ---- 鼠标 / 滚动(Render 注册)----

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.is_selecting = true;
        // 拖拽只对主光标生效:拖动前先把光标组收拢(同 VS Code 的行为;
        // zed 的 workshops 为补上模式,属于后续增量)
        if !event.modifiers.alt {
            self.selections.truncate(self.selections.len() - 1);
            self.normalize_selections();
        }
        let offset = self.index_for_mouse_position(event.position);
        if event.click_count == 2 {
            // 双击选词
            let range = self.word_range_at(offset);
            let selection = Selection::new(range.end, range.start);
            if event.modifiers.alt && self.selections.len() > 1 {
                self.add_selection(selection);
            } else {
                self.set_selection(selection);
            }
            self.change_selections(cx);
            return;
        }
        if event.modifiers.alt {
            // alt+click:加/撤一个光标(与已有光标重合时撤掉)
            self.add_selection(Selection::new(offset, offset));
            self.change_selections(cx);
            return;
        }
        if event.modifiers.shift {
            self.selection_mut().set_head(offset, SelectionGoal::None);
        } else {
            self.selection_mut()
                .collapse_to(offset, SelectionGoal::None);
        }
        self.change_selections(cx);
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let offset = self.index_for_mouse_position(event.position);
            self.selection_mut().set_head(offset, SelectionGoal::None);
            self.change_selections(cx);
        }
    }

    /// 滚轮滚动(zed 式自绘):直接累加 `scroll_position`,element 下一帧
    /// 用它平移 content_origin。clamp 到 [0, 内容高-视口高]。
    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.mode.is_multi_line() {
            return;
        }
        let line = if self.last_line_height > px(0.) {
            self.last_line_height
        } else {
            px(20.)
        };
        let delta_y = event.delta.pixel_delta(line).y;
        let max_scroll = self.max_scroll_offset();
        let next = (self.scroll_position - delta_y).clamp(px(0.), max_scroll);
        if next != self.scroll_position {
            tracing::debug!(
                from_y = delta_y.as_f32(),
                next = next.as_f32(),
                "wheel scroll"
            );
            self.scroll_position = next;
            cx.emit(EditorEvent::ScrollPositionChanged);
            cx.notify();
        }
    }
}

/// 把编辑动作绑到 [`EDITOR_KEY_CONTEXT`] 上。
///
/// 剪贴板/全选用 `secondary`(macOS→cmd,其余→ctrl),见
/// `bind_input_keys` 的说明。
pub fn bind_editor_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("delete", Delete, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("left", Left, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("right", Right, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("up", Up, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("down", Down, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-a", SelectAll, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-v", Paste, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-c", Copy, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-x", Cut, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-z", Undo, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-shift-z", Redo, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-left", WordLeft, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("secondary-right", WordRight, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new(
            "secondary-backspace",
            DeleteToPreviousWordStart,
            Some(EDITOR_KEY_CONTEXT),
        ),
        KeyBinding::new(
            "secondary-delete",
            DeleteToNextWordEnd,
            Some(EDITOR_KEY_CONTEXT),
        ),
        KeyBinding::new("home", Home, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new("end", End, Some(EDITOR_KEY_CONTEXT)),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some(EDITOR_KEY_CONTEXT),
        ),
        KeyBinding::new("enter", Newline, Some(EDITOR_KEY_CONTEXT)),
        // ---- 多光标(zed 默认键位:cmd-ctrl-p/n / cmd-alt-up/down)----
        KeyBinding::new(
            "secondary-alt-up",
            AddSelectionAbove,
            Some(EDITOR_KEY_CONTEXT),
        ),
        KeyBinding::new(
            "secondary-alt-down",
            AddSelectionBelow,
            Some(EDITOR_KEY_CONTEXT),
        ),
        // VS Code 的 cmd-d(zed 里叫 editor::SelectNext)
        KeyBinding::new(
            "secondary-d",
            SelectNextOccurrence,
            Some(EDITOR_KEY_CONTEXT),
        ),
        // escape:收拢为单个主光标(zed editor::Cancel)
        KeyBinding::new("escape", Cancel, Some(EDITOR_KEY_CONTEXT)),
    ]);
}

// ---- UTF-16 ↔ UTF-8(IME 接口,走 Rope 树内查询)----

impl Editor {
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

// ---- 命中测试与鼠标(渲染层把布局快照写回后调用)----

impl Editor {
    /// 垂直滚动上限(像素),对齐 zed element.rs 的 `max_scroll_top`:
    /// - OnePage: `max_row` 行 → 最后一行可贴视口顶,下方留空白;
    /// - Off: `max_row - 视口行数 + 1` → 滚到底即最后一行在视口底。
    ///
    /// 注意 OnePage 下内容不足一屏时上限仍 > 0,这就是「内容不多也能滚」的来源。
    pub(crate) fn max_scroll_offset(&self) -> Pixels {
        let line_h = if self.last_line_height > px(0.) {
            self.last_line_height
        } else {
            px(20.)
        };
        // 以**视觉行**为单位:软换行会让一个 buffer 行占多条视觉行
        let rows = self.display_rows() as f32;
        let rows = match self.scroll_beyond_last_line {
            ScrollBeyondLastLine::OnePage => (rows - 1.).max(0.),
            ScrollBeyondLastLine::Off => {
                let viewport_lines = self.viewport_height / line_h;
                (rows - viewport_lines).max(0.)
            }
        };
        px(rows * line_h.as_f32()).max(px(0.))
    }

    /// 视觉行总数:软换行开启且映射就绪时用 display_map,否则退化为
    /// buffer 行数(首帧/未换行时)。
    pub(crate) fn display_rows(&self) -> u32 {
        let mapped = self.display_map.display_rows();
        if mapped > 0 {
            mapped
        } else {
            self.rope.summary().lines.row + 1
        }
    }

    /// 文本字节下标 → display(视觉)坐标。
    pub(crate) fn display_point_for_offset(&self, offset: usize) -> DisplayPoint {
        let point = self.rope.offset_to_point(offset);
        if self.display_map.display_rows() == 0 {
            DisplayPoint::new(point.row, point.column)
        } else {
            self.display_map.buffer_point_to_display_point(point)
        }
    }

    /// 文本字节下标 → 视觉行号(软换行下与 buffer 行号不同)。
    pub(crate) fn display_row_for_offset(&self, offset: usize) -> u32 {
        if self.display_map.display_rows() == 0 {
            return self.rope.offset_to_point(offset).row;
        }
        let point = self.rope.offset_to_point(offset);
        self.display_map.buffer_point_to_display_point(point).row
    }

    /// 鼠标位置 → 文本字节下标(单行/多行统一)。
    pub(crate) fn index_for_mouse_position(&self, position: gpui::Point<Pixels>) -> usize {
        if self.mode.is_single_line() {
            if self.rope.is_empty() {
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
                return self.rope.len();
            }
            return line.closest_index_for_x(position.x - bounds.left());
        }

        // 多行:先按 y 定位**可见窗口内的视觉行**,再用该行的 ShapedLine
        // 命中行内位置。last_lines 只存可见行(可见行优化),所以
        // 全局视觉行号 = first_visible_display_row + 行内下标;
        // 内容 y = 鼠标 y - bounds.top + scroll_position(与绘制严格互逆);
        // 软换行下视觉行 ≠ buffer 行,字节区间由 last_line_ranges 给出。
        let Some(bounds) = self.last_content_bounds else {
            return 0;
        };
        if self.last_lines.is_empty() || self.last_line_height <= px(0.) {
            return 0;
        }
        let rel_y = position.y - bounds.top() + self.scroll_position;
        let display_row =
            self.first_visible_display_row + (rel_y / self.last_line_height).max(0.) as u32;
        let index_in_window = (display_row - self.first_visible_display_row) as usize;
        let Some(line) = self.last_lines.get(index_in_window) else {
            return self.rope.len();
        };
        let Some(range) = self.last_line_ranges.get(index_in_window) else {
            return self.rope.len();
        };
        let index = line
            .closest_index_for_x(position.x - bounds.left())
            .min(range.len());
        range.start + index
    }

    /// 请求下一帧把光标滚入视口(编辑/移动选区后调用)。
    ///
    /// 跟随的是主光标([`Editor::selection`])——多光标时 zed 滚的是
    /// `newest_selection`,这里就是组内最后一个。
    pub(crate) fn request_autoscroll(&mut self) {
        if self.mode.is_multi_line() {
            tracing::debug!(
                head = self.selection().head(),
                cursors = self.selections.len(),
                "autoscroll requested (needs_autoscroll=true)"
            );
            self.needs_autoscroll = true;
        }
    }
}

impl gpui::Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_context = if self.mode.is_single_line() {
            // 单行兼容:bind_input_keys 绑定的旧 context 名
            INPUT_KEY_CONTEXT
        } else {
            EDITOR_KEY_CONTEXT
        };

        // ---- 外框尺寸(按 mode)----
        // 关键:滚动容器的 size_full 需要有界的父高度,因此外框必须显式给高,
        // 否则百分比高度在 auto 容器里解析为 0,滚动面塌陷、无法滚动。
        let line_height = if self.last_line_height > px(0.) {
            self.last_line_height
        } else {
            window.line_height()
        };
        let total_rows = self.rope.summary().lines.row + 1;
        let (frame_height, scrollable) = match self.mode {
            EditorMode::SingleLine => (window.line_height(), false),
            EditorMode::AutoHeight { min_rows, max_rows } => {
                // 内容行数 clamp 到 [min, max];超出 max 的部分走滚动
                let rows =
                    (total_rows as usize).clamp(min_rows.max(1), max_rows.max(min_rows.max(1)));
                (px(line_height.as_f32() * rows as f32), true)
            }
            EditorMode::MultiLine { rows } => (px(line_height.as_f32() * rows.max(1) as f32), true),
        };

        // zed 式自绘滚动:滚动状态在本体(scroll_position),element 绘制时
        // 自行平移 content_origin 并用 ContentMask 裁剪,不依赖 div 的
        // overflow_scroll——坐标自控,命中测试与绘制严格互逆。
        let _ = scrollable;
        let editor_element = EditorElement {
            editor: cx.entity(),
        };

        div()
            .id("editor-root")
            .key_context(key_context)
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::move_to_previous_word_start))
            .on_action(cx.listener(Self::move_to_next_word_end))
            .on_action(cx.listener(Self::delete_to_previous_word_start))
            .on_action(cx.listener(Self::delete_to_next_word_end))
            .on_action(cx.listener(Self::add_selection_above))
            .on_action(cx.listener(Self::add_selection_below))
            .on_action(cx.listener(Self::select_next_occurrence))
            .on_action(cx.listener(Self::cancel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .px_2()
            .py_1()
            .border_1()
            .rounded_md()
            .w_full()
            .h(frame_height)
            .bg(self.bg_color)
            .border_color(self.border_color)
            .text_size(px(14.))
            .overflow_hidden()
            .child(editor_element)
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
            editor.set_selection(Selection::new(5, 5));
            editor.replace_selections(" world", EditIntent::Typing, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e.selection().head()),
            11,
            "光标应折叠到插入尾端"
        );
    }

    #[gpui::test]
    fn test_undo_redo_restores_selection(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("hello"));
        editor.update(cx, |editor, cx| {
            editor.set_selection(Selection::new(5, 5));
            editor.replace_selections(" world", EditIntent::Typing, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );

        editor.update(cx, |editor, cx| editor.undo_core(cx));
        assert_eq!(editor.read_with(cx, |e, _| e.value().to_string()), "hello");
        assert_eq!(
            editor.read_with(cx, |e, _| e.selection().head()),
            5,
            "undo 应恢复事务前选区"
        );

        editor.update(cx, |editor, cx| editor.redo_core(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
        assert_eq!(editor.read_with(cx, |e, _| e.selection().head()), 11);
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

        editor.update(cx, |editor, cx| editor.undo_core(cx));
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
            editor.set_selection(Selection::new(6, 11));
            editor.replace_selections("gpui", EditIntent::Atomic, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello gpui"
        );

        editor.update(cx, |editor, cx| editor.undo_core(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world"
        );
    }

    #[gpui::test]
    fn test_multiline_newline_inherits_indent(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 5 }, cx).default_value("  fn main()")
        });
        editor.update(cx, |editor, cx| {
            let end = editor.rope.len();
            editor.set_selection(Selection::new(end, end));
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
            editor.set_selection(Selection::new(end, end));
            editor.newline_core(cx);
            // core 直接插换行——单行必须经由动作包装分流,这里核对 core 行为
            assert_eq!(editor.rope.len(), 4);
        });
    }

    #[gpui::test]
    fn test_multiline_vertical_movement(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 3 }, cx)
                .default_value("abcd\nef\nghijk")
        });
        editor.update(cx, |editor, cx| {
            // 光标到第一行行尾(offset 4)
            editor.set_selection(Selection::new(4, 4));
            editor.move_vertical_core(1, cx);
            // 第二行只有 "ef":列被 clamp 到行尾(offset 7)
            assert_eq!(editor.selection().head(), 7, "下移应 clamp 到短行行尾");

            editor.move_vertical_core(1, cx);
            // 第三行 "ghijk":列 2 对应 offset 8+2=10
            assert_eq!(editor.selection().head(), 10);

            editor.move_vertical_core(-1, cx);
            assert_eq!(editor.selection().head(), 7, "上移应回到第二行");
        });
    }

    #[gpui::test]
    fn test_multiline_backspace_across_lines(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 3 }, cx).default_value("ab\ncd")
        });
        editor.update(cx, |editor, cx| {
            // 光标在第二行行首(offset 3),退格应删除换行合并两行
            editor.set_selection(Selection::new(3, 3));
            assert!(!editor.backspace_core(cx));
        });
        assert_eq!(editor.read_with(cx, |e, _| e.value().to_string()), "abcd");
        assert_eq!(editor.read_with(cx, |e, _| e.selection().head()), 2);
    }

    #[gpui::test]
    fn test_backspace_at_boundary_rings_bell(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| Editor::single_line(cx).default_value("ab"));
        editor.update(cx, |editor, cx| {
            editor.set_selection(Selection::new(0, 0));
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
        let selection = self.selection();
        Some(UTF16Selection {
            range: self.range_to_utf16(&selection.range()),
            reversed: selection.reversed,
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
        // 组字结束:必须复位,否则后续所有输入都会沿用 Atomic intent(影响 undo 合并)
        self.ime_composing = false;
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
        // 上屏(含普通键入):组字阶段结束
        self.ime_composing = false;
        // 指定区间(IME 上屏)→ 用之;否则替换当前选区
        if let Some(range_utf16) = range_utf16 {
            let range = self.range_from_utf16(&range_utf16);
            let old_text = self.rope.text_in_range(range.clone());
            let selection_before = self.selection();
            self.transact(
                |this, _| {
                    this.rope.replace(range.clone(), new_text);
                    let new_head = range.start + new_text.len();
                    this.selection_mut()
                        .collapse_to(new_head, SelectionGoal::None);
                    this.marked_range = None;
                    let change = Change::new(
                        range.clone(),
                        &old_text,
                        range.start..new_head,
                        new_text,
                        selection_before,
                        *this.selection_mut(),
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
            self.undo_manager.begin_transaction(self.selections.clone());
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selection().range());

        let old_text = self.rope.text_in_range(range.clone());
        let selection_before = self.selection();
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
        *self.selection_mut() = Selection::new(new_selected.end, new_selected.start);
        let new_range = range.start..range.start + new_text.len();
        let change = Change::new(
            range,
            &old_text,
            new_range,
            new_text,
            selection_before,
            self.selection(),
        );
        self.undo_manager
            .record_transaction(change, EditIntent::Atomic);
        // 组字区也要可见:中文输入法组字时若光标在视口外,同样需要滚回
        self.request_autoscroll();
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

#[cfg(test)]
mod autoscroll_tests {
    use super::*;
    use crate::editor::actions::Down;
    use gpui::{AppContext as _, VisualTestContext};

    /// 真实绘制管线下的自动滚动回归:光标从行 0 下移到视口(4 行)之外,
    /// scroll_position 应随之增大。prepaint 里的 autoscroll 只有真实 draw
    /// 才会执行,这是单元测试覆盖不到、只能用 VisualTestContext 验证的路径。
    #[gpui::test]
    fn test_autoscroll_follows_cursor_via_move_down(cx: &mut gpui::TestAppContext) {
        let text: String = (0..20).map(|i| format!("line{i}\n")).collect();
        let text = text.trim_end_matches('\n').to_string();

        // editor 在 add_window 闭包内创建(窗口根即 editor),通过 slot 捕获到外部
        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(&text);
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        // zed editor 测试同款:window.draw(cx) 绘制整窗(含 view stack)
        let draw_frame = |cx: &mut VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                let _ = window.draw(cx);
            });
        };

        // 初始一帧:光标在行 0,不应滚动
        draw_frame(&mut cx);
        let initial = cx.update(|_, cx| editor.read(cx).scroll_position);
        assert_eq!(initial, px(0.), "初始帧不应滚动");

        // 连续下移 19 次:光标从行 0 到行 19,远超 4 行视口
        for _ in 0..19 {
            cx.dispatch_action(Down);
            draw_frame(&mut cx);
        }

        let final_scroll = cx.update(|_, cx| editor.read(cx).scroll_position).as_f32();
        assert!(
            final_scroll > 0.,
            "光标下移到行 19(视口 4 行)后应自动滚动, got scroll={final_scroll}"
        );
    }

    /// 内容不足一屏时也能继续滚(对齐 zed 的 ScrollBeyondLastLine::OnePage):
    /// 两行内容、8 行视口,滚轮向下后 scroll_position 应 > 0。
    #[gpui::test]
    fn test_scroll_beyond_last_line_when_content_is_short(cx: &mut gpui::TestAppContext) {
        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let editor = Editor::with_mode(EditorMode::MultiLine { rows: 8 }, cx)
                .default_value("two\nlines");
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        // 先绘制一帧,让 last_line_height / viewport_height 量出来
        cx.update(|window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        });

        // 内容只有 2 行,视口 8 行:OnePage 下上限仍应 > 0
        let max_scroll = cx
            .update(|_, cx| editor.read(cx).max_scroll_offset())
            .as_f32();
        assert!(
            max_scroll > 0.,
            "内容不足一屏时也应能滚(scroll beyond last line), got max={max_scroll}"
        );

        // 向下滚轮一格
        cx.simulate_event(ScrollWheelEvent {
            // 位置必须在 editor 元素内,否则 hitbox 命中不到
            position: gpui::point(px(50.), px(50.)),
            delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-40.))),
            ..Default::default()
        });

        let scrolled = cx.update(|_, cx| editor.read(cx).scroll_position).as_f32();
        assert!(
            scrolled > 0.,
            "滚轮向下后应产生滚动, got scroll={scrolled}, max={max_scroll}"
        );
    }

    /// 打字(走 IME/InputHandler 的 replace_text_in_range)也要把光标拉回视口:
    /// 先把光标放到行 19 并滚到可见,再手动滚回顶部(模拟用户滚走),
    /// 然后 simulate_input 打字 —— 视口应重新跟到光标行。
    #[gpui::test]
    fn test_autoscroll_follows_cursor_on_typing(cx: &mut gpui::TestAppContext) {
        let text: String = (0..20).map(|i| format!("line{i}\n")).collect();
        let text = text.trim_end_matches('\n').to_string();
        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(&text);
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        let draw_frame = |cx: &mut VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                let _ = window.draw(cx);
            });
        };

        draw_frame(&mut cx);

        // 光标移到行 19 末尾 → 自动滚到该行可见
        cx.update(|_, cx| {
            editor.update(cx, |editor, cx| {
                let head = editor.rope.len();
                editor.set_selection(Selection::new(head, head));
                editor.change_selections(cx);
            })
        });
        draw_frame(&mut cx);
        let scrolled_to_cursor = cx.update(|_, cx| editor.read(cx).scroll_position).as_f32();
        assert!(scrolled_to_cursor > 0., "光标移到行 19 后应滚动");

        // 手动滚回顶部(模拟用户滚走)
        cx.update(|_, cx| {
            editor.update(cx, |editor, cx| {
                editor.scroll_position = px(0.);
                cx.notify();
            })
        });
        draw_frame(&mut cx);
        assert_eq!(
            cx.update(|_, cx| editor.read(cx).scroll_position),
            px(0.),
            "手动滚回顶部后应保持 0"
        );

        // 打字:视口应重新跟随光标(行 19)。
        // 直接调 handler,避免 simulate_input 内部自动 draw 干扰时序。
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.replace_text_in_range(None, "x", window, cx)
            })
        });
        // 注意: update 返回时 effects flush 会触发一次自动 draw,
        // prepaint 消费 needs_autoscroll 并写入 scroll_position(标志被清属正常)
        draw_frame(&mut cx);
        let after_typing = cx.update(|_, cx| editor.read(cx).scroll_position).as_f32();
        assert!(
            after_typing > 0.,
            "打字后应把光标重新滚入视口, got scroll={after_typing}"
        );
    }

    /// 软换行下的垂直移动:行 0 被折成多条视觉行时,按 ↓ 应停在**同一
    /// buffer 行**的下一个视觉行,而不是跳到行 1。
    #[gpui::test]
    fn test_move_down_stays_within_wrapped_line(cx: &mut gpui::TestAppContext) {
        let long_line = "word ".repeat(80);
        let text = format!("{long_line}\nsecond");
        let first_line_len = long_line.trim_end().len();

        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 6 }, cx).default_value(&text);
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        let draw_frame = |cx: &mut VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                let _ = window.draw(cx);
            });
        };
        draw_frame(&mut cx);

        // 折行确实发生(否则这个测试没有意义)
        let display_rows = cx.update(|_, cx| editor.read(cx).display_rows());
        assert!(
            display_rows > 2,
            "行 0 应被折成多条视觉行, display_rows={display_rows}"
        );

        // 显式把光标放到文档开头(default_value 不动 selection, 但测试要可控)
        cx.update(|_, cx| {
            editor.update(cx, |editor, cx| {
                editor.set_selection(Selection::new(0, 0));
                editor.change_selections(cx);
            })
        });
        draw_frame(&mut cx);
        // 光标在文档开头,按 ↓ 一次:应落在行 0 的第二个视觉行内
        cx.dispatch_action(Down);
        draw_frame(&mut cx);
        let head = cx.update(|_, cx| editor.read(cx).selection().head());
        assert!(
            head > 0 && head < first_line_len,
            "软换行下 ↓ 应停在同一 buffer 行的下一视觉行, got head={head}, first_line_len={first_line_len}"
        );
    }

    /// 软换行:超宽的长行应被折成多条视觉行——display 行数 > buffer 行数,
    /// 且视觉行布局数量与 display 行数一致(全量渲染)。
    #[gpui::test]
    fn test_soft_wrap_splits_long_line(cx: &mut gpui::TestAppContext) {
        let long_text = "word ".repeat(80);
        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(&long_text);
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        cx.update(|window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        });

        let (display_rows, buffer_rows, layout_lines) = cx.update(|_, cx| {
            let editor = editor.read(cx);
            (
                editor.display_rows(),
                editor.rope.summary().lines.row + 1,
                editor.last_lines.len(),
            )
        });
        assert!(
            display_rows > buffer_rows,
            "超宽长行应被软换行拆成多条视觉行: display_rows={display_rows}, buffer_rows={buffer_rows}"
        );
        assert_eq!(
            layout_lines, display_rows as usize,
            "视觉行布局数量应与 display 行数一致"
        );

        // 关闭软换行后不再折行
        cx.update(|_, cx| {
            editor.update(cx, |editor, cx| {
                editor.soft_wrap = false;
                cx.notify();
            })
        });
        cx.update(|window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        });
        let rows_after = cx.update(|_, cx| editor.read(cx).last_lines.len());
        assert_eq!(
            rows_after, buffer_rows as usize,
            "关闭软换行后回到 buffer 行数"
        );
    }

    /// 真实输入路径(IME/键入走 EntityInputHandler)也要对多光标生效:
    /// 两行开头各一个光标,`replace_text_in_range` 应给每行都插入。
    #[gpui::test]
    fn test_typing_through_input_handler_applies_to_all_cursors(cx: &mut gpui::TestAppContext) {
        let mut editor_slot = None;
        let window = cx.add_window(|window, cx| {
            let mut editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value("aa\nbb");
            editor.selections = vec![Selection::new(0, 0), Selection::new(3, 3)];
            let handle = editor.focus_handle.clone();
            window.focus(&handle, cx);
            editor_slot = Some(cx.entity());
            editor
        });
        let editor = editor_slot.expect("editor captured");
        let mut cx = VisualTestContext::from_window(window.into(), &cx);

        cx.update(|window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        });
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.replace_text_in_range(None, "X", window, cx)
            })
        });

        let (text, cursors) = cx.update(|_, cx| {
            let editor = editor.read(cx);
            (
                editor.value().to_string(),
                editor
                    .selections()
                    .iter()
                    .map(|s| s.head())
                    .collect::<Vec<_>>(),
            )
        });
        assert_eq!(text, "Xaa\nXbb", "键入应在每个光标处插入");
        // 第二行的光标原在 offset 3,前一处插入把它推后 1 → head = 5
        assert_eq!(cursors, vec![1, 5]);
    }

    /// Off 模式:内容不足一屏时上限为 0(普通滚动行为)。
    #[gpui::test]
    fn test_scroll_beyond_last_line_off(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 8 }, cx)
                .default_value("two\nlines")
                .scroll_beyond_last_line(ScrollBeyondLastLine::Off)
        });
        editor.update(cx, |editor, _| {
            editor.last_line_height = px(20.);
            editor.viewport_height = px(160.);
        });
        let max_scroll = editor.read_with(cx, |e, _| e.max_scroll_offset());
        assert_eq!(max_scroll, px(0.), "Off 模式下内容不足一屏不可滚");
    }
}

#[cfg(test)]
mod word_movement_tests {
    use super::*;
    use gpui::AppContext as _;

    #[gpui::test]
    fn test_word_movement_and_delete(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx)
                .default_value("hello world foo")
        });

        // 词移动:末尾 → "foo" 首 → "world" 首 → "hello" 首
        editor.update(cx, |editor, cx| {
            let end = editor.rope.len();
            editor.set_selection(Selection::new(end, end));
            assert_eq!(editor.previous_word_start_offset(end), 12);
            assert_eq!(editor.previous_word_start_offset(12), 6);
            assert_eq!(editor.previous_word_start_offset(6), 0);
            assert_eq!(editor.next_word_end_offset(0), 5);
            assert_eq!(editor.next_word_end_offset(5), 11);
        });

        // 词删除:光标在末尾,删除 "foo"(动作包装的等价 core 操作)
        editor.update(cx, |editor, cx| {
            let head = editor.rope.len();
            let target = editor.previous_word_start_offset(head);
            editor.selection_mut().set_head(target, SelectionGoal::None);
            editor.replace_selections("", EditIntent::Atomic, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "hello world "
        );

        // 双击选词:点击 "world" 中间应选中整个词
        editor.update(cx, |editor, cx| {
            let range = editor.word_range_at(8); // "world" 内
            assert_eq!(range, 6..11, "双击应选中整个词");
        });
    }
}

#[cfg(test)]
mod multi_cursor_tests {
    use super::*;
    use gpui::AppContext as _;

    /// 三个光标:每行一个,位于行首。
    fn cursors_at_line_starts(editor: &mut Editor, text: &str) {
        let mut selections = Vec::new();
        for (index, _) in text.split('\n').enumerate() {
            let offset = editor
                .rope
                .point_to_offset(crate::engine::Point::new(index as u32, 0));
            selections.push(Selection::new(offset, offset));
        }
        editor.set_selections(selections);
    }

    #[gpui::test]
    fn test_insert_applies_to_every_cursor(cx: &mut gpui::TestAppContext) {
        let text = "aa\nbb\ncc";
        let editor = cx.new(|cx| {
            let mut editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(text);
            cursors_at_line_starts(&mut editor, text);
            editor
        });

        editor.update(cx, |editor, cx| {
            assert_eq!(editor.cursor_count(), 3);
            editor.replace_selections("X", EditIntent::Atomic, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "Xaa\nXbb\nXcc",
            "每个光标都应插入一份"
        );
        // 插入后光标各自行首 + 1
        assert_eq!(
            editor.read_with(cx, |e, _| e
                .selections()
                .iter()
                .map(|s| s.head())
                .collect::<Vec<_>>()),
            vec![1, 5, 9]
        );
    }

    #[gpui::test]
    fn test_multi_cursor_edit_is_one_undo_step(cx: &mut gpui::TestAppContext) {
        let text = "aa\nbb\ncc";
        let editor = cx.new(|cx| {
            let mut editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(text);
            cursors_at_line_starts(&mut editor, text);
            editor
        });

        editor.update(cx, |editor, cx| {
            editor.replace_selections("XYZ", EditIntent::Atomic, cx);
        });
        editor.update(cx, |editor, cx| editor.undo_core(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            text,
            "一次 undo 撤掉所有光标处的编辑"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e.cursor_count()),
            3,
            "光标组随 undo 一起恢复"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e
                .selections()
                .iter()
                .map(|s| s.head())
                .collect::<Vec<_>>()),
            vec![0, 3, 6]
        );

        editor.update(cx, |editor, cx| editor.redo_core(cx));
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "XYZaa\nXYZbb\nXYZcc"
        );
    }

    #[gpui::test]
    fn test_backspace_deletes_at_every_cursor(cx: &mut gpui::TestAppContext) {
        let text = "aa\nbb\ncc";
        let editor = cx.new(|cx| {
            let mut editor =
                Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value(text);
            // 每行第 2 个字符后:offset 2 / 5 / 8
            editor.set_selections(vec![
                Selection::new(2, 2),
                Selection::new(5, 5),
                Selection::new(8, 8),
            ]);
            editor
        });
        editor.update(cx, |editor, cx| {
            editor.backspace_core(cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "a\nb\nc",
            "每个光标处各删一个字符"
        );
    }

    #[gpui::test]
    fn test_add_selection_below_and_cancel(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx)
                .default_value("aaaa\nbbbb\ncccc")
        });
        editor.update(cx, |editor, cx| {
            editor.set_selection(Selection::new(2, 2));
            editor.add_selection_vertical(false, cx);
        });
        // 行 0 列 2 → 行 1 列 2(offset 7);同列而不是同行尾
        assert_eq!(
            editor.read_with(cx, |e, _| e
                .selections()
                .iter()
                .map(|s| s.head())
                .collect::<Vec<_>>()),
            vec![2, 7]
        );

        editor.update(cx, |editor, cx| {
            editor.add_selection_vertical(false, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.cursor_count()),
            3,
            "每个现有光标都往下加一个"
        );

        // 再按一次:末行没有下一行了,且下落点重合的被 merge —— 数量不再增长
        editor.update(cx, |editor, cx| {
            editor.add_selection_vertical(false, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.cursor_count()),
            3,
            "末行无法继续往下加,重合者被合并"
        );

        // escape:收拢成单个主光标
        editor.update(cx, |editor, cx| {
            editor.cancel_core(cx);
        });
        assert_eq!(editor.read_with(cx, |e, _| e.cursor_count()), 1);
    }

    #[gpui::test]
    fn test_select_next_occurrence_and_edit(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value("foo bar foo")
        });
        editor.update(cx, |editor, cx| {
            editor.set_selection(Selection::new(0, 0));
            editor.select_next_occurrence_core(cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e
                .selections()
                .iter()
                .map(|s| s.range())
                .collect::<Vec<_>>()),
            vec![0..3, 8..11],
            "空光标:先选词,再把下一处加成第二个光标"
        );

        editor.update(cx, |editor, cx| {
            editor.replace_selections("X", EditIntent::Atomic, cx);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e.value().to_string()),
            "X bar X"
        );
    }

    #[gpui::test]
    fn test_overlapping_cursors_are_merged(cx: &mut gpui::TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::with_mode(EditorMode::MultiLine { rows: 4 }, cx).default_value("abcdef")
        });
        editor.update(cx, |editor, _| {
            editor.set_selections(vec![
                Selection::new(1, 1),
                Selection::new(3, 3),
                Selection::new(1, 1),
            ]);
        });
        assert_eq!(
            editor.read_with(cx, |e, _| e
                .selections()
                .iter()
                .map(|s| s.head())
                .collect::<Vec<_>>()),
            vec![1, 3],
            "重合的光标合并为一个"
        );
    }
}
