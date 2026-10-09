use crate::Edit;
// `debug_ranges` 只在 debug 下存在（`buffer/mod.rs` 里 `#[cfg(debug_assertions)] pub mod`），
// 用到它的 `debug` / `debug_with_key` 同样有这个门控。release 下这里不导入，
// 否则 `cargo build --release` 会报 E0432。
#[cfg(debug_assertions)]
use crate::buffer::debug_ranges as debug;
use crate::buffer::fragment::FragmentBuilder;
use crate::locator::Locator;
use crate::patch::Patch;
use crate::undo_map::UndoMap;
use crate::{anchor::Anchor, buffer::LineIndent};
use clock::{Global, ReplicaId};
use rope::{Chunks, OffsetUtf16, Point, PointUtf16, Rope, TextDimension, TextSummary, Unclipped};
use std::{
    borrow::Cow,
    cmp::{self, Ordering, Reverse},
    fmt::Display,
    future::Future,
    iter::Iterator,
    num::NonZeroU64,
    ops::{self, Deref, Range, Sub},
    str,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use sum_tree::{Bias, Dimensions, SumTree, TreeMap, TreeSet};
use util::debug_panic;

use super::buffer_id::BufferId;
use super::dimensions::{FullOffset, VersionedFullOffset};
use super::edits_iter::Edits;
use super::fragment::{
    Fragment, FragmentSummary, FragmentTextSummary, InsertionFragment, InsertionFragmentKey,
    InsertionSlice,
};
use super::history::Transaction;
use super::line_ending::{LineEnding, chunks_with_line_ending};
use super::offset_traits::ToOffset;
use super::operation::{EditOperation, Operation, UndoOperation};
use super::rope_builder::RopeBuilder;
use super::util::push_fragments_for_insertion;
use super::{BufferId as _BufId, TransactionId};

#[derive(Clone)]
pub struct BufferSnapshot {
    pub(crate) visible_text: Rope,
    pub(crate) deleted_text: Rope,
    pub(crate) fragments: SumTree<Fragment>,
    pub(crate) insertions: SumTree<InsertionFragment>,
    pub(crate) insertion_slices: TreeSet<InsertionSlice>,
    pub(crate) undo_map: UndoMap,
    pub version: Global,
    pub(crate) remote_id: BufferId,
    pub(crate) replica_id: clock::ReplicaId,
    pub(crate) line_ending: LineEnding,
}

impl BufferSnapshot {
    pub(crate) fn apply_edit_internal(
        &mut self,
        edits: Vec<(Range<usize>, Arc<str>)>,
        timestamp: clock::Lamport,
    ) -> (EditOperation, Patch<usize>) {
        let mut edits_patch = Patch::default();
        let mut edit_op = EditOperation {
            timestamp,
            version: self.version.clone(),
            ranges: Vec::with_capacity(edits.len()),
            new_text: Vec::with_capacity(edits.len()),
        };
        let mut new_insertions = Vec::new();
        let mut insertion_offset: u32 = 0;
        let mut insertion_slices = Vec::new();

        let mut edits = edits.into_iter().peekable();

        if edits.peek().is_none() {
            return (edit_op, edits_patch);
        }

        let mut new_ropes =
            RopeBuilder::new(self.visible_text.cursor(0), self.deleted_text.cursor(0));
        let mut old_fragments = self.fragments.cursor::<FragmentTextSummary>(&None);
        let mut new_fragments =
            FragmentBuilder::new(old_fragments.slice(&edits.peek().unwrap().0.start, Bias::Right));
        new_ropes.append(new_fragments.summary().text);

        let mut fragment_start = old_fragments.start().visible;
        for (range, new_text) in edits {
            let new_text: Arc<str> = LineEnding::normalize_arc(new_text);
            let fragment_end = old_fragments.end().visible;

            if fragment_end < range.start {
                if fragment_start > old_fragments.start().visible {
                    if fragment_end > fragment_start {
                        let mut suffix = old_fragments.item().unwrap().clone();
                        suffix.len = (fragment_end - fragment_start) as u32;
                        suffix.insertion_offset +=
                            (fragment_start - old_fragments.start().visible) as u32;
                        new_insertions.push(InsertionFragment::insert_new(&suffix));
                        new_ropes.push_fragment(&suffix, suffix.visible);
                        new_fragments.push(suffix, &None);
                    }
                    old_fragments.next();
                }

                let slice = old_fragments.slice(&range.start, Bias::Right);
                new_ropes.append(slice.summary().text);
                new_fragments.append(slice, &None);
                fragment_start = old_fragments.start().visible;
            }

            let full_range_start = FullOffset(range.start + old_fragments.start().deleted);

            if fragment_start < range.start {
                let mut prefix = old_fragments.item().unwrap().clone();
                prefix.len = (range.start - fragment_start) as u32;
                prefix.insertion_offset += (fragment_start - old_fragments.start().visible) as u32;
                prefix.id = Locator::between(&new_fragments.summary().max_id, &prefix.id);
                new_insertions.push(InsertionFragment::insert_new(&prefix));
                new_ropes.push_fragment(&prefix, prefix.visible);
                new_fragments.push(prefix, &None);
                fragment_start = range.start;
            }

            if !new_text.is_empty() {
                let new_start = new_fragments.summary().text.visible;

                let next_fragment_id = old_fragments
                    .item()
                    .map_or(Locator::max_ref(), |old_fragment| &old_fragment.id);
                push_fragments_for_insertion(
                    new_text.as_ref(),
                    timestamp,
                    &mut insertion_offset,
                    &mut new_fragments,
                    &mut new_insertions,
                    &mut insertion_slices,
                    &mut new_ropes,
                    next_fragment_id,
                    timestamp,
                );
                edits_patch.push(Edit {
                    old: fragment_start..fragment_start,
                    new: new_start..new_start + new_text.len(),
                });
            }

            while fragment_start < range.end {
                let fragment = old_fragments.item().unwrap();
                let fragment_end = old_fragments.end().visible;
                let mut intersection = fragment.clone();
                let intersection_end = cmp::min(range.end, fragment_end);
                if fragment.visible {
                    intersection.len = (intersection_end - fragment_start) as u32;
                    intersection.insertion_offset +=
                        (fragment_start - old_fragments.start().visible) as u32;
                    intersection.id =
                        Locator::between(&new_fragments.summary().max_id, &intersection.id);
                    intersection.deletions.push(timestamp);
                    intersection.visible = false;
                }
                if intersection.len > 0 {
                    if fragment.visible && !intersection.visible {
                        let new_start = new_fragments.summary().text.visible;
                        edits_patch.push(Edit {
                            old: fragment_start..intersection_end,
                            new: new_start..new_start,
                        });
                        insertion_slices
                            .push(InsertionSlice::from_fragment(timestamp, &intersection));
                    }
                    new_insertions.push(InsertionFragment::insert_new(&intersection));
                    new_ropes.push_fragment(&intersection, fragment.visible);
                    new_fragments.push(intersection, &None);
                    fragment_start = intersection_end;
                }
                if fragment_end <= range.end {
                    old_fragments.next();
                }
            }

            let full_range_end = FullOffset(range.end + old_fragments.start().deleted);
            edit_op.ranges.push(full_range_start..full_range_end);
            edit_op.new_text.push(new_text);
        }

        if fragment_start > old_fragments.start().visible {
            let fragment_end = old_fragments.end().visible;
            if fragment_end > fragment_start {
                let mut suffix = old_fragments.item().unwrap().clone();
                suffix.len = (fragment_end - fragment_start) as u32;
                suffix.insertion_offset += (fragment_start - old_fragments.start().visible) as u32;
                new_insertions.push(InsertionFragment::insert_new(&suffix));
                new_ropes.push_fragment(&suffix, suffix.visible);
                new_fragments.push(suffix, &None);
            }
            old_fragments.next();
        }

        let suffix = old_fragments.suffix();
        new_ropes.append(suffix.summary().text);
        new_fragments.append(suffix, &None);
        let (visible_text, deleted_text) = new_ropes.finish();
        drop(old_fragments);

        self.fragments = new_fragments.to_sum_tree(&None);
        self.insertions.edit(new_insertions, ());
        self.visible_text = visible_text;
        self.deleted_text = deleted_text;
        self.insertion_slices.extend(insertion_slices);
        (edit_op, edits_patch)
    }

    pub fn as_rope(&self) -> &Rope { &self.visible_text }

    pub fn rope_for_version(&self, version: &clock::Global) -> Rope {
        let mut rope = Rope::new();

        let mut cursor = self
            .fragments
            .filter::<_, FragmentTextSummary>(&None, move |summary| {
                !version.observed_all(&summary.max_version)
            });
        cursor.next();

        let mut visible_cursor = self.visible_text.cursor(0);
        let mut deleted_cursor = self.deleted_text.cursor(0);

        while let Some(fragment) = cursor.item() {
            if cursor.start().visible > visible_cursor.offset() {
                let text = visible_cursor.slice(cursor.start().visible);
                rope.append(text);
            }

            if fragment.was_visible(version, &self.undo_map) {
                if fragment.visible {
                    let text = visible_cursor.slice(cursor.end().visible);
                    rope.append(text);
                } else {
                    deleted_cursor.seek_forward(cursor.start().deleted);
                    let text = deleted_cursor.slice(cursor.end().deleted);
                    rope.append(text);
                }
            } else if fragment.visible {
                visible_cursor.seek_forward(cursor.end().visible);
            }

            cursor.next();
        }

        if cursor.start().visible > visible_cursor.offset() {
            let text = visible_cursor.slice(cursor.start().visible);
            rope.append(text);
        }

        rope
    }

    pub fn remote_id(&self) -> BufferId { self.remote_id }

    pub fn replica_id(&self) -> ReplicaId { self.replica_id }

    pub fn row_count(&self) -> u32 { self.max_point().row + 1 }

    pub fn len(&self) -> usize { self.visible_text.len() }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ { self.chars_at(0) }

    pub fn chars_for_range<T: ToOffset>(&self, range: Range<T>) -> impl Iterator<Item = char> + '_ {
        self.text_for_range(range).flat_map(str::chars)
    }

    pub fn reversed_chars_for_range<T: ToOffset>(
        &self,
        range: Range<T>,
    ) -> impl Iterator<Item = char> + '_ {
        self.reversed_chunks_in_range(range)
            .flat_map(|chunk| chunk.chars().rev())
    }

    pub fn contains_str_at<T>(&self, position: T, needle: &str) -> bool
    where
        T: ToOffset,
    {
        let position = position.to_offset(self);
        position == self.clip_offset(position, Bias::Left)
            && self
                .bytes_in_range(position..self.len())
                .flatten()
                .copied()
                .take(needle.len())
                .eq(needle.bytes())
    }

    pub fn common_prefix_at<T>(&self, position: T, needle: &str) -> Range<T>
    where
        T: ToOffset + TextDimension,
    {
        let offset = position.to_offset(self);
        let common_prefix_len = needle
            .char_indices()
            .map(|(index, _)| index)
            .chain([needle.len()])
            .take_while(|&len| len <= offset)
            .filter(|&len| {
                let left = self
                    .chars_for_range(offset - len..offset)
                    .flat_map(char::to_lowercase);
                let right = needle[..len].chars().flat_map(char::to_lowercase);
                left.eq(right)
            })
            .last()
            .unwrap_or(0);
        let start_offset = offset - common_prefix_len;
        let start = self.text_summary_for_range(0..start_offset);
        start..position
    }

    pub fn text(&self) -> String { self.visible_text.to_string() }

    pub fn text_with_line_endings(&self) -> String {
        chunks_with_line_ending(&self.visible_text, self.line_ending).collect()
    }

    pub fn line_ending(&self) -> LineEnding { self.line_ending }

    pub fn deleted_text(&self) -> String { self.deleted_text.to_string() }

    pub fn text_summary(&self) -> TextSummary { self.visible_text.summary() }

    pub fn max_point(&self) -> Point { self.visible_text.max_point() }

    pub fn max_point_utf16(&self) -> PointUtf16 { self.visible_text.max_point_utf16() }

    pub fn point_to_offset(&self, point: Point) -> usize {
        self.visible_text.point_to_offset(point)
    }

    pub fn point_to_offset_utf16(&self, point: Point) -> OffsetUtf16 {
        self.visible_text.point_to_offset_utf16(point)
    }

    pub fn point_utf16_to_offset_utf16(&self, point: PointUtf16) -> OffsetUtf16 {
        self.visible_text.point_utf16_to_offset_utf16(point)
    }

    pub fn point_utf16_to_offset(&self, point: PointUtf16) -> usize {
        self.visible_text.point_utf16_to_offset(point)
    }

    pub fn unclipped_point_utf16_to_offset(&self, point: Unclipped<PointUtf16>) -> usize {
        self.visible_text.unclipped_point_utf16_to_offset(point)
    }

    pub fn unclipped_point_utf16_to_point(&self, point: Unclipped<PointUtf16>) -> Point {
        self.visible_text.unclipped_point_utf16_to_point(point)
    }

    pub fn offset_utf16_to_offset(&self, offset: OffsetUtf16) -> usize {
        self.visible_text.offset_utf16_to_offset(offset)
    }

    pub fn offset_to_offset_utf16(&self, offset: usize) -> OffsetUtf16 {
        self.visible_text.offset_to_offset_utf16(offset)
    }

    pub fn offset_to_point(&self, offset: usize) -> Point {
        self.visible_text.offset_to_point(offset)
    }

    pub fn offset_to_point_utf16(&self, offset: usize) -> PointUtf16 {
        self.visible_text.offset_to_point_utf16(offset)
    }

    pub fn point_to_point_utf16(&self, point: Point) -> PointUtf16 {
        self.visible_text.point_to_point_utf16(point)
    }

    pub fn point_utf16_to_point(&self, point: PointUtf16) -> Point {
        self.visible_text.point_utf16_to_point(point)
    }

    pub fn version(&self) -> &clock::Global { &self.version }

    pub fn chars_at<T: ToOffset>(&self, position: T) -> impl Iterator<Item = char> + '_ {
        let offset = position.to_offset(self);
        self.visible_text.chars_at(offset)
    }

    pub fn reversed_chars_at<T: ToOffset>(&self, position: T) -> impl Iterator<Item = char> + '_ {
        let offset = position.to_offset(self);
        self.visible_text.reversed_chars_at(offset)
    }

    pub fn reversed_chunks_in_range<T: ToOffset>(&self, range: Range<T>) -> rope::Chunks<'_> {
        let range = range.start.to_offset(self)..range.end.to_offset(self);
        self.visible_text.reversed_chunks_in_range(range)
    }

    pub fn bytes_in_range<T: ToOffset>(&self, range: Range<T>) -> rope::Bytes<'_> {
        let start = range.start.to_offset(self);
        let end = range.end.to_offset(self);
        self.visible_text.bytes_in_range(start..end)
    }

    pub fn reversed_bytes_in_range<T: ToOffset>(&self, range: Range<T>) -> rope::Bytes<'_> {
        let start = range.start.to_offset(self);
        let end = range.end.to_offset(self);
        self.visible_text.reversed_bytes_in_range(start..end)
    }

    pub fn text_for_range<T: ToOffset>(&self, range: Range<T>) -> Chunks<'_> {
        let start = range.start.to_offset(self);
        let end = range.end.to_offset(self);
        self.visible_text.chunks_in_range(start..end)
    }

    pub fn line_len(&self, row: u32) -> u32 {
        let row_start_offset = Point::new(row, 0).to_offset(self);
        let row_end_offset = if row >= self.max_point().row {
            self.len()
        } else {
            Point::new(row + 1, 0).to_previous_offset(self)
        };
        (row_end_offset - row_start_offset) as u32
    }

    /// A function to convert character offsets from e.g. user's `go.mod:22:33` input into byte-offset Point columns.
    pub fn point_from_external_input(&self, row: u32, characters: u32) -> Point {
        const MAX_BYTES_IN_UTF_8: u32 = 4;

        let row = row.min(self.max_point().row);
        let start = Point::new(row, 0);
        let end = self.clip_point(
            Point::new(
                row,
                characters
                    .saturating_mul(MAX_BYTES_IN_UTF_8)
                    .saturating_add(1),
            ),
            Bias::Right,
        );
        let range = start..end;
        let mut point = range.start;
        let mut remaining_columns = characters;

        for chunk in self.text_for_range(range) {
            for character in chunk.chars() {
                if remaining_columns == 0 {
                    return point;
                }
                remaining_columns -= 1;
                point.column += character.len_utf8() as u32;
            }
        }
        point
    }

    pub fn line_indents_in_row_range(
        &self,
        row_range: Range<u32>,
    ) -> impl Iterator<Item = (u32, LineIndent)> + '_ {
        let start = Point::new(row_range.start, 0).to_offset(self);
        let end = Point::new(row_range.end, self.line_len(row_range.end)).to_offset(self);

        let mut chunks = self.as_rope().chunks_in_range(start..end);
        let mut row = row_range.start;
        let mut done = false;
        std::iter::from_fn(move || {
            if done {
                None
            } else {
                let indent = (row, LineIndent::from_chunks(&mut chunks));
                done = !chunks.next_line();
                row += 1;
                Some(indent)
            }
        })
    }

    /// Returns the line indents in the given row range, exclusive of end row, in reversed order.
    pub fn reversed_line_indents_in_row_range(
        &self,
        row_range: Range<u32>,
    ) -> impl Iterator<Item = (u32, LineIndent)> + '_ {
        let start = Point::new(row_range.start, 0).to_offset(self);

        let end_point;
        let end;
        if row_range.end > row_range.start {
            end_point = Point::new(row_range.end - 1, self.line_len(row_range.end - 1));
            end = end_point.to_offset(self);
        } else {
            end_point = Point::new(row_range.start, 0);
            end = start;
        };

        let mut chunks = self.as_rope().chunks_in_range(start..end);
        // Move the cursor to the start of the last line if it's not empty.
        chunks.seek(end);
        if end_point.column > 0 {
            chunks.prev_line();
        }

        let mut row = end_point.row;
        let mut done = false;
        std::iter::from_fn(move || {
            if done {
                None
            } else {
                let initial_offset = chunks.offset();
                let indent = (row, LineIndent::from_chunks(&mut chunks));
                if chunks.offset() > initial_offset {
                    chunks.prev_line();
                }
                done = !chunks.prev_line();
                if !done {
                    row -= 1;
                }

                Some(indent)
            }
        })
    }

    pub fn line_indent_for_row(&self, row: u32) -> LineIndent {
        LineIndent::from_iter(self.chars_at(Point::new(row, 0)))
    }

    pub fn is_line_blank(&self, row: u32) -> bool {
        self.text_for_range(Point::new(row, 0)..Point::new(row, self.line_len(row)))
            .all(|chunk| chunk.matches(|c: char| !c.is_whitespace()).next().is_none())
    }

    pub fn text_summary_for_range<D, O: ToOffset>(&self, range: Range<O>) -> D
    where
        D: TextDimension,
    {
        self.visible_text
            .cursor(range.start.to_offset(self))
            .summary(range.end.to_offset(self))
    }

    pub fn summaries_for_anchors<'a, D, A>(&'a self, anchors: A) -> impl 'a + Iterator<Item = D>
    where
        D: 'a + TextDimension,
        A: 'a + IntoIterator<Item = Anchor>,
    {
        let anchors = anchors.into_iter();
        self.summaries_for_anchors_with_payload::<D, _, ()>(anchors.map(|a| (a, ())))
            .map(|d| d.0)
    }

    pub fn summaries_for_anchors_unordered<'a, D, A>(
        &'a self,
        anchors: A,
    ) -> impl 'a + Iterator<Item = D>
    where
        D: 'a + TextDimension,
        A: 'a + IntoIterator<Item = Anchor>,
    {
        self.summaries_for_anchors_with_payload_impl::<D, _, (), true>(
            anchors.into_iter().map(|anchor| (anchor, ())),
        )
        .map(|(summary, ())| summary)
    }

    pub fn summaries_for_anchors_with_payload<'a, D, A, T>(
        &'a self,
        anchors: A,
    ) -> impl 'a + Iterator<Item = (D, T)>
    where
        D: 'a + TextDimension,
        A: 'a + IntoIterator<Item = (Anchor, T)>,
    {
        self.summaries_for_anchors_with_payload_impl::<D, A, T, false>(anchors)
    }

    fn summaries_for_anchors_with_payload_impl<'a, D, A, T, const ALLOW_BACKWARDS: bool>(
        &'a self,
        anchors: A,
    ) -> impl 'a + Iterator<Item = (D, T)>
    where
        D: 'a + TextDimension,
        A: 'a + IntoIterator<Item = (Anchor, T)>,
    {
        let anchors = anchors.into_iter();
        let mut fragment_cursor = self
            .fragments
            .cursor::<Dimensions<Option<&Locator>, usize>>(&None);
        let mut text_cursor = self.visible_text.cursor(0);
        let mut position = D::zero(());

        anchors.map(move |(anchor, payload)| {
            if anchor.is_min() {
                return (D::zero(()), payload);
            } else if anchor.is_max() {
                return (D::from_text_summary(&self.visible_text.summary()), payload);
            }

            let Some(insertion) = self.try_find_fragment(&anchor) else {
                panic!(
                    "invalid insertion for buffer {}@{:?} with anchor {:?}",
                    self.remote_id(),
                    self.version,
                    anchor
                );
            };
            // TODO verbose debug because we are seeing is_max return false unexpectedly,
            // remove this once that is understood and fixed
            assert_eq!(
                insertion.timestamp,
                anchor.timestamp(),
                "invalid insertion for buffer {}@{:?}. anchor: {:?}, {:?}, {:?}, {:?}, {:?}. timestamp: {:?}, offset: {:?}, bias: {:?}",
                self.remote_id(),
                self.version,
                anchor.timestamp_replica_id,
                anchor.timestamp_value,
                anchor.offset,
                anchor.bias,
                anchor.buffer_id,
                anchor.timestamp() == clock::Lamport::MAX,
                anchor.offset == u32::MAX,
                anchor.bias == Bias::Right,
            );

            let fragment_id = Some(&insertion.fragment_id);
            // The cursor's start locator belongs to the preceding fragment.
            if ALLOW_BACKWARDS && fragment_id <= fragment_cursor.start().0 {
                fragment_cursor.seek(&fragment_id, Bias::Left);
            } else {
                fragment_cursor.seek_forward(&fragment_id, Bias::Left);
            }
            let fragment = fragment_cursor.item().unwrap();
            let mut fragment_offset = fragment_cursor.start().1;
            if fragment.visible {
                fragment_offset += (anchor.offset - insertion.split_offset) as usize;
            }

            if ALLOW_BACKWARDS && fragment_offset < text_cursor.offset() {
                text_cursor = self.visible_text.cursor(0);
                position = D::zero(());
            }
            position.add_assign(&text_cursor.summary(fragment_offset));
            (position, payload)
        })
    }

    pub fn summary_for_anchor<D>(&self, anchor: &Anchor) -> D
    where
        D: TextDimension,
    {
        self.text_summary_for_range(0..self.offset_for_anchor(anchor))
    }

    pub fn offset_for_anchor(&self, anchor: &Anchor) -> usize {
        if anchor.is_min() {
            0
        } else if anchor.is_max() {
            self.visible_text.len()
        } else {
            debug_assert_eq!(anchor.buffer_id, self.remote_id);
            debug_assert!(
                self.version.observed(anchor.timestamp()),
                "Anchor timestamp {:?} not observed by buffer {:?}",
                anchor.timestamp(),
                self.version
            );
            let item = self.try_find_fragment(anchor);
            let Some(insertion) =
                item.filter(|insertion| insertion.timestamp == anchor.timestamp())
            else {
                self.panic_bad_anchor(anchor);
            };

            let (start, _, item) = self
                .fragments
                .find::<Dimensions<Option<&Locator>, usize>, _>(
                    &None,
                    &Some(&insertion.fragment_id),
                    Bias::Left,
                );
            let fragment = item.unwrap();
            let mut fragment_offset = start.1;
            if fragment.visible {
                fragment_offset += (anchor.offset - insertion.split_offset) as usize;
            }
            fragment_offset
        }
    }

    #[cold]
    fn panic_bad_anchor(&self, anchor: &Anchor) -> ! {
        if anchor.buffer_id != self.remote_id {
            panic!(
                "invalid anchor - buffer id does not match: anchor {anchor:?}; buffer id: {}, version: {:?}",
                self.remote_id, self.version
            );
        } else if !self.version.observed(anchor.timestamp()) {
            panic!(
                "invalid anchor - snapshot has not observed lamport: {:?}; version: {:?}",
                anchor, self.version
            );
        } else {
            panic!(
                "invalid anchor {:?}. buffer id: {}, version: {:?}",
                anchor, self.remote_id, self.version
            );
        }
    }

    pub(crate) fn fragment_id_for_anchor(&self, anchor: &Anchor) -> &Locator {
        self.try_fragment_id_for_anchor(anchor)
            .unwrap_or_else(|| self.panic_bad_anchor(anchor))
    }

    pub(crate) fn try_fragment_id_for_anchor(&self, anchor: &Anchor) -> Option<&Locator> {
        if anchor.is_min() {
            Some(Locator::min_ref())
        } else if anchor.is_max() {
            Some(Locator::max_ref())
        } else {
            let item = self.try_find_fragment(anchor);
            item.filter(|insertion| {
                !cfg!(debug_assertions) || insertion.timestamp == anchor.timestamp()
            })
            .map(|insertion| &insertion.fragment_id)
        }
    }

    pub(crate) fn try_find_fragment(&self, anchor: &Anchor) -> Option<&InsertionFragment> {
        let anchor_key = InsertionFragmentKey {
            timestamp: anchor.timestamp(),
            split_offset: anchor.offset,
        };
        match self.insertions.find_with_prev::<InsertionFragmentKey, _>(
            (),
            &anchor_key,
            anchor.bias,
        ) {
            (_, _, Some((prev, insertion))) => {
                let comparison = sum_tree::KeyedItem::key(insertion).cmp(&anchor_key);
                if comparison == Ordering::Greater
                    || (anchor.bias == Bias::Left
                        && comparison == Ordering::Equal
                        && anchor.offset > 0)
                {
                    prev
                } else {
                    Some(insertion)
                }
            }
            _ => self.insertions.last(),
        }
    }

    /// Returns an anchor range for the given input position range that is anchored to the text in the range.
    pub fn anchor_range_inside<T: ToOffset>(&self, position: Range<T>) -> Range<Anchor> {
        self.anchor_after(position.start)..self.anchor_before(position.end)
    }

    /// Returns an anchor range for the given input position range that is anchored to the text before and after.
    pub fn anchor_range_outside<T: ToOffset>(&self, position: Range<T>) -> Range<Anchor> {
        self.anchor_before(position.start)..self.anchor_after(position.end)
    }

    /// Returns an anchor for the given input position that is anchored to the text before the position.
    pub fn anchor_before<T: ToOffset>(&self, position: T) -> Anchor {
        self.anchor_at(position, Bias::Left)
    }

    /// Returns an anchor for the given input position that is anchored to the text after the position.
    pub fn anchor_after<T: ToOffset>(&self, position: T) -> Anchor {
        self.anchor_at(position, Bias::Right)
    }

    pub fn anchor_at<T: ToOffset>(&self, position: T, bias: Bias) -> Anchor {
        self.anchor_at_offset(position.to_offset(self), bias)
    }

    fn anchor_at_offset(&self, mut offset: usize, bias: Bias) -> Anchor {
        if bias == Bias::Left && offset == 0 {
            Anchor::min_for_buffer(self.remote_id)
        } else if bias == Bias::Right
            && ((!cfg!(debug_assertions) && offset >= self.len()) || offset == self.len())
        {
            Anchor::max_for_buffer(self.remote_id)
        } else {
            if !self
                .visible_text
                .assert_char_boundary::<{ cfg!(debug_assertions) }>(offset)
            {
                offset = match bias {
                    Bias::Left => self.visible_text.floor_char_boundary(offset),
                    Bias::Right => self.visible_text.ceil_char_boundary(offset),
                };
            }
            let (start, _, item) = self.fragments.find::<usize, _>(&None, &offset, bias);
            let Some(fragment) = item else {
                // We got a bad offset, likely out of bounds
                debug_panic!(
                    "Failed to find fragment at offset {} (len: {})",
                    offset,
                    self.len()
                );
                return Anchor::max_for_buffer(self.remote_id);
            };
            let overshoot = offset - start;
            Anchor::new(
                fragment.timestamp,
                fragment.insertion_offset + overshoot as u32,
                bias,
                self.remote_id,
            )
        }
    }

    pub fn can_resolve(&self, anchor: &Anchor) -> bool {
        self.remote_id == anchor.buffer_id
            && (anchor.is_min() || anchor.is_max() || self.version.observed(anchor.timestamp()))
    }

    pub fn clip_offset(&self, offset: usize, bias: Bias) -> usize {
        self.visible_text.clip_offset(offset, bias)
    }

    pub fn clip_point(&self, point: Point, bias: Bias) -> Point {
        self.visible_text.clip_point(point, bias)
    }

    pub fn clip_offset_utf16(&self, offset: OffsetUtf16, bias: Bias) -> OffsetUtf16 {
        self.visible_text.clip_offset_utf16(offset, bias)
    }

    pub fn clip_point_utf16(&self, point: Unclipped<PointUtf16>, bias: Bias) -> PointUtf16 {
        self.visible_text.clip_point_utf16(point, bias)
    }

    pub fn edits_since<'a, D>(
        &'a self,
        since: &'a clock::Global,
    ) -> impl 'a + Iterator<Item = Edit<D>>
    where
        D: TextDimension + Ord,
    {
        self.edits_since_in_range(
            since,
            Anchor::min_for_buffer(self.remote_id)..Anchor::max_for_buffer(self.remote_id),
        )
    }

    pub fn anchored_edits_since<'a, D>(
        &'a self,
        since: &'a clock::Global,
    ) -> impl 'a + Iterator<Item = (Edit<D>, Range<Anchor>)>
    where
        D: TextDimension + Ord,
    {
        self.anchored_edits_since_in_range(
            since,
            Anchor::min_for_buffer(self.remote_id)..Anchor::max_for_buffer(self.remote_id),
        )
    }

    pub fn edits_since_in_range<'a, D>(
        &'a self,
        since: &'a clock::Global,
        range: Range<Anchor>,
    ) -> impl 'a + Iterator<Item = Edit<D>>
    where
        D: TextDimension + Ord,
    {
        self.anchored_edits_since_in_range(since, range)
            .map(|item| item.0)
    }

    pub fn anchored_edits_since_in_range<'a, D>(
        &'a self,
        since: &'a clock::Global,
        range: Range<Anchor>,
    ) -> impl 'a + Iterator<Item = (Edit<D>, Range<Anchor>)>
    where
        D: TextDimension + Ord,
    {
        if *since == self.version {
            return None.into_iter().flatten();
        }
        let mut cursor = self.fragments.filter(&None, move |summary| {
            !since.observed_all(&summary.max_version)
        });
        cursor.next();
        let fragments_cursor = Some(cursor);
        let start_fragment_id = self.fragment_id_for_anchor(&range.start);
        let (start, _, item) = self
            .fragments
            .find::<Dimensions<Option<&Locator>, FragmentTextSummary>, _>(
                &None,
                &Some(start_fragment_id),
                Bias::Left,
            );
        let mut visible_start = start.1.visible;
        let mut deleted_start = start.1.deleted;
        if let Some(fragment) = item {
            let overshoot = (range.start.offset - fragment.insertion_offset) as usize;
            if fragment.visible {
                visible_start += overshoot;
            } else {
                deleted_start += overshoot;
            }
        }
        let end_fragment_id = self.fragment_id_for_anchor(&range.end);

        Some(Edits {
            visible_cursor: self.visible_text.cursor(visible_start),
            deleted_cursor: self.deleted_text.cursor(deleted_start),
            fragments_cursor,
            undos: &self.undo_map,
            since,
            old_end: D::zero(()),
            new_end: D::zero(()),
            range: (start_fragment_id, range.start.offset)..(end_fragment_id, range.end.offset),
            buffer_id: self.remote_id,
        })
        .into_iter()
        .flatten()
    }

    pub fn has_edits_since_in_range(&self, since: &clock::Global, range: Range<Anchor>) -> bool {
        if *since != self.version {
            let start_fragment_id = self.fragment_id_for_anchor(&range.start);
            let end_fragment_id = self.fragment_id_for_anchor(&range.end);
            let mut cursor = self.fragments.filter::<_, usize>(&None, move |summary| {
                !since.observed_all(&summary.max_version)
            });
            cursor.next();
            while let Some(fragment) = cursor.item() {
                if fragment.id > *end_fragment_id {
                    break;
                }
                if fragment.id > *start_fragment_id {
                    let was_visible = fragment.was_visible(since, &self.undo_map);
                    let is_visible = fragment.visible;
                    if was_visible != is_visible {
                        return true;
                    }
                }
                cursor.next();
            }
        }
        false
    }

    pub fn has_edits_since(&self, since: &clock::Global) -> bool {
        if *since != self.version {
            let mut cursor = self.fragments.filter::<_, usize>(&None, move |summary| {
                !since.observed_all(&summary.max_version)
            });
            cursor.next();
            while let Some(fragment) = cursor.item() {
                let was_visible = fragment.was_visible(since, &self.undo_map);
                let is_visible = fragment.visible;
                if was_visible != is_visible {
                    return true;
                }
                cursor.next();
            }
        }
        false
    }

    pub fn range_to_version(&self, range: Range<usize>, version: &clock::Global) -> Range<usize> {
        let mut offsets = self.offsets_to_version([range.start, range.end], version);
        offsets.next().unwrap()..offsets.next().unwrap()
    }

    /// Converts the given sequence of offsets into their corresponding offsets
    /// at a prior version of this buffer.
    pub fn offsets_to_version<'a>(
        &'a self,
        offsets: impl 'a + IntoIterator<Item = usize>,
        version: &'a clock::Global,
    ) -> impl 'a + Iterator<Item = usize> {
        let mut edits = self.edits_since(version).peekable();
        let mut last_old_end = 0;
        let mut last_new_end = 0;
        offsets.into_iter().map(move |new_offset| {
            while let Some(edit) = edits.peek() {
                if edit.new.start > new_offset {
                    break;
                }

                if edit.new.end <= new_offset {
                    last_new_end = edit.new.end;
                    last_old_end = edit.old.end;
                    edits.next();
                    continue;
                }

                let overshoot = new_offset - edit.new.start;
                return (edit.old.start + overshoot).min(edit.old.end);
            }

            last_old_end + new_offset.saturating_sub(last_new_end)
        })
    }

    /// Visually annotates a position or range with the `Debug` representation of a value. The
    /// callsite of this function is used as a key - previous annotations will be removed.
    #[cfg(debug_assertions)]
    #[track_caller]
    pub fn debug<R, V>(&self, ranges: &R, value: V)
    where
        R: debug::ToDebugRanges,
        V: std::fmt::Debug,
    {
        self.debug_with_key(std::panic::Location::caller(), ranges, value);
    }

    /// Visually annotates a position or range with the `Debug` representation of a value. Previous
    /// debug annotations with the same key will be removed. The key is also used to determine the
    /// annotation's color.
    #[cfg(debug_assertions)]
    pub fn debug_with_key<K, R, V>(&self, key: &K, ranges: &R, value: V)
    where
        K: std::hash::Hash + 'static,
        R: debug::ToDebugRanges,
        V: std::fmt::Debug,
    {
        let ranges = ranges
            .to_debug_ranges(self)
            .into_iter()
            .map(|range| self.anchor_after(range.start)..self.anchor_before(range.end))
            .collect();
        debug::GlobalDebugRanges::with_locked(|debug_ranges| {
            debug_ranges.insert(key, ranges, format!("{value:?}").into());
        });
    }
}
