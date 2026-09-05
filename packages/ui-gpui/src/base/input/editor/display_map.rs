//! 显示映射:buffer 坐标 ↔ display 坐标(软换行层)。
//!
//! 对齐 zed display_map 的概念但只实现 WrapMap 一层(zed 是
//! inlay→fold→tab→wrap→block 五层链;见 docs/editor-roadmap.md 的
//! 有意差异清单)。核心思想不变:**软换行是映射层,不是排版副作用**
//! ——选区/移动在 display 空间进行,与换行宽度解耦。
//!
//! 数据模型:每个 buffer 行记录软换行后的字节分段
//! `Vec<Range<usize>>`(空 Vec 表示整行一行放得下),用 SumTree
//! 维护两个可加维度(buffer 行数 / display 行数),支持 O(log n) 双向查询。
//!
//! 与 zed 的差异:
//! - display column 用**字节偏移**(zed 用字符索引):未换行时映射是
//!   identity,渲染层的点击换算拿到的是 shape 后字节偏移,无需字符计数;
//! - 换行点由调用方提供:测试用「每 N 字节断行」的伪度量做 property
//!   对拍,渲染阶段(阶段 C)传入真实 shape 度量产出的断行点。

use std::ops::Range;

use sum_tree::{Bias, Dimensions, SumTree};

use crate::base::input::engine::Point as BufferPoint;

/// display(视觉)空间坐标:row 是软换行后的视觉行,column 是该视觉行
/// 内的字节偏移。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayPoint {
    pub row: u32,
    pub column: u32,
}

impl DisplayPoint {
    pub fn new(row: u32, column: u32) -> Self {
        DisplayPoint { row, column }
    }

    pub fn zero() -> Self {
        DisplayPoint::new(0, 0)
    }
}

/// 视觉行号。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayRow(pub u32);

/// 一个 buffer 行的换行分段。
///
/// `segments` 为空 ⇒ 该行未换行(整行一个 display 行);
/// 否则每段是该视觉行覆盖的字节范围(相对 buffer 行首)。
#[derive(Clone, Debug)]
struct WrapItem {
    segments: Vec<Range<usize>>,
}

impl sum_tree::Item for WrapItem {
    type Summary = WrapSummary;

    fn summary(&self, _: ()) -> Self::Summary {
        WrapSummary {
            buffer_lines: 1,
            display_lines: self.display_line_count(),
        }
    }
}

impl WrapItem {
    fn display_line_count(&self) -> u32 {
        self.segments.len().max(1) as u32
    }

