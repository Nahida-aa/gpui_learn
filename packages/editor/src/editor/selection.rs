//! 选区数据结构,对齐 zed `crates/text/src/selection.rs`。
//!
//! `Selection<T>` 泛型于坐标:阶段 A 用 `usize`(buffer 字节偏移),
//! display_map 落地后移动动作在 display 空间操作(`Selection<DisplayPoint>`)。
//! `goal` 记录上下移动时要保持的像素列位——这是"上下行光标不跳列"的关键。

use std::{cmp::Ordering, ops::Range};

/// 选区的"期望列位"。
///
/// 上下移动时光标应保持视觉列,而不是字节列(等宽字体下字节列也会变),
/// 因此记像素位置;任何水平移动(左右/点击)都应重置为 [`SelectionGoal::None`]。
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum SelectionGoal {
    #[default]
    None,
    /// 上下移动时保持的 x 坐标(像素)。
    HorizontalPosition(f64),
}

/// 半开区间 `[start, end)` 上的一个选区。
///
/// `reversed` 标记方向:`head()`(活动端)在 `reversed` 时是 `start`。
/// zed 用 `id` 支持多光标去重,单选区阶段省略。
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Selection<T> {
    pub start: T,
    pub end: T,
    pub reversed: bool,
    pub goal: SelectionGoal,
}

impl<T: Copy + Default> Default for Selection<T> {
    fn default() -> Self {
        Selection {
            start: T::default(),
            end: T::default(),
            reversed: false,
            goal: SelectionGoal::None,
        }
    }
}

impl<T: Clone> Selection<T> {
    /// 半开区间视图(编辑路径最常用)。
    pub fn range(&self) -> Range<T> {
        self.start.clone()..self.end.clone()
    }

    /// 选区当前停留的一端(键盘/编辑作用点)。
    pub fn head(&self) -> T {
        if self.reversed {
            self.start.clone()
        } else {
            self.end.clone()
        }
    }

    /// 选区的起始端(拖拽起点)。
    pub fn tail(&self) -> T {
        if self.reversed {
            self.end.clone()
        } else {
            self.start.clone()
        }
    }

    /// 折叠选区到一点(光标移动后调用)。
    pub fn collapse_to(&mut self, point: T, new_goal: SelectionGoal) {
        self.start = point.clone();
        self.end = point;
        self.goal = new_goal;
        self.reversed = false;
    }

    /// builder 风格设置 goal(测试与调用方便利)。
    pub fn with_goal(mut self, goal: SelectionGoal) -> Self {
        self.goal = goal;
        self
    }
}

impl<T: Copy + Ord> Selection<T> {
    pub fn new(head: T, tail: T) -> Self {
        let mut selection = Selection {
            start: tail,
            end: head,
            reversed: false,
            goal: SelectionGoal::None,
        };
        selection.set_head_tail(head, tail, SelectionGoal::None);
        selection
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// 把活动端移到 `head`(Shift 选区的核心语义:可越过 tail 翻转方向)。
    pub fn set_head(&mut self, head: T, new_goal: SelectionGoal) {
        if head.cmp(&self.tail()) < Ordering::Equal {
            if !self.reversed {
                self.end = self.start;
                self.reversed = true;
            }
            self.start = head;
        } else {
            if self.reversed {
                self.start = self.end;
                self.reversed = false;
            }
            self.end = head;
        }
        self.goal = new_goal;
    }

    /// 把起始端移到 `tail`(行为与 set_head 对称,编辑中较少直接用)。
    pub fn set_tail(&mut self, tail: T, new_goal: SelectionGoal) {
        if tail.cmp(&self.head()) <= Ordering::Equal {
            if self.reversed {
                self.end = self.start;
                self.reversed = false;
            }
            self.start = tail;
        } else {
            if !self.reversed {
                self.start = self.end;
                self.reversed = true;
            }
            self.end = tail;
        }
        self.goal = new_goal;
    }

    /// 两端同时指定(head 是否超过 tail 决定方向)。
    pub fn set_head_tail(&mut self, head: T, tail: T, new_goal: SelectionGoal) {
        if head < tail {
            self.reversed = true;
            self.start = head;
            self.end = tail;
        } else {
            self.reversed = false;
            self.start = tail;
            self.end = head;
        }
        self.goal = new_goal;
    }
}

impl<T: Copy + Ord> From<Range<T>> for Selection<T> {
    fn from(range: Range<T>) -> Self {
        Selection {
            start: range.start,
            end: range.end,
            reversed: false,
            goal: SelectionGoal::None,
        }
    }
}

impl<T: Copy + Ord> From<&Selection<T>> for Range<T> {
    fn from(selection: &Selection<T>) -> Self {
        selection.start..selection.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_tail_reversal() {
        let mut s = Selection::new(5, 5);
        assert!(s.is_empty());
        assert_eq!(s.head(), 5);

        // 向左扩展 → 翻转为 reversed,head 变 start
        s.set_head(2, SelectionGoal::None);
        assert!(s.reversed);
        assert_eq!(s.head(), 2);
        assert_eq!(s.tail(), 5);

        // 再向右越过 tail → 翻回正向
        s.set_head(7, SelectionGoal::None);
        assert!(!s.reversed);
        assert_eq!(s.head(), 7);
        assert_eq!(s.tail(), 5);
        assert_eq!(Range::from(&s), 5..7);
    }

    #[test]
    fn test_collapse() {
        let mut s = Selection::new(2, 8);
        s.collapse_to(4, SelectionGoal::HorizontalPosition(12.0));
        assert!(s.is_empty());
        assert!(!s.reversed);
        assert_eq!(s.goal, SelectionGoal::HorizontalPosition(12.0));
    }

    #[test]
    fn test_set_head_tail_direction() {
        // head < tail → reversed(选区从 tail 拖到 head)
        let s = Selection::new(3, 9);
        assert!(s.reversed);
        assert_eq!(Range::from(&s), 3..9);
        assert_eq!(s.head(), 3);
        assert_eq!(s.tail(), 9);
    }
}
