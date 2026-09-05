//! display 空间的移动原语,函数名与语义对齐 zed `crates/editor/src/movement.rs`。
//!
//! 全部是纯函数:入参 `(map, rope, point)`、出参 `DisplayPoint`,不碰
//! Editor 状态——这样移动逻辑与软换行解耦(map 没有分段时退化为
//! 按行移动),也便于 property 测试。
//!
//! goal 列保持:zed 用像素位(`HorizontalPosition(f64)`);阶段 B 的
//! display 字节列充当临时度量(渲染层接入后换成真实像素,接口不变)。

use unicode_segmentation::UnicodeSegmentation;

use super::display_map::{DisplayPoint, DisplayMap};
use super::selection::SelectionGoal;
use crate::base::input::engine::{Point as BufferPoint, Rope};

// ---- 水平移动 ----

/// 左移一个字符(UTF-8 边界);视觉行首则跳到上一视觉行最后一个字符。
/// 对齐 zed `movement::left`。
pub fn left(map: &DisplayMap, rope: &Rope, point: DisplayPoint) -> DisplayPoint {
    saturating_left(map, rope, point).unwrap_or(point)
}

/// 左移,若已在文档起点则返回 None。
///
/// 实现:display 点 → buffer 字节偏移回退一个字符 → 转回 display。
/// 视觉行尾与下一视觉行首共享同一 buffer 位置,回退自动跨视觉行,
/// 无需特判行首。
pub fn saturating_left(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
) -> Option<DisplayPoint> {
    let buffer = map.display_point_to_buffer_point(point);
    let offset = rope.point_to_offset(buffer);
    if offset == 0 {
        return None;
    }
    let prev = rope.floor_char_boundary(offset - 1);
    Some(map.buffer_point_to_display_point(rope.offset_to_point(prev)))
}

/// 右移一个字符;视觉行尾则跳到下一视觉行首。
pub fn right(map: &DisplayMap, rope: &Rope, point: DisplayPoint) -> DisplayPoint {
    saturating_right(map, rope, point).unwrap_or(point)
}

pub fn saturating_right(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
) -> Option<DisplayPoint> {
    let buffer = map.display_point_to_buffer_point(point);
    let offset = rope.point_to_offset(buffer);
    if offset >= rope.len() {
        return None;
    }
    let next = rope.ceil_char_boundary(offset + 1);
    Some(map.buffer_point_to_display_point(rope.offset_to_point(next)))
}

// ---- 垂直移动(goal 列保持)----

/// 上移一行,保持 goal 列(无 goal 时记当前列)。已在首行则原样返回。
/// 对齐 zed `movement::up`。
pub fn up(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
    goal: SelectionGoal,
) -> (DisplayPoint, SelectionGoal) {
    if point.row == 0 {
        return (point, SelectionGoal::None);
    }
    let goal = resolve_goal(point, goal);
    let target_row = point.row - 1;
    let column = clamp_to_row(map, rope, target_row, goal);
    (DisplayPoint::new(target_row, column), SelectionGoal::HorizontalPosition(goal))
}

/// 下移一行,保持 goal 列。已在末行则跳到末行行尾(zed 语义:不能下移时
/// 折到行尾而不是不动,宿主决定 propagate)。
pub fn down(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
    goal: SelectionGoal,
) -> (DisplayPoint, SelectionGoal) {
    if point.row >= map.max_display_row() {
        let width = display_row_width(map, rope, point.row);
        return (DisplayPoint::new(point.row, width), SelectionGoal::None);
    }
    let goal = resolve_goal(point, goal);
    let target_row = point.row + 1;
    let column = clamp_to_row(map, rope, target_row, goal);
    (DisplayPoint::new(target_row, column), SelectionGoal::HorizontalPosition(goal))
}

fn resolve_goal(point: DisplayPoint, goal: SelectionGoal) -> f64 {
    match goal {
        // 阶段 B 临时度量:display 字节列;阶段 C 换 shape 像素
        SelectionGoal::HorizontalPosition(x) => x,
        SelectionGoal::None => point.column as f64,
    }
}

