//! 撤销/重做:事务 + 意图驱动合并,参照 gpui-component `undo_manager.rs`。
//!
//! 设计要点:
//! - 连续同类编辑(连续打字、连续退格)自动合并为一个事务,
//!   undo 一次撤一串;IME 组字用 begin/commit 显式括成一个 Atomic 事务;
//! - **多光标**编辑也用一个显式事务装载 —— 每个光标产生独立的
//!   [`Change`],但整组是一次撤销单位;
//! - 事务自带前后**光标组**快照:单光标时从 Change 推导,多光标时由
//!   [`UndoManager::begin_transaction`] / [`commit_transaction`] 显式给出,
//!   undo/redo 时光标组随文本一起恢复。

use std::ops::Range;

use super::selection::Selection;

const MAX_UNDO_TRANSACTIONS: usize = 1000;
const MAX_CHANGES_PER_TRANSACTION: usize = 1000;

/// 编辑意图,决定相邻事务能否合并。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditIntent {
    /// 连续键入:只合并"插入点相邻且不含换行"的纯插入。
    Typing,
    /// 连续退格:只合并"纯删除且位置连续向前"的编辑。
    Backspace,
    /// 连续向前删除。
    DeleteForward,
    /// 原子事务(IME 组字、粘贴、换行等),永不合并。
    Atomic,
}

/// 一次文本替换的完整记录。
#[derive(Debug, PartialEq, Clone)]
pub struct Change {
    /// 编辑前的字节区间。
    pub old_range: Range<usize>,
    pub old_text: String,
    /// 编辑后的字节区间。
    pub new_range: Range<usize>,
    pub new_text: String,
    /// 编辑前的选区。
    pub selection_before: Selection<usize>,
    /// 编辑后的选区。
    pub selection_after: Selection<usize>,
}

impl Change {
    pub fn new(
        old_range: Range<usize>,
        old_text: &str,
        new_range: Range<usize>,
        new_text: &str,
        selection_before: Selection<usize>,
        selection_after: Selection<usize>,
    ) -> Self {
        Self {
            old_range,
            old_text: old_text.to_string(),
            new_range,
            new_text: new_text.to_string(),
            selection_before,
            selection_after,
        }
    }
}

#[derive(Debug)]
struct UndoTransaction {
    intent: EditIntent,
    changes: Vec<Change>,
    /// 事务开始前的光标组(多光标由 `begin_transaction` 给出)。
    selections_before: Vec<Selection<usize>>,
    /// 事务结束后的光标组(多光标由 `commit_transaction` 给出)。
    selections_after: Vec<Selection<usize>>,
}

/// 撤销/重做的回放单元:要回放的 changes + 回放后应恢复的光标组。
///
/// changes 的顺序即当初的**应用顺序**;撤销时逆序回放(见
/// [`UndoManager::undo`]), redo 时正序回放。
#[derive(Debug, Clone)]
pub struct UndoStep {
    pub changes: Vec<Change>,
    /// 回放后编辑器应恢复的光标组(空表示不改动当前光标)。
    pub selections: Vec<Selection<usize>>,
}

/// 协调 undo/redo 的事务管理器。
///
/// 每次编辑产生一个事务;兼容的相邻事务合并,直到显式边界
/// ([`UndoManager::break_transaction_coalescing`],如光标移动/换行/IME 提交)。
#[derive(Debug, Default)]
pub struct UndoManager {
    undo_transactions: Vec<UndoTransaction>,
    redo_transactions: Vec<UndoTransaction>,
    transaction_open: bool,
    pending_changes: Vec<Change>,
    pending_selections_before: Vec<Selection<usize>>,
    coalescing_boundary: bool,
}

impl UndoManager {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 记录一次编辑。事务打开期间(IME / 多光标)只累加。
    pub(super) fn record_transaction(&mut self, change: Change, intent: EditIntent) {
        // 无变化的编辑(如 set_head 到原位)只打断合并
        if change.old_range == change.new_range && change.old_text == change.new_text {
            self.break_transaction_coalescing();
            return;
        }

        if self.transaction_open {
            self.pending_changes.push(change);
            return;
        }
        self.push_transaction(vec![change], intent, None, None);
    }

    /// 打开显式事务,并登记事务前的光标组。
    ///
    /// 期间的多次编辑合并为**一条**事务(一次 undo 全撤):
    /// - IME 组字:多次替换同一处,产生一串前后衔接的 Change;
    /// - 多光标:每个光标各自一个 Change,互不相邻。
    pub(super) fn begin_transaction(&mut self, selections_before: Vec<Selection<usize>>) {
        if self.transaction_open {
            return;
        }
        self.transaction_open = true;
        self.pending_selections_before = selections_before;
    }

