//! 撤销/重做:事务 + 意图驱动合并,参照 gpui-component `undo_manager.rs`。
//!
//! 设计要点:
//! - 连续同类编辑(连续打字、连续退格)自动合并为一个事务,
//!   undo 一次撤一串;IME 组字用 begin/commit 显式括成一个 Atomic 事务;
//! - [`Change`] 自带前后选区快照——undo/redo 时选区随文本一起恢复
//!   (对应 zed 的 selection_history + buffer transaction 分工)。

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
    pending_change: Option<Change>,
    coalescing_boundary: bool,
}

impl UndoManager {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 记录一次编辑。事务打开期间(IME)只更新 pending。
    pub(super) fn record_transaction(&mut self, change: Change, intent: EditIntent) {
        // 无变化的编辑(如 set_head 到原位)只打断合并
        if change.old_range == change.new_range && change.old_text == change.new_text {
            self.break_transaction_coalescing();
            return;
        }

        if self.transaction_open {
            if let Some(pending) = self.pending_change.as_mut() {
                pending.new_range = change.new_range;
                pending.new_text = change.new_text.clone();
                pending.selection_after = change.selection_after;
            } else {
                self.pending_change = Some(change);
            }
        } else {
            self.push_transaction(change, intent);
        }
    }

    /// 打开显式事务(IME 组字开始)。期间的多次编辑折叠为一条 Change。
    pub(super) fn begin_transaction(&mut self) {
        if self.transaction_open {
            return;
        }
        self.transaction_open = true;
        self.pending_change = None;
    }

    /// 提交显式事务,pending 变为 Atomic(不与任何后续合并)。
    pub(super) fn commit_transaction(&mut self) {
        if !self.transaction_open {
            return;
        }
        self.transaction_open = false;
        if let Some(change) = self.pending_change.take()
            && (change.old_range != change.new_range || change.old_text != change.new_text)
        {
            self.push_transaction(change, EditIntent::Atomic);
        }
    }

    fn push_transaction(&mut self, change: Change, intent: EditIntent) {
        self.redo_transactions.clear();
        let can_coalesce = !self.coalescing_boundary
            && intent != EditIntent::Atomic
            && self.undo_transactions.last().is_some_and(|previous| {
                previous.intent == intent
                    && previous.changes.len() < MAX_CHANGES_PER_TRANSACTION
                    && previous
                        .changes
                        .last()
                        .is_some_and(|last| is_adjacent(intent, last, &change))
            });

        if can_coalesce {
            self.undo_transactions
                .last_mut()
                .expect("coalescing requires a previous transaction")
                .changes
                .push(change);
            return;
        }

        if self.undo_transactions.len() >= MAX_UNDO_TRANSACTIONS {
            self.undo_transactions.remove(0);
        }
        self.undo_transactions.push(UndoTransaction {
            intent,
            changes: vec![change],
        });
        self.coalescing_boundary = intent == EditIntent::Atomic;
    }

    /// 打断合并(光标移动等非编辑操作后,下次打字不应并进上一串)。
    pub(super) fn break_transaction_coalescing(&mut self) {
        self.commit_transaction();
        self.coalescing_boundary = true;
    }

    pub(super) fn clear(&mut self) {
        self.undo_transactions.clear();
        self.redo_transactions.clear();
        self.transaction_open = false;
        self.pending_change = None;
        self.coalescing_boundary = false;
    }

    /// 撤销一个事务,返回其 changes 的**逆序**(回放时从后往前 apply)。
    pub(super) fn undo(&mut self) -> Option<Vec<Change>> {
        self.commit_transaction();
        let transaction = self.undo_transactions.pop()?;
        let changes = transaction.changes.iter().rev().cloned().collect();
        self.redo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(changes)
    }

    /// 重做一个事务,返回其 changes 正序。
    pub(super) fn redo(&mut self) -> Option<Vec<Change>> {
        self.commit_transaction();
        let transaction = self.redo_transactions.pop()?;
        let changes = transaction.changes.clone();
        self.undo_transactions.push(transaction);
        self.coalescing_boundary = true;
        Some(changes)
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

    #[test]
    fn adjacent_typing_transactions_coalesce() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);

        assert_eq!(manager.undo().unwrap().len(), 2);
        assert!(manager.undo().is_none());
    }

    #[test]
    fn newline_breaks_coalescing() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(1, "\n"), EditIntent::Typing); // 含换行不合并
        assert_eq!(manager.undo().unwrap().len(), 1);
        assert_eq!(manager.undo().unwrap().len(), 1);
    }

    #[test]
    fn cursor_move_breaks_coalescing() {
        let mut manager = UndoManager::new();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.break_transaction_coalescing();
        manager.record_transaction(typing_change(1, "b"), EditIntent::Typing);
        assert_eq!(manager.undo().unwrap().len(), 1);
        assert_eq!(manager.undo().unwrap().len(), 1);
    }

    #[test]
    fn explicit_transaction_collects_multiple_changes() {
        let mut manager = UndoManager::new();
        manager.begin_transaction();
        manager.record_transaction(typing_change(0, "a"), EditIntent::Typing);
        manager.record_transaction(typing_change(0, "ab"), EditIntent::Typing);
        manager.commit_transaction();

        let transaction = manager.undo().unwrap();
        assert_eq!(transaction.len(), 1);
        assert_eq!(transaction[0].new_text, "ab");
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

        assert_eq!(manager.undo().unwrap().len(), 100);
        assert_eq!(manager.undo().unwrap().len(), MAX_CHANGES_PER_TRANSACTION);
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
