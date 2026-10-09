use std::cmp::{self, Ordering, Reverse};
use std::ops::Range;

use smallvec::SmallVec;
use sum_tree::{SumTree, Summary};

use crate::locator::Locator;
use crate::undo_map::UndoMap;

use super::dimensions::{FullOffset, VersionedFullOffset};
use super::rope_builder::RopeBuilder;

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct Fragment {
    pub(crate) id: Locator,
    pub(crate) timestamp: clock::Lamport,
    pub(crate) insertion_offset: u32,
    pub(crate) len: u32,
    pub(crate) visible: bool,
    pub(crate) deletions: SmallVec<[clock::Lamport; 2]>,
    pub(crate) max_undos: clock::Global,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct FragmentSummary {
    pub(crate) text: FragmentTextSummary,
    pub(crate) max_id: Locator,
    pub(crate) max_version: clock::Global,
    pub(crate) min_insertion_version: clock::Global,
    pub(crate) max_insertion_version: clock::Global,
}

#[derive(Copy, Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct FragmentTextSummary {
    pub(crate) visible: usize,
    pub(crate) deleted: usize,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct InsertionFragment {
    pub(crate) timestamp: clock::Lamport,
    pub(crate) split_offset: u32,
    pub(crate) fragment_id: Locator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InsertionFragmentKey {
    pub(crate) timestamp: clock::Lamport,
    pub(crate) split_offset: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InsertionSlice {
    // 内联 lamport 字段以共享对齐
    pub(crate) edit_id_value: clock::Seq,
    pub(crate) edit_id_replica_id: clock::ReplicaId,
    pub(crate) insertion_id_value: clock::Seq,
    pub(crate) insertion_id_replica_id: clock::ReplicaId,
    pub(crate) range: Range<u32>,
}

impl Ord for InsertionSlice {
    fn cmp(&self, other: &Self) -> Ordering {
        clock::Lamport {
            value: self.edit_id_value,
            replica_id: self.edit_id_replica_id,
        }
        .cmp(&clock::Lamport {
            value: other.edit_id_value,
            replica_id: other.edit_id_replica_id,
        })
        .then_with(|| {
            clock::Lamport {
                value: self.insertion_id_value,
                replica_id: self.insertion_id_replica_id,
            }
            .cmp(&clock::Lamport {
                value: other.insertion_id_value,
                replica_id: other.insertion_id_replica_id,
            })
        })
        .then_with(|| self.range.start.cmp(&other.range.start))
        .then_with(|| self.range.end.cmp(&other.range.end))
    }
}

impl PartialOrd for InsertionSlice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl InsertionSlice {
    pub(crate) fn from_fragment(edit_id: clock::Lamport, fragment: &Fragment) -> Self {
        Self {
            edit_id_value: edit_id.value,
            edit_id_replica_id: edit_id.replica_id,
            insertion_id_value: fragment.timestamp.value,
            insertion_id_replica_id: fragment.timestamp.replica_id,
            range: fragment.insertion_offset..fragment.insertion_offset + fragment.len,
        }
    }
}

impl Fragment {
    pub(crate) fn is_visible(&self, undos: &UndoMap) -> bool {
        !undos.is_undone(self.timestamp) && self.deletions.iter().all(|d| undos.is_undone(*d))
    }

    pub(crate) fn was_visible(&self, version: &clock::Global, undos: &UndoMap) -> bool {
        (version.observed(self.timestamp) && !undos.was_undone(self.timestamp, version))
            && self
                .deletions
                .iter()
                .all(|d| !version.observed(*d) || undos.was_undone(*d, version))
    }
}

impl sum_tree::Item for Fragment {
    type Summary = FragmentSummary;

    fn summary(&self, _cx: &Option<clock::Global>) -> Self::Summary {
        let mut max_version = clock::Global::new();
        max_version.observe(self.timestamp);
        for deletion in &self.deletions {
            max_version.observe(*deletion);
        }
        max_version.join(&self.max_undos);

        let mut min_insertion_version = clock::Global::new();
        min_insertion_version.observe(self.timestamp);
        let max_insertion_version = min_insertion_version.clone();
        if self.visible {
            FragmentSummary {
                max_id: self.id.clone(),
                text: FragmentTextSummary {
                    visible: self.len as usize,
                    deleted: 0,
                },
                max_version,
                min_insertion_version,
                max_insertion_version,
            }
        } else {
            FragmentSummary {
                max_id: self.id.clone(),
                text: FragmentTextSummary {
                    visible: 0,
                    deleted: self.len as usize,
                },
                max_version,
                min_insertion_version,
                max_insertion_version,
            }
        }
    }
}

impl sum_tree::Summary for FragmentSummary {
    type Context<'a> = &'a Option<clock::Global>;

    fn zero(_cx: Self::Context<'_>) -> Self { Default::default() }
    fn add_summary(&mut self, other: &Self, _: Self::Context<'_>) {
        self.max_id.assign(&other.max_id);
        self.text.visible += &other.text.visible;
        self.text.deleted += &other.text.deleted;
        self.max_version.join(&other.max_version);
        self.min_insertion_version
            .meet(&other.min_insertion_version);
        self.max_insertion_version
            .join(&other.max_insertion_version);
    }
}

impl Default for FragmentSummary {
    fn default() -> Self {
        FragmentSummary {
            max_id: Locator::min(),
            text: FragmentTextSummary::default(),
            max_version: clock::Global::new(),
            min_insertion_version: clock::Global::new(),
            max_insertion_version: clock::Global::new(),
        }
    }
}

impl sum_tree::Item for InsertionFragment {
    type Summary = InsertionFragmentKey;
    fn summary(&self, _cx: ()) -> Self::Summary {
        InsertionFragmentKey {
            timestamp: self.timestamp,
            split_offset: self.split_offset,
        }
    }
}

impl sum_tree::KeyedItem for InsertionFragment {
    type Key = InsertionFragmentKey;
    fn key(&self) -> Self::Key { sum_tree::Item::summary(self, ()) }
}

impl InsertionFragment {
    pub(crate) fn new(fragment: &Fragment) -> Self {
        Self {
            timestamp: fragment.timestamp,
            split_offset: fragment.insertion_offset,
            fragment_id: fragment.id.clone(),
        }
    }

    pub(crate) fn insert_new(fragment: &Fragment) -> sum_tree::Edit<Self> {
        sum_tree::Edit::Insert(Self::new(fragment))
    }
}

impl sum_tree::ContextLessSummary for InsertionFragmentKey {
    fn zero() -> Self {
        InsertionFragmentKey {
            timestamp: clock::Lamport::MIN,
            split_offset: 0,
        }
    }
    fn add_summary(&mut self, summary: &Self) { *self = *summary; }
}

/// `FragmentBuilder` / `FragmentChunk`
pub(crate) enum FragmentChunk {
    Tree(SumTree<Fragment>),
    Loose(Vec<Fragment>),
}

pub(crate) struct FragmentBuilder {
    pub(crate) chunks: Vec<FragmentChunk>,
    pub(crate) summary: FragmentSummary,
}

impl FragmentBuilder {
    pub(crate) fn new(init: SumTree<Fragment>) -> Self {
        let summary = init.summary().clone();
        let mut chunks = Vec::new();
        if !init.is_empty() {
            chunks.push(FragmentChunk::Tree(init));
        }
        Self { chunks, summary }
    }
    pub(crate) fn append(&mut self, items: SumTree<Fragment>, cx: &Option<clock::Global>) {
        if !items.is_empty() {
            self.summary.add_summary(items.summary(), cx);
            self.chunks.push(FragmentChunk::Tree(items));
        }
    }
    pub(crate) fn push(&mut self, fragment: Fragment, cx: &Option<clock::Global>) {
        self.summary
            .add_summary(&sum_tree::Item::summary(&fragment, cx), cx);
        match self.chunks.last_mut() {
            Some(FragmentChunk::Loose(fragments)) => fragments.push(fragment),
            _ => self.chunks.push(FragmentChunk::Loose(vec![fragment])),
        }
    }
    pub(crate) fn to_sum_tree(self, cx: &Option<clock::Global>) -> SumTree<Fragment> {
        // Appending a `Tree` chunk only touches the right spine and grafts the
        // subtree by cloning `Arc`s, so the untouched regions stay shared with
        // the previous fragment tree. `Loose` runs (newly inserted or rewritten
        // fragments) are built in one pass, parallelizing the large ones.
        let mut tree = SumTree::new(cx);
        for chunk in self.chunks {
            match chunk {
                FragmentChunk::Tree(subtree) => tree.append(subtree, cx),
                FragmentChunk::Loose(fragments) => {
                    if fragments.len() > 1024 {
                        tree.append(SumTree::from_par_iter(fragments, cx), cx);
                    } else {
                        tree.append(SumTree::from_iter(fragments, cx), cx);
                    }
                }
            }
        }
        tree
    }
    pub(crate) fn summary(&self) -> &FragmentSummary { &self.summary }
}

// 下面这些 impl 之前被分散在 lib.rs 顶部与底部，现在集中到 fragment.rs
impl<'a> sum_tree::Dimension<'a, FragmentSummary> for FragmentTextSummary {
    fn zero(_: &Option<clock::Global>) -> Self { Default::default() }
    fn add_summary(&mut self, summary: &'a FragmentSummary, _: &Option<clock::Global>) {
        self.visible += summary.text.visible;
        self.deleted += summary.text.deleted;
    }
}
