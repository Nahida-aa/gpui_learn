use super::summary::TextSummary;
use super::*;

use std::cmp;
use std::fmt;
use std::mem;
use std::ops::Range;

use ztracing::instrument;
use heapless::Vec as ArrayVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator as _};
use sum_tree::{Bias, Dimension, Dimensions, SumTree};

use crate::chunk::{Bitmap, Chunk, ChunkSlice};

#[derive(Clone, Default)]
pub struct Rope {
    pub(crate) chunks: SumTree<Chunk>,
}

impl Rope {
    pub fn new() -> Self { Self::default() }

    /// Checks that `index`-th byte is the first byte in a UTF-8 code point
    /// sequence or the end of the string.
    ///
    /// The start and end of the string (when `index == self.len()`) are
    /// considered to be boundaries.
    ///
    /// Returns `false` if `index` is greater than `self.len()`.
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        if self.chunks.is_empty() {
            return offset == 0;
        }
        let (start, _, item) = self.chunks.find::<usize, _>((), &offset, Bias::Left);
        let chunk_offset = offset - start;
        item.map(|chunk| chunk.is_char_boundary(chunk_offset))
            .unwrap_or(false)
    }

    #[track_caller]
    #[inline(always)]
    pub fn assert_char_boundary<const PANIC: bool>(&self, offset: usize) -> bool {
        if self.chunks.is_empty() && offset == 0 {
            return true;
        }
        let (start, _, item) = self.chunks.find::<usize, _>((), &offset, Bias::Left);
        match item {
            Some(chunk) => {
                let chunk_offset = offset - start;
                chunk.assert_char_boundary::<PANIC>(chunk_offset)
            }
            None if PANIC => {
                panic!(
                    "byte index {} is out of bounds of rope (length: {})",
                    offset,
                    self.len()
                );
            }
            None => {
                log::error!(
                    "byte index {} is out of bounds of rope (length: {})",
                    offset,
                    self.len()
                );
                false
            }
        }
    }

    pub fn floor_char_boundary(&self, index: usize) -> usize {
        if index >= self.len() {
            self.len()
        } else {
            let (start, _, item) = self.chunks.find::<usize, _>((), &index, Bias::Left);
            let chunk_offset = index - start;
            let lower_idx = item.map(|chunk| chunk.text.floor_char_boundary(chunk_offset));
            lower_idx.map_or_else(|| self.len(), |idx| start + idx)
        }
    }

    pub fn ceil_char_boundary(&self, index: usize) -> usize {
        if index > self.len() {
            self.len()
        } else {
            let (start, _, item) = self.chunks.find::<usize, _>((), &index, Bias::Left);
            let chunk_offset = index - start;
            let upper_idx = item.map(|chunk| chunk.text.ceil_char_boundary(chunk_offset));
            upper_idx.map_or_else(|| self.len(), |idx| start + idx)
        }
    }

    pub fn append(&mut self, rope: Rope) {
        if let Some(chunk) = rope.chunks.first()
            && (self
                .chunks
                .last()
                .is_some_and(|c| c.text.len() < chunk::MIN_BASE)
                || chunk.text.len() < chunk::MIN_BASE)
        {
            self.push_chunk(chunk.as_slice());

            let mut chunks = rope.chunks.cursor::<()>(());
            chunks.next();
            chunks.next();
            self.chunks.append(chunks.suffix(), ());
        } else {
            self.chunks.append(rope.chunks, ());
        }
        self.check_invariants();
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        let mut new_rope = Rope::new();
        let mut cursor = self.cursor(0);
        new_rope.append(cursor.slice(range.start));
        cursor.seek_forward(range.end);
        new_rope.push(text);
        new_rope.append(cursor.suffix());
        *self = new_rope;
    }

    pub fn slice(&self, range: Range<usize>) -> Rope {
        let mut cursor = self.cursor(0);
        cursor.seek_forward(range.start);
        cursor.slice(range.end)
    }

    pub fn slice_rows(&self, range: Range<u32>) -> Rope {
        // This would be more efficient with a forward advance after the first, but it's fine.
        let start = self.point_to_offset(Point::new(range.start, 0));
        let end = self.point_to_offset(Point::new(range.end, 0));
        self.slice(start..end)
    }

    pub fn push(&mut self, mut text: &str) {
        self.chunks.update_last(
            |last_chunk| {
                let split_ix = if last_chunk.text.len() + text.len() <= chunk::MAX_BASE {
                    text.len()
                } else {
                    let mut split_ix = cmp::min(
                        chunk::MIN_BASE.saturating_sub(last_chunk.text.len()),
                        text.len(),
                    );
                    while !text.is_char_boundary(split_ix) {
                        split_ix += 1;
                    }
                    split_ix
                };

                let (suffix, remainder) = text.split_at(split_ix);
                last_chunk.push_str(suffix);
                text = remainder;
            },
            (),
        );

        if text.is_empty() {
            self.check_invariants();
            return;
        }

        #[cfg(all(test, not(rust_analyzer)))]
        const NUM_CHUNKS: usize = 16;
        #[cfg(not(all(test, not(rust_analyzer))))]
        const NUM_CHUNKS: usize = 4;

        // We accommodate for NUM_CHUNKS chunks of size MAX_BASE
        // but given the chunk boundary can land within a character
        // we need to accommodate for the worst case where every chunk gets cut short by up to 4 bytes
        if text.len() > NUM_CHUNKS * chunk::MAX_BASE - NUM_CHUNKS * 4 {
            return self.push_large(text);
        }
        // 16 is enough as otherwise we will hit the branch above
        let mut new_chunks = ArrayVec::<_, NUM_CHUNKS, u8>::new();

        while !text.is_empty() {
            let mut split_ix = cmp::min(chunk::MAX_BASE, text.len());
            while !text.is_char_boundary(split_ix) {
                split_ix -= 1;
            }
            let (chunk, remainder) = text.split_at(split_ix);
            new_chunks.push(chunk).unwrap();
            text = remainder;
        }
        self.chunks
            .extend(new_chunks.into_iter().map(Chunk::new), ());

        self.check_invariants();
    }

    /// A copy of `push` specialized for working with large quantities of text.
    fn push_large(&mut self, mut text: &str) {
        // To avoid frequent reallocs when loading large swaths of file contents,
        // we estimate worst-case `new_chunks` capacity;
        // Chunk is a fixed-capacity buffer. If a character falls on
        // chunk boundary, we push it off to the following chunk (thus leaving a small bit of capacity unfilled in current chunk).
        // Worst-case chunk count when loading a file is then a case where every chunk ends up with that unused capacity.
        // Since we're working with UTF-8, each character is at most 4 bytes wide. It follows then that the worst case is where
        // a chunk ends with 3 bytes of a 4-byte character. These 3 bytes end up being stored in the following chunk, thus wasting
        // 3 bytes of storage in current chunk.
        // For example, a 1024-byte string can occupy between 32 (full ASCII, 1024/32) and 36 (full 4-byte UTF-8, 1024 / 29 rounded up) chunks.
        const MIN_CHUNK_SIZE: usize = chunk::MAX_BASE - 3;

        // We also round up the capacity up by one, for a good measure; we *really* don't want to realloc here, as we assume that the # of characters
        // we're working with there is large.
        let capacity = text.len().div_ceil(MIN_CHUNK_SIZE);
        let mut new_chunks = Vec::with_capacity(capacity);

        while !text.is_empty() {
            let mut split_ix = cmp::min(chunk::MAX_BASE, text.len());
            while !text.is_char_boundary(split_ix) {
                split_ix -= 1;
            }
            let (chunk, remainder) = text.split_at(split_ix);
            new_chunks.push(chunk);
            text = remainder;
        }

        #[cfg(all(test, not(rust_analyzer)))]
        const PARALLEL_THRESHOLD: usize = 4;
        #[cfg(not(all(test, not(rust_analyzer))))]
        const PARALLEL_THRESHOLD: usize = 84 * (2 * sum_tree::TREE_BASE);

        if new_chunks.len() >= PARALLEL_THRESHOLD {
            self.chunks
                .par_extend(new_chunks.into_par_iter().map(Chunk::new), ());
        } else {
            self.chunks
                .extend(new_chunks.into_iter().map(Chunk::new), ());
        }

        self.check_invariants();
    }

    pub(crate) fn push_chunk(&mut self, mut chunk: ChunkSlice) {
        self.chunks.update_last(
            |last_chunk| {
                let split_ix = if last_chunk.text.len() + chunk.len() <= chunk::MAX_BASE {
                    chunk.len()
                } else {
                    let mut split_ix = cmp::min(
                        chunk::MIN_BASE.saturating_sub(last_chunk.text.len()),
                        chunk.len(),
                    );
                    while !chunk.is_char_boundary(split_ix) {
                        split_ix += 1;
                    }
                    split_ix
                };

                let (suffix, remainder) = chunk.split_at(split_ix);
                last_chunk.append(suffix);
                chunk = remainder;
            },
            (),
        );

        if !chunk.is_empty() {
            self.chunks.push(chunk.into(), ());
        }
    }

    pub fn push_front(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.is_empty() {
            self.push(text);
            return;
        }
        if self
            .chunks
            .first()
            .is_some_and(|c| c.text.len() + text.len() <= chunk::MAX_BASE)
        {
            self.chunks
                .update_first(|first_chunk| first_chunk.prepend_str(text), ());
            self.check_invariants();
            return;
        }
        let suffix = mem::replace(self, Rope::from(text));
        self.append(suffix);
    }

    fn check_invariants(&self) {
        #[cfg(test)]
        {
            // Ensure all chunks except maybe the last one are not underflowing.
            // Allow some wiggle room for multibyte characters at chunk boundaries.
            let mut chunks = self.chunks.cursor::<()>(()).peekable();
            while let Some(chunk) = chunks.next() {
                if chunks.peek().is_some() {
                    assert!(chunk.text.len() + 3 >= chunk::MIN_BASE);
                }
            }
        }
    }

    pub fn summary(&self) -> TextSummary { self.chunks.summary().text }

    pub fn len(&self) -> usize { self.chunks.extent(()) }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn max_point(&self) -> Point { self.chunks.extent(()) }

    pub fn max_point_utf16(&self) -> PointUtf16 { self.chunks.extent(()) }

    pub fn cursor(&self, offset: usize) -> Cursor<'_> { Cursor::new(self, offset) }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ { self.chars_at(0) }

    pub fn chars_at(&self, start: usize) -> impl Iterator<Item = char> + '_ {
        self.chunks_in_range(start..self.len()).flat_map(str::chars)
    }

    pub fn reversed_chars_at(&self, start: usize) -> impl Iterator<Item = char> + '_ {
        self.reversed_chunks_in_range(0..start)
            .flat_map(|chunk| chunk.chars().rev())
    }

    pub fn bytes_in_range(&self, range: Range<usize>) -> Bytes<'_> {
        Bytes::new(self, range, false)
    }

    pub fn reversed_bytes_in_range(&self, range: Range<usize>) -> Bytes<'_> {
        Bytes::new(self, range, true)
    }

    pub fn chunks(&self) -> Chunks<'_> { self.chunks_in_range(0..self.len()) }

    pub fn chunks_in_range(&self, range: Range<usize>) -> Chunks<'_> {
        Chunks::new(self, range, false)
    }

    pub fn reversed_chunks_in_range(&self, range: Range<usize>) -> Chunks<'_> {
        Chunks::new(self, range, true)
    }

    pub fn offset_to_offset_utf16(&self, offset: usize) -> OffsetUtf16 {
        if offset >= self.summary().len {
            return self.summary().len_utf16;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<usize, OffsetUtf16>, _>((), &offset, Bias::Left);
        let overshoot = offset - start.0;
        start.1
            + item.map_or(Default::default(), |chunk| {
                chunk.as_slice().offset_to_offset_utf16(overshoot)
            })
    }

    pub fn offset_utf16_to_offset(&self, offset: OffsetUtf16) -> usize {
        if offset >= self.summary().len_utf16 {
            return self.summary().len;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<OffsetUtf16, usize>, _>((), &offset, Bias::Left);
        let overshoot = offset - start.0;
        start.1
            + item.map_or(Default::default(), |chunk| {
                chunk.as_slice().offset_utf16_to_offset(overshoot)
            })
    }

    pub fn offset_to_point(&self, offset: usize) -> Point {
        if offset >= self.summary().len {
            return self.summary().lines;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<usize, Point>, _>((), &offset, Bias::Left);
        let overshoot = offset - start.0;
        start.1
            + item.map_or(Point::zero(), |chunk| {
                chunk.as_slice().offset_to_point(overshoot)
            })
    }

    pub fn offset_to_point_utf16(&self, offset: usize) -> PointUtf16 {
        if offset >= self.summary().len {
            return self.summary().lines_utf16();
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<usize, PointUtf16>, _>((), &offset, Bias::Left);
        let overshoot = offset - start.0;
        start.1
            + item.map_or(PointUtf16::zero(), |chunk| {
                chunk.as_slice().offset_to_point_utf16(overshoot)
            })
    }

    pub fn point_to_point_utf16(&self, point: Point) -> PointUtf16 {
        if point >= self.summary().lines {
            return self.summary().lines_utf16();
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<Point, PointUtf16>, _>((), &point, Bias::Left);
        let overshoot = point - start.0;
        start.1
            + item.map_or(PointUtf16::zero(), |chunk| {
                chunk.as_slice().point_to_point_utf16(overshoot)
            })
    }

    pub fn point_utf16_to_point(&self, point: PointUtf16) -> Point {
        if point >= self.summary().lines_utf16() {
            return self.summary().lines;
        }
        let mut cursor = self.chunks.cursor::<Dimensions<PointUtf16, Point>>(());
        cursor.seek(&point, Bias::Left);
        let overshoot = point - cursor.start().0;
        cursor.start().1
            + cursor.item().map_or(Point::zero(), |chunk| {
                chunk
                    .as_slice()
                    .offset_to_point(chunk.as_slice().point_utf16_to_offset(overshoot, false))
            })
    }

    #[instrument(skip_all)]
    pub fn point_to_offset(&self, point: Point) -> usize {
        if point >= self.summary().lines {
            return self.summary().len;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<Point, usize>, _>((), &point, Bias::Left);
        let overshoot = point - start.0;
        start.1 + item.map_or(0, |chunk| chunk.as_slice().point_to_offset(overshoot))
    }

    pub fn point_to_offset_utf16(&self, point: Point) -> OffsetUtf16 {
        if point >= self.summary().lines {
            return self.summary().len_utf16;
        }
        let mut cursor = self.chunks.cursor::<Dimensions<Point, OffsetUtf16>>(());
        cursor.seek(&point, Bias::Left);
        let overshoot = point - cursor.start().0;
        cursor.start().1
            + cursor.item().map_or(OffsetUtf16(0), |chunk| {
                chunk.as_slice().point_to_offset_utf16(overshoot)
            })
    }

    pub fn point_utf16_to_offset(&self, point: PointUtf16) -> usize {
        self.point_utf16_to_offset_impl(point, false)
    }

    pub fn point_utf16_to_offset_utf16(&self, point: PointUtf16) -> OffsetUtf16 {
        self.point_utf16_to_offset_utf16_impl(point, false)
    }

    pub fn unclipped_point_utf16_to_offset(&self, point: Unclipped<PointUtf16>) -> usize {
        self.point_utf16_to_offset_impl(point.0, true)
    }

    fn point_utf16_to_offset_impl(&self, point: PointUtf16, clip: bool) -> usize {
        if point >= self.summary().lines_utf16() {
            return self.summary().len;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<PointUtf16, usize>, _>((), &point, Bias::Left);
        let overshoot = point - start.0;
        start.1
            + item.map_or(0, |chunk| {
                chunk.as_slice().point_utf16_to_offset(overshoot, clip)
            })
    }

    fn point_utf16_to_offset_utf16_impl(&self, point: PointUtf16, clip: bool) -> OffsetUtf16 {
        if point >= self.summary().lines_utf16() {
            return self.summary().len_utf16;
        }
        let mut cursor = self
            .chunks
            .cursor::<Dimensions<PointUtf16, OffsetUtf16>>(());
        cursor.seek(&point, Bias::Left);
        let overshoot = point - cursor.start().0;
        cursor.start().1
            + cursor.item().map_or(OffsetUtf16(0), |chunk| {
                chunk
                    .as_slice()
                    .offset_to_offset_utf16(chunk.as_slice().point_utf16_to_offset(overshoot, clip))
            })
    }

    pub fn unclipped_point_utf16_to_point(&self, point: Unclipped<PointUtf16>) -> Point {
        if point.0 >= self.summary().lines_utf16() {
            return self.summary().lines;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<PointUtf16, Point>, _>((), &point.0, Bias::Left);
        let overshoot = Unclipped(point.0 - start.0);
        start.1
            + item.map_or(Point::zero(), |chunk| {
                chunk.as_slice().unclipped_point_utf16_to_point(overshoot)
            })
    }

    pub fn clip_offset(&self, offset: usize, bias: Bias) -> usize {
        match bias {
            Bias::Left => self.floor_char_boundary(offset),
            Bias::Right => self.ceil_char_boundary(offset),
        }
    }

    pub fn clip_offset_utf16(&self, offset: OffsetUtf16, bias: Bias) -> OffsetUtf16 {
        let (start, _, item) = self.chunks.find::<OffsetUtf16, _>((), &offset, Bias::Right);
        if let Some(chunk) = item {
            let overshoot = offset - start;
            start + chunk.as_slice().clip_offset_utf16(overshoot, bias)
        } else {
            self.summary().len_utf16
        }
    }

    pub fn clip_point(&self, point: Point, bias: Bias) -> Point {
        let (start, _, item) = self.chunks.find::<Point, _>((), &point, Bias::Right);
        if let Some(chunk) = item {
            let overshoot = point - start;
            start + chunk.as_slice().clip_point(overshoot, bias)
        } else {
            self.summary().lines
        }
    }

    pub fn clip_point_utf16(&self, point: Unclipped<PointUtf16>, bias: Bias) -> PointUtf16 {
        let (start, _, item) = self.chunks.find::<PointUtf16, _>((), &point.0, Bias::Right);
        if let Some(chunk) = item {
            let overshoot = Unclipped(point.0 - start);
            start + chunk.as_slice().clip_point_utf16(overshoot, bias)
        } else {
            self.summary().lines_utf16()
        }
    }

    pub fn starts_with(&self, pattern: &str) -> bool {
        if pattern.len() > self.len() {
            return false;
        }
        let mut remaining = pattern;
        for chunk in self.chunks_in_range(0..self.len()) {
            let Some(chunk) = chunk.get(..remaining.len().min(chunk.len())) else {
                return false;
            };
            if remaining.starts_with(chunk) {
                remaining = &remaining[chunk.len()..];
                if remaining.is_empty() {
                    return true;
                }
            } else {
                return false;
            }
        }
        remaining.is_empty()
    }

    pub fn ends_with(&self, pattern: &str) -> bool {
        if pattern.len() > self.len() {
            return false;
        }
        let mut remaining = pattern;
        for chunk in self.reversed_chunks_in_range(0..self.len()) {
            let Some(chunk) = chunk.get(chunk.len() - remaining.len().min(chunk.len())..) else {
                return false;
            };
            if remaining.ends_with(chunk) {
                remaining = &remaining[..remaining.len() - chunk.len()];
                if remaining.is_empty() {
                    return true;
                }
            } else {
                return false;
            }
        }
        remaining.is_empty()
    }

    pub fn line_len(&self, row: u32) -> u32 {
        self.clip_point(Point::new(row, u32::MAX), Bias::Left)
            .column
    }
}

impl<'a> From<&'a str> for Rope {
    fn from(text: &'a str) -> Self {
        let mut rope = Self::new();
        rope.push(text);
        rope
    }
}

impl<'a> FromIterator<&'a str> for Rope {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut rope = Rope::new();
        for chunk in iter {
            rope.push(chunk);
        }
        rope
    }
}

impl From<String> for Rope {
    #[inline(always)]
    fn from(text: String) -> Self { Rope::from(text.as_str()) }
}

impl From<&String> for Rope {
    #[inline(always)]
    fn from(text: &String) -> Self { Rope::from(text.as_str()) }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.chunks() {
            write!(f, "{}", chunk)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Write as _;

        write!(f, "\"")?;
        let mut format_string = String::new();
        for chunk in self.chunks() {
            write!(&mut format_string, "{:?}", chunk)?;
            write!(f, "{}", &format_string[1..format_string.len() - 1])?;
            format_string.clear();
        }
        write!(f, "\"")?;
        Ok(())
    }
}
