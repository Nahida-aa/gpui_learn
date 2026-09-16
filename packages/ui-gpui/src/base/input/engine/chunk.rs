//! Chunk：Rope 的叶子节点，一段 ≤128 字节的文本 + 三个位图。
//!
//! 移植自 zed `crates/rope/src/chunk.rs`。位图是这套数据结构的性能核心：
//!
//! - `chars`：bit[i] = 1 表示字节 i 是某个 UTF-8 字符的首字节。
//!   `chars & mask` 再 `count_ones` 即可 O(1) 数出任意前缀里的字符数。
//! - `chars_utf16`：bit[i] = 1 表示「UTF-16 前缀里的一个 code unit 起点」。
//!   首字节 ≥ 0xF0 的字符（U+10000 以上）占两个 UTF-16 unit，所以
//!   构造时是 `(surrogate_bits << 1) | chars`。
//! - `newlines`：bit[i] = 1 表示字节 i 是 `\n`。
//!
//! 精简点：去掉 `tabs` 位图（indent 场景再加）；`text` 用 `String`
//! 而非 `heapless::ArrayString`（学习版优先可读性，容量由模块自身保证）。

use super::point::{OffsetUtf16, Point, PointUtf16};
use super::summary::TextSummary;
use std::{cmp, ops::Range};
use sum_tree::Bias;

pub(crate) type Bitmap = u128;

/// chunk 的最大字节数 = 位图位数；低于它的 chunk 会被合并。
pub(crate) const MIN_BASE: usize = MAX_BASE / 2;
pub const MAX_BASE: usize = Bitmap::BITS as usize;

#[derive(Clone, Debug, Default)]
pub struct Chunk {
    chars: Bitmap,
    chars_utf16: Bitmap,
    newlines: Bitmap,
    pub text: String,
}

impl From<ChunkSlice<'_>> for Chunk {
    fn from(slice: ChunkSlice<'_>) -> Self {
        Chunk {
            chars: slice.chars,
            chars_utf16: slice.chars_utf16,
            newlines: slice.newlines,
            text: slice.text.to_owned(),
        }
    }
}

#[inline(always)]
const fn saturating_shl_mask(offset: u32) -> Bitmap {
    (1 as Bitmap).unbounded_shl(offset).wrapping_sub(1)
}

#[inline(always)]
const fn saturating_shr_mask(offset: u32) -> Bitmap {
    !Bitmap::MAX.unbounded_shr(offset)
}

impl Chunk {
    #[inline(always)]
    pub fn new(text: &str) -> Self {
        debug_assert!(text.len() <= MAX_BASE);

        const CHUNK_SIZE: usize = 8;

        let mut chars_bytes = [0u8; MAX_BASE / CHUNK_SIZE];
        let mut newlines_bytes = [0u8; MAX_BASE / CHUNK_SIZE];
        let mut chars_utf16_bytes = [0u8; MAX_BASE / CHUNK_SIZE];

        let mut chunk_ix = 0;
        let mut bytes = text.as_bytes();
        while !bytes.is_empty() {
            let (chunk, rest) = bytes.split_at(bytes.len().min(CHUNK_SIZE));
            bytes = rest;

            let mut chars = 0u8;
            let mut newlines = 0u8;
            let mut chars_utf16 = 0u8;

            for (ix, &b) in chunk.iter().enumerate() {
                chars |= (((b & 0xC0) != 0x80) as u8) << ix; // UTF-8 首字节：非 10xxxxxx
                newlines |= ((b == b'\n') as u8) << ix;
                // 首字节 ≥ 0xF0 → U+10000 以上 → 占两个 UTF-16 code unit
                chars_utf16 |= ((b >= 240) as u8) << ix;
            }

            chars_bytes[chunk_ix] = chars;
            newlines_bytes[chunk_ix] = newlines;
            chars_utf16_bytes[chunk_ix] = chars_utf16;

            chunk_ix += 1;
        }

        let chars = Bitmap::from_le_bytes(chars_bytes);
        Chunk {
            text: text.to_owned(),
            chars,
            chars_utf16: (Bitmap::from_le_bytes(chars_utf16_bytes) << 1) | chars,
            newlines: Bitmap::from_le_bytes(newlines_bytes),
        }
    }

    #[inline(always)]
    pub fn push_str(&mut self, text: &str) {
        self.append(Chunk::new(text).as_slice());
    }

