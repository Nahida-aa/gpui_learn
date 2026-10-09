//! 全局设置存储（对齐 Zed `crates/settings/src/settings_store.rs`）。
//!
//! 持有一棵 `serde_json::Value` 值树（从 RustEmbed 内嵌的 default.json 解析），
//! 运行时可叠加用户 settings.json 覆盖。
//! 各 Setting struct 通过 `SettingsStore::get_path` / `get_raw` 读取。

mod value;

mod error;
mod file;
mod local;
mod parse_result;
mod recompute;
mod schema;
mod semantic_tokens;
mod settings;
mod updates;
mod watcher;
pub use crate::settings_store::{
    error::InvalidSettingsError,
    file::{LocalSettingsKind, LocalSettingsPath, SettingsFile},
    parse_result::{MigrationStatus, SettingsParseResult},
    schema::{LSP_SETTINGS_SCHEMA_URL_PREFIX, SettingsJsonSchemaParams},
    semantic_tokens::DefaultSemanticTokenRules,
    settings::{RegisteredSetting, Settings, SettingsKey, SettingsLocation},
};
use crate::{ActiveSettingsProfileName, EditorconfigStore, UserSettingsContentExt, WorktreeId};
use anyhow::{Context as _, Result};
use collections::{BTreeMap, HashMap, TypeIdHashMap, btree_map, hash_map};
use fs::Fs;
use futures::future::LocalBoxFuture;
use futures::{StreamExt, channel::mpsc};
use gpui::{
    App, AppContext as _, AsyncApp, BorrowAppContext, Entity, Global, SharedString, Task,
    UpdateGlobal as _,
};
use gpui_util::ResultExt as _;
use path::rel_path::RelPath;
use serde::de::DeserializeOwned;
use serde_json::Value;
use settings_content::{
    MergeFrom, RootUserSettings, SemanticTokenRules, SettingsContent, UserSettingsContent,
};
use std::any::{TypeId, type_name};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use value::AnySettingValue;
pub use value::SettingValue;

/// 全局设置存储。gpui `Global` trait 让它能通过 `cx.global::<SettingsStore>()` 访问。
pub struct SettingsStore {
    setting_values: TypeIdHashMap<Box<dyn AnySettingValue>>,
    default_settings: Rc<SettingsContent>,
    user_settings: Option<UserSettingsContent>,
    global_settings: Option<Box<SettingsContent>>,

    extension_settings: Option<Box<SettingsContent>>,
    server_settings: Option<Box<SettingsContent>>,
    language_semantic_token_rules: HashMap<SharedString, SemanticTokenRules>,
    merged_settings: Rc<SettingsContent>,

    last_user_settings_content: Option<String>,
    last_global_settings_content: Option<String>,
    local_settings: BTreeMap<(WorktreeId, Arc<RelPath>), SettingsContent>,
    pub editorconfig_store: Entity<EditorconfigStore>,

    _settings_files_watcher: Option<Task<()>>,
    _setting_file_updates: Task<()>,
    setting_file_updates_tx:
        mpsc::UnboundedSender<Box<dyn FnOnce(AsyncApp) -> LocalBoxFuture<'static, Result<()>>>>,
    file_errors: BTreeMap<SettingsFile, SettingsParseResult>,
}
impl Global for SettingsStore {}
impl SettingsStore {
    pub fn new(cx: &mut App, default_settings: &str) -> Self {
        Self::new_with_semantic_tokens(cx, default_settings)
    }

    /// 简化入口：从空配置初始化，后续可以扩展为加载 default.json
    pub fn init(cx: &mut App) {
        let store = Self::new(cx, "{}");
        cx.set_global(store);
    }

