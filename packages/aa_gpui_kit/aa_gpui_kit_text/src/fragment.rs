use sum_tree::SumTree;

use super::*;

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct Fragment {
    id: Locator,
    timestamp: clock::Lamport,
    insertion_offset: u32,
    len: u32,
    visible: bool,
    deletions: SmallVec<[clock::Lamport; 2]>,
    max_undos: clock::Global,
}

/// A chunk of fragments accumulated by [`FragmentBuilder`]. `Tree` chunks are
/// subtrees sliced off the previous fragment tree and are kept intact so they
/// continue to share nodes with it; `Loose` chunks batch individually pushed
/// fragments so they can be turned into a subtree in one shot.
pub(crate) enum FragmentChunk {
    Tree(SumTree<Fragment>),
    Loose(Vec<Fragment>),
}
pub(crate) struct FragmentBuilder {
    chunks: Vec<FragmentChunk>,
    summary: FragmentSummary,
}

impl FragmentBuilder {
    fn new(init: SumTree<Fragment>) -> Self {
        let summary = init.summary().clone();
        let mut chunks = Vec::new();
        if !init.is_empty() {
            chunks.push(FragmentChunk::Tree(init));
        }
        Self { chunks, summary }
    }
    fn append(&mut self, items: SumTree<Fragment>, cx: &Option<clock::Global>) {
        if !items.is_empty() {
            self.summary.add_summary(items.summary(), cx);
            self.chunks.push(FragmentChunk::Tree(items));
        }
    }
    fn push(&mut self, fragment: Fragment, cx: &Option<clock::Global>) {
        self.summary
            .add_summary(&sum_tree::Item::summary(&fragment, cx), cx);
        match self.chunks.last_mut() {
            Some(FragmentChunk::Loose(fragments)) => fragments.push(fragment),
            _ => self.chunks.push(FragmentChunk::Loose(vec![fragment])),
        }
    }
    fn to_sum_tree(self, cx: &Option<clock::Global>) -> SumTree<Fragment> {
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
    fn summary(&self) -> &FragmentSummary { &self.summary }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct FragmentSummary {
    text: FragmentTextSummary,
    max_id: Locator,
    max_version: clock::Global,
    min_insertion_version: clock::Global,
    max_insertion_version: clock::Global,
}

#[derive(Copy, Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct FragmentTextSummary {
    visible: usize,
    deleted: usize,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) struct InsertionFragment {
    timestamp: clock::Lamport,
    split_offset: u32,
    fragment_id: Locator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InsertionFragmentKey {
    timestamp: clock::Lamport,
    split_offset: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InsertionSlice {
    // Inline the lamports to allow the replica ids to share the same alignment
    // saving 4 bytes space edit_id: clock::Lamport,
    edit_id_value: clock::Seq,
    edit_id_replica_id: ReplicaId,
    // insertion_id: clock::Lamport,
    insertion_id_value: clock::Seq,
    insertion_id_replica_id: ReplicaId,
    range: Range<u32>,
}

impl Ord for InsertionSlice {
    fn cmp(&self, other: &Self) -> Ordering {
        Lamport {
            value: self.edit_id_value,
            replica_id: self.edit_id_replica_id,
        }
        .cmp(&Lamport {
            value: other.edit_id_value,
            replica_id: other.edit_id_replica_id,
        })
        .then_with(|| {
            Lamport {
                value: self.insertion_id_value,
                replica_id: self.insertion_id_replica_id,
            }
            .cmp(&Lamport {
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
    fn from_fragment(edit_id: clock::Lamport, fragment: &Fragment) -> Self {
        Self {
            edit_id_value: edit_id.value,
            edit_id_replica_id: edit_id.replica_id,
            insertion_id_value: fragment.timestamp.value,
            insertion_id_replica_id: fragment.timestamp.replica_id,
            range: fragment.insertion_offset..fragment.insertion_offset + fragment.len,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum VersionedFullOffset {
    Offset(FullOffset),
    Invalid,
}

impl VersionedFullOffset {
    fn full_offset(&self) -> FullOffset {
        if let Self::Offset(position) = self {
            *position
        } else {
            panic!("invalid version")
        }
    }
}

impl Default for VersionedFullOffset {
    fn default() -> Self { Self::Offset(Default::default()) }
}

impl<'a> sum_tree::Dimension<'a, FragmentSummary> for VersionedFullOffset {
    fn zero(_cx: &Option<clock::Global>) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a FragmentSummary, cx: &Option<clock::Global>) {
        if let Self::Offset(offset) = self {
            let version = cx.as_ref().unwrap();
            if version.observed_all(&summary.max_insertion_version) {
                *offset += summary.text.visible + summary.text.deleted;
            } else if version.observed_any(&summary.min_insertion_version) {
                *self = Self::Invalid;
            }
        }
    }
}

impl sum_tree::SeekTarget<'_, FragmentSummary, Self> for VersionedFullOffset {
    fn cmp(&self, cursor_position: &Self, _: &Option<clock::Global>) -> cmp::Ordering {
        match (self, cursor_position) {
            (Self::Offset(a), Self::Offset(b)) => Ord::cmp(a, b),
            (Self::Offset(_), Self::Invalid) => cmp::Ordering::Less,
            (Self::Invalid, _) => unreachable!(),
        }
    }
}