    /// buffer 行内字节偏移 → (视觉行 index, 视觉行内偏移)。
    fn locate(&self, column: usize) -> (u32, u32) {
        if self.segments.is_empty() {
            return (0, column as u32);
        }
        for (ix, segment) in self.segments.iter().enumerate() {
            if column < segment.end {
                return (ix as u32, (column - segment.start) as u32);
            }
        }
        // column 落在最后一个分段内/行尾
        let last = self.segments.len() - 1;
        let segment = &self.segments[last];
        (last as u32, (column.min(segment.end) - segment.start) as u32)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WrapSummary {
    buffer_lines: u32,
    display_lines: u32,
}

impl sum_tree::ContextLessSummary for WrapSummary {
    fn zero() -> Self {
        Default::default()
    }

    fn add_summary(&mut self, summary: &Self) {
        self.buffer_lines += summary.buffer_lines;
        self.display_lines += summary.display_lines;
    }
}

/// seek 维度:newtype 各自绑定一个计数——裸 `u32` 只能实现一种
/// `Dimension`(sum_tree 的 blanket 限制),display/buffer 行数各需独立类型。
/// buffer 行 seek 用 [`BufferRows`],display 行 seek 用 [`DisplayRows`]。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BufferRowCount(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DisplayRowCount(pub u32);

type BufferRows = Dimensions<BufferRowCount, DisplayRowCount, ()>;
type DisplayRows = Dimensions<DisplayRowCount, BufferRowCount, ()>;

impl<'a> sum_tree::Dimension<'a, WrapSummary> for BufferRowCount {
    fn zero(_: ()) -> Self {
        BufferRowCount(0)
    }

    fn add_summary(&mut self, summary: &'a WrapSummary, _: ()) {
        self.0 += summary.buffer_lines;
    }
}

impl<'a> sum_tree::Dimension<'a, WrapSummary> for DisplayRowCount {
    fn zero(_: ()) -> Self {
        DisplayRowCount(0)
    }

    fn add_summary(&mut self, summary: &'a WrapSummary, _: ()) {
        self.0 += summary.display_lines;
    }
}

/// 软换行映射树。
#[derive(Clone, Debug, Default)]
pub struct DisplayMap {
    wraps: SumTree<WrapItem>,
    /// buffer 总行数(行号从 0 起;空 buffer 视为 1 个空行)。
    buffer_rows: u32,
}

impl DisplayMap {
    /// 建立与 buffer 行数一致的映射(全部未换行)。
    pub fn new(buffer_rows: u32) -> Self {
        let buffer_rows = buffer_rows.max(1);
        let mut map = DisplayMap {
            wraps: SumTree::default(),
            buffer_rows,
        };
        let items = (0..buffer_rows).map(|_| WrapItem { segments: Vec::new() });
        map.wraps.extend(items, ());
        map
    }

    pub fn buffer_rows(&self) -> u32 {
        self.buffer_rows
    }

    /// 最后一个 display 行号(总 display 行数 - 1)。
    pub fn max_display_row(&self) -> u32 {
        self.wraps.summary().display_lines.saturating_sub(1)
    }

    /// 更新某个 buffer 行的换行分段(渲染层在度量变化后调用)。
    ///
    /// 实现:整树重建。行数量级为可视文档,重建成本可接受;
    /// SumTree 的 splice 式增量更新留到性能需要时做。
    pub fn set_row_segments(&mut self, buffer_row: u32, segments: Vec<Range<usize>>) {
        debug_assert!(buffer_row < self.buffer_rows);
        let old = std::mem::take(&mut self.wraps);
        let mut new_tree = SumTree::default();
        let mut cursor = old.cursor::<BufferRows>(());
        cursor.seek(&BufferRowCount(0), Bias::Left);
        let mut row = 0u32;
        while let Some(item) = cursor.item() {
            if row == buffer_row {
                new_tree.push(WrapItem { segments: segments.clone() }, ());
            } else {
                new_tree.push(item.clone(), ());
            }
            row += 1;
            cursor.next();
        }
        self.wraps = new_tree;
    }

    /// buffer 行 → 该行第一个 display 行号。
    pub fn display_row_for_buffer_row(&self, buffer_row: u32) -> DisplayRow {
        let (start, _, _) = self
            .wraps
            .find::<BufferRows, _>((), &BufferRowCount(buffer_row), Bias::Right);
        DisplayRow(start.1 .0)
    }

    /// buffer Point(行, 字节列)→ DisplayPoint。
    pub fn buffer_point_to_display_point(&self, point: BufferPoint) -> DisplayPoint {
        let (start, _, item) = self
            .wraps
            .find::<BufferRows, _>((), &BufferRowCount(point.row), Bias::Right);
        let display_row = start.1 .0;
        match item {
            Some(wrap) => {
                let (sub_ix, column) = wrap.locate(point.column as usize);
                DisplayPoint::new(display_row + sub_ix, column)
            }
            None => DisplayPoint::new(display_row, point.column),
        }
    }

    /// DisplayPoint → buffer Point。
    pub fn display_point_to_buffer_point(&self, point: DisplayPoint) -> BufferPoint {
        let mut cursor = self.wraps.cursor::<DisplayRows>(());
        cursor.seek(&DisplayRowCount(point.row), Bias::Right);
        // DisplayRows = Dimensions<DisplayRowCount, BufferRowCount>:0 是 display、1 是 buffer
        let display_start = cursor.start().0 .0;
        let buffer_row = cursor.start().1 .0;
        let Some(wrap) = cursor.item() else {
            return BufferPoint::new(buffer_row, point.column);
        };
        if wrap.segments.is_empty() {
            return BufferPoint::new(buffer_row, point.column);
        }
        let sub_ix = (point.row - display_start) as usize;
        let segment = wrap
            .segments
            .get(sub_ix)
            .unwrap_or_else(|| wrap.segments.last().unwrap());
        BufferPoint::new(buffer_row, (segment.start + point.column as usize) as u32)
    }

    /// 某个 display 行的覆盖字节数。
    ///
    /// 未换行的行返回 `None`(宽度信息在 buffer 侧,由 Editor 提供)。
    pub fn display_row_len(&self, row: u32) -> Option<u32> {
        let mut cursor = self.wraps.cursor::<DisplayRows>(());
        cursor.seek(&DisplayRowCount(row), Bias::Right);
        let display_start = cursor.start().0 .0;
        let wrap = cursor.item()?;
        if wrap.segments.is_empty() {
            return None;
        }
        let sub_ix = (row - display_start) as usize;
        wrap.segments.get(sub_ix).map(|s| s.len() as u32)
    }

    pub fn snapshot(&self) -> DisplaySnapshot {
        DisplaySnapshot { map: self.clone() }
    }
}

/// 只读快照(对齐 zed DisplaySnapshot 的角色:渲染帧与移动逻辑持有)。
#[derive(Clone, Debug, Default)]
pub struct DisplaySnapshot {
    map: DisplayMap,
}

impl DisplaySnapshot {
    pub fn buffer_point_to_display_point(&self, point: BufferPoint) -> DisplayPoint {
        self.map.buffer_point_to_display_point(point)
    }

    pub fn display_point_to_buffer_point(&self, point: DisplayPoint) -> BufferPoint {
        self.map.display_point_to_buffer_point(point)
    }

    pub fn max_display_row(&self) -> u32 {
        self.map.max_display_row()
    }

    /// 视觉行宽;`None` 表示未换行(宽度由 buffer 行长度决定)。
    pub fn display_row_len(&self, row: u32) -> Option<u32> {
        self.map.display_row_len(row)
    }

    pub fn buffer_rows(&self) -> u32 {
        self.map.buffer_rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::input::engine::Point;

    /// 每行按 width 字节断行(伪度量;真实度量由渲染层提供)。
    fn segments_for(line_len: usize, width: usize) -> Vec<Range<usize>> {
        if line_len <= width {
            return Vec::new();
        }
        let mut segments = Vec::new();
        let mut start = 0;
        while start < line_len {
            let end = (start + width).min(line_len);
            segments.push(start..end);
            start = end;
        }
        segments
    }

    fn wrap_all(map: &mut DisplayMap, text: &str, width: usize) {
        for (row, line) in text.split('\n').enumerate() {
            map.set_row_segments(row as u32, segments_for(line.len(), width));
        }
    }

    #[test]
    fn test_identity_without_wrap() {
        let map = DisplayMap::new(3);
        // 3 行不换行:display row == buffer row
        assert_eq!(
            map.buffer_point_to_display_point(Point::new(1, 1)),
            DisplayPoint::new(1, 1)
        );
        assert_eq!(
            map.display_point_to_buffer_point(DisplayPoint::new(2, 0)),
            Point::new(2, 0)
        );
        assert_eq!(map.max_display_row(), 2);
    }

    #[test]
    fn test_wrap_expands_display_rows() {
        let mut map = DisplayMap::new(3);
        wrap_all(&mut map, "abcdef\nx\nghijkl", 4);
        // 第 1 行 6 字节按 4 断成 2 段;第 3 行 6 字节 2 段
        assert_eq!(map.display_row_for_buffer_row(0), DisplayRow(0));
        assert_eq!(map.display_row_for_buffer_row(1), DisplayRow(2));
        assert_eq!(map.display_row_for_buffer_row(2), DisplayRow(3));
        assert_eq!(map.max_display_row(), 4); // 2+1+2-1
    }

    #[test]
    fn test_point_mapping_roundtrip_with_wrap() {
        let mut map = DisplayMap::new(2);
        wrap_all(&mut map, "abcdef\ngh", 4);

        // buffer (0,5) → display (1,1):第一行折为 [0..4][4..6],列 5 在第二段偏移 1
        let display = map.buffer_point_to_display_point(Point::new(0, 5));
        assert_eq!(display, DisplayPoint::new(1, 1));
        assert_eq!(map.display_point_to_buffer_point(display), Point::new(0, 5));

        // buffer (1,1) → display (2,1):行0 折为 2 个视觉行(0、1),行1 从 2 起
        let display = map.buffer_point_to_display_point(Point::new(1, 1));
        assert_eq!(display, DisplayPoint::new(2, 1));
        assert_eq!(map.display_point_to_buffer_point(display), Point::new(1, 1));

        // 视觉行宽:第一行第二段 4..6 → 2 字节
        assert_eq!(map.display_row_len(1), Some(2));
        assert_eq!(map.display_row_len(2), None, "未换行行宽度在 buffer 侧");
    }

    #[test]
    fn test_property_random_wraps_roundtrip() {
        let mut rng = rand::rng();
        use rand::Rng;
        for _ in 0..50 {
            let rows = rng.random_range(1..6);
            let mut text = String::new();
            for r in 0..rows {
                let len = rng.random_range(0..30);
                for i in 0..len {
                    // 只用 ASCII 保证字节列 == 字符列
                    text.push((b'a' + (i % 26) as u8) as char);
                }
                if r + 1 < rows {
                    text.push('\n');
                }
            }
            let width = rng.random_range(1..20);

            let mut map = DisplayMap::new(rows as u32);
            wrap_all(&mut map, &text, width);

            // 每个 buffer 点往返一致
            for (row, line) in text.split('\n').enumerate() {
                for col in 0..=line.len() {
                    let buffer = Point::new(row as u32, col as u32);
                    let display = map.buffer_point_to_display_point(buffer);
                    assert_eq!(
                        map.display_point_to_buffer_point(display),
                        buffer,
                        "roundtrip failed: text={text:?} width={width} buffer={buffer:?}"
                    );
                }
            }
        }
    }
}
