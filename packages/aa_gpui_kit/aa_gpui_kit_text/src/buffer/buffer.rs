//! The [`Buffer`] type: a concurrent, rope-backed editable text buffer.
//!
//! `Buffer` owns a [`BufferSnapshot`], an undo/redo [`History`], and the
//! per-replica state needed to exchange operations with other buffers. All
//! text edits funnel through [`Buffer::edit`], which records an
//! [`Operation`] into the history and publishes a [`Patch`] on
//! `self.subscriptions`.

use collections::{HashMap, HashSet};
use std::cmp::{self, Reverse};
use std::ops::{Deref, Range};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clock::{Global, Lamport, ReplicaId};
use anyhow::Result;
use postage::{oneshot, prelude::*};
use sum_tree::{Bias, Dimensions, SumTree, TreeMap, TreeSet};

use crate::anchor::OffsetRangeExt as _;
use crate::locator::Locator;
use crate::operation_queue::OperationQueue;
use crate::subscription::{Subscription, Topic};
use crate::{Anchor, Edit, Patch};
use rope::{Rope, TextDimension};

use super::TransactionId;
use super::buffer_id::BufferId;
use super::constants::MAX_INSERTION_LEN;
use super::dimensions::{FullOffset, VersionedFullOffset};
use super::edit_snapshot::EditedBufferSnapshot;
use super::fragment::{
    Fragment, FragmentBuilder, InsertionFragment, InsertionFragmentKey, InsertionSlice,
};
use super::history::{History, HistoryEntry, Transaction};
use super::line_ending::LineEnding;
use super::offset_traits::ToOffset;
use super::operation::{EditOperation, Operation, UndoOperation};
use super::rope_builder::RopeBuilder;
use super::snapshot::BufferSnapshot;
use super::util::push_fragments_for_insertion;

/// An editable text buffer.
///
/// The buffer is a CRDT: each replica has a unique [`ReplicaId`], and edits
/// are exchanged as [`Operation`]s which are applied in a deterministic order
/// by all replicas. Read access happens through [`Buffer::snapshot`], which
/// returns a cheap, immutable [`BufferSnapshot`].
pub struct Buffer {
    /// The current state of the text.
    pub(crate) snapshot: BufferSnapshot,
    /// Undo/redo history, plus the set of all operations ever applied.
    pub(crate) history: History,
    /// Operations that arrived out of causal order and are waiting for their
    /// dependencies to be observed.
    pub(crate) deferred_ops: OperationQueue<Operation>,
    /// Replicas whose operations are currently deferred, so we can skip
    /// attempting to apply their operations until we've flushed the queue.
    pub(crate) deferred_replicas: HashSet<ReplicaId>,
    /// This replica's lamport clock; ticks on every local operation.
    pub lamport_clock: Lamport,
    /// Publishes edit patches to subscribers (see [`Buffer::subscribe`]).
    pub(crate) subscriptions: Topic<usize>,
    /// Channels waiting for a specific edit id to be observed.
    pub(crate) edit_id_resolvers: HashMap<Lamport, Vec<oneshot::Sender<()>>>,
    /// Channels waiting for a specific version to be observed.
    pub(crate) wait_for_version_txs: Vec<(Global, oneshot::Sender<()>)>,
}

impl Buffer {
    /// Creates a new buffer from the given base text. Line endings are
    /// detected and normalized to `\n` internally.
    pub fn new(replica_id: ReplicaId, remote_id: BufferId, base_text: impl Into<String>) -> Buffer {
        let mut base_text = base_text.into();
        let line_ending = LineEnding::detect(&base_text);
        LineEnding::normalize(&mut base_text);
        Self::new_normalized(replica_id, remote_id, line_ending, Rope::from(&*base_text))
    }