fn clamp_to_row(map: &DisplayMap, rope: &Rope, row: u32, goal: f64) -> u32 {
    let width = display_row_width(map, rope, row);
    (goal as u32).min(width)
}

// ---- 行首尾 ----

/// 行首。`indented = false` 回到列 0;`true` 先跳过行首空白,
/// 若已在缩进后则回列 0(对齐 zed line_beginning/indented_line_beginning)。
pub fn line_beginning(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
    indented: bool,
) -> DisplayPoint {
    let row_start = map.display_point_to_buffer_point(DisplayPoint::new(point.row, 0));
    let row_start_offset = rope.point_to_offset(row_start);
    let mut target = DisplayPoint::new(point.row, 0);
    if indented {
        let head_offset = map.display_point_to_buffer_point(point);
        let head_offset = rope.point_to_offset(head_offset);
        let prefix = rope.text_in_range(row_start_offset..head_offset);
        let indent_width = prefix.len() - prefix.trim_start().len();
        let indent_point = DisplayPoint::new(point.row, indent_width as u32);
        if indent_point < point {
            target = indent_point;
        }
    }
    target
}

/// 行尾(当前视觉行的最后一列)。
pub fn line_end(map: &DisplayMap, rope: &Rope, point: DisplayPoint) -> DisplayPoint {
    DisplayPoint::new(point.row, display_row_width(map, rope, point.row))
}

// ---- 词边界 ----

/// 上一个词首。词边界在视觉行内计算(词不跨视觉行);
/// 空白段不计为词(对齐 zed 的 word-char 分类行为)。
pub fn previous_word_start(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
) -> DisplayPoint {
    let (row_text, column) = row_text_and_column(map, rope, point);
    let mut target = point;
    for (idx, word) in row_text.split_word_bound_indices() {
        if idx >= column {
            break;
        }
        if word.trim().is_empty() {
            continue; // 空白段不是词
        }
        target = DisplayPoint::new(point.row, idx as u32);
    }
    target
}

/// 下一个词尾。
pub fn next_word_end(map: &DisplayMap, rope: &Rope, point: DisplayPoint) -> DisplayPoint {
    let (row_text, column) = row_text_and_column(map, rope, point);
    for (idx, word) in row_text.split_word_bound_indices() {
        let end = idx + word.len();
        if end > column && !word.trim().is_empty() {
            return DisplayPoint::new(point.row, end as u32);
        }
    }
    DisplayPoint::new(point.row, row_text.len() as u32)
}

// ---- 辅助 ----

/// 视觉行的字节宽度。
fn display_row_width(map: &DisplayMap, rope: &Rope, row: u32) -> u32 {
    if let Some(len) = map.display_row_len(row) {
        return len;
    }
    // 未换行行:buffer 行长(不含换行符)
    let buffer_row = map.display_point_to_buffer_point(DisplayPoint::new(row, 0)).row;
    rope.line_len(buffer_row) as u32
}

/// 取视觉行文本与光标列(供词边界计算)。
fn row_text_and_column(
    map: &DisplayMap,
    rope: &Rope,
    point: DisplayPoint,
) -> (String, usize) {
    let width = display_row_width(map, rope, point.row) as usize;
    let start = map.display_point_to_buffer_point(DisplayPoint::new(point.row, 0));
    let start = rope.point_to_offset(start);
    (rope.text_in_range(start..start + width), point.column as usize)
}

fn prev_char_boundary(rope: &Rope, point: BufferPoint) -> BufferPoint {
    let offset = rope.point_to_offset(point);
    let prev = rope.floor_char_boundary(offset.saturating_sub(1));
    rope.offset_to_point(prev)
}

