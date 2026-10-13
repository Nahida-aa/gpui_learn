mod base_keymap_setting;
mod content_into_gpui;
mod editable_setting_control;
mod editorconfig_store;
mod granted_write_path;
mod keymap_file;
mod settings_file;
mod settings_store;
mod vscode_import;

pub use settings_macros::RegisterSetting;

pub mod settings_content {
    pub use ::settings_content::*;
}

pub mod fallible_options {
    pub use ::settings_content::{FallibleOption, parse_json};
}

#[doc(hidden)]
pub mod private {
    pub use crate::settings_store::{RegisteredSetting, SettingValue};
    pub use inventory;
}

use gpui::{App, Global};

use std::env;
use std::{borrow::Cow, fmt, str};
use ::util::asset_str;

pub use ::settings_content::*;
pub use base_keymap_setting::*;
pub use content_into_gpui::IntoGpui;
pub use editable_setting_control::*;
pub use editorconfig_store::{
    Editorconfig, EditorconfigEvent, EditorconfigProperties, EditorconfigStore,
};
pub use granted_write_path::GrantedWritePath;
pub use keymap_file::{
    KeyBindingValidator, KeyBindingValidatorRegistration, KeybindSource, KeybindUpdateOperation,
    KeybindUpdateTarget, KeymapFile, KeymapFileLoadResult,
};
pub use settings_file::*;
pub use settings_json::*;
pub use settings_store::{
    DefaultSemanticTokenRules, InvalidSettingsError, LSP_SETTINGS_SCHEMA_URL_PREFIX,
    LocalSettingsKind, LocalSettingsPath, MigrationStatus, Settings, SettingsFile,
    SettingsJsonSchemaParams, SettingsKey, SettingsLocation, SettingsParseResult, SettingsStore,
};

pub use vscode_import::{VsCodeSettings, VsCodeSettingsSource};

pub use keymap_file::ActionSequence;

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveSettingsProfileName(pub String);

impl Global for ActiveSettingsProfileName {}

pub trait UserSettingsContentExt {
    fn for_profile(&self, cx: &App) -> Option<&SettingsProfile>;
    fn for_release_channel(&self) -> Option<&SettingsContent>;
    fn for_os(&self) -> Option<&SettingsContent>;
}

impl UserSettingsContentExt for UserSettingsContent {
    fn for_profile(&self, cx: &App) -> Option<&SettingsProfile> {
        let Some(active_profile) = cx.try_global::<ActiveSettingsProfileName>() else {
            return None;
        };
        self.profiles.get(&active_profile.0)
    }

    fn for_release_channel(&self) -> Option<&SettingsContent> {
        self.release_channel_overrides
            .get_by_key(release_channel::RELEASE_CHANNEL.dev_name())
    }

    fn for_os(&self) -> Option<&SettingsContent> {
        self.platform_overrides.get_by_key(env::consts::OS)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, serde::Serialize)]
pub struct WorktreeId(usize);

impl From<WorktreeId> for usize {
    fn from(value: WorktreeId) -> Self { value.0 }
}

impl WorktreeId {
    pub fn from_usize(handle_id: usize) -> Self { Self(handle_id) }

    pub fn from_proto(id: u64) -> Self { Self(id as usize) }

    pub fn to_proto(self) -> u64 { self.0 as u64 }

    pub fn to_usize(self) -> usize { self.0 }
}

impl fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { std::fmt::Display::fmt(&self.0, f) }
}

// Dev builds read the checkout's files at runtime instead of embedding them;
// see the `assets` crate for the rationale.
//
// `crate_relative` 必须回退**三级**到仓库根的 `assets/`，而 `root_relative`
// 只有一级（后者相对 git 根，前者相对本 crate 的 Cargo.toml 目录）。
// 本 crate 在 `packages/zed/zed_settings/`，两级只回到 `packages/`，会指向
// `packages/assets/` —— 那里曾放资源 crate 的目录（只有 src/ 和 Cargo.toml，
// 没有任何 settings 文件），于是 release 构建内嵌了个空目录，启动时
// `asset_str::<SettingsAssets>("settings/default.json")` 拿不到文件直接 panic。
// dev 构建因为走 root_relative 读运行时文件系统，反而一直正常 —— 只有
// release 包会崩。三个回退可与 `aa_assets_core` 的对照：它在
// `packages/aa/aa_assets/aa_assets_core/`，三级回退正好落到仓库根。
::util::fs_embed! {
    pub struct SettingsAssets,
    crate_relative = "../../../assets",
    root_relative = "assets",
    include = ["settings/*", "keymaps/*"],
    exclude = ["*.DS_Store"],
}

