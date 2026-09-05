//! 文本坐标类型：字节偏移、UTF-16 偏移、行列坐标。
//!
//! 移植自 zed `crates/rope/src/{point.rs, point_utf16.rs, offset_utf16.rs}`。
//! 精简点：`Ord` 直接用元组比较（zed 用 64 位打包位运算，语义相同）；
//! 去掉 `Unclipped` newtype（统一用「是否裁剪」参数表达）。

use std::{
    cmp::Ordering,
    fmt,
    ops::{Add, AddAssign, Sub},
};

/// UTF-8 字节偏移（0-indexed）。
///
/// 这是一个 newtype 而非裸 `usize`：在 sum_tree 的维度查询里，
/// 它与 `OffsetUtf16` 是两个不同的「维度」，newtype 保证不会混用。
#[derive(Clone, Copy, Default, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct OffsetUtf16(pub usize);

impl OffsetUtf16 {
    pub fn zero() -> Self {
        OffsetUtf16(0)
    }
}

impl Add for OffsetUtf16 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        OffsetUtf16(self.0 + rhs.0)
    }
}

impl AddAssign for OffsetUtf16 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for OffsetUtf16 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        OffsetUtf16(self.0 - rhs.0)
    }
}

impl PartialEq<usize> for OffsetUtf16 {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

/// 以 UTF-8 字节计的行列坐标（`column` 是行内字节数）。
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct Point {
    pub row: u32,
    pub column: u32,
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Point({}:{})", self.row, self.column)
    }
}

impl Point {
    pub const MAX: Self = Point::new(u32::MAX, u32::MAX);

    pub const fn new(row: u32, column: u32) -> Self {
        Point { row, column }
    }

    pub const fn zero() -> Self {
        Point::new(0, 0)
    }

    pub fn is_zero(&self) -> bool {
        self.row == 0 && self.column == 0
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        if self < other {
            Self::zero()
        } else {
            self - other
        }
    }
}

/// `Point` 的加法语义：`other` 跨行时直接落到 `other` 的行列上。
///
/// 例如 `Point(1, 5) + Point(2, 3) = Point(3, 3)`——右边跨了 2 行，
/// 相加后的列取右边的列（左边的列被换行吞掉）。
impl Add for Point {
    type Output = Point;

    fn add(self, other: Self) -> Self::Output {
        if other.row == 0 {
            Point::new(self.row, self.column + other.column)
        } else {
            Point::new(self.row + other.row, other.column)
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Self) -> Self::Output {
        debug_assert!(other <= self);
        if self.row == other.row {
            Point::new(0, self.column - other.column)
        } else {
            Point::new(self.row - other.row, self.column)
        }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.row, self.column).cmp(&(other.row, other.column))
    }
}

impl<'a> sum_tree::Dimension<'a, super::summary::ChunkSummary> for Point {
    fn zero(_: ()) -> Self {
        Point::zero()
    }

    fn add_summary(&mut self, summary: &'a super::summary::ChunkSummary, _: ()) {
        *self += summary.text.lines;
    }
}

/// 以 UTF-16 code unit 计的行列坐标（IME / 平台文本接口用）。
#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct PointUtf16 {
    pub row: u32,
    pub column: u32,
}

impl fmt::Debug for PointUtf16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PointUtf16({}:{})", self.row, self.column)
    }
}

impl PointUtf16 {
    pub const fn new(row: u32, column: u32) -> Self {
        PointUtf16 { row, column }
    }

    pub const fn zero() -> Self {
        PointUtf16::new(0, 0)
    }
}

impl Add for PointUtf16 {
    type Output = PointUtf16;

    fn add(self, other: Self) -> Self::Output {
        if other.row == 0 {
            PointUtf16::new(self.row, self.column + other.column)
        } else {
            PointUtf16::new(self.row + other.row, other.column)
        }
    }
}

impl AddAssign for PointUtf16 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for PointUtf16 {
    type Output = PointUtf16;

    fn sub(self, other: Self) -> Self::Output {
        debug_assert!(other <= self);
        if self.row == other.row {
            PointUtf16::new(0, self.column - other.column)
        } else {
            PointUtf16::new(self.row - other.row, self.column)
        }
    }
}

impl PartialOrd for PointUtf16 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PointUtf16 {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.row, self.column).cmp(&(other.row, other.column))
    }
}

impl<'a> sum_tree::Dimension<'a, super::summary::ChunkSummary> for PointUtf16 {
    fn zero(_: ()) -> Self {
        PointUtf16::zero()
    }

    fn add_summary(&mut self, summary: &'a super::summary::ChunkSummary, _: ()) {
        *self += summary.text.lines_utf16();
    }
}

impl<'a> sum_tree::Dimension<'a, super::summary::ChunkSummary> for OffsetUtf16 {
    fn zero(_: ()) -> Self {
        OffsetUtf16::zero()
    }

    fn add_summary(&mut self, summary: &'a super::summary::ChunkSummary, _: ()) {
        *self += summary.text.len_utf16;
    }
}

impl<'a> sum_tree::Dimension<'a, super::summary::ChunkSummary> for usize {
    fn zero(_: ()) -> Self {
        0
    }

    fn add_summary(&mut self, summary: &'a super::summary::ChunkSummary, _: ()) {
        *self += summary.text.len;
    }
}

/// 整段摘要本身也可作为维度（Cursor::summary::<TextSummary> 用）。
impl<'a> sum_tree::Dimension<'a, super::summary::ChunkSummary>
    for super::summary::TextSummary
{
    fn zero(_: ()) -> Self {
        super::summary::TextSummary::default()
    }

    fn add_summary(&mut self, summary: &'a super::summary::ChunkSummary, _: ()) {
        *self += &summary.text;
    }
}
