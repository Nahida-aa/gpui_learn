//! 逻辑时钟：`ReplicaId` / `Lamport` / `Global`（版本向量）。
//!
//! 与 zed `crates/clock/src/clock.rs` 同构，但去掉了 zed 协同协议特有的副本常量
//! （`REMOTE_SERVER` / `AGENT` / `LOCAL_BRANCH` / `FIRST_COLLAB_ID` 与 `is_remote`）——
//! 它们是 zed collab 服务端的语义，我们目前只有本地副本。将来真做协同时再按自己的
//! 协议加回来，别照抄。

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    cmp::{self, Ordering},
    fmt,
};

/// A unique identifier for each replica that can produce edits.
///
/// 我们目前只有一个本地副本，所以只定义 [`ReplicaId::LOCAL`]。
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ReplicaId(u16);

impl ReplicaId {
    /// The local replica.
    pub const LOCAL: ReplicaId = ReplicaId(0);

    pub fn new(id: u16) -> Self {
        ReplicaId(id)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

impl fmt::Debug for ReplicaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == ReplicaId::LOCAL {
            write!(f, "<local>")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// A [Lamport sequence number](https://en.wikipedia.org/wiki/Lamport_timestamp).
pub type Seq = u32;

/// A [Lamport timestamp](https://en.wikipedia.org/wiki/Lamport_timestamp),
/// used to determine the ordering of events.
#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Lamport {
    pub value: Seq,
    pub replica_id: ReplicaId,
}

/// A [version vector](https://en.wikipedia.org/wiki/Version_vector).
///
/// 记录「每个副本各自观察到的最大序号」。比较操作见 [`Global::observed_all`] /
/// [`Global::changed_since`]。
#[derive(Default, Hash, Eq, PartialEq)]
pub struct Global {
    // 4 is chosen as it is the biggest count that does not increase the size of the field itself.
    values: SmallVec<[u32; 4]>,
}

impl Clone for Global {
    fn clone(&self) -> Self {
        // We manually implement clone to avoid the overhead of SmallVec's clone implementation.
        // Using `from_slice` is faster than `clone` for SmallVec as we can use our `Copy` implementation of u32.
        Self {
            values: SmallVec::from_slice(&self.values),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.values.clone_from(&source.values);
    }
}

impl Global {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fetches the sequence number for the given replica ID.
    pub fn get(&self, replica_id: ReplicaId) -> Seq {
        self.values.get(replica_id.0 as usize).copied().unwrap_or(0) as Seq
    }

    /// Observe the lamport timestamp.
    ///
    /// This sets the current sequence number of the observed replica ID to the maximum of this global's observed sequence and the observed timestamp.
    pub fn observe(&mut self, timestamp: Lamport) {
        debug_assert_ne!(timestamp.replica_id, Lamport::MAX.replica_id);
        if timestamp.value > 0 {
            let new_len = timestamp.replica_id.0 as usize + 1;
            if new_len > self.values.len() {
                self.values.resize(new_len, 0);
            }

            let entry = &mut self.values[timestamp.replica_id.0 as usize];
            *entry = cmp::max(*entry, timestamp.value);
        }
    }

    /// Join another global.
    ///
    /// This observes all timestamps from the other global.
    #[doc(alias = "synchronize")]
    pub fn join(&mut self, other: &Self) {
        if other.values.len() > self.values.len() {
            self.values.resize(other.values.len(), 0);
        }

        for (left, right) in self.values.iter_mut().zip(&other.values) {
            *left = cmp::max(*left, *right);
        }
    }

    /// Meet another global.
    ///
    /// Sets all unobserved timestamps of this global to the sequences of other and sets all observed timestamps of this global to the minimum observed of both globals.
    pub fn meet(&mut self, other: &Self) {
        if other.values.len() > self.values.len() {
            self.values.resize(other.values.len(), 0);
        }

        let mut new_len = 0;
        for (ix, (left, &right)) in self.values.iter_mut().zip(&other.values).enumerate() {
            match (*left, right) {
                // left has not observed the replica
                (0, _) => *left = right,
                // right has not observed the replica
                (_, 0) => (),
                (_, _) => *left = cmp::min(*left, right),
            }
            if *left != 0 {
                new_len = ix + 1;
            }
        }
        if other.values.len() == self.values.len() {
            // only truncate if other was equal or shorter (which at this point
            // cant be due to the resize above) to `self` as otherwise we would
            // truncate the unprocessed tail that is guaranteed to contain
            // non-null timestamps
            self.values.truncate(new_len);
        }
    }

    pub fn observed(&self, timestamp: Lamport) -> bool {
        self.get(timestamp.replica_id) >= timestamp.value
    }

    pub fn observed_any(&self, other: &Self) -> bool {
        self.iter()
            .zip(other.iter())
            .any(|(left, right)| right.value > 0 && left.value >= right.value)
    }

    pub fn observed_all(&self, other: &Self) -> bool {
        if self.values.len() < other.values.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .all(|(left, right)| left.value >= right.value)
    }

    pub fn changed_since(&self, other: &Self) -> bool {
        self.values.len() > other.values.len()
            || self
                .values
                .iter()
                .zip(other.values.iter())
                .any(|(left, right)| left > right)
    }

    pub fn most_recent(&self) -> Option<Lamport> {
        self.iter().max_by_key(|timestamp| timestamp.value)
    }

    /// Iterates all replicas observed by this global as well as any unobserved replicas whose ID is lower than the highest observed replica.
    pub fn iter(&self) -> impl Iterator<Item = Lamport> + '_ {
        self.values
            .iter()
            .enumerate()
            .map(|(replica_id, seq)| Lamport {
                replica_id: ReplicaId(replica_id as u16),
                value: *seq,
            })
    }
}

impl FromIterator<Lamport> for Global {
    fn from_iter<T: IntoIterator<Item = Lamport>>(locals: T) -> Self {
        let mut result = Self::new();
        for local in locals {
            result.observe(local);
        }
        result
    }
}

impl Ord for Lamport {
    fn cmp(&self, other: &Self) -> Ordering {
        // Use the replica id to break ties between concurrent events.
        self.value
            .cmp(&other.value)
            .then_with(|| self.replica_id.cmp(&other.replica_id))
    }
}

impl PartialOrd for Lamport {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Lamport {
    pub const MIN: Self = Self {
        replica_id: ReplicaId(u16::MIN),
        value: Seq::MIN,
    };

    pub const MAX: Self = Self {
        replica_id: ReplicaId(u16::MAX),
        value: Seq::MAX,
    };

    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            value: 1,
            replica_id,
        }
    }

    pub fn as_u64(self) -> u64 {
        ((self.value as u64) << 32) | (self.replica_id.0 as u64)
    }

    pub fn tick(&mut self) -> Self {
        let timestamp = *self;
        self.value += 1;
        timestamp
    }

    pub fn observe(&mut self, timestamp: Self) {
        self.value = cmp::max(self.value, timestamp.value) + 1;
    }
}

impl fmt::Debug for Lamport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::MAX {
            write!(f, "Lamport {{MAX}}")
        } else if *self == Self::MIN {
            write!(f, "Lamport {{MIN}}")
        } else {
            write!(f, "Lamport {{{:?}: {}}}", self.replica_id, self.value)
        }
    }
}

