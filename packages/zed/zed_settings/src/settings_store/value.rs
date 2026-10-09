//! RegisterSetting 编译期注册项 + SettingValue 运行时存储。
//!
//! 对齐 Zed `crates/settings/src/settings_store.rs` 中的同名类型。

use std::any::{Any, TypeId, type_name};
use std::path::PathBuf;
use std::sync::Arc;

use path::rel_path::RelPath;

use crate::{Settings, WorktreeId};
use crate::{SettingsContent, SettingsLocation};

// ---------- SettingValue ----------

/// 每种 setting 类型的运行时值存储。
///
/// 对齐 Zed：
/// - `global_value` — 全局（worktree 级别）的 setting 值
/// - `local_values` — per-worktree + per-project 的本地覆盖
///   当前 aacode 还没多 worktree / 多 workspace，Vec 始终为空，
///   但 Zed 形状先完整搭好，以后直接填值。
#[doc(hidden)]
#[derive(Debug)]
pub struct SettingValue<T> {
    #[doc(hidden)]
    pub global_value: Option<T>,
    #[doc(hidden)]
    pub local_values: Vec<(WorktreeId, Arc<RelPath>, T)>,
}

impl<T> Default for SettingValue<T> {
    fn default() -> Self {
        Self {
            global_value: None,
            local_values: Vec::new(),
        }
    }
}

#[doc(hidden)]
pub trait AnySettingValue: 'static + Send + Sync {
    fn setting_type_name(&self) -> &'static str;

    fn from_settings(&self, s: &SettingsContent) -> Box<dyn Any>;

    fn value_for_path(&self, path: Option<SettingsLocation>) -> &dyn Any;
    fn all_local_values(&self) -> Vec<(WorktreeId, Arc<RelPath>, &dyn Any)>;
    fn set_global_value(&mut self, value: Box<dyn Any>);
    fn set_local_value(&mut self, root_id: WorktreeId, path: Arc<RelPath>, value: Box<dyn Any>);
    fn clear_local_values(&mut self, root_id: WorktreeId);
}

impl<T: Settings> AnySettingValue for SettingValue<T> {
    fn from_settings(&self, s: &SettingsContent) -> Box<dyn Any> {
        Box::new(T::from_settings(s)) as _
    }

    fn setting_type_name(&self) -> &'static str { type_name::<T>() }

    fn all_local_values(&self) -> Vec<(WorktreeId, Arc<RelPath>, &dyn Any)> {
        self.local_values
            .iter()
            .map(|(id, path, value)| (*id, path.clone(), value as _))
            .collect()
    }

    fn value_for_path(&self, path: Option<SettingsLocation>) -> &dyn Any {
        if let Some(SettingsLocation { worktree_id, path }) = path {
            for (settings_root_id, settings_path, value) in self.local_values.iter().rev() {
                if worktree_id == *settings_root_id && path.starts_with(settings_path) {
                    return value;
                }
            }
        }

        self.global_value
            .as_ref()
            .unwrap_or_else(|| panic!("no default value for setting {}", self.setting_type_name()))
    }

    fn set_global_value(&mut self, value: Box<dyn Any>) {
        self.global_value = Some(*value.downcast().unwrap());
    }

    fn set_local_value(&mut self, root_id: WorktreeId, path: Arc<RelPath>, value: Box<dyn Any>) {
        let value = *value.downcast().unwrap();
        match self
            .local_values
            .binary_search_by_key(&(root_id, &path), |e| (e.0, &e.1))
        {
            Ok(ix) => self.local_values[ix].2 = value,
            Err(ix) => self.local_values.insert(ix, (root_id, path, value)),
        }
    }

    fn clear_local_values(&mut self, root_id: WorktreeId) {
        self.local_values
            .retain(|(worktree_id, _, _)| *worktree_id != root_id);
    }
}
