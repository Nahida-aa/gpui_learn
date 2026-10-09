use std::fmt::Display;
use std::num::NonZeroU64;

use anyhow::{Context as _, Result};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct BufferId(NonZeroU64);

impl Display for BufferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl From<NonZeroU64> for BufferId {
    fn from(id: NonZeroU64) -> Self { BufferId(id) }
}

impl BufferId {
    /// Returns Err if `id` is outside of BufferId domain.
    pub fn new(id: u64) -> Result<Self> {
        let id = NonZeroU64::new(id).context("Buffer id cannot be 0.")?;
        Ok(Self(id))
    }

    /// Increments this buffer id, returning the old value.
    /// So that's a post-increment operator in disguise.
    pub fn next(&mut self) -> Self {
        let old = *self;
        self.0 = self.0.saturating_add(1);
        old
    }

    pub fn to_proto(self) -> u64 { self.into() }
}

impl From<BufferId> for u64 {
    fn from(id: BufferId) -> Self { id.0.get() }
}
