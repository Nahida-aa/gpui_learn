//! 文本摘要（summary）：一段文本的「可加统计量」。
//!
//! 移植自 zed `crates/rope/src/rope.rs` 的 `TextSummary`/`ChunkSummary`。
//! sum_tree 的核心思想：树上每个节点缓存子树的 summary，任意两个 summary
//! 可以 O(1) 合并，因此「字节 / 字符 / UTF-16 / 行」四种坐标都能在对数
//! 时间内定位，而不需要遍历文本。

use super::point::OffsetUtf16;
use super::point::Point;
use std::ops::{Add, AddAssign};

/// 一段文本的统计摘要。
///
/// `lines` 的语义是「文本末尾（EOF 假想字符）的坐标」：
/// `row` 是换行数，`column` 是最后一行的字节长度。
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct TextSummary {
    /// 字节长度。
    pub len: usize,
    /// UTF-8 字符数。
    pub chars: usize,
    /// UTF-16 code unit 数。
    pub len_utf16: OffsetUtf16,
    /// 行数 + 最后一行字节长度。
    pub lines: Point,
    /// 第一行的字符数。
    pub first_line_chars: u32,
    /// 最后一行的字符数。
    pub last_line_chars: u32,
    /// 最后一行的 UTF-16 code unit 数。
    pub last_line_len_utf16: u32,
    /// 最长行的行号。
    pub longest_row: u32,
    /// 最长行的字符数。
    pub longest_row_chars: u32,
}

impl TextSummary {
    /// `lines` 的 UTF-16 版本（row 相同，column 换成最后一行的 UTF-16 长度）。
    pub fn lines_utf16(&self) -> super::point::PointUtf16 {
        super::point::PointUtf16 {
            row: self.lines.row,
            column: self.last_line_len_utf16,
        }
    }

    /// 单个换行符的摘要。
    pub fn newline() -> Self {
        Self {
            len: 1,
            chars: 1,
            len_utf16: OffsetUtf16(1),
            first_line_chars: 0,
            last_line_chars: 0,
            last_line_len_utf16: 0,
            lines: Point::new(1, 0),
            longest_row: 0,
            longest_row_chars: 0,
        }
    }
}

/// 对任一 `&str` 一次遍历算出全部统计量。
impl<'a> From<&'a str> for TextSummary {
    fn from(text: &'a str) -> Self {
        let mut len_utf16 = OffsetUtf16(0);
        let mut lines = Point::new(0, 0);
        let mut first_line_chars = 0;
        let mut last_line_chars = 0;
        let mut last_line_len_utf16 = 0;
        let mut longest_row = 0;
        let mut longest_row_chars = 0;
        let mut chars = 0;
        for c in text.chars() {
            chars += 1;
            len_utf16.0 += c.len_utf16();

            if c == '\n' {
                lines += Point::new(1, 0);
                last_line_len_utf16 = 0;
                last_line_chars = 0;
            } else {
                lines.column += c.len_utf8() as u32;
                last_line_len_utf16 += c.len_utf16() as u32;
                last_line_chars += 1;
            }

            if lines.row == 0 {
                first_line_chars = last_line_chars;
            }

            if last_line_chars > longest_row_chars {
                longest_row = lines.row;
                longest_row_chars = last_line_chars;
            }
        }

        TextSummary {
            len: text.len(),
            chars,
            len_utf16,
            lines,
            first_line_chars,
            last_line_chars,
            last_line_len_utf16,
            longest_row,
            longest_row_chars,
        }
    }
}

impl sum_tree::ContextLessSummary for TextSummary {
    fn zero() -> Self {
        Default::default()
    }

    fn add_summary(&mut self, summary: &Self) {
        *self += summary;
    }
}

impl Add for TextSummary {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        AddAssign::add_assign(&mut self, &rhs);
        self
    }
}

impl AddAssign<&Self> for TextSummary {
    fn add_assign(&mut self, other: &Self) {
        // 拼接处可能形成「更长的一行」：self 的最后一行 + other 的第一行。
        let joined_chars = self.last_line_chars + other.first_line_chars;
        if joined_chars > self.longest_row_chars {
            self.longest_row = self.lines.row;
            self.longest_row_chars = joined_chars;
        }
        if other.longest_row_chars > self.longest_row_chars {
            self.longest_row = self.lines.row + other.longest_row;
            self.longest_row_chars = other.longest_row_chars;
        }

        // 第一行 / 最后一行的归属随换行数变化。
        if self.lines.row == 0 {
            self.first_line_chars += other.first_line_chars;
        }

        if other.lines.row == 0 {
            self.last_line_chars += other.first_line_chars;
            self.last_line_len_utf16 += other.last_line_len_utf16;
        } else {
            self.last_line_chars = other.last_line_chars;
            self.last_line_len_utf16 = other.last_line_len_utf16;
        }

        self.chars += other.chars;
        self.len += other.len;
        self.len_utf16 += other.len_utf16;
        self.lines += other.lines;
    }
}

impl AddAssign<TextSummary> for TextSummary {
    fn add_assign(&mut self, other: TextSummary) {
        *self += &other;
    }
}

/// sum_tree 叶子（Chunk）的 summary。包一层是为了将来给 chunk 加
/// 非文本统计量（如 zed 的 tabs 位图摘要）留扩展位。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChunkSummary {
    pub text: TextSummary,
}

impl sum_tree::ContextLessSummary for ChunkSummary {
    fn zero() -> Self {
        Default::default()
    }

    fn add_summary(&mut self, summary: &Self) {
        self.text += &summary.text;
    }
}

impl sum_tree::Item for super::chunk::Chunk {
    type Summary = ChunkSummary;

    fn summary(&self, _: ()) -> Self::Summary {
        ChunkSummary {
            text: self.as_slice().text_summary(),
        }
    }
}