    #[inline(always)]
    pub fn prepend_str(&mut self, text: &str) {
        self.prepend(Chunk::new(text).as_slice());
    }

    #[inline(always)]
    pub fn append(&mut self, slice: ChunkSlice) {
        if slice.is_empty() {
            return;
        }

        let base_ix = self.text.len();
        self.chars |= slice.chars << base_ix;
        self.chars_utf16 |= slice.chars_utf16 << base_ix;
        self.newlines |= slice.newlines << base_ix;
        self.text.push_str(slice.text);
    }

    #[inline(always)]
    pub fn prepend(&mut self, slice: ChunkSlice) {
        if slice.is_empty() {
            return;
        }
        if self.text.is_empty() {
            *self = Chunk::new(slice.text);
            return;
        }

        let shift = slice.text.len();
        self.chars = slice.chars | (self.chars << shift);
        self.chars_utf16 = slice.chars_utf16 | (self.chars_utf16 << shift);
        self.newlines = slice.newlines | (self.newlines << shift);

        let mut new_text = String::with_capacity(self.text.len() + slice.text.len());
        new_text.push_str(slice.text);
        new_text.push_str(&self.text);
        self.text = new_text;
    }

    #[inline(always)]
    pub fn as_slice(&self) -> ChunkSlice<'_> {
        ChunkSlice {
            chars: self.chars,
            chars_utf16: self.chars_utf16,
            newlines: self.newlines,
            text: &self.text,
        }
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> ChunkSlice<'_> {
        self.as_slice().slice(range)
    }

    #[inline(always)]
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        (1 as Bitmap).unbounded_shl(offset as u32) & self.chars != 0 || offset == self.text.len()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkSlice<'a> {
    pub chars: Bitmap,
    pub chars_utf16: Bitmap,
    pub newlines: Bitmap,
    pub text: &'a str,
}

