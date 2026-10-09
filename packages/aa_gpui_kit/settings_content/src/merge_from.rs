/// Trait for recursively merging settings structures.
///
/// 完全对齐 Zed `crates/settings_content/src/merge_from.rs`。
///
/// The default behaviour of merging is:
/// * For objects with named keys (HashMap, structs, etc.). The values are merged deeply
///   (so if the default settings has languages.JSON.prettier.allowed = true, and the user's settings has
///    languages.JSON.tab_size = 4; the merged settings file will have both settings).
/// * For options, a None value is ignored, but Some values are merged recursively.
/// * For other types (including Vec), a merge overwrites the current value.
#[allow(unused)]
pub trait MergeFrom {
    /// Merge from a source of the same type.
    fn merge_from(&mut self, other: &Self);

    /// Merge from an optional source of the same type.
    fn merge_from_option(&mut self, other: Option<&Self>) {
        if let Some(other) = other {
            self.merge_from(other);
        }
    }
}

macro_rules! merge_from_overwrites {
    ($($type:ty),+ $(,)?) => {
        $(
            impl MergeFrom for $type {
                fn merge_from(&mut self, other: &Self) {
                    *self = other.clone();
                }
            }
        )+
    }
}

merge_from_overwrites!(
    u16,
    u32,
    u64,
    usize,
    i16,
    i32,
    i64,
    bool,
    f64,
    f32,
    char,
    std::num::NonZeroUsize,
    std::num::NonZeroU32,
    String,
    std::sync::Arc<str>,
    std::path::PathBuf,
    std::sync::Arc<std::path::Path>,
    language_model_core::Speed,
);

impl<T: Clone + MergeFrom> MergeFrom for Option<T> {
    fn merge_from(&mut self, other: &Self) {
        let Some(other) = other else {
            return;
        };

        if let Some(this) = self {
            this.merge_from(other);
        } else {
            self.replace(other.clone());
        }
    }
}

impl<T: Clone> MergeFrom for Vec<T> {
    fn merge_from(&mut self, other: &Self) { *self = other.clone() }
}

impl<T: MergeFrom> MergeFrom for Box<T> {
    fn merge_from(&mut self, other: &Self) { self.as_mut().merge_from(other.as_ref()) }
}

// Implementations for collections that extend/merge their contents
// 全用 collections::* — collections crate 通过 pub use std::collections::*
// 同时 re-export 了 BTreeMap/BTreeSet，也定义了 type alias HashMap/HashSet/IndexMap。
impl<K, V> MergeFrom for collections::HashMap<K, V>
where
    K: Clone + std::hash::Hash + Eq,
    V: Clone + MergeFrom,
{
    fn merge_from(&mut self, other: &Self) {
        for (key, value) in other {
            if let Some(existing) = self.get_mut(key) {
                existing.merge_from(value);
            } else {
                self.insert(key.clone(), value.clone());
            }
        }
    }
}

impl<K, V> MergeFrom for collections::BTreeMap<K, V>
where
    K: Clone + std::hash::Hash + Eq + Ord,
    V: Clone + MergeFrom,
{
    fn merge_from(&mut self, other: &Self) {
        for (key, value) in other {
            if let Some(existing) = self.get_mut(key) {
                existing.merge_from(value);
            } else {
                self.insert(key.clone(), value.clone());
            }
        }
    }
}

impl<K, V> MergeFrom for collections::IndexMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone + MergeFrom,
{
    fn merge_from(&mut self, other: &Self) {
        for (key, value) in other {
            if let Some(existing) = self.get_mut(key) {
                existing.merge_from(value);
            } else {
                self.insert(key.clone(), value.clone());
            }
        }
    }
}

impl<T> MergeFrom for collections::BTreeSet<T>
where
    T: Clone + Ord,
{
    fn merge_from(&mut self, other: &Self) {
        for item in other {
            self.insert(item.clone());
        }
    }
}

impl<T> MergeFrom for collections::HashSet<T>
where
    T: Clone + std::hash::Hash + Eq,
{
    fn merge_from(&mut self, other: &Self) {
        for item in other {
            self.insert(item.clone());
        }
    }
}

impl MergeFrom for serde_json::Value {
    fn merge_from(&mut self, other: &Self) {
        match (self, other) {
            (serde_json::Value::Object(this), serde_json::Value::Object(other)) => {
                for (key, value) in other {
                    if let Some(existing) = this.get_mut(key) {
                        existing.merge_from(value);
                    } else {
                        this.insert(key.clone(), value.clone());
                    }
                }
            }
            (this, other) => *this = other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn option_merge_some() {
        let mut base: Option<u32> = Some(10);
        base.merge_from(&Some(20));
        assert_eq!(base, Some(20)); // u32 是 overwrite
    }

    #[test]
    fn option_merge_none_ignored() {
        let mut base: Option<u32> = Some(10);
        base.merge_from(&None);
        assert_eq!(base, Some(10));
    }

    #[test]
    fn btree_map_deep_merge() {
        let mut base: collections::BTreeMap<String, Option<u32>> =
            [("a".into(), Some(1)), ("b".into(), None)].into();
        let other: collections::BTreeMap<String, Option<u32>> =
            [("b".into(), Some(2)), ("c".into(), Some(3))].into();
        base.merge_from(&other);
        assert_eq!(base.get("a"), Some(&Some(1)));
        assert_eq!(base.get("b"), Some(&Some(2)));
        assert_eq!(base.get("c"), Some(&Some(3)));
    }

    #[test]
    fn serde_json_value_object_deep_merge() {
        let mut base = json!({"a": {"x": 1, "y": 2}, "b": 3});
        let other = json!({"a": {"y": 20, "z": 30}, "c": 4});
        base.merge_from(&other);
        assert_eq!(
            base,
            json!({"a": {"x": 1, "y": 20, "z": 30}, "b": 3, "c": 4})
        );
    }
}