    /// Creates a new buffer from already-normalized base text. The
    /// `line_ending` describes the file's on-disk line endings and is used by
    /// [`BufferSnapshot::text_with_line_endings`].
    pub fn new_normalized(
        replica_id: ReplicaId,
        remote_id: BufferId,
        line_ending: LineEnding,
        normalized: Rope,
    ) -> Buffer {
        let history = History::new(normalized);
        let mut fragments = SumTree::new(&None);
        let mut insertions = SumTree::default();

        let mut lamport_clock = Lamport::new(replica_id);
        let mut version = Global::new();

        let visible_text = history.base_text.clone();
        if !visible_text.is_empty() {
            let insertion_timestamp = Lamport::new(ReplicaId::LOCAL);
            lamport_clock.observe(insertion_timestamp);
            version.observe(insertion_timestamp);

            let mut insertion_offset: u32 = 0;
            let mut text_offset: usize = 0;
            let mut prev_locator = Locator::min();

            while text_offset < visible_text.len() {
                let target_end = visible_text.len().min(text_offset + MAX_INSERTION_LEN);
                let chunk_end = if target_end == visible_text.len() {
                    target_end
                } else {
                    visible_text.floor_char_boundary(target_end)
                };
                let chunk_len = chunk_end - text_offset;

                let fragment_id = Locator::between(&prev_locator, &Locator::max());
                let fragment = Fragment {
                    id: fragment_id.clone(),
                    timestamp: insertion_timestamp,
                    insertion_offset,
                    len: chunk_len as u32,
                    visible: true,
                    deletions: Default::default(),
                    max_undos: Default::default(),
                };
                insertions.push(InsertionFragment::new(&fragment), ());
                fragments.push(fragment, &None);

                prev_locator = fragment_id;
                insertion_offset += chunk_len as u32;
                text_offset = chunk_end;
            }
        }

        Buffer {
            snapshot: BufferSnapshot {
                replica_id,
                remote_id,
                visible_text,
                deleted_text: Rope::new(),
                line_ending,
                fragments,
                insertions,
                version,
                undo_map: Default::default(),
                insertion_slices: Default::default(),
            },
            history,
            deferred_ops: OperationQueue::new(),
            deferred_replicas: HashSet::default(),
            lamport_clock,
            subscriptions: Default::default(),
            edit_id_resolvers: Default::default(),
            wait_for_version_txs: Default::default(),
        }
    }

    /// Returns the current version vector of the buffer.
    pub fn version(&self) -> Global { self.version.clone() }

    /// Returns a cheap, immutable snapshot of the buffer's current state.
    pub fn snapshot(&self) -> &BufferSnapshot { &self.snapshot }

    /// Consumes the buffer and returns its snapshot.
    pub fn into_snapshot(self) -> BufferSnapshot { self.snapshot }

    /// Creates a branch of this buffer for speculative edits. Edits to the
    /// branch are not applied to this buffer until
    /// `merge_into_base` is called on it.
    pub fn branch(&self) -> Self {
        Self {
            snapshot: self.snapshot.clone(),
            history: History::new(self.base_text().clone()),
            deferred_ops: OperationQueue::new(),
            deferred_replicas: HashSet::default(),
            lamport_clock: Lamport::new(ReplicaId::LOCAL_BRANCH),
            subscriptions: Default::default(),
            edit_id_resolvers: Default::default(),
            wait_for_version_txs: Default::default(),
        }
    }

    /// Returns this buffer's replica id.
    pub fn replica_id(&self) -> ReplicaId { self.lamport_clock.replica_id }

    /// Returns the (shared) remote id of this buffer.
    pub fn remote_id(&self) -> BufferId { self.remote_id }

    /// Returns the number of operations currently waiting to be applied.
    pub fn deferred_ops_len(&self) -> usize { self.deferred_ops.len() }

    /// Returns the interval within which consecutive transactions are grouped
    /// into a single undo step.
    pub fn transaction_group_interval(&self) -> Duration { self.history.group_interval }