impl<'a> ChunkSlice<'a> {
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.text.len()
    }

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
                text: left_text,
            };
            let right = ChunkSlice {
                chars: self.chars >> mid,
                chars_utf16: self.chars_utf16 >> mid,
                newlines: self.newlines >> mid,
                text: right_text,
            };
            (left, right)
        }
    }

    #[inline(always)]
    pub fn slice(self, range: Range<usize>) -> Self {
        let mask = (1 as Bitmap)
            .unbounded_shl(range.end as u32)
            .wrapping_sub(1);
        Self {
            chars: (self.chars & mask) >> range.start,
            chars_utf16: (self.chars_utf16 & mask) >> range.start,
            newlines: (self.newlines & mask) >> range.start,
            text: &self.text[range],
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

    /// UTF-16 code unit 长度 = `chars_utf16` 位图中 1 的个数。
    #[inline(always)]
    pub fn len_utf16(&self) -> OffsetUtf16 {
        OffsetUtf16(self.chars_utf16.count_ones() as usize)
    }

    /// 行数 + 最后一行字节长度。
    ///
    /// `leading_zeros` 技巧：newlines 位图最高位的 1 就是最后一个换行，
    /// 它之后（含其后所有字节）就是最后一行的内容。
    #[inline(always)]
    pub fn lines(&self) -> Point {
        let row = self.newlines.count_ones();
        let column = self.newlines.leading_zeros() - (Bitmap::BITS - self.text.len() as u32);
        Point::new(row, column)
    }

    #[inline(always)]
    pub fn first_line_chars(&self) -> u32 {
        (self.chars & saturating_shl_mask(self.newlines.trailing_zeros())).count_ones()
    }

    #[inline(always)]
    pub fn last_line_chars(&self) -> u32 {
        (self.chars & saturating_shr_mask(self.newlines.leading_zeros())).count_ones()
    }

    #[inline(always)]
    pub fn last_line_len_utf16(&self) -> u32 {
        (self.chars_utf16 & saturating_shr_mask(self.newlines.leading_zeros())).count_ones()
    }

    /// 最长行及其字符数；顺带累计总字符数。
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
            *total_chars += 1; // 换行符本身
        }

        let row_chars = chars.count_ones() as u8;
        *total_chars += usize::from(row_chars);
        if row_chars > longest_row_chars {
            (row, row_chars as u32)
        } else {
            (longest_row, longest_row_chars as u32)
        }
    }

    /// 字节偏移 → 行列（chunk 内部，offset 必须是字符边界）。
    #[inline(always)]
    pub fn offset_to_point(&self, offset: usize) -> Point {
        let mask = (1 as Bitmap).unbounded_shl(offset as u32).wrapping_sub(1);
        let row = (self.newlines & mask).count_ones();
        let newline_ix = Bitmap::BITS - (self.newlines & mask).leading_zeros();
        let column = (offset - newline_ix as usize) as u32;
        Point::new(row, column)
    }

    /// 行列 → 字节偏移（chunk 内部）。
    #[inline(always)]
    pub fn point_to_offset(&self, point: Point) -> usize {
        if point.row > self.lines().row {
            return self.len();
        }

        let row_offset_range = self.offset_range_for_row(point.row);
        if point.column > row_offset_range.len() as u32 {
            row_offset_range.end
        } else {
            row_offset_range.start + point.column as usize
        }
    }

    #[inline(always)]
    pub fn offset_to_offset_utf16(&self, offset: usize) -> OffsetUtf16 {
        let mask = (1 as Bitmap).unbounded_shl(offset as u32).wrapping_sub(1);
        OffsetUtf16((self.chars_utf16 & mask).count_ones() as usize)
    }

    /// UTF-16 偏移 → 字节偏移（chunk 内部）。
    ///
    /// `nth_set_bit` 找到第 n 个 UTF-16 unit 起点；若落点在多 unit 字符
    /// （U+10000+）的中间，向后补齐到该字符的字节末尾。
    #[inline(always)]
    pub fn offset_utf16_to_offset(&self, target: OffsetUtf16) -> usize {
        if target.0 == 0 {
            0
        } else {
            let ix = nth_set_bit(self.chars_utf16, target.0) + 1;
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

    /// UTF-16 行列 → 字节偏移。`clip = true` 时越界/跨字符都收敛到合法位置，
    /// `clip = false` 时只允许精确坐标（越界返回 chunk 末尾）。
    #[inline(always)]
    pub fn point_utf16_to_offset(&self, point: PointUtf16, clip: bool) -> usize {
        let lines = self.lines();
        if point.row > lines.row {
            return self.len();
        }

        let row_offset_range = self.offset_range_for_row(point.row);
        let line = self.slice(row_offset_range.clone());
        if point.column > line.last_line_len_utf16() {
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
                debug_assert!(
                    clip,
                    "point {:?} is within character in chunk {:?}",
                    point, self.text
                );
            }
        }
        offset
    }

    /// 把任意字节偏移收敛到字符边界。Rope 层多 chunk 裁剪的 chunk 内部分量;
    /// 当前只被测试引用,后续 clip_point* 系列会接入。
    #[allow(dead_code)]
    pub fn clip_offset(&self, offset: usize, bias: Bias) -> usize {
        if offset >= self.text.len() {
            return self.text.len();
        }
        let mut offset = offset.min(self.text.len());
        match bias {
            Bias::Left => {
                while offset > 0 && !self.text.is_char_boundary(offset) {
                    offset -= 1;
                }
            }
            Bias::Right => {
                while offset < self.text.len() && !self.text.is_char_boundary(offset) {
                    offset += 1;
                }
            }
        }
        offset
    }

    /// 第 `row` 行在 chunk 内的字节范围。
    #[inline(always)]
    pub fn offset_range_for_row(&self, row: u32) -> Range<usize> {
        let row_start = if row > 0 {
            nth_set_bit(self.newlines, row as usize) + 1
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
}

/// 找出 `v` 中第 `n` 个（1-based）为 1 的位的位置。
#[inline(always)]
fn nth_set_bit(v: Bitmap, n: usize) -> usize {
    let low = v as u64;
    let high = (v >> 64) as u64;

    let low_count = low.count_ones() as usize;
    if n > low_count {
        64 + nth_set_bit_u64(high, (n - low_count) as u64) as usize
    } else {
        nth_set_bit_u64(low, n as u64) as usize
    }
}

/// u64 版的「第 n 个置位位」。这里用朴素的位扫描实现——
/// zed 用了一个分支消除的手工展开版本（见其 chunk.rs `nth_set_bit_u64`），
/// 语义完全相同；学习版优先可读性，hot path 不够快时再换。
#[inline(always)]
fn nth_set_bit_u64(v: u64, n: u64) -> u64 {
    let mut v = v;
    let mut consumed = 0;
    while v != 0 {
        let lowest = v.trailing_zeros();
        consumed += 1;
        if consumed as u64 == n {
            return lowest as u64;
        }
        v &= v - 1; // 清掉最低位的 1
    }
    64
}
