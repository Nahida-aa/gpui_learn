use super::*;
use crate::{OffsetUtf16, Point, PointUtf16, TextSummary, Unclipped};
use std::cmp;
use std::ops::Range;
use sum_tree::Bias;
use unicode_segmentation::GraphemeCursor;
use util::debug_panic;

#[derive(Clone, Copy, Debug)]
pub struct ChunkSlice<'a> {
    pub(crate) chars: Bitmap,
    pub(crate) chars_utf16: Bitmap,
    pub(crate) newlines: Bitmap,
    pub(crate) tabs: Bitmap,
    pub(crate) text: &'a str,
}

impl Into<Chunk> for ChunkSlice<'_> {
    fn into(self) -> Chunk {
        Chunk {
            chars: self.chars,
            chars_utf16: self.chars_utf16,
            newlines: self.newlines,
            tabs: self.tabs,
            text: self.text.try_into().unwrap(),
        }
    }
}

impl<'a> ChunkSlice<'a> {
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.text.is_empty() }

    #[inline(always)]
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        (1 as Bitmap).unbounded_shl(offset as u32) & self.chars != 0 || offset == self.text.len()
    }

    #[inline(always)]
    pub fn split_at(self, mid: usize) -> (ChunkSlice<'a>, ChunkSlice<'a>) {
        if mid == MAX_BASE {
            let left = self;
            let right = ChunkSlice {
                chars: 0,
                chars_utf16: 0,
                newlines: 0,
                tabs: 0,
                text: "",
            };
            (left, right)
        } else {
            let mask = ((1 as Bitmap) << mid) - 1;
            let (left_text, right_text) = self.text.split_at(mid);
            let left = ChunkSlice {
                chars: self.chars & mask,
                chars_utf16: self.chars_utf16 & mask,
                newlines: self.newlines & mask,
                tabs: self.tabs & mask,
                text: left_text,
            };
            let right = ChunkSlice {
                chars: self.chars >> mid,
                chars_utf16: self.chars_utf16 >> mid,
                newlines: self.newlines >> mid,
                tabs: self.tabs >> mid,
                text: right_text,
            };
            (left, right)
        }
    }

    #[inline(always)]
    pub fn slice(self, mut range: Range<usize>) -> Self {
        if range.start == MAX_BASE {
            Self {
                chars: 0,
                chars_utf16: 0,
                newlines: 0,
                tabs: 0,
                text: "",
            }
        } else {
            if !self.assert_char_boundary::<false>(range.start) {
                range.start = self.text.ceil_char_boundary(range.start);
            }
            if !self.assert_char_boundary::<false>(range.end) {
                range.end = if range.end < range.start {
                    range.start
                } else {
                    self.text.floor_char_boundary(range.end)
                };
            }
            let mask = (1 as Bitmap)
                .unbounded_shl(range.end as u32)
                .wrapping_sub(1);
            Self {
                chars: (self.chars & mask) >> range.start,
                chars_utf16: (self.chars_utf16 & mask) >> range.start,
                newlines: (self.newlines & mask) >> range.start,
                tabs: (self.tabs & mask) >> range.start,
                text: &self.text[range],
            }
        }
    }

    #[inline(always)]
    pub fn text_summary(&self) -> TextSummary {
        let mut chars = 0;
        let (longest_row, longest_row_chars) = self.longest_row(&mut chars);
        TextSummary {
            len: self.len(),
            chars,
            len_utf16: self.len_utf16(),
            lines: self.lines(),
            first_line_chars: self.first_line_chars(),
            last_line_chars: self.last_line_chars(),
            last_line_len_utf16: self.last_line_len_utf16(),
            longest_row,
            longest_row_chars,
        }
    }

    /// Get length in bytes
    #[inline(always)]
    pub fn len(&self) -> usize { self.text.len() }

    /// Get length in UTF-16 code units
    #[inline(always)]
    pub fn len_utf16(&self) -> OffsetUtf16 { OffsetUtf16(self.chars_utf16.count_ones() as usize) }

    /// Get point representing number of lines and length of last line
    #[inline(always)]
    pub fn lines(&self) -> Point {
        let row = self.newlines.count_ones();
        let column = self.newlines.leading_zeros() - (Bitmap::BITS - self.text.len() as u32);
        Point::new(row, column)
    }

    /// Get number of chars in first line
    #[inline(always)]
    pub fn first_line_chars(&self) -> u32 {
        (self.chars & saturating_shl_mask(self.newlines.trailing_zeros())).count_ones()
    }

    /// Get number of chars in last line
    #[inline(always)]
    pub fn last_line_chars(&self) -> u32 {
        (self.chars & saturating_shr_mask(self.newlines.leading_zeros())).count_ones()
    }

    /// Get number of UTF-16 code units in last line
    #[inline(always)]
    pub fn last_line_len_utf16(&self) -> u32 {
        (self.chars_utf16 & saturating_shr_mask(self.newlines.leading_zeros())).count_ones()
    }

    /// Get the longest row in the chunk and its length in characters.
    /// Calculate the total number of characters in the chunk along the way.
    #[inline(always)]
    pub fn longest_row(&self, total_chars: &mut usize) -> (u32, u32) {
        let mut chars = self.chars;
        let mut newlines = self.newlines;
        *total_chars = 0;
        let mut row = 0;
        let mut longest_row = 0;
        let mut longest_row_chars = 0;
        while newlines > 0 {
            let newline_ix = newlines.trailing_zeros();
            let row_chars = (chars & ((1 << newline_ix) - 1)).count_ones() as u8;
            *total_chars += usize::from(row_chars);
            if row_chars > longest_row_chars {
                longest_row = row;
                longest_row_chars = row_chars;
            }

            newlines >>= newline_ix;
            newlines >>= 1;
            chars >>= newline_ix;
            chars >>= 1;
            row += 1;
            *total_chars += 1;
        }

        let row_chars = chars.count_ones() as u8;
        *total_chars += usize::from(row_chars);
        if row_chars > longest_row_chars {
            (row, row_chars as u32)
        } else {
            (longest_row, longest_row_chars as u32)
        }
    }

    #[inline(always)]
    pub fn offset_to_point(&self, offset: usize) -> Point {
        let mask = (1 as Bitmap).unbounded_shl(offset as u32).wrapping_sub(1);
        let row = (self.newlines & mask).count_ones();
        let newline_ix = Bitmap::BITS - (self.newlines & mask).leading_zeros();
        let column = (offset - newline_ix as usize) as u32;
        Point::new(row, column)
    }

    #[inline(always)]
    pub fn point_to_offset(&self, point: Point) -> usize {
        if point.row > self.lines().row {
            debug_panic!(
                "point {:?} extends beyond rows for string {:?}",
                point,
                self.text
            );
            return self.len();
        }

        let row_offset_range = self.offset_range_for_row(point.row);
        if point.column > row_offset_range.len() as u32 {
            debug_panic!(
                "point {:?} extends beyond row for string {:?}",
                point,
                self.text
            );
            row_offset_range.end
        } else {
            row_offset_range.start + point.column as usize
        }
    }

    #[track_caller]
    #[inline(always)]
    pub fn assert_char_boundary<const PANIC: bool>(&self, offset: usize) -> bool {
        if self.is_char_boundary(offset) {
            return true;
        }
        if PANIC {
            panic_char_boundary(self.text, offset);
        } else {
            log_err_char_boundary(self.text, offset);
            false
        }
    }

    pub fn floor_char_boundary(&self, index: usize) -> usize {
        self.text.floor_char_boundary(index)
    }

    #[inline(always)]
    pub fn point_to_offset_utf16(&self, point: Point) -> OffsetUtf16 {
        if point.row > self.lines().row {
            debug_panic!(
                "point {:?} extends beyond rows for string {:?}",
                point,
                self.text
            );
            return self.len_utf16();
        }
        self.offset_to_offset_utf16(self.point_to_offset(point))
    }

    #[inline(always)]
    pub fn offset_to_offset_utf16(&self, offset: usize) -> OffsetUtf16 {
        let mask = (1 as Bitmap).unbounded_shl(offset as u32).wrapping_sub(1);
        OffsetUtf16((self.chars_utf16 & mask).count_ones() as usize)
    }

    #[inline(always)]
    pub fn offset_utf16_to_offset(&self, target: OffsetUtf16) -> usize {
        if target.0 == 0 {
            0
        } else {
            #[cfg(not(test))]
            let chars_utf16 = self.chars_utf16;
            #[cfg(test)]
            let chars_utf16 = self.chars_utf16 as u128;
            let ix = nth_set_bit(chars_utf16, target.0) + 1;
            if ix == MAX_BASE {
                MAX_BASE
            } else {
                let utf8_additional_len = cmp::min(
                    (self.chars_utf16 >> ix).trailing_zeros() as usize,
                    self.text.len() - ix,
                );
                ix + utf8_additional_len
            }
        }
    }

    #[inline(always)]
    pub fn offset_to_point_utf16(&self, offset: usize) -> PointUtf16 {
        let mask = saturating_shl_mask(offset as u32);
        let row = (self.newlines & saturating_shl_mask(offset as u32)).count_ones();
        let newline_ix = Bitmap::BITS - (self.newlines & mask).leading_zeros();
        let column = if newline_ix as usize == MAX_BASE {
            0
        } else {
            ((self.chars_utf16 & mask) >> newline_ix).count_ones()
        };
        PointUtf16::new(row, column)
    }

    #[inline(always)]
    pub fn point_to_point_utf16(&self, point: Point) -> PointUtf16 {
        self.offset_to_point_utf16(self.point_to_offset(point))
    }

    #[inline(always)]
    pub fn point_utf16_to_offset(&self, point: PointUtf16, clip: bool) -> usize {
        let lines = self.lines();
        if point.row > lines.row {
            if !clip {
                debug_panic!(
                    "point {:?} is beyond this chunk's extent {:?}",
                    point,
                    self.text
                );
            }
            return self.len();
        }

        let row_offset_range = self.offset_range_for_row(point.row);
        let line = self.slice(row_offset_range.clone());
        if point.column > line.last_line_len_utf16() {
            if !clip {
                debug_panic!(
                    "point {:?} is beyond the end of the line in chunk {:?}",
                    point,
                    self.text
                );
            }
            return row_offset_range.end;
        }

        let mut offset = row_offset_range.start;
        if point.column > 0 {
            offset += line.offset_utf16_to_offset(OffsetUtf16(point.column as usize));
            if !self.text.is_char_boundary(offset) {
                offset -= 1;
                while !self.text.is_char_boundary(offset) {
                    offset -= 1;
                }
                if !clip {
                    debug_panic!(
                        "point {:?} is within character in chunk {:?}",
                        point,
                        self.text,
                    );
                }
            }
        }
        offset
    }

    #[inline(always)]
    pub fn unclipped_point_utf16_to_point(&self, point: Unclipped<PointUtf16>) -> Point {
        let max_point = self.lines();
        if point.0.row > max_point.row {
            return max_point;
        }

        let row_offset_range = self.offset_range_for_row(point.0.row);
        let line = self.slice(row_offset_range);
        if point.0.column == 0 {
            Point::new(point.0.row, 0)
        } else if point.0.column >= line.len_utf16().0 as u32 {
            Point::new(point.0.row, line.len() as u32)
        } else {
            let mut column = line.offset_utf16_to_offset(OffsetUtf16(point.0.column as usize));
            while !line.text.is_char_boundary(column) {
                column -= 1;
            }
            Point::new(point.0.row, column as u32)
        }
    }

    #[inline(always)]
    pub fn clip_point(&self, point: Point, bias: Bias) -> Point {
        let max_point = self.lines();
        if point.row > max_point.row {
            return max_point;
        }

        let line = self.slice(self.offset_range_for_row(point.row));
        if point.column == 0 {
            point
        } else if point.column >= line.len() as u32 {
            Point::new(point.row, line.len() as u32)
        } else {
            let mut column = point.column as usize;
            let bytes = line.text.as_bytes();
            if bytes[column - 1] < 128 && bytes[column] < 128 {
                return Point::new(point.row, column as u32);
            }

            let mut grapheme_cursor = GraphemeCursor::new(column, bytes.len(), true);
            loop {
                if line.is_char_boundary(column)
                    && grapheme_cursor.is_boundary(line.text, 0).unwrap_or(false)
                {
                    break;
                }

                match bias {
                    Bias::Left => column -= 1,
                    Bias::Right => column += 1,
                }
                grapheme_cursor.set_cursor(column);
            }
            Point::new(point.row, column as u32)
        }
    }

    #[inline(always)]
    pub fn clip_point_utf16(&self, point: Unclipped<PointUtf16>, bias: Bias) -> PointUtf16 {
        let max_point = self.lines();
        if point.0.row > max_point.row {
            PointUtf16::new(max_point.row, self.last_line_len_utf16())
        } else {
            let line = self.slice(self.offset_range_for_row(point.0.row));
            let column = line.clip_offset_utf16(OffsetUtf16(point.0.column as usize), bias);
            PointUtf16::new(point.0.row, column.0 as u32)
        }
    }

    #[inline(always)]
    pub fn clip_offset_utf16(&self, target: OffsetUtf16, bias: Bias) -> OffsetUtf16 {
        if target == OffsetUtf16::default() {
            OffsetUtf16::default()
        } else if target >= self.len_utf16() {
            self.len_utf16()
        } else {
            let mut offset = self.offset_utf16_to_offset(target);
            while !self.text.is_char_boundary(offset) {
                if bias == Bias::Left {
                    offset -= 1;
                } else {
                    offset += 1;
                }
            }
            self.offset_to_offset_utf16(offset)
        }
    }

    #[inline(always)]
    fn offset_range_for_row(&self, row: u32) -> Range<usize> {
        let row_start = if row > 0 {
            #[cfg(not(test))]
            let newlines = self.newlines;
            #[cfg(test)]
            let newlines = self.newlines as u128;
            nth_set_bit(newlines, row as usize) + 1
        } else {
            0
        };
        let row_len = if row_start == MAX_BASE {
            0
        } else {
            cmp::min(
                (self.newlines >> row_start).trailing_zeros(),
                (self.text.len() - row_start) as u32,
            )
        };
        row_start..row_start + row_len as usize
    }

    #[inline(always)]
    pub fn tabs(&self) -> Tabs {
        Tabs {
            tabs: self.tabs,
            chars: self.chars,
        }
    }
}
