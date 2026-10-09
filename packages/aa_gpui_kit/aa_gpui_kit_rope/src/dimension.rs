use super::summary::{ChunkSummary, TextSummary};
use super::*;

use std::cmp;
use std::ops;

use sum_tree::{Dimension, Dimensions};

use crate::chunk::ChunkSlice;

pub trait TextDimension:
    'static + Clone + Copy + Default + for<'a> Dimension<'a, ChunkSummary> + std::fmt::Debug
{
    fn from_text_summary(summary: &TextSummary) -> Self;
    fn from_chunk(chunk: ChunkSlice) -> Self;
    fn add_assign(&mut self, other: &Self);
}

impl<D1: TextDimension, D2: TextDimension> TextDimension for Dimensions<D1, D2, ()> {
    fn from_text_summary(summary: &TextSummary) -> Self {
        Dimensions(
            D1::from_text_summary(summary),
            D2::from_text_summary(summary),
            (),
        )
    }

    fn from_chunk(chunk: ChunkSlice) -> Self {
        Dimensions(D1::from_chunk(chunk), D2::from_chunk(chunk), ())
    }

    fn add_assign(&mut self, other: &Self) {
        self.0.add_assign(&other.0);
        self.1.add_assign(&other.1);
    }
}

impl<'a> sum_tree::Dimension<'a, ChunkSummary> for TextSummary {
    fn zero(_cx: ()) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a ChunkSummary, _: ()) { *self += &summary.text; }
}

impl TextDimension for TextSummary {
    fn from_text_summary(summary: &TextSummary) -> Self { *summary }

    fn from_chunk(chunk: ChunkSlice) -> Self { chunk.text_summary() }

    fn add_assign(&mut self, other: &Self) { *self += other; }
}

impl<'a> sum_tree::Dimension<'a, ChunkSummary> for usize {
    fn zero(_cx: ()) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a ChunkSummary, _: ()) { *self += summary.text.len; }
}

impl TextDimension for usize {
    fn from_text_summary(summary: &TextSummary) -> Self { summary.len }

    fn from_chunk(chunk: ChunkSlice) -> Self { chunk.len() }

    fn add_assign(&mut self, other: &Self) { *self += other; }
}

impl<'a> sum_tree::Dimension<'a, ChunkSummary> for OffsetUtf16 {
    fn zero(_cx: ()) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a ChunkSummary, _: ()) { *self += summary.text.len_utf16; }
}

impl TextDimension for OffsetUtf16 {
    fn from_text_summary(summary: &TextSummary) -> Self { summary.len_utf16 }

    fn from_chunk(chunk: ChunkSlice) -> Self { chunk.len_utf16() }

    fn add_assign(&mut self, other: &Self) { *self += other; }
}

impl<'a> sum_tree::Dimension<'a, ChunkSummary> for Point {
    fn zero(_cx: ()) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a ChunkSummary, _: ()) { *self += summary.text.lines; }
}

impl TextDimension for Point {
    fn from_text_summary(summary: &TextSummary) -> Self { summary.lines }

    fn from_chunk(chunk: ChunkSlice) -> Self { chunk.lines() }

    fn add_assign(&mut self, other: &Self) { *self += other; }
}

impl<'a> sum_tree::Dimension<'a, ChunkSummary> for PointUtf16 {
    fn zero(_cx: ()) -> Self { Default::default() }

    fn add_summary(&mut self, summary: &'a ChunkSummary, _: ()) {
        *self += summary.text.lines_utf16();
    }
}

impl TextDimension for PointUtf16 {
    fn from_text_summary(summary: &TextSummary) -> Self { summary.lines_utf16() }

    fn from_chunk(chunk: ChunkSlice) -> Self {
        PointUtf16 {
            row: chunk.lines().row,
            column: chunk.last_line_len_utf16(),
        }
    }

    fn add_assign(&mut self, other: &Self) { *self += other; }
}

/// A pair of text dimensions in which only the first dimension is used for comparison,
/// but both dimensions are updated during addition and subtraction.
#[derive(Clone, Copy, Debug)]
pub struct DimensionPair<K, V> {
    pub key: K,
    pub value: Option<V>,
}

impl<K: Default, V: Default> Default for DimensionPair<K, V> {
    fn default() -> Self {
        Self {
            key: Default::default(),
            value: Some(Default::default()),
        }
    }
}

impl<K, V> cmp::Ord for DimensionPair<K, V>
where
    K: cmp::Ord,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering { self.key.cmp(&other.key) }
}

impl<K, V> cmp::PartialOrd for DimensionPair<K, V>
where
    K: cmp::PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.key.partial_cmp(&other.key)
    }
}

impl<K, V> cmp::PartialEq for DimensionPair<K, V>
where
    K: cmp::PartialEq,
{
    fn eq(&self, other: &Self) -> bool { self.key.eq(&other.key) }
}

impl<R, R2, K, V> ops::Sub for DimensionPair<K, V>
where
    K: ops::Sub<K, Output = R>,
    V: ops::Sub<V, Output = R2>,
{
    type Output = DimensionPair<R, R2>;

    fn sub(self, rhs: Self) -> Self::Output {
        DimensionPair {
            key: self.key - rhs.key,
            value: self.value.zip(rhs.value).map(|(a, b)| a - b),
        }
    }
}

impl<R, R2, K, V> ops::AddAssign<DimensionPair<R, R2>> for DimensionPair<K, V>
where
    K: ops::AddAssign<R>,
    V: ops::AddAssign<R2>,
{
    fn add_assign(&mut self, rhs: DimensionPair<R, R2>) {
        self.key += rhs.key;
        if let Some(value) = &mut self.value {
            if let Some(other_value) = rhs.value {
                *value += other_value;
            } else {
                self.value.take();
            }
        }
    }
}

impl<D> std::ops::AddAssign<DimensionPair<Point, D>> for Point {
    fn add_assign(&mut self, rhs: DimensionPair<Point, D>) { *self += rhs.key; }
}

impl<K, V> cmp::Eq for DimensionPair<K, V> where K: cmp::Eq {}

impl<'a, K, V, S> sum_tree::Dimension<'a, S> for DimensionPair<K, V>
where
    S: sum_tree::Summary,
    K: sum_tree::Dimension<'a, S>,
    V: sum_tree::Dimension<'a, S>,
{
    fn zero(cx: S::Context<'_>) -> Self {
        Self {
            key: K::zero(cx),
            value: Some(V::zero(cx)),
        }
    }

    fn add_summary(&mut self, summary: &'a S, cx: S::Context<'_>) {
        self.key.add_summary(summary, cx);
        if let Some(value) = &mut self.value {
            value.add_summary(summary, cx);
        }
    }
}

impl<K, V> TextDimension for DimensionPair<K, V>
where
    K: TextDimension,
    V: TextDimension,
{
    fn add_assign(&mut self, other: &Self) {
        self.key.add_assign(&other.key);
        if let Some(value) = &mut self.value {
            if let Some(other_value) = other.value.as_ref() {
                value.add_assign(other_value);
            } else {
                self.value.take();
            }
        }
    }

    fn from_chunk(chunk: ChunkSlice) -> Self {
        Self {
            key: K::from_chunk(chunk),
            value: Some(V::from_chunk(chunk)),
        }
    }

    fn from_text_summary(summary: &TextSummary) -> Self {
        Self {
            key: K::from_text_summary(summary),
            value: Some(V::from_text_summary(summary)),
        }
    }
}