    pub fn new_with_semantic_tokens(cx: &mut App, default_settings: &str) -> Self {
        let default_settings = Self::parse_default_settings(default_settings).unwrap();
        Self::from_settings_content(cx, default_settings)
    }
    fn from_settings_content(cx: &mut App, default_settings: SettingsContent) -> Self {
        let (setting_file_updates_tx, mut setting_file_updates_rx) = mpsc::unbounded();
        if !cx.has_global::<DefaultSemanticTokenRules>() {
            cx.set_global::<DefaultSemanticTokenRules>(
                crate::parse_json_with_comments::<SemanticTokenRules>(
                    &crate::default_semantic_token_rules(),
                )
                .map(DefaultSemanticTokenRules)
                .unwrap_or_default(),
            );
        }
        let default_settings: Rc<SettingsContent> = default_settings.into();
        let mut this = Self {
            setting_values: Default::default(),
            default_settings: default_settings.clone(),
            global_settings: None,
            server_settings: None,
            user_settings: None,
            extension_settings: None,
            language_semantic_token_rules: HashMap::default(),

            merged_settings: default_settings,
            last_user_settings_content: None,
            last_global_settings_content: None,
            local_settings: BTreeMap::default(),
            editorconfig_store: cx.new(|_| EditorconfigStore::default()),
            _settings_files_watcher: None,
            setting_file_updates_tx,
            _setting_file_updates: cx.spawn(async move |cx| {
                while let Some(setting_file_update) = setting_file_updates_rx.next().await {
                    (setting_file_update)(cx.clone()).await.log_err();
                }
            }),
            file_errors: BTreeMap::default(),
        };

        this.load_settings_types();

        this
    }

    pub fn observe_active_settings_profile_name(cx: &mut App) -> gpui::Subscription {
        cx.observe_global::<ActiveSettingsProfileName>(|cx| {
            Self::update_global(cx, |store, cx| {
                store.recompute_values(None, cx);
            });
        })
    }

    pub fn update<C, R>(cx: &mut C, f: impl FnOnce(&mut Self, &mut C) -> R) -> R
    where
        C: BorrowAppContext,
    {
        cx.update_global(f)
    }

    /// Add a new type of setting to the store.
    pub fn register_setting<T: Settings>(&mut self) {
        self.register_setting_internal(&RegisteredSetting {
            settings_value: || {
                Box::new(SettingValue::<T> {
                    global_value: None,
                    local_values: Vec::new(),
                })
            },
            from_settings: |content| Box::new(T::from_settings(content)),
            id: || TypeId::of::<T>(),
        });
    }

    fn load_settings_types(&mut self) {
        for registered_setting in inventory::iter::<RegisteredSetting>() {
            self.register_setting_internal(registered_setting);
        }
    }

    fn register_setting_internal(&mut self, registered_setting: &RegisteredSetting) {
        let entry = self.setting_values.entry((registered_setting.id)());

        if matches!(entry, hash_map::Entry::Occupied(_)) {
            return;
        }

        let setting_value = entry.or_insert((registered_setting.settings_value)());
        let value = (registered_setting.from_settings)(&self.merged_settings);
        setting_value.set_global_value(value);
    }

    pub fn merged_settings(&self) -> &SettingsContent { &self.merged_settings }

    /// Get the value of a setting.
    ///
    /// Panics if the given setting type has not been registered, or if there is no
    /// value for this setting.
    pub fn get<T: Settings>(&self, path: Option<SettingsLocation>) -> &T {
        self.setting_values
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
            .value_for_path(path)
            .downcast_ref::<T>()
            .expect("no default value for setting type")
    }

    /// Get the value of a setting.
    ///
    /// Does not panic
    pub fn try_get<T: Settings>(&self, path: Option<SettingsLocation>) -> Option<&T> {
        self.setting_values
            .get(&TypeId::of::<T>())
            .map(|value| value.value_for_path(path))
            .and_then(|value| value.downcast_ref::<T>())
    }

    /// Get all values from project specific settings
    pub fn get_all_locals<T: Settings>(&self) -> Vec<(WorktreeId, Arc<RelPath>, &T)> {
        self.setting_values
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
            .all_local_values()
            .into_iter()
            .map(|(id, path, any)| {
                (
                    id,
                    path,
                    any.downcast_ref::<T>()
                        .expect("wrong value type for setting"),
                )
            })
            .collect()
    }