    /// Applies a set of edits to the buffer and returns the resulting
    /// operation. Edits are specified as `(range, new_text)` pairs; adjacent
    /// edits are coalesced by `TextBuffer::edit`.
    pub fn edit<R, I, S, T>(&mut self, edits: R) -> Operation
    where
        R: IntoIterator<IntoIter = I>,
        I: ExactSizeIterator<Item = (Range<S>, T)>,
        S: ToOffset,
        T: Into<Arc<str>>,
    {
        let edits = edits
            .into_iter()
            .map(|(range, new_text)| (range, new_text.into()));

        self.start_transaction();
        let timestamp = self.lamport_clock.tick();
        let operation = Operation::Edit(self.apply_local_edit(edits, timestamp));

        self.history.push(operation.clone());
        self.history.push_undo(operation.timestamp());
        self.snapshot.version.observe(operation.timestamp());
        self.end_transaction();
        operation
    }

    fn apply_local_edit<S: ToOffset, T: Into<Arc<str>>>(
        &mut self,
        edits: impl ExactSizeIterator<Item = (Range<S>, T)>,
        timestamp: Lamport,
    ) -> EditOperation {
        let edits: Vec<_> = edits
            .map(|(range, new_text)| (range.to_offset(&*self), new_text.into()))
            .collect();
        let (edit_op, edits_patch) = self.snapshot.apply_edit_internal(edits, timestamp);
        self.subscriptions.publish_mut(&edits_patch);
        edit_op
    }

    /// Sets the buffer's on-disk line ending convention.
    pub fn set_line_ending(&mut self, line_ending: LineEnding) {
        self.snapshot.line_ending = line_ending;
    }

    /// Applies remote operations to the buffer. Operations whose dependencies
    /// haven't been observed yet are deferred.
    pub fn apply_ops<I: IntoIterator<Item = Operation>>(&mut self, ops: I) {
        let mut deferred_ops = Vec::new();
        for op in ops {
            self.history.push(op.clone());
            if self.can_apply_op(&op) {
                self.apply_op(op);
            } else {
                self.deferred_replicas.insert(op.replica_id());
                deferred_ops.push(op);
            }
        }
        self.deferred_ops.insert(deferred_ops);
        self.flush_deferred_ops();
    }

    fn apply_op(&mut self, op: Operation) {
        match op {
            Operation::Edit(edit) => {
                if !self.version.observed(edit.timestamp) {
                    self.apply_remote_edit(
                        &edit.version,
                        &edit.ranges,
                        &edit.new_text,
                        edit.timestamp,
                    );
                    self.snapshot.version.observe(edit.timestamp);
                    self.lamport_clock.observe(edit.timestamp);
                    self.resolve_edit(edit.timestamp);
                }
            }
            Operation::Undo(undo) => {
                if !self.version.observed(undo.timestamp) {
                    self.apply_undo(&undo);
                    self.snapshot.version.observe(undo.timestamp);
                    self.lamport_clock.observe(undo.timestamp);
                }
            }
        }
        self.wait_for_version_txs.retain_mut(|(version, tx)| {
            if self.snapshot.version().observed_all(version) {
                tx.try_send(()).ok();
                false
            } else {
                true
            }
        });
    }

    /// The core CRDT merge algorithm. Given a set of `(range, new_text)` edits
    /// expressed in the version `version`, rebuilds the fragment tree so that
    /// concurrent insertions are ordered by lamport timestamp and deletions
    /// are recorded on the fragments they cover.
    fn apply_remote_edit(
        &mut self,
        version: &Global,
        ranges: &[Range<FullOffset>],
        new_text: &[Arc<str>],
        timestamp: Lamport,
    ) {
        // 原样搬入 lib.rs 里 `impl Buffer { fn apply_remote_edit(...) { ... } }`
        // 的整个方法体。
        //
        // 涉及的私有字段（现已提升为 pub(crate)）：
        //   self.snapshot.fragments
        //   self.snapshot.visible_text
        //   self.snapshot.deleted_text
        //   self.snapshot.insertions
        //   self.snapshot.insertion_slices
        //   self.snapshot.undo_map
        // 涉及的辅助函数：
        //   RopeBuilder::new / push_fragment / append / finish
        //   FragmentBuilder::new / push / append / summary / to_sum_tree
        //   InsertionFragment::insert_new
        //   InsertionSlice::from_fragment
        //   push_fragments_for_insertion (来自 super::util)
        //   Locator::between / min / max_ref
    }

