use std::cmp;
use std::ops;


use crate::locator::Locator;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FullOffset(pub usize);

impl ops::AddAssign<usize> for FullOffset {
    fn add_assign(&mut self, rhs: usize) { self.0 += rhs; }
}

impl ops::Add<usize> for FullOffset {
    type Output = Self;

    fn add(mut self, rhs: usize) -> Self::Output {
        self += rhs;
        self
    }
}

impl ops::Sub for FullOffset {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output { self.0 - rhs.0 }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum VersionedFullOffset {
    Offset(FullOffset),
    Invalid,
}

impl VersionedFullOffset {
    pub(crate) fn full_offset(&self) -> FullOffset {
        if let Self::Offset(position) = self {
            *position
        } else {
            panic!("invalid version")
        }
    }
}

impl Default for VersionedFullOffset {
    fn default() -> Self { Self::Offset(Default::default()) }
}

// 下面这些 sum_tree::Dimension / SeekTarget impl 依赖 FragmentSummary，
// 因此它们更自然地属于 fragment.rs；如果不想让 dimensions.rs 依赖
// fragment.rs，可以把这几个 impl 一起搬到 fragment.rs 里。

impl<'a> sum_tree::Dimension<'a, super::fragment::FragmentSummary> for usize {
    fn zero(_: &Option<clock::Global>) -> Self { Default::default() }
    fn add_summary(
        &mut self,
        summary: &super::fragment::FragmentSummary,
        _: &Option<clock::Global>,
    ) {
        *self += summary.text.visible;
    }
}

impl<'a> sum_tree::Dimension<'a, super::fragment::FragmentSummary> for FullOffset {
    fn zero(_: &Option<clock::Global>) -> Self { Default::default() }
    fn add_summary(
        &mut self,
        summary: &super::fragment::FragmentSummary,
        _: &Option<clock::Global>,
    ) {
        self.0 += summary.text.visible + summary.text.deleted;
    }
}

impl<'a> sum_tree::Dimension<'a, super::fragment::FragmentSummary> for Option<&'a Locator> {
    fn zero(_: &Option<clock::Global>) -> Self { Default::default() }
    fn add_summary(
        &mut self,
        summary: &'a super::fragment::FragmentSummary,
        _: &Option<clock::Global>,
    ) {
        *self = Some(&summary.max_id);
    }
}

impl<'a> sum_tree::Dimension<'a, super::fragment::FragmentSummary> for VersionedFullOffset {
    fn zero(_cx: &Option<clock::Global>) -> Self { Default::default() }

    fn add_summary(
        &mut self,
        summary: &'a super::fragment::FragmentSummary,
        cx: &Option<clock::Global>,
    ) {
        if let Self::Offset(offset) = self {
            let version = cx.as_ref().unwrap();
            if version.observed_all(&summary.max_insertion_version) {
                *offset += summary.text.visible + summary.text.deleted;
            } else if version.observed_any(&summary.min_insertion_version) {
                *self = Self::Invalid;
            }
        }
    }
}

impl
    sum_tree::SeekTarget<'_, super::fragment::FragmentSummary, super::fragment::FragmentTextSummary>
    for usize
{
    fn cmp(
        &self,
        cursor_location: &super::fragment::FragmentTextSummary,
        _: &Option<clock::Global>,
    ) -> cmp::Ordering {
        Ord::cmp(self, &cursor_location.visible)
    }
}

impl sum_tree::SeekTarget<'_, super::fragment::FragmentSummary, Self> for VersionedFullOffset {
    fn cmp(&self, cursor_position: &Self, _: &Option<clock::Global>) -> cmp::Ordering {
        match (self, cursor_position) {
            (Self::Offset(a), Self::Offset(b)) => Ord::cmp(a, b),
            (Self::Offset(_), Self::Invalid) => cmp::Ordering::Less,
            (Self::Invalid, _) => unreachable!(),
        }
    }
}
