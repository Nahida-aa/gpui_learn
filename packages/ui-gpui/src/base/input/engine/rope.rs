//! Rope：基于 sum_tree 的可持久化文本序列。
//!
//! 移植自 zed `crates/rope/src/rope.rs` 的精简版。核心结构：
//!
//! ```text
//! Rope { chunks: SumTree<Chunk> }
//! ```
//!
//! 每个 Chunk 是 ≤128 字节的文本段（见 [`super::chunk`]），树节点缓存
//! 子树的 [`TextSummary`]。于是：
//!
//! - 编辑（push/replace）：O(log n) 拆合 chunk；
//! - 坐标互转（字节 ↔ UTF-16 ↔ 行列）：`find::<Dimensions<A, B>, _>`
//!   在一棵树里同时累计两个维度，O(log n) 定位后落到 chunk 内 O(1) 位图运算；
//! - 遍历：`Chunks` 迭代器逐段给出 `&str`，避免整段拷贝。
//!
//! 精简点：无 rayon 并行构建、无 `Unclipped`、无 Lines/Bytes 迭代器、
//! 无 starts_with/ends_with；`clip_point` 暂为字节边界版（字素簇版
//! 在光标移动阶段按需加入，InputState 现在自己用 unicode-segmentation）。

use std::{cmp, fmt, ops::Range};

use sum_tree::{Bias, Dimensions, SumTree};

use super::chunk::{Chunk, MAX_BASE, MIN_BASE};
use super::point::{OffsetUtf16, Point, PointUtf16};
use super::summary::TextSummary;

#[derive(Clone, Default)]
pub struct Rope {
    chunks: SumTree<Chunk>,
}