    /// 提交显式事务,并登记事务后的光标组。pending 变为 Atomic(不与任何后续合并)。
    pub(super) fn commit_transaction(&mut self, selections_after: Vec<Selection<usize>>) {
        if !self.transaction_open {
            return;
        }
        self.transaction_open = false;
        let changes = std::mem::take(&mut self.pending_changes);
        if changes.is_empty() {
            return;
        }
        let selections_before = std::mem::take(&mut self.pending_selections_before);
        self.push_transaction(
            changes,
            EditIntent::Atomic,
            Some(selections_before),
            Some(selections_after),
        );
    }

    fn push_transaction(
        &mut self,
        changes: Vec<Change>,
        intent: EditIntent,
        selections_before: Option<Vec<Selection<usize>>>,
        selections_after: Option<Vec<Selection<usize>>>,
    ) {
        let Some((first, last)) = changes.first().cloned().zip(changes.last().cloned()) else {
            return;
        };
        self.redo_transactions.clear();
        let can_coalesce = !self.coalescing_boundary
            && intent != EditIntent::Atomic
            && self.undo_transactions.last().is_some_and(|previous| {
                previous.intent == intent
                    && previous.changes.len() < MAX_CHANGES_PER_TRANSACTION
                    && previous
                        .changes
                        .last()
                        .is_some_and(|last| is_adjacent(intent, last, &first))
            });

        if can_coalesce {
            let previous = self
                .undo_transactions
                .last_mut()
                .expect("coalescing requires a previous transaction");
            previous.changes.extend(changes);
            // 合并后事务的终点是最后一条 Change 的选区(第一条的选区仍是起点)
            previous.selections_after = vec![last.selection_after];
            return;
        }

        if self.undo_transactions.len() >= MAX_UNDO_TRANSACTIONS {
            self.undo_transactions.remove(0);
        }
        self.undo_transactions.push(UndoTransaction {
            intent,
            changes,
            // 显式给空 → 回落到「单光标」语义:从 Change 自带的选区快照推导
            selections_before: selections_before
                .filter(|selections| !selections.is_empty())
                .unwrap_or_else(|| vec![first.selection_before]),
            selections_after: selections_after
                .filter(|selections| !selections.is_empty())
                .unwrap_or_else(|| vec![last.selection_after]),
        });
        self.coalescing_boundary = intent == EditIntent::Atomic;
    }

    /// 打断合并(光标移动等非编辑操作后,下次打字不应并进上一串)。
    pub(super) fn break_transaction_coalescing(&mut self) {
        self.commit_transaction(Vec::new());
        self.coalescing_boundary = true;
    }

    pub(super) fn clear(&mut self) {
        self.undo_transactions.clear();
        self.redo_transactions.clear();
        self.transaction_open = false;
        self.pending_changes.clear();
        self.pending_selections_before.clear();
        self.coalescing_boundary = false;
    }

    /// 撤销一个事务:返回 changes 的**逆序**(回放时从后往前 apply)与
    /// 事务前的光标组。
    pub(super) fn undo(&mut self) -> Option<UndoStep> {
        self.commit_transaction(Vec::new());
        let transaction = self.undo_transactions.pop()?;
        let step = UndoStep {
            changes: transaction.changes.iter().rev().cloned().collect(),
            selections: transaction.selections_before.clone(),
        };
        self.redo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(step)
    }

    /// 重做一个事务:返回 changes 正序与事务后的光标组。
    pub(super) fn redo(&mut self) -> Option<UndoStep> {
        self.commit_transaction(Vec::new());
        let transaction = self.redo_transactions.pop()?;
        let step = UndoStep {
            changes: transaction.changes.clone(),
            selections: transaction.selections_after.clone(),
        };
        self.undo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(step)
    }
}

