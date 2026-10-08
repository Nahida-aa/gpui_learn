use std::cmp::{self, Ordering};
use std::ops::{Range, RangeInclusive};

use itertools::Either;

pub fn expanded_and_wrapped_usize_range(
    range: Range<usize>,
    additional_before: usize,
    additional_after: usize,
    wrap_length: usize,
) -> impl Iterator<Item = usize> {
    let start_wraps = range.start < additional_before;
    let end_wraps = wrap_length < range.end + additional_after;
    if start_wraps && end_wraps {
        Either::Left(0..wrap_length)
    } else if start_wraps {
        let wrapped_start = (range.start + wrap_length).saturating_sub(additional_before);
        if wrapped_start <= range.end {
            Either::Left(0..wrap_length)
        } else {
            Either::Right((0..range.end + additional_after).chain(wrapped_start..wrap_length))
        }
    } else if end_wraps {
        let wrapped_end = range.end + additional_after - wrap_length;
        if range.start <= wrapped_end {
            Either::Left(0..wrap_length)
        } else {
            Either::Right((0..wrapped_end).chain(range.start - additional_before..wrap_length))
        }
    } else {
        Either::Left((range.start - additional_before)..(range.end + additional_after))
    }
}

/// Yields `[i, i + 1, i - 1, i + 2, ..]`, each modulo `wrap_length` and bounded by
/// `additional_before` and `additional_after`. If the wrapping causes overlap, duplicates are not
/// emitted. If wrap_length is 0, nothing is yielded.
pub fn wrapped_usize_outward_from(
    start: usize,
    additional_before: usize,
    additional_after: usize,
    wrap_length: usize,
) -> impl Iterator<Item = usize> {
    let mut count = 0;
    let mut after_offset = 1;
    let mut before_offset = 1;

    std::iter::from_fn(move || {
        count += 1;
        if count > wrap_length {
            None
        } else if count == 1 {
            Some(start % wrap_length)
        } else if after_offset <= additional_after && after_offset <= before_offset {
            let value = (start + after_offset) % wrap_length;
            after_offset += 1;
            Some(value)
        } else if before_offset <= additional_before {
            let value = (start + wrap_length - before_offset) % wrap_length;
            before_offset += 1;
            Some(value)
        } else if after_offset <= additional_after {
            let value = (start + after_offset) % wrap_length;
            after_offset += 1;
            Some(value)
        } else {
            None
        }
    })
}

pub trait RangeExt<T> {
    fn sorted(&self) -> Self;
    fn to_inclusive(&self) -> RangeInclusive<T>;
    fn overlaps(&self, other: &Range<T>) -> bool;
    fn contains_inclusive(&self, other: &Range<T>) -> bool;
}

impl<T: Ord + Clone> RangeExt<T> for Range<T> {
    fn sorted(&self) -> Self {
        cmp::min(&self.start, &self.end).clone()..cmp::max(&self.start, &self.end).clone()
    }

    fn to_inclusive(&self) -> RangeInclusive<T> { self.start.clone()..=self.end.clone() }

    fn overlaps(&self, other: &Range<T>) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn contains_inclusive(&self, other: &Range<T>) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

impl<T: Ord + Clone> RangeExt<T> for RangeInclusive<T> {
    fn sorted(&self) -> Self {
        cmp::min(self.start(), self.end()).clone()..=cmp::max(self.start(), self.end()).clone()
    }

    fn to_inclusive(&self) -> RangeInclusive<T> { self.clone() }

    fn overlaps(&self, other: &Range<T>) -> bool {
        self.start() < &other.end && &other.start <= self.end()
    }

    fn contains_inclusive(&self, other: &Range<T>) -> bool {
        self.start() <= &other.start && &other.end <= self.end()
    }
}