impl Rope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_char_boundary(&self, offset: usize) -> bool {
        if self.chunks.is_empty() {
            return offset == 0;
        }
        let (start, _, item) = self.chunks.find::<usize, _>((), &offset, Bias::Left);
        let chunk_offset = offset - start;
        item.map(|chunk| chunk.is_char_boundary(chunk_offset))
            .unwrap_or(false)
    }

    pub fn floor_char_boundary(&self, index: usize) -> usize {
        if index >= self.len() {
            self.len()
        } else {
            let (start, _, item) = self.chunks.find::<usize, _>((), &index, Bias::Left);
            let chunk_offset = index - start;
            // chunk 内逐字节回退到 UTF-8 首字节（非 10xxxxxx）
            let lower_idx = item.map(|chunk| {
                let mut i = chunk_offset.min(chunk.text.len());
                while i > 0 && !chunk.text.is_char_boundary(i) {
                    i -= 1;
                }
                i
            });
            lower_idx.map_or_else(|| self.len(), |idx| start + idx)
        }
    }

    pub fn ceil_char_boundary(&self, index: usize) -> usize {
        if index > self.len() {
            self.len()
        } else {
            let (start, _, item) = self.chunks.find::<usize, _>((), &index, Bias::Left);
            let chunk_offset = index - start;
            let upper_idx = item.map(|chunk| {
                let mut i = chunk_offset;
                while i < chunk.text.len() && !chunk.text.is_char_boundary(i) {
                    i += 1;
                }
                i
            });
            upper_idx.map_or_else(|| self.len(), |idx| start + idx)
        }
    }

    /// 把任意字节偏移收敛到字符边界（光标/选区裁剪用）。
    pub fn clip_offset(&self, offset: usize, bias: Bias) -> usize {
        match bias {
            Bias::Left => self.floor_char_boundary(offset),
            Bias::Right => self.ceil_char_boundary(offset),
        }
    }

    /// 拼接另一棵 Rope。右 rope 首块若过小则先并入末块，避免碎片化。
    pub fn append(&mut self, rope: Rope) {
        if let Some(chunk) = rope.chunks.first()
            && (self.chunks.last().is_some_and(|c| c.text.len() < MIN_BASE)
                || chunk.text.len() < MIN_BASE)
        {
            self.push_chunk(chunk.as_slice());

            let mut chunks = rope.chunks.cursor::<()>(());
            chunks.next();
            chunks.next();
            self.chunks.append(chunks.suffix(), ());
        } else {
            self.chunks.append(rope.chunks, ());
        }
    }

    /// 把 `[range)` 替换为 `text`——输入框编辑的主入口。
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

    /// 追加文本。优先并入末块；超过单块容量时按 `MAX_BASE` 分块。
    pub fn push(&mut self, mut text: &str) {
        self.chunks.update_last(
            |last_chunk| {
                let split_ix = if last_chunk.text.len() + text.len() <= MAX_BASE {
                    text.len()
                } else {
                    let mut split_ix =
                        cmp::min(MIN_BASE.saturating_sub(last_chunk.text.len()), text.len());
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
            return;
        }

        let mut new_chunks = Vec::new();
        while !text.is_empty() {
            let mut split_ix = cmp::min(MAX_BASE, text.len());
            while !text.is_char_boundary(split_ix) {
                split_ix -= 1;
            }
            let (chunk, remainder) = text.split_at(split_ix);
            new_chunks.push(chunk);
            text = remainder;
        }
        self.chunks
            .extend(new_chunks.into_iter().map(Chunk::new), ());
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
            .is_some_and(|c| c.text.len() + text.len() <= MAX_BASE)
        {
            self.chunks
                .update_first(|first_chunk| first_chunk.prepend_str(text), ());
            return;
        }
        let suffix = std::mem::replace(self, Rope::from(text));
        self.append(suffix);
    }

    fn push_chunk(&mut self, mut chunk: super::chunk::ChunkSlice) {
        self.chunks.update_last(
            |last_chunk| {
                let split_ix = if last_chunk.text.len() + chunk.len() <= MAX_BASE {
                    chunk.len()
                } else {
                    let mut split_ix =
                        cmp::min(MIN_BASE.saturating_sub(last_chunk.text.len()), chunk.len());
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

    pub fn summary(&self) -> TextSummary {
        self.chunks.summary().text
    }

    /// 字节长度。
    pub fn len(&self) -> usize {
        self.chunks.extent(())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn max_point(&self) -> Point {
        self.chunks.extent(())
    }

    /// 某 buffer 行的长度(不含换行符)。
    pub fn line_len(&self, row: u32) -> usize {
        let row_start = self.point_to_offset(Point::new(row, 0));
        let max_row = self.summary().lines.row;
        if row >= max_row {
            return self.len() - row_start;
        }
        let next_start = self.point_to_offset(Point::new(row + 1, 0));
        next_start - row_start - 1 // 去掉 '\n'
    }

    pub fn cursor(&self, offset: usize) -> Cursor<'_> {
        Cursor::new(self, offset)
    }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.chars_at(0)
    }

    pub fn chars_at(&self, start: usize) -> impl Iterator<Item = char> + '_ {
        self.chunks_in_range(start..self.len()).flat_map(str::chars)
    }

    pub fn chunks(&self) -> Chunks<'_> {
        self.chunks_in_range(0..self.len())
    }

    pub fn chunks_in_range(&self, range: Range<usize>) -> Chunks<'_> {
        Chunks::new(self, range)
    }

    /// 取 `[range)` 的文本，跨 chunk 拼成一个 `String`。
    ///
    /// 单行输入框用它做剪贴板拷贝与字素边界查询；多行渲染应走
    /// `chunks_in_range` 逐段零拷贝遍历，不要整段物化。
    pub fn text_in_range(&self, range: Range<usize>) -> String {
        let mut text = String::with_capacity(range.len());
        for chunk in self.chunks_in_range(range) {
            text.push_str(chunk);
        }
        text
    }

    // ---- 坐标互转：树内双维度联合查询 + chunk 内位图运算 ----

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

    /// UTF-16 偏移 → 字节偏移。
    ///
    /// 与 zed 的差异：zed 原版允许返回落在多字节字符内部的偏移、由调用方
    /// clip；我们直接收敛到字符边界（IME/选区换算的直接语义，见
    /// `InputState::range_from_utf16`），调用方无需再处理。
    pub fn offset_utf16_to_offset(&self, offset: OffsetUtf16) -> usize {
        if offset >= self.summary().len_utf16 {
            return self.summary().len;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<OffsetUtf16, usize>, _>((), &offset, Bias::Left);
        let overshoot = offset - start.0;
        let result = start.1
            + item.map_or(Default::default(), |chunk| {
                chunk.as_slice().offset_utf16_to_offset(overshoot)
            });
        self.floor_char_boundary(result)
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

    pub fn point_utf16_to_offset(&self, point: PointUtf16) -> usize {
        if point >= self.summary().lines_utf16() {
            return self.summary().len;
        }
        let (start, _, item) =
            self.chunks
                .find::<Dimensions<PointUtf16, usize>, _>((), &point, Bias::Left);
        let overshoot = point - start.0;
        start.1
            + item.map_or(0, |chunk| {
                chunk.as_slice().point_utf16_to_offset(overshoot, true)
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
}

impl<'a> From<&'a str> for Rope {
    fn from(text: &'a str) -> Self {
        let mut rope = Self::new();
        rope.push(text);
        rope
    }
}

impl From<String> for Rope {
    fn from(text: String) -> Self {
        Rope::from(text.as_str())
    }
}

impl FromIterator<&'static str> for Rope {
    fn from_iter<T: IntoIterator<Item = &'static str>>(iter: T) -> Self {
        let mut rope = Rope::new();
        for chunk in iter {
            rope.push(chunk);
        }
        rope
    }
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
        write!(f, "Rope({:?})", self.to_string())
    }
}

/// 与字符串逐块比较（property 测试与断言用）。
impl PartialEq<str> for Rope {
    fn eq(&self, other: &str) -> bool {
        let mut remaining = other;
        for chunk in self.chunks_in_range(0..self.len()) {
            let Some(head) = chunk.get(..remaining.len().min(chunk.len())) else {
                return false;
            };
            if !remaining.starts_with(head) {
                return false;
            }
            remaining = &remaining[head.len()..];
        }
        remaining.is_empty()
    }
}

impl PartialEq<&str> for Rope {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

/// 正向遍历光标：`replace`/`slice` 的底层支撑。
pub struct Cursor<'a> {
    rope: &'a Rope,
    chunks: sum_tree::Cursor<'a, 'static, Chunk, usize>,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(rope: &'a Rope, offset: usize) -> Self {
        let mut chunks = rope.chunks.cursor(());
        chunks.seek(&offset, Bias::Right);
        Self {
            rope,
            chunks,
            offset,
        }
    }

    pub fn seek_forward(&mut self, end_offset: usize) {
        assert!(
            end_offset >= self.offset,
            "cannot seek backward from {} to {}",
            self.offset,
            end_offset
        );
        assert!(
            end_offset <= self.rope.len(),
            "cannot summarize past end of rope"
        );

        self.chunks.seek_forward(&end_offset, Bias::Right);
        self.offset = end_offset;
    }

    /// 从当前位置截取到 `end_offset`，返回这段文本组成的新 Rope。
    pub fn slice(&mut self, end_offset: usize) -> Rope {
        assert!(
            end_offset >= self.offset,
            "cannot slice backward from {} to {}",
            self.offset,
            end_offset
        );
        assert!(
            end_offset <= self.rope.len(),
            "cannot summarize past end of rope"
        );

        let mut slice = Rope::new();
        if let Some(start_chunk) = self.chunks.item() {
            let start_ix = self.offset - self.chunks.start();
            let end_ix = cmp::min(end_offset, self.chunks.end()) - self.chunks.start();
            slice.push_chunk(start_chunk.slice(start_ix..end_ix));
        }

        if end_offset > self.chunks.end() {
            self.chunks.next();
            slice.append(Rope {
                chunks: self.chunks.slice(&end_offset, Bias::Right),
            });
            if let Some(end_chunk) = self.chunks.item() {
                let end_ix = end_offset - self.chunks.start();
                slice.push_chunk(end_chunk.slice(0..end_ix));
            }
        }

        self.offset = end_offset;
        slice
    }

    /// 截取当前位置到 rope 末尾。
    pub fn suffix(mut self) -> Rope {
        self.slice(self.rope.chunks.extent(()))
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

/// 逐块 `&str` 迭代器（正序）。
#[derive(Clone)]
pub struct Chunks<'a> {
    chunks: sum_tree::Cursor<'a, 'static, Chunk, usize>,
    range: Range<usize>,
    offset: usize,
}

impl<'a> Chunks<'a> {
    pub fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        let mut chunks = rope.chunks.cursor(());
        chunks.seek(&range.start, Bias::Right);
        let offset = range.start;
        let chunk_offset = offset - chunks.start();
        if let Some(chunk) = chunks.item() {
            assert!(
                chunk.is_char_boundary(chunk_offset),
                "offset {offset} is not a char boundary"
            );
        }
        Self {
            chunks,
            range,
            offset,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn seek(&mut self, offset: usize) {
        let offset = offset.clamp(self.range.start, self.range.end);
        if offset >= self.chunks.end() {
            self.chunks.seek_forward(&offset, Bias::Right);
        } else if offset < *self.chunks.start() {
            self.chunks.seek(&offset, Bias::Right);
        }
        self.offset = offset;
    }

    pub fn peek(&self) -> Option<&'a str> {
        if self.offset < self.range.start || self.offset >= self.range.end {
            return None;
        }

        let chunk = self.chunks.item()?;
        let chunk_start = self.chunks.start();
        let slice_start = self.offset - chunk_start;
        let slice_end = cmp::min(self.chunks.end(), self.range.end) - chunk_start;
        Some(&chunk.text[slice_start..slice_end])
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.peek()?;
        self.offset += chunk.len();
        if self.offset >= self.chunks.end() {
            self.chunks.next();
        }
        Some(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use sum_tree::Bias;

    /// 与 `String` 参照实现做随机编辑序列对拍。
    #[test]
    fn test_random_editions_match_string() {
        let mut rng = rand::rng();
        for seed in 0..50 {
            let mut rope = Rope::new();
            let mut reference = String::new();

            for step in 0..200 {
                let op = rng.random_range(0..4);
                match op {
                    0 => {
                        // 末尾追加
                        let len = rng.random_range(0..300);
                        let text = random_text(&mut rng, len);
                        rope.push(&text);
                        reference.push_str(&text);
                    }
                    1 => {
                        // 区间替换
                        let (start, end) = random_char_range(&reference, &mut rng);
                        let len = rng.random_range(0..50);
                        let text = random_text(&mut rng, len);
                        rope.replace(start..end, &text);
                        reference.replace_range(start..end, &text);
                    }
                    2 => {
                        // 头部插入
                        let len = rng.random_range(0..40);
                        let text = random_text(&mut rng, len);
                        rope.push_front(&text);
                        reference.insert_str(0, &text);
                    }
                    _ => {
                        // 坐标互转抽查
                        let offset = rope.clip_offset(rng.random_range(0..=rope.len()), Bias::Left);
                        assert_eq!(
                            rope.offset_to_offset_utf16(offset).0,
                            str_offset_to_utf16(&reference, offset),
                            "seed {seed} step {step}: utf16 mismatch"
                        );
                        // UTF-16 → 字节往返（offset 为合法边界时必须回到原点）
                        let utf16 = rope.offset_to_offset_utf16(offset);
                        assert_eq!(
                            rope.offset_utf16_to_offset(utf16),
                            offset,
                            "seed {seed} step {step}: utf16 roundtrip"
                        );
                        let point = rope.offset_to_point(offset);
                        assert_eq!(
                            point,
                            str_offset_to_point(&reference, offset),
                            "seed {seed} step {step}: point mismatch"
                        );
                        assert_eq!(rope.point_to_offset(point), offset);
                    }
                }

                assert_eq!(rope.len(), reference.len(), "seed {seed} step {step}");
                assert!(
                    rope == reference.as_str(),
                    "seed {seed} step {step}: text mismatch"
                );
            }
        }
    }

    #[test]
    fn test_summary_fields() {
        let text = "ab\ncd\u{1F600}\n"; // 第二行含 4 字节 emoji
        let rope = Rope::from(text);
        let s = rope.summary();
        assert_eq!(s.len, text.len());
        assert_eq!(s.chars, "ab\ncd\u{1F600}\n".chars().count());
        assert_eq!(
            s.len_utf16.0,
            text.chars().map(char::len_utf16).sum::<usize>()
        );
        assert_eq!(s.lines, Point::new(2, 0)); // 两个换行，末行为空
        assert_eq!(s.first_line_chars, 2);
        assert_eq!(s.last_line_chars, 0);
    }

    #[test]
    fn test_replace_across_chunks() {
        // >128 字节，保证多 chunk
        let base = "x".repeat(300);
        let mut rope = Rope::from(base.as_str());
        let reference = base.clone();
        assert!(rope.len() > MAX_BASE);

        rope.replace(100..200, "hello");
        let mut expected = reference;
        expected.replace_range(100..200, "hello");
        assert!(rope == expected.as_str());
        assert_eq!(rope.len(), expected.len());
    }

    #[test]
    fn test_utf16_roundtrip_with_astral() {
        let text = "a\u{1F600}b\u{1F601}c"; // 每个 emoji 占 2 个 UTF-16 unit
        let rope = Rope::from(text);
        let utf16 = rope.offset_to_offset_utf16(rope.len());
        assert_eq!(utf16.0, text.chars().map(char::len_utf16).sum::<usize>());
        assert_eq!(rope.offset_utf16_to_offset(utf16), rope.len());

        // 落在 emoji 中间的 UTF-16 偏移，向前收敛
        let mid = OffsetUtf16(2); // "a" + emoji 的第一个 unit
        assert_eq!(rope.offset_utf16_to_offset(mid), 1); // 收敛到 emoji 首字节
    }

    #[test]
    fn test_cursor_slice_suffix() {
        let text = "hello world, this is a longer rope for chunks".repeat(10);
        let rope = Rope::from(text.as_str());
        let mut cursor = rope.cursor(0);
        let head = cursor.slice(100);
        assert!(head == text[..100]);
        let rest = cursor.suffix();
        assert!(rest == text[100..]);
    }

    #[test]
    fn test_clip_offset() {
        let rope = Rope::from("ab\u{1F600}cd"); // emoji 占 2..6 字节
        assert_eq!(rope.clip_offset(0, Bias::Left), 0);
        assert_eq!(rope.clip_offset(3, Bias::Left), 2);
        assert_eq!(rope.clip_offset(3, Bias::Right), 6);
        assert_eq!(rope.clip_offset(99, Bias::Left), rope.len());
    }

    /// InputState 的 copy/paste 链路：text_in_range 取选中段 →
    /// replace 插到光标处。含单 chunk 与跨 chunk 两种情形。
    #[test]
    fn test_copy_paste_flow() {
        let mut rope = Rope::from("hello world");
        let copied = rope.text_in_range(6..11);
        assert_eq!(copied, "world");

        // 末尾粘贴
        rope.replace(11..11, &copied);
        assert!(rope == "hello worldworld", "got {:?}", rope.to_string());

        // 中间粘贴（光标在 5，即 "hello| worldworld" 的空格前）
        rope.replace(5..5, &copied);
        assert!(
            rope == "helloworld worldworld",
            "got {:?}",
            rope.to_string()
        );

        // 全选复制 → 全选替换（覆盖粘贴）
        let all = rope.text_in_range(0..rope.len());
        let upper = all.to_uppercase();
        rope.replace(0..rope.len(), &upper);
        assert!(rope == upper.as_str());

        // 跨 chunk 拷贝：>128 字节，取中段
        let big = Rope::from("y".repeat(300).as_str());
        let mid = big.text_in_range(100..200);
        assert_eq!(mid.len(), 100);

        // 跨 chunk 粘贴多字节文本
        let mut rope2 = Rope::from("y".repeat(300).as_str());
        rope2.replace(150..150, "中文\u{1F600}");
        assert_eq!(rope2.len(), 300 + 2 * 3 + 4);
        let expected = format!("{}中文\u{1F600}{}", "y".repeat(150), "y".repeat(150));
        assert!(rope2 == expected.as_str(), "got {:?}", rope2.to_string());
    }

    fn random_text(rng: &mut impl rand::Rng, len: usize) -> String {
        // 混入 ASCII / 中文 / emoji / 换行，覆盖 1-4 字节字符
        let alphabet = [
            'a',
            'b',
            'c',
            '\n',
            '中',
            '文',
            '\u{1F600}',
            '\u{1F601}',
            'x',
            ' ',
            '\t',
        ];
        let mut s = String::new();
        while s.len() < len {
            let c = alphabet[rng.random_range(0..alphabet.len())];
            s.push(c);
        }
        s
    }

    fn random_char_range(text: &str, rng: &mut impl rand::Rng) -> (usize, usize) {
        let offsets: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        if offsets.is_empty() {
            return (0, 0);
        }
        let a = rng.random_range(0..offsets.len());
        let b = rng.random_range(a..offsets.len());
        (offsets[a], offsets[b])
    }

    fn str_offset_to_utf16(text: &str, offset: usize) -> usize {
        text[..offset].chars().map(char::len_utf16).sum()
    }

    fn str_offset_to_point(text: &str, offset: usize) -> Point {
        let mut point = Point::zero();
        for c in text[..offset].chars() {
            if c == '\n' {
                point.row += 1;
                point.column = 0;
            } else {
                point.column += c.len_utf8() as u32;
            }
        }
        point
    }
}
