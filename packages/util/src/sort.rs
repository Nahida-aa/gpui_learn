use std::cmp::{self, Ordering};

use unicase::UniCase;

/// Extend a sorted vector with a sorted sequence of items, maintaining the vector's sort order and
/// enforcing a maximum length. This also de-duplicates items. Sort the items according to the given callback. Before calling this,
/// both `vec` and `new_items` should already be sorted according to the `cmp` comparator.
pub fn extend_sorted<T, I, F>(vec: &mut Vec<T>, new_items: I, limit: usize, mut cmp: F)
where
    I: IntoIterator<Item = T>,
    F: FnMut(&T, &T) -> Ordering,
{
    let mut start_index = 0;
    for new_item in new_items {
        if let Err(i) = vec[start_index..].binary_search_by(|m| cmp(m, &new_item)) {
            let index = start_index + i;
            if vec.len() < limit {
                vec.insert(index, new_item);
            } else if index < vec.len() {
                vec.pop();
                vec.insert(index, new_item);
            }
            start_index = index;
        }
    }
}

/// A way to sort strings with starting numbers numerically first, falling back to alphanumeric one,
/// case-insensitive.
#[derive(Debug, PartialEq, Eq)]
pub struct NumericPrefixWithSuffix<'a>(pub(crate) Option<u64>, pub(crate) &'a str);

impl<'a> NumericPrefixWithSuffix<'a> {
    pub fn from_numeric_prefixed_str(str: &'a str) -> Self {
        let i = str.chars().take_while(|c| c.is_ascii_digit()).count();
        let (prefix, remainder) = str.split_at(i);

        let prefix = prefix.parse().ok();
        Self(prefix, remainder)
    }
}

/// When dealing with equality, we need to consider the case of the strings to achieve strict equality
/// to handle cases like "a" < "A" instead of "a" == "A".
impl Ord for NumericPrefixWithSuffix<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0, other.0) {
            (None, None) => UniCase::new(self.1)
                .cmp(&UniCase::new(other.1))
                .then_with(|| self.1.cmp(other.1).reverse()),
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(&b).then_with(|| {
                UniCase::new(self.1)
                    .cmp(&UniCase::new(other.1))
                    .then_with(|| self.1.cmp(other.1).reverse())
            }),
        }
    }
}

impl PartialOrd for NumericPrefixWithSuffix<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