pub fn init(cx: &mut App) {
    let settings = SettingsStore::new(cx, &default_settings());
    cx.set_global(settings);
    SettingsStore::observe_active_settings_profile_name(cx).detach();
}

pub fn default_settings() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/default.json")
}

pub fn default_semantic_token_rules() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/default_semantic_token_rules.json")
}

#[cfg(target_os = "macos")]
pub const DEFAULT_KEYMAP_PATH: &str = "keymaps/default-macos.json";

#[cfg(target_os = "windows")]
pub const DEFAULT_KEYMAP_PATH: &str = "keymaps/default-windows.json";

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const DEFAULT_KEYMAP_PATH: &str = "keymaps/default-linux.json";

pub fn default_keymap() -> Cow<'static, str> { asset_str::<SettingsAssets>(DEFAULT_KEYMAP_PATH) }

pub const VIM_KEYMAP_PATH: &str = "keymaps/vim.json";

pub fn vim_keymap() -> Cow<'static, str> { asset_str::<SettingsAssets>(VIM_KEYMAP_PATH) }

/// Specific keybinding overrides. Loaded after the base keymap so they win over
/// conflicting base-keymap (and default `Editor`) bindings for the same chords,
/// while still allowing user keymaps (loaded last) to override them. Shared
/// across features - prefer adding a context block here over creating another
/// override keymap file.
#[cfg(target_os = "macos")]
pub const SPECIFIC_OVERRIDES_KEYMAP_PATH: &str = "keymaps/specific-overrides-macos.json";

#[cfg(not(target_os = "macos"))]
pub const SPECIFIC_OVERRIDES_KEYMAP_PATH: &str = "keymaps/specific-overrides.json";

pub fn initial_user_settings_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_user_settings.json")
}

pub fn initial_server_settings_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_server_settings.json")
}

pub fn initial_project_settings_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_local_settings.json")
}

pub fn initial_keymap_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("keymaps/initial.json")
}

pub fn initial_tasks_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_tasks.json")
}

pub fn initial_worktree_setup_tasks_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_worktree_setup_tasks.json")
}

pub fn initial_debug_tasks_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_debug_tasks.json")
}

pub fn initial_local_debug_tasks_content() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/initial_local_debug_tasks.json")
}

#[cfg(test)]
mod embed_regression_tests {
    // 回归测试：曾因 fs_embed! 的 crate_relative 层级错位（"../../assets" 在
    // packages/zed/zed_settings/ 下只回到 packages/，指向某个 crate 的目录
    // 而非仓库根 assets/），导致 release 构建内嵌空目录，
    // default_settings() 一调用就 panic：settings/default.json。
    // dev 构建走 root_relative 读运行时文件系统所以一直正常 —— 只有 release 会崩。
    //
    // 本测试在 debug 下也强制走编译期内嵌（debug-embed 是 aa_util 的默认 feature），
    // 因此能覆盖到与 release 同一条代码路径。
    use super::*;

    #[test]
    fn settings_assets_embed_default_settings() {
        let s = default_settings();
        assert!(!s.is_empty(), "settings/default.json 内嵌为空");
        assert!(s.contains("agent"), "内容不像 settings JSON: {}", &s[..s.len().min(80)]);
    }

    #[test]
    fn settings_assets_embed_semantic_token_rules() {
        assert!(!default_semantic_token_rules().is_empty());
    }

    #[test]
    fn settings_assets_embed_keymaps() {
        let names: Vec<_> = SettingsAssets::iter().collect();
        assert!(
            names.iter().any(|n| n.starts_with("keymaps/")),
            "keymaps 未内嵌，实际内嵌: {names:?}"
        );
    }
}