/// 相邻判定:同意图且字节位置衔接才合并。
fn is_adjacent(intent: EditIntent, previous: &Change, current: &Change) -> bool {
    match intent {
        EditIntent::Typing => {
            previous.old_range.is_empty()
                && current.old_range.is_empty()
                && !previous.new_text.contains(['\n', '\r'])
                && !current.new_text.contains(['\n', '\r'])
                && previous.new_range.end == current.old_range.start
        }
        EditIntent::Backspace => {
            previous.new_text.is_empty()
                && current.new_text.is_empty()
                && current.old_range.end == previous.old_range.start
        }
        EditIntent::DeleteForward => {
            previous.new_text.is_empty()
                && current.new_text.is_empty()
                && current.old_range.start == previous.old_range.start
        }
        EditIntent::Atomic => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::selection::SelectionGoal;
    use super::*;

    fn typing_change(offset: usize, text: &str) -> Change {
        let end = offset + text.len();
        Change::new(
            offset..offset,
            "",
            offset..end,
            text,
            Selection::new(offset, offset),
            Selection::new(end, end),
        )
    }

    fn change_at(offset: usize, old_text: &str, new_text: &str) -> Change {
        let end = offset + new_text.len();
        Change::new(
            offset..offset + old_text.len(),
            old_text,
            offset..end,
            new_text,
            Selection::new(offset, offset + old_text.len()),
            Selection::new(end, end),
        )
    }

    #[test]
    fn adjacent_typing_transactions_coalesce() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);

        assert_eq!(manager.undo().unwrap().changes.len(), 2);
        assert!(manager.undo().is_none());
    }

    #[test]
    fn newline_breaks_coalescing() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "\n"), EditIntent::Typing); // 含换行不合并
        assert_eq!(manager.undo().unwrap().changes.len(), 1);
        assert_eq!(manager.undo().unwrap().changes.len(), 1);
    }

    #[test]
    fn cursor_move_breaks_coalescing() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.break_transaction_coalescing();
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        assert_eq!(manager.undo().unwrap().changes.len(), 1);
        assert_eq!(manager.undo().unwrap().changes.len(), 1);
    }

    /// IME 组字:多次替换同一处 → 显式事务里是一串前后衔接的 Change,
    /// 一次 undo 全部回退到组字前的样子。
    #[test]
    fn explicit_transaction_collects_multiple_changes() {
        let mut manager = UndoManager::new();
        let before = vec![Selection::new(0, 0)];
        manager.begin_transaction(before.clone());
        manager.record_transaction(change_at(0, "", "n"), EditIntent::Typing);
        manager.record_transaction(change_at(0, "n", "ni"), EditIntent::Typing);
        manager.record_transaction(change_at(0, "ni", "你"), EditIntent::Typing);
        let after = vec![Selection::new(3, 3)];
        manager.commit_transaction(after.clone());

        // 一条事务、三次替换
        let step = manager.undo().unwrap();
        assert_eq!(step.changes.len(), 3);
        // 逆序回放:最后一次先撤 → 回退到 ""
        let mut text = String::from("你");
        for change in &step.changes {
            text.replace_range(
                change.new_range.start..change.new_range.end,
                &change.old_text,
            );
        }
        assert_eq!(text, "", "undo 应把整串组字一起撤销");
        assert_eq!(step.selections, before, "应恢复到组字前的光标");
        assert!(manager.undo().is_none(), "整串组字只占一个事务");

        let step = manager.redo().unwrap();
        assert_eq!(step.selections, after);
        assert_eq!(step.changes.len(), 3);
    }

    /// 多光标:一个事务里装多个互不相邻的 Change(一次 undo 全撤)。
    #[test]
    fn multi_cursor_transaction_is_one_undo_step() {
        let mut manager = UndoManager::new();
        let before = vec![Selection::new(5, 5), Selection::new(12, 12)];
        let after = vec![Selection::new(6, 6), Selection::new(13, 13)];
        manager.begin_transaction(before.clone());
        manager.record_transaction(typing_change(12, "y"), EditIntent::Typing);
        manager.record_transaction(typing_change(5, "x"), EditIntent::Typing);
        manager.commit_transaction(after.clone());

        let step = manager.undo().unwrap();
        assert_eq!(step.changes.len(), 2, "两个光标各一条 Change");
        assert_eq!(step.selections, before, "应恢复多光标组");
        assert!(manager.undo().is_none(), "多光标编辑是一次撤销单位");
        assert_eq!(manager.redo().unwrap().selections, after);
    }

    #[test]
    fn redo_is_cleared_by_new_edit() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        assert!(manager.undo().is_some());
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        assert!(manager.redo().is_none());
    }

    #[test]
    fn limits_the_number_of_retained_transactions() {
        let mut manager = UndoManager::new();

        for offset in 0..1_100 {
            manager.record_transaction(typing_change(offset, "a"), EditIntent::Atomic);
        }

        for _ in 0..MAX_UNDO_TRANSACTIONS {
            assert!(manager.undo().is_some());
        }
        assert!(manager.undo().is_none());
    }

    #[test]
    fn splits_a_coalesced_transaction_before_its_change_list_grows_too_large() {
        let mut manager = UndoManager::new();

        for offset in 0..1_100 {
            manager.record_transaction(typing_change(offset, "a"), EditIntent::Typing);
        }

        assert_eq!(manager.undo().unwrap().changes.len(), 100);
        assert_eq!(
            manager.undo().unwrap().changes.len(),
            MAX_CHANGES_PER_TRANSACTION
        );
        assert!(manager.undo().is_none());
    }

    #[test]
    fn selection_travels_with_change() {
        let change = Change::new(
            2..4,
            "cd",
            2..2,
            "",
            // head=2(活动端在前)→ reversed=true
            Selection::new(2, 4),
            Selection::new(2, 2).with_goal(SelectionGoal::None),
        );
        assert_eq!(change.selection_before.head(), 2);
        assert_eq!(change.selection_before.tail(), 4);
        assert_eq!(change.selection_after.head(), 2);
    }
}