    /// Given a set of edit ids, returns the ids of the fragments they
    /// touched, sorted by locator. Used by `apply_undo` to iterate only the
    /// fragments affected by a transaction.
    fn fragment_ids_for_edits<'a>(
        &'a self,
        edit_ids: impl Iterator<Item = &'a Lamport>,
    ) -> Vec<&'a Locator> {
        // Get all of the insertion slices changed by the given edits.
        let mut insertion_slices = Vec::new();
        for edit_id in edit_ids {
            let insertion_slice = InsertionSlice {
                edit_id_value: edit_id.value,
                edit_id_replica_id: edit_id.replica_id,
                insertion_id_value: Lamport::MIN.value,
                insertion_id_replica_id: Lamport::MIN.replica_id,
                range: 0..0,
            };
            let slices = self
                .snapshot
                .insertion_slices
                .iter_from(&insertion_slice)
                .take_while(|slice| {
                    Lamport {
                        value: slice.edit_id_value,
                        replica_id: slice.edit_id_replica_id,
                    } == *edit_id
                });
            insertion_slices.extend(slices)
        }
        insertion_slices.sort_unstable_by_key(|s| {
            (
                Lamport {
                    value: s.insertion_id_value,
                    replica_id: s.insertion_id_replica_id,
                },
                s.range.start,
                Reverse(s.range.end),
            )
        });

        // Get all of the fragments corresponding to these insertion slices.
        let mut fragment_ids = Vec::new();
        let mut insertions_cursor = self.insertions.cursor::<InsertionFragmentKey>(());
        for insertion_slice in &insertion_slices {
            let insertion_id = Lamport {
                value: insertion_slice.insertion_id_value,
                replica_id: insertion_slice.insertion_id_replica_id,
            };
            if insertion_id != insertions_cursor.start().timestamp
                || insertion_slice.range.start > insertions_cursor.start().split_offset
            {
                insertions_cursor.seek_forward(
                    &InsertionFragmentKey {
                        timestamp: insertion_id,
                        split_offset: insertion_slice.range.start,
                    },
                    Bias::Left,
                );
            }
            while let Some(item) = insertions_cursor.item() {
                if item.timestamp != insertion_id || item.split_offset >= insertion_slice.range.end
                {
                    break;
                }
                fragment_ids.push(&item.fragment_id);
                insertions_cursor.next();
            }
        }
        fragment_ids.sort_unstable();
        fragment_ids
    }

    /// Reverses the visibility of the fragments recorded in `undo.counts`.
    fn apply_undo(&mut self, undo: &UndoOperation) {
        self.snapshot.undo_map.insert(undo);

        let mut edits = Patch::default();
        let mut old_fragments = self
            .fragments
            .cursor::<Dimensions<Option<&Locator>, usize>>(&None);
        let mut new_fragments = SumTree::new(&None);
        let mut new_ropes =
            RopeBuilder::new(self.visible_text.cursor(0), self.deleted_text.cursor(0));

        for fragment_id in self.fragment_ids_for_edits(undo.counts.keys()) {
            let preceding_fragments = old_fragments.slice(&Some(fragment_id), Bias::Left);
            new_ropes.append(preceding_fragments.summary().text);
            new_fragments.append(preceding_fragments, &None);

            if let Some(fragment) = old_fragments.item() {
                let mut fragment = fragment.clone();
                let fragment_was_visible = fragment.visible;

                fragment.visible = fragment.is_visible(&self.undo_map);
                fragment.max_undos.observe(undo.timestamp);

                let old_start = old_fragments.start().1;
                let new_start = new_fragments.summary().text.visible;
                if fragment_was_visible && !fragment.visible {
                    edits.push(Edit {
                        old: old_start..old_start + fragment.len as usize,
                        new: new_start..new_start,
                    });
                } else if !fragment_was_visible && fragment.visible {
                    edits.push(Edit {
                        old: old_start..old_start,
                        new: new_start..new_start + fragment.len as usize,
                    });
                }
                new_ropes.push_fragment(&fragment, fragment_was_visible);
                new_fragments.push(fragment, &None);

                old_fragments.next();
            }
        }

        let suffix = old_fragments.suffix();
        new_ropes.append(suffix.summary().text);
        new_fragments.append(suffix, &None);

        drop(old_fragments);
        let (visible_text, deleted_text) = new_ropes.finish();
        self.snapshot.fragments = new_fragments;
        self.snapshot.visible_text = visible_text;
        self.snapshot.deleted_text = deleted_text;
        self.subscriptions.publish_mut(&edits);
    }

    fn flush_deferred_ops(&mut self) {
        self.deferred_replicas.clear();
        let mut deferred_ops = Vec::new();
        for op in self.deferred_ops.drain().iter().cloned() {
            if self.can_apply_op(&op) {
                self.apply_op(op);
            } else {
                self.deferred_replicas.insert(op.replica_id());
                deferred_ops.push(op);
            }
        }
        self.deferred_ops.insert(deferred_ops);
    }

    fn can_apply_op(&self, op: &Operation) -> bool {
        if self.deferred_replicas.contains(&op.replica_id()) {
            false
        } else {
            self.version.observed_all(match op {
                Operation::Edit(edit) => &edit.version,
                Operation::Undo(undo) => &undo.version,
            })
        }
    }

    /// Whether any operations are currently deferred.
    pub fn has_deferred_ops(&self) -> bool { !self.deferred_ops.is_empty() }

    /// Returns the transaction at the top of the undo stack, if any.
    pub fn peek_undo_stack(&self) -> Option<&HistoryEntry> { self.history.undo_stack.last() }

    /// Returns the transaction at the top of the redo stack, if any.
    pub fn peek_redo_stack(&self) -> Option<&HistoryEntry> { self.history.redo_stack.last() }

    /// Begins a new transaction. Nested transactions are collapsed into the
    /// outermost one.
    pub fn start_transaction(&mut self) -> Option<TransactionId> {
        self.start_transaction_at(Instant::now())
    }

    /// Like [`Buffer::start_transaction`], but with a caller-supplied time so
    /// that edits occurring close together can be grouped.
    pub fn start_transaction_at(&mut self, now: Instant) -> Option<TransactionId> {
        self.history
            .start_transaction(self.version.clone(), now, &mut self.lamport_clock)
    }

    /// Ends the current transaction, returning its id and starting version if
    /// this was the outermost transaction and it recorded any edits.
    pub fn end_transaction(&mut self) -> Option<(TransactionId, Global)> {
        self.end_transaction_at(Instant::now())
    }

    /// Like [`Buffer::end_transaction`], but with a caller-supplied time.
    pub fn end_transaction_at(&mut self, now: Instant) -> Option<(TransactionId, Global)> {
        if let Some(entry) = self.history.end_transaction(now) {
            let since = entry.transaction.start.clone();
            let id = self.history.group().unwrap();
            Some((id, since))
        } else {
            None
        }
    }

    /// Prevents the most recent transaction from being grouped with future
    /// transactions even if they happen within the group interval.
    pub fn finalize_last_transaction(&mut self) -> Option<&Transaction> {
        self.history.finalize_last_transaction()
    }

    /// Groups the most recent transactions into the transaction with the given
    /// id. Used to fold speculative edits into their originating transaction.
    pub fn group_until_transaction(&mut self, transaction_id: TransactionId) {
        self.history.group_until(transaction_id);
    }

    /// Returns the buffer's original base text (before any edits).
    pub fn base_text(&self) -> &Rope { &self.history.base_text }

    /// Returns the tree of every operation applied to this buffer.
    pub fn operations(&self) -> &TreeMap<Lamport, Operation> { &self.history.operations }

    /// Undoes the most recent transaction.
    pub fn undo(&mut self) -> Option<(TransactionId, Operation)> {
        if let Some(entry) = self.history.pop_undo() {
            let transaction = entry.transaction.clone();
            let transaction_id = transaction.id;
            let op = self.undo_or_redo(transaction);
            Some((transaction_id, op))
        } else {
            None
        }
    }

    /// Undoes a specific transaction by id, if present in the undo stack.
    pub fn undo_transaction(&mut self, transaction_id: TransactionId) -> Option<Operation> {
        let transaction = self
            .history
            .remove_from_undo(transaction_id)?
            .transaction
            .clone();
        Some(self.undo_or_redo(transaction))
    }

    /// Undoes every transaction after the given one (inclusive), returning the
    /// resulting operations.
    pub fn undo_to_transaction(&mut self, transaction_id: TransactionId) -> Vec<Operation> {
        let transactions = self
            .history
            .remove_from_undo_until(transaction_id)
            .iter()
            .map(|entry| entry.transaction.clone())
            .collect::<Vec<_>>();

        transactions
            .into_iter()
            .map(|transaction| self.undo_or_redo(transaction))
            .collect()
    }

    /// Removes a transaction from the undo/redo history without applying it.
    pub fn forget_transaction(&mut self, transaction_id: TransactionId) -> Option<Transaction> {
        self.history.forget(transaction_id)
    }

    /// Returns the transaction with the given id, whether it's in the undo or
    /// redo stack.
    pub fn get_transaction(&self, transaction_id: TransactionId) -> Option<&Transaction> {
        self.history.transaction(transaction_id)
    }

    /// Merges `transaction` into `destination`, moving all edit ids.
    pub fn merge_transactions(&mut self, transaction: TransactionId, destination: TransactionId) {
        self.history.merge_transactions(transaction, destination);
    }

    /// Redoes the most recently undone transaction.
    pub fn redo(&mut self) -> Option<(TransactionId, Operation)> {
        if let Some(entry) = self.history.pop_redo() {
            let transaction = entry.transaction.clone();
            let transaction_id = transaction.id;
            let op = self.undo_or_redo(transaction);
            Some((transaction_id, op))
        } else {
            None
        }
    }

    /// Redoes every transaction up to and including the given one.
    pub fn redo_to_transaction(&mut self, transaction_id: TransactionId) -> Vec<Operation> {
        let transactions = self
            .history
            .remove_from_redo(transaction_id)
            .iter()
            .map(|entry| entry.transaction.clone())
            .collect::<Vec<_>>();

        transactions
            .into_iter()
            .map(|transaction| self.undo_or_redo(transaction))
            .collect()
    }

    pub(crate) fn undo_or_redo(&mut self, transaction: Transaction) -> Operation {
        let mut counts = HashMap::default();
        for edit_id in transaction.edit_ids {
            counts.insert(edit_id, self.undo_map.undo_count(edit_id).saturating_add(1));
        }

        let operation = self.undo_operations(counts);
        self.history.push(operation.clone());
        operation
    }

    /// Issues an undo operation that toggles the visibility of the fragments
    /// produced by the given edit ids, according to `counts`.
    pub fn undo_operations(&mut self, counts: HashMap<Lamport, u32>) -> Operation {
        let timestamp = self.lamport_clock.tick();
        let version = self.version();
        self.snapshot.version.observe(timestamp);
        let undo = UndoOperation {
            timestamp,
            version,
            counts,
        };
        self.apply_undo(&undo);
        Operation::Undo(undo)
    }

    /// Manually pushes a transaction onto the undo stack. Clears the redo
    /// stack.
    pub fn push_transaction(&mut self, transaction: Transaction, now: Instant) {
        self.history.push_transaction(transaction, now);
    }

    /// Pushes an empty parent transaction onto the undo stack without clearing
    /// the redo stack. Used to build composite transactions; the caller must
    /// `forget_transaction` it if nothing is merged in.
    pub fn push_empty_transaction(&mut self, now: Instant) -> TransactionId {
        self.history
            .push_empty_transaction(self.version.clone(), now, &mut self.lamport_clock)
    }

    /// Returns the ranges edited by the transaction with the given id,
    /// expressed in the given `TextDimension`.
    pub fn edited_ranges_for_transaction_id<D>(
        &self,
        transaction_id: TransactionId,
    ) -> impl '_ + Iterator<Item = Range<D>>
    where
        D: TextDimension,
    {
        self.history
            .transaction(transaction_id)
            .into_iter()
            .flat_map(|transaction| self.edited_ranges_for_transaction(transaction))
    }

    /// Returns the ranges edited by the given edit ids, coalescing adjacent
    /// ranges.
    pub fn edited_ranges_for_edit_ids<'a, D>(
        &'a self,
        edit_ids: impl IntoIterator<Item = &'a Lamport>,
    ) -> impl 'a + Iterator<Item = Range<D>>
    where
        D: TextDimension,
    {
        // get fragment ranges
        let mut cursor = self
            .fragments
            .cursor::<Dimensions<Option<&Locator>, usize>>(&None);
        let offset_ranges = self
            .fragment_ids_for_edits(edit_ids.into_iter())
            .into_iter()
            .filter_map(move |fragment_id| {
                cursor.seek_forward(&Some(fragment_id), Bias::Left);
                let fragment = cursor.item()?;
                let start_offset = cursor.start().1;
                let end_offset = start_offset
                    + if fragment.visible {
                        fragment.len as usize
                    } else {
                        0
                    };
                Some(start_offset..end_offset)
            });

        // combine adjacent ranges
        let mut prev_range: Option<Range<usize>> = None;
        let disjoint_ranges = offset_ranges
            .map(Some)
            .chain([None])
            .filter_map(move |range| {
                if let Some((range, prev_range)) = range.as_ref().zip(prev_range.as_mut())
                    && prev_range.end == range.start
                {
                    prev_range.end = range.end;
                    return None;
                }
                let result = prev_range.clone();
                prev_range = range;
                result
            });

        // convert to the desired text dimension.
        let mut position = D::zero(());
        let mut rope_cursor = self.visible_text.cursor(0);
        disjoint_ranges.map(move |range| {
            position.add_assign(&rope_cursor.summary(range.start));
            let start = position;
            position.add_assign(&rope_cursor.summary(range.end));
            let end = position;
            start..end
        })
    }

    /// Returns the ranges edited by the given transaction, expressed in the
    /// given `TextDimension`.
    pub fn edited_ranges_for_transaction<'a, D>(
        &'a self,
        transaction: &'a Transaction,
    ) -> impl 'a + Iterator<Item = Range<D>>
    where
        D: TextDimension,
    {
        self.edited_ranges_for_edit_ids(&transaction.edit_ids)
    }

    /// Subscribes to text edit patches. Each patch contains a sequence of
    /// `(old_range, new_range)` pairs in byte offsets, describing the diff
    /// between the pre-edit and post-edit text.
    pub fn subscribe(&mut self) -> Subscription<usize> { self.subscriptions.subscribe() }

    /// Returns a future that resolves once all the given edit ids have been
    /// observed by this buffer.
    pub fn wait_for_edits<It: IntoIterator<Item = Lamport>>(
        &mut self,
        edit_ids: It,
    ) -> impl 'static + Future<Output = Result<()>> + use<It> {
        let mut futures = Vec::new();
        for edit_id in edit_ids {
            if !self.version.observed(edit_id) {
                let (tx, rx) = oneshot::channel();
                self.edit_id_resolvers.entry(edit_id).or_default().push(tx);
                futures.push(rx);
            }
        }

        async move {
            for mut future in futures {
                if future.recv().await.is_none() {
                    anyhow::bail!("gave up waiting for edits");
                }
            }
            Ok(())
        }
    }

    /// Returns a future that resolves once all the given anchors can be
    /// resolved by this buffer.
    pub fn wait_for_anchors<It: IntoIterator<Item = Anchor>>(
        &mut self,
        anchors: It,
    ) -> impl 'static + Future<Output = Result<()>> + use<It> {
        let mut futures = Vec::new();
        for anchor in anchors {
            if !self.version.observed(anchor.timestamp()) && !anchor.is_max() && !anchor.is_min() {
                let (tx, rx) = oneshot::channel();
                self.edit_id_resolvers
                    .entry(anchor.timestamp())
                    .or_default()
                    .push(tx);
                futures.push(rx);
            }
        }

        async move {
            for mut future in futures {
                if future.recv().await.is_none() {
                    anyhow::bail!("gave up waiting for anchors");
                }
            }
            Ok(())
        }
    }

    /// Returns a future that resolves once this buffer has observed the given
    /// version.
    pub fn wait_for_version(
        &mut self,
        version: Global,
    ) -> impl Future<Output = Result<()>> + use<> {
        let mut rx = None;
        if !self.snapshot.version.observed_all(&version) {
            let channel = oneshot::channel();
            self.wait_for_version_txs.push((version, channel.0));
            rx = Some(channel.1);
        }
        async move {
            if let Some(mut rx) = rx
                && rx.recv().await.is_none()
            {
                anyhow::bail!("gave up waiting for version");
            }
            Ok(())
        }
    }

    /// Makes every pending `wait_for_*` future resolve with an error.
    pub fn give_up_waiting(&mut self) {
        self.edit_id_resolvers.clear();
        self.wait_for_version_txs.clear();
    }

    fn resolve_edit(&mut self, edit_id: Lamport) {
        for mut tx in self
            .edit_id_resolvers
            .remove(&edit_id)
            .into_iter()
            .flatten()
        {
            tx.try_send(()).ok();
        }
    }

    /// Sets the duration within which consecutive transactions are grouped.
    pub fn set_group_interval(&mut self, group_interval: Duration) {
        self.history.group_interval = group_interval;
    }

    /// Returns a snapshot with the given edits applied on top, without
    /// mutating this buffer. Used for speculative computation.
    pub fn snapshot_with_edits<I, S, T>(&mut self, edits: I) -> EditedBufferSnapshot
    where
        I: IntoIterator<Item = (Range<S>, T)>,
        S: ToOffset,
        T: Into<Arc<str>>,
    {
        let mut snapshot = self.snapshot.clone();
        let base_version = self.version();
        let edits: Vec<_> = edits
            .into_iter()
            .map(|(range, new_text)| (range.to_offset(&snapshot), new_text.into()))
            .collect();
        if edits.is_empty() {
            return EditedBufferSnapshot {
                base_version,
                snapshot,
                did_edit: false,
            };
        }
        let timestamp = self.lamport_clock.tick();
        snapshot.apply_edit_internal(edits, timestamp);
        snapshot.version.observe(timestamp);
        EditedBufferSnapshot {
            base_version,
            snapshot,
            did_edit: true,
        }
    }

    /// Adopts an [`EditedBufferSnapshot`] produced by
    /// [`Buffer::snapshot_with_edits`] as this buffer's new state. Panics if
    /// the buffer has been edited since the snapshot was taken.
    pub fn fast_forward(&mut self, edited: EditedBufferSnapshot) {
        if self.version.changed_since(&edited.base_version) {
            panic!("buffer cannot be fast-forwarded")
        }
        self.snapshot = edited.snapshot.clone();
        for timestamp in edited.snapshot.version.iter() {
            self.lamport_clock.observe(timestamp);
        }
    }
}

impl Deref for Buffer {
    type Target = BufferSnapshot;

    fn deref(&self) -> &Self::Target { &self.snapshot }
}