fn next_char_boundary(rope: &Rope, point: BufferPoint) -> BufferPoint {
    let offset = rope.point_to_offset(point);
    let next = rope.ceil_char_boundary((offset + 1).min(rope.len()));
    rope.offset_to_point(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::input::engine::Point;

    fn setup(text: &str, width: usize) -> (DisplayMap, Rope) {
        let rope = Rope::from(text);
        let mut map = DisplayMap::new(rope.summary().lines.row + 1);
        for (row, line) in text.split('\n').enumerate() {
            let segments = if line.len() > width {
                let mut segments = Vec::new();
                let mut start = 0;
                while start < line.len() {
                    let end = (start + width).min(line.len());
                    segments.push(start..end);
                    start = end;
                }
                segments
            } else {
                Vec::new()
            };
            map.set_row_segments(row as u32, segments);
        }
        (map, rope)
    }

    fn dp(row: u32, column: u32) -> DisplayPoint {
        DisplayPoint::new(row, column)
    }

    #[test]
    fn test_left_right_within_row() {
        let (map, rope) = setup("abcdef", 100);
        let p = dp(0, 3);
        assert_eq!(left(&map, &rope, p), dp(0, 2));
        assert_eq!(right(&map, &rope, p), dp(0, 4));
        // 行尾右移 → 下一视觉行首(无换行时 saturating 返回 None → right 原样)
        let end = dp(0, 6);
        assert_eq!(saturating_right(&map, &rope, end), None);
        // 行首左移 → None
        assert_eq!(saturating_left(&map, &rope, dp(0, 0)), None);
    }

    #[test]
    fn test_left_right_across_wrap() {
        let (map, rope) = setup("abcdef", 4); // 折为 [0..4][4..6]
        // 第一段行首左移 → 上一行?没有上一行 → None
        assert_eq!(saturating_left(&map, &rope, dp(0, 0)), None);
        // 第二段(视觉行 1)首左移 → 视觉行 0 的列 3(字符边界:字节 3)
        assert_eq!(left(&map, &rope, dp(1, 0)), dp(0, 3));
        // 视觉行 0 尾右移 → 视觉行 1 首
        assert_eq!(right(&map, &rope, dp(0, 3)), dp(1, 0));
    }

    #[test]
    fn test_up_down_with_goal() {
        let (map, rope) = setup("abcdefgh\nxyz", 4); // 第 1 行折 2 视觉行
        // display (1, 2) 上移 → (0, 2),goal 记 2
        let (p, goal) = up(&map, &rope, dp(1, 2), SelectionGoal::None);
        assert_eq!(p, dp(0, 2));
        assert_eq!(goal, SelectionGoal::HorizontalPosition(2.0));

        // 保持 goal 下移到第二行:列 2
        let (p, _) = down(&map, &rope, p, goal);
        assert_eq!(p, dp(1, 2));

        // 从第二行(短行 xyz)下移到末行:列 clamp
        let (p, _) = down(&map, &rope, dp(1, 2), SelectionGoal::None);
        assert_eq!(p, dp(2, 2));
    }

    #[test]
    fn test_line_beginning_indented() {
        let (map, rope) = setup("    hello", 100);
        let p = dp(0, 7);
        // indented:先到缩进后(列 4)
        assert_eq!(line_beginning(&map, &rope, p, true), dp(0, 4));
        // 已在缩进后再次触发 → 列 0(ze 行为首按语义:回到 0)
        assert_eq!(line_beginning(&map, &rope, dp(0, 4), true), dp(0, 0));
        assert_eq!(line_end(&map, &rope, p), dp(0, 9));
    }

    #[test]
    fn test_word_movement() {
        let (map, rope) = setup("hello world foo", 100);
        let p = dp(0, 5); // "hello|" 行尾(在 'o' 后)
        assert_eq!(previous_word_start(&map, &rope, p), dp(0, 0));
        assert_eq!(next_word_end(&map, &rope, dp(0, 0)), dp(0, 5));
        assert_eq!(next_word_end(&map, &rope, dp(0, 5)), dp(0, 11));
    }
}