impl fmt::Debug for Global {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Global {{")?;
        for timestamp in self.iter().filter(|t| t.value > 0) {
            if timestamp.replica_id.0 > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}: {}", timestamp.replica_id, timestamp.value)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lamport(replica_id: u16, value: Seq) -> Lamport {
        Lamport {
            replica_id: ReplicaId::new(replica_id),
            value,
        }
    }

    #[test]
    fn test_observe_and_get() {
        let mut global = Global::new();
        assert_eq!(global.get(ReplicaId::new(3)), 0);

        global.observe(lamport(3, 7));
        assert_eq!(global.get(ReplicaId::new(3)), 7);

        // 观察更小的序号不应回退。
        global.observe(lamport(3, 2));
        assert_eq!(global.get(ReplicaId::new(3)), 7);

        // 观察更大的序号才推进。
        global.observe(lamport(3, 9));
        assert_eq!(global.get(ReplicaId::new(3)), 9);

        assert!(global.observed(lamport(3, 9)));
        assert!(!global.observed(lamport(3, 10)));
    }

    #[test]
    fn test_lamport_tick_and_observe() {
        let mut clock = Lamport::new(ReplicaId::LOCAL);
        assert_eq!(clock.value, 1);

        let first = clock.tick();
        assert_eq!(first.value, 1);
        assert_eq!(clock.value, 2);

        // 观察到比自己大的序号时 +1，保证严格大于被观察值。
        clock.observe(Lamport {
            value: 10,
            replica_id: ReplicaId::new(2),
        });
        assert_eq!(clock.value, 11);
    }

