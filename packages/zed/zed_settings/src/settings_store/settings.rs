use std::any::{Any, TypeId};

use gpui::{App, AsyncApp, BorrowAppContext, UpdateGlobal as _};
use path::rel_path::RelPath;

use super::SettingsStore;
use crate::{WorktreeId, settings_store::value::AnySettingValue};
use settings_content::{SettingsContent, UserSettingsContent};

pub trait SettingsKey: 'static + Send + Sync {
    /// The name of a key within the JSON file from which this setting should
    /// be deserialized. If this is `None`, then the setting will be deserialized
    /// from the root object.
    const KEY: Option<&'static str>;

    const FALLBACK_KEY: Option<&'static str> = None;
}

/// A value that can be defined as a user setting.
///
/// Settings can be loaded from a combination of multiple JSON files.
pub trait Settings: 'static + Send + Sync + Sized {
    /// The name of the keys in the [`SettingsContent`] that should
    /// always be written to a settings file, even if their value matches the default
    /// value.
    ///
    /// This is useful for tagged [`SettingsContent`]s where the tag
    /// is a "version" field that should always be persisted, even if the current
    /// user settings match the current version of the settings.
    const PRESERVED_KEYS: Option<&'static [&'static str]> = None;

    /// Read the value from default.json.
    ///
    /// This function *should* panic if default values are missing,
    /// and you should add a default to default.json for documentation.
    fn from_settings(content: &SettingsContent) -> Self;

    #[track_caller]
    fn register(cx: &mut App)
    where
        Self: Sized,
    {
        SettingsStore::update_global(cx, |store, _| {
            store.register_setting::<Self>();
        });
    }

    #[track_caller]
    fn get<'a>(path: Option<SettingsLocation>, cx: &'a App) -> &'a Self
    where
        Self: Sized,
    {
        cx.global::<SettingsStore>().get(path)
    }

    #[track_caller]
    fn get_global(cx: &App) -> &Self
    where
        Self: Sized,
    {
        cx.global::<SettingsStore>().get(None)
    }

    #[track_caller]
    fn try_get(cx: &App) -> Option<&Self>
    where
        Self: Sized,
    {
        if cx.has_global::<SettingsStore>() {
            cx.global::<SettingsStore>().try_get(None)
        } else {
            None
        }
    }

    #[track_caller]
    fn try_read_global<R>(cx: &AsyncApp, f: impl FnOnce(&Self) -> R) -> Option<R>
    where
        Self: Sized,
    {
        cx.try_read_global(|s: &SettingsStore, _| f(s.get(None)))
    }

    #[track_caller]
    fn override_global(settings: Self, cx: &mut App)
    where
        Self: Sized,
    {
        cx.global_mut::<SettingsStore>().override_global(settings)
    }
}
/// inventory 编译期收集的注册条目。
pub struct RegisteredSetting {
    pub settings_value: fn() -> Box<dyn AnySettingValue>,
    pub from_settings: fn(&SettingsContent) -> Box<dyn Any>,
    pub id: fn() -> TypeId,
}
inventory::collect!(RegisteredSetting);

#[derive(Clone, Copy, Debug)]
pub struct SettingsLocation<'a> {
    pub worktree_id: WorktreeId,
    pub path: &'a RelPath,
}