    /// Override the global value for a setting.
    ///
    /// The given value will be overwritten if the user settings file changes.
    pub fn override_global<T: Settings>(&mut self, value: T) {
        self.setting_values
            .get_mut(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
            .set_global_value(Box::new(value))
    }

    /// Get the user's settings content.
    ///
    /// For user-facing functionality use the typed setting interface.
    /// (e.g. ProjectSettings::get_global(cx))
    pub fn raw_user_settings(&self) -> Option<&UserSettingsContent> { self.user_settings.as_ref() }

    /// Get the default settings content as a raw JSON value.
    pub fn raw_default_settings(&self) -> &SettingsContent { &self.default_settings }

    /// Get the configured settings profile names.
    pub fn configured_settings_profiles(&self) -> impl Iterator<Item = &str> {
        self.user_settings
            .iter()
            .flat_map(|settings| settings.profiles.keys().map(|k| k.as_str()))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test(cx: &mut App) -> Self {
        static CACHED_SETTINGS_CONTENT: std::sync::LazyLock<SettingsContent> =
            std::sync::LazyLock::new(|| {
                SettingsContent::parse_json_with_comments(crate::test_settings()).unwrap()
            });
        Self::from_settings_content(cx, CACHED_SETTINGS_CONTENT.clone())
    }

    /// Updates the value of a setting in the user's global configuration.
    ///
    /// This is only for tests. Normally, settings are only loaded from
    /// JSON files.
    #[cfg(any(test, feature = "test-support"))]
    pub fn update_user_settings(
        &mut self,
        cx: &mut App,
        update: impl FnOnce(&mut SettingsContent),
    ) {
        let mut content = self.user_settings.clone().unwrap_or_default().content;
        update(&mut content);
        fn trail(this: &mut SettingsStore, content: Box<SettingsContent>, cx: &mut App) {
            let new_text = serde_json::to_string(&UserSettingsContent {
                content,
                ..Default::default()
            })
            .unwrap();
            _ = this.set_user_settings(&new_text, cx);
        }
        trail(self, content, cx);
    }

    pub async fn load_settings(fs: &Arc<dyn Fs>) -> Result<String> {
        match fs.load(paths::settings_file()).await {
            result @ Ok(_) => result,
            Err(err) => {
                if let Some(e) = err.downcast_ref::<std::io::Error>()
                    && e.kind() == std::io::ErrorKind::NotFound
                {
                    return Ok(crate::initial_user_settings_content().to_string());
                }
                Err(err)
            }
        }
    }

    pub fn get_all_files(&self) -> Vec<SettingsFile> {
        let mut files = Vec::from_iter(
            self.local_settings
                .keys()
                // rev because these are sorted by path, so highest precedence is last
                .rev()
                .cloned()
                .map(SettingsFile::Project),
        );

        if self.server_settings.is_some() {
            files.push(SettingsFile::Server);
        }
        // ignoring profiles
        // ignoring os profiles
        // ignoring release channel profiles
        // ignoring global
        // ignoring extension

        if self.user_settings.is_some() {
            files.push(SettingsFile::User);
        }
        files.push(SettingsFile::Default);
        files
    }

    pub fn get_content_for_file(&self, file: SettingsFile) -> Option<&SettingsContent> {
        match file {
            SettingsFile::User => self
                .user_settings
                .as_ref()
                .map(|settings| settings.content.as_ref()),
            SettingsFile::Default => Some(self.default_settings.as_ref()),
            SettingsFile::Server => self.server_settings.as_deref(),
            SettingsFile::Project(ref key) => self.local_settings.get(key),
            SettingsFile::Global => self.global_settings.as_deref(),
        }
    }

    pub fn get_overrides_for_field<T>(
        &self,
        target_file: SettingsFile,
        get: fn(&SettingsContent) -> &Option<T>,
    ) -> Vec<SettingsFile> {
        let all_files = self.get_all_files();
        let mut found_file = false;
        let mut overrides = Vec::new();

        for file in all_files.into_iter().rev() {
            if !found_file {
                found_file = file == target_file;
                continue;
            }

            if let SettingsFile::Project((wt_id, ref path)) = file
                && let SettingsFile::Project((target_wt_id, ref target_path)) = target_file
                && (wt_id != target_wt_id || !target_path.starts_with(path))
            {
                // if requesting value from a local file, don't return values from local files in different worktrees
                continue;
            }

            let Some(content) = self.get_content_for_file(file.clone()) else {
                continue;
            };
            if get(content).is_some() {
                overrides.push(file);
            }
        }

        overrides
    }

    /// Checks the given file, and files that the passed file overrides for the given field.
    /// Returns the first file found that contains the value.
    /// The value will only be None if no file contains the value.
    /// I.e. if no file contains the value, returns `(File::Default, None)`
    pub fn get_value_from_file<'a, T: 'a>(
        &'a self,
        target_file: SettingsFile,
        pick: fn(&'a SettingsContent) -> Option<T>,
    ) -> (SettingsFile, Option<T>) {
        self.get_value_from_file_inner(target_file, pick, true)
    }

    /// Same as `Self::get_value_from_file` except that it does not include the current file.
    /// Therefore it returns the value that was potentially overloaded by the target file.
    pub fn get_value_up_to_file<'a, T: 'a>(
        &'a self,
        target_file: SettingsFile,
        pick: fn(&'a SettingsContent) -> Option<T>,
    ) -> (SettingsFile, Option<T>) {
        self.get_value_from_file_inner(target_file, pick, false)
    }

    fn get_value_from_file_inner<'a, T: 'a>(
        &'a self,
        target_file: SettingsFile,
        pick: fn(&'a SettingsContent) -> Option<T>,
        include_target_file: bool,
    ) -> (SettingsFile, Option<T>) {
        // todo(settings_ui): Add a metadata field for overriding the "overrides" tag, for contextually different settings
        //  e.g. disable AI isn't overridden, or a vec that gets extended instead or some such

        // todo(settings_ui) cache all files
        let all_files = self.get_all_files();
        let mut found_file = false;

        for file in all_files.into_iter() {
            if !found_file && file != SettingsFile::Default {
                if file != target_file {
                    continue;
                }
                found_file = true;
                if !include_target_file {
                    continue;
                }
            }

            if let SettingsFile::Project((worktree_id, ref path)) = file
                && let SettingsFile::Project((target_worktree_id, ref target_path)) = target_file
                && (worktree_id != target_worktree_id || !target_path.starts_with(&path))
            {
                // if requesting value from a local file, don't return values from local files in different worktrees
                continue;
            }

            let Some(content) = self.get_content_for_file(file.clone()) else {
                continue;
            };
            if let Some(value) = pick(content) {
                return (file, Some(value));
            }
        }

        (SettingsFile::Default, None)
    }

    #[inline(always)]
    fn parse_and_migrate_zed_settings<SettingsContentType: RootUserSettings>(
        &mut self,
        user_settings_content: &str,
        file: SettingsFile,
    ) -> (Option<SettingsContentType>, SettingsParseResult) {
        let mut migration_status = MigrationStatus::NotNeeded;
        let (settings, parse_status) = if user_settings_content.is_empty() {
            SettingsContentType::parse_json("{}")
        } else {
            let migration_res = migrator::migrate_settings(user_settings_content);
            migration_status = match &migration_res {
                Ok(Some(_)) => MigrationStatus::Succeeded,
                Ok(None) => MigrationStatus::NotNeeded,
                Err(err) => MigrationStatus::Failed {
                    error: err.to_string(),
                },
            };
            let content = match &migration_res {
                Ok(Some(content)) => content,
                Ok(None) => user_settings_content,
                Err(_) => user_settings_content,
            };
            SettingsContentType::parse_json(content)
        };

        let result = SettingsParseResult {
            parse_status,
            migration_status,
        };
        self.file_errors.insert(file, result.clone());
        return (settings, result);
    }

    pub fn error_for_file(&self, file: SettingsFile) -> Option<SettingsParseResult> {
        self.file_errors
            .get(&file)
            .filter(|parse_result| parse_result.requires_user_action())
            .cloned()
    }
}

/// 递归合并两个 JSON Value（source 覆盖 target 中同路径的值）。
fn merge_values(target: Value, source: Value) -> Value {
    match (target, source) {
        (Value::Object(mut t), Value::Object(s)) => {
            for (k, sv) in s {
                let tv = t.remove(&k);
                let merged = match tv {
                    Some(tv) => merge_values(tv, sv),
                    None => sv,
                };
                t.insert(k, merged);
            }
            Value::Object(t)
        }
        (_, source) => source,
    }
}

/// 用户 settings.json 路径：`$HOME/.config/aa/settings.json`。
/// Zed 对应 `~/.config/zed/settings.json`。
fn user_settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("aa")
        .join("settings.json")
}