    #[test]
    fn test_lamport_ordering_breaks_ties_by_replica_id() {
        let a = lamport(1, 5);
        let b = lamport(2, 5);
        assert!(a < b, "序号相同时用 replica_id 打破平局");

        assert!(lamport(1, 5) < lamport(1, 6));
        assert_eq!(Lamport::MIN.value, 0);
        assert_eq!(Lamport::MAX.value, Seq::MAX);
    }

    #[test]
    fn test_join_takes_max() {
        let left: Global = [lamport(0, 3), lamport(1, 1)].into_iter().collect();
        let right: Global = [lamport(0, 1), lamport(1, 5), lamport(2, 2)]
            .into_iter()
            .collect();

        let mut joined = left.clone();
        joined.join(&right);
        assert_eq!(joined.get(ReplicaId::new(0)), 3);
        assert_eq!(joined.get(ReplicaId::new(1)), 5);
        assert_eq!(joined.get(ReplicaId::new(2)), 2);
    }

    #[test]
    fn test_meet_takes_min_and_adopts_unobserved() {
        let left: Global = [lamport(0, 3), lamport(1, 4)].into_iter().collect();
        let right: Global = [lamport(0, 5), lamport(1, 2)].into_iter().collect();

        let mut met = left;
        met.meet(&right);
        // left 观察过 3，right 观察过 5 → 取 min 3。
        assert_eq!(met.get(ReplicaId::new(0)), 3);
        // 两边都观察过 → 取 min 2。
        assert_eq!(met.get(ReplicaId::new(1)), 2);
    }

    #[test]
    fn test_meet_adopts_when_left_unobserved() {
        let mut left = Global::new();
        let right: Global = [lamport(0, 4)].into_iter().collect();
        left.meet(&right);
        // left 完全没观察过 → 直接采用 right 的值。
        assert_eq!(left.get(ReplicaId::new(0)), 4);
    }

    #[test]
    fn test_observed_all_and_changed_since() {
        let older: Global = [lamport(0, 2), lamport(1, 2)].into_iter().collect();
        let newer: Global = [lamport(0, 3), lamport(1, 2)].into_iter().collect();

        assert!(newer.observed_all(&older));
        assert!(!older.observed_all(&newer));
        assert!(newer.changed_since(&older));
        assert!(!older.changed_since(&newer));

        // 更长的版本向量视为「变了」（有副本是对方不知道的）。
        let longer: Global = [lamport(0, 2), lamport(1, 2), lamport(2, 1)]
            .into_iter()
            .collect();
        assert!(longer.changed_since(&older));
        assert!(!older.observed_all(&longer));
    }

    #[test]
    fn test_most_recent_and_iter() {
        let global: Global = [lamport(0, 2), lamport(1, 9), lamport(2, 4)]
            .into_iter()
            .collect();
        assert_eq!(global.most_recent(), Some(lamport(1, 9)));
        assert_eq!(global.iter().count(), 3);
        assert_eq!(Global::new().most_recent(), None);
    }

    #[test]
    fn test_debug() {
        let global: Global = [lamport(0, 2), lamport(1, 9)].into_iter().collect();
        assert_eq!(format!("{global:?}"), "Global {<local>: 2, 1: 9}");
        assert_eq!(format!("{:?}", Lamport::MAX), "Lamport {MAX}");
        assert_eq!(format!("{:?}", ReplicaId::LOCAL), "<local>");
    }
}
