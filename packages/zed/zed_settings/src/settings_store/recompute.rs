use anyhow::{Context as _, Result};
use gpui::{App, SharedString};
use gpui_util::ResultExt as _;
use path::rel_path::RelPath;
use settings_content::{
    MergeFrom, ParseStatus, ProfileBase, RootUserSettings, SemanticTokenRules, SettingsContent,
    UserSettingsContent,
};
use std::rc::Rc;

use crate::{
    UserSettingsContentExt, WorktreeId,
    settings_store::{
        file::SettingsFile,
        parse_result::{MigrationStatus, SettingsParseResult},
    },
};

use super::SettingsStore;

impl SettingsStore {
    /// Mutates the default settings in place and recomputes all setting values.
    pub fn update_default_settings(
        &mut self,
        cx: &mut App,
        update: impl FnOnce(&mut SettingsContent),
    ) {
        let default_settings = Rc::make_mut(&mut self.default_settings);
        update(default_settings);
        self.recompute_values(None, cx);
    }

    /// Sets the default settings via a JSON string.
    ///
    /// The string should contain a JSON object with a default value for every setting.
    pub fn set_default_settings(
        &mut self,
        default_settings_content: &str,
        cx: &mut App,
    ) -> Result<()> {
        self.default_settings = Self::parse_default_settings(default_settings_content)?.into();
        self.recompute_values(None, cx);
        Ok(())
    }

    /// Parses the default settings JSON and folds any `dev`/`nightly`/`preview`/`stable`
    /// release-channel overrides and `macos`/`linux`/`windows` platform overrides into
    /// the returned [`SettingsContent`].
    ///
    /// Unlike user settings, default settings are used directly as the base for all
    /// merges, so overrides must be resolved up front.
    pub(super) fn parse_default_settings(default_settings: &str) -> Result<SettingsContent> {
        let parsed = UserSettingsContent::parse_json_with_comments(default_settings)?;
        let mut merged = (*parsed.content).clone();
        merged.merge_from_option(parsed.for_release_channel());
        merged.merge_from_option(parsed.for_os());
        Ok(merged)
    }

    /// Sets the user settings via a JSON string.
    #[must_use]
    pub fn set_user_settings(
        &mut self,
        user_settings_content: &str,
        cx: &mut App,
    ) -> SettingsParseResult {
        if self.last_user_settings_content.as_deref() == Some(user_settings_content) {
            return SettingsParseResult {
                parse_status: ParseStatus::Unchanged,
                migration_status: MigrationStatus::NotNeeded,
            };
        }
        self.last_user_settings_content = Some(user_settings_content.to_string());

        let (settings, parse_result) = self.parse_and_migrate_zed_settings::<UserSettingsContent>(
            user_settings_content,
            SettingsFile::User,
        );

        if let Some(settings) = settings {
            self.user_settings = Some(settings);
            self.recompute_values(None, cx);
        }
        return parse_result;
    }

    /// Sets the global settings via a JSON string.
    #[must_use]
    pub fn set_global_settings(
        &mut self,
        global_settings_content: &str,
        cx: &mut App,
    ) -> SettingsParseResult {
        if self.last_global_settings_content.as_deref() == Some(global_settings_content) {
            return SettingsParseResult {
                parse_status: ParseStatus::Unchanged,
                migration_status: MigrationStatus::NotNeeded,
            };
        }
        self.last_global_settings_content = Some(global_settings_content.to_string());

        let (settings, parse_result) = self.parse_and_migrate_zed_settings::<SettingsContent>(
            global_settings_content,
            SettingsFile::Global,
        );

        if let Some(settings) = settings {
            self.global_settings = Some(Box::new(settings));
            self.recompute_values(None, cx);
        }
        return parse_result;
    }

    pub fn set_server_settings(
        &mut self,
        server_settings_content: &str,
        cx: &mut App,
    ) -> Result<()> {
        let settings = if server_settings_content.is_empty() {
            None
        } else {
            Option::<SettingsContent>::parse_json_with_comments(server_settings_content)?
        };

        // Rewrite the server settings into a content type
        self.server_settings = settings.map(|settings| Box::new(settings));

        self.recompute_values(None, cx);
        Ok(())
    }

    /// Sets language-specific semantic token rules.
    ///
    /// These rules are registered by language modules (e.g. the Rust language module)
    /// or by third-party extensions (via `semantic_token_rules.json` in their language
    /// directories). They are stored separately from the global rules and are only
    /// applied to buffers of the matching language by the `SemanticTokenStylizer`.
    ///
    /// This triggers a settings recomputation so that observers (e.g. `LspStore`)
    /// are notified and can invalidate cached stylizers.
    pub fn set_language_semantic_token_rules(
        &mut self,
        language: SharedString,
        rules: SemanticTokenRules,
        cx: &mut App,
    ) {
        self.language_semantic_token_rules.insert(language, rules);
        self.recompute_values(None, cx);
    }

    /// Removes language-specific semantic token rules for the given language.
    ///
    /// This should be called when an extension that registered rules for a language
    /// is unloaded. Triggers a settings recomputation so that observers (e.g.
    /// `LspStore`) are notified and can invalidate cached stylizers.
    pub fn remove_language_semantic_token_rules(&mut self, language: &str, cx: &mut App) {
        self.language_semantic_token_rules.remove(language);
        self.recompute_values(None, cx);
    }

    /// Returns the language-specific semantic token rules for the given language,
    /// if any have been registered.
    pub fn language_semantic_token_rules(&self, language: &str) -> Option<&SemanticTokenRules> {
        self.language_semantic_token_rules.get(language)
    }

    pub(super) fn recompute_values(
        &mut self,
        changed_local_path: Option<(WorktreeId, &RelPath)>,
        cx: &mut App,
    ) {
        // Reload the global and local values for every setting.
        let mut project_settings_stack = Vec::<SettingsContent>::new();
        let mut paths_stack = Vec::<Option<(WorktreeId, &RelPath)>>::new();

        if changed_local_path.is_none() {
            let mut merged = self.default_settings.as_ref().clone();
            merged.merge_from_option(self.extension_settings.as_deref());
            merged.merge_from_option(self.global_settings.as_deref());
            if let Some(user_settings) = self.user_settings.as_ref() {
                let active_profile = user_settings.for_profile(cx);
                let should_merge_user_settings =
                    active_profile.is_none_or(|profile| profile.base == ProfileBase::User);

                if should_merge_user_settings {
                    merged.merge_from(&user_settings.content);
                    merged.merge_from_option(user_settings.for_release_channel());
                    merged.merge_from_option(user_settings.for_os());
                }

                if let Some(profile) = active_profile {
                    merged.merge_from(&profile.settings);
                }
            }
            merged.merge_from_option(self.server_settings.as_deref());

            // Merge `disable_ai` from all project/local settings into the global value.
            // Since `SaturatingBool` uses OR logic, if any project has `disable_ai: true`,
            // the global value will be true. This allows project-level `disable_ai` to
            // affect the global setting used by UI elements without file context.
            for local_settings in self.local_settings.values() {
                merged
                    .project
                    .disable_ai
                    .merge_from(&local_settings.project.disable_ai);
            }

            self.merged_settings = Rc::new(merged);

            for setting_value in self.setting_values.values_mut() {
                let value = setting_value.from_settings(&self.merged_settings);
                setting_value.set_global_value(value);
            }
        } else {
            // When only a local path changed, we still need to recompute the global
            // `disable_ai` value since it depends on all local settings.
            let mut merged = (*self.merged_settings).clone();
            // Reset disable_ai to compute fresh from base settings
            merged.project.disable_ai = self.default_settings.project.disable_ai;
            if let Some(global) = &self.global_settings {
                merged
                    .project
                    .disable_ai
                    .merge_from(&global.project.disable_ai);
            }
            if let Some(user) = &self.user_settings {
                merged
                    .project
                    .disable_ai
                    .merge_from(&user.content.project.disable_ai);
            }
            if let Some(server) = &self.server_settings {
                merged
                    .project
                    .disable_ai
                    .merge_from(&server.project.disable_ai);
            }
            for local_settings in self.local_settings.values() {
                merged
                    .project
                    .disable_ai
                    .merge_from(&local_settings.project.disable_ai);
            }
            self.merged_settings = Rc::new(merged);

            for setting_value in self.setting_values.values_mut() {
                let value = setting_value.from_settings(&self.merged_settings);
                setting_value.set_global_value(value);
            }
        }

        for ((root_id, directory_path), local_settings) in &self.local_settings {
            // Build a stack of all of the local values for that setting.
            while let Some(prev_entry) = paths_stack.last() {
                if let Some((prev_root_id, prev_path)) = prev_entry
                    && (root_id != prev_root_id || !directory_path.starts_with(prev_path))
                {
                    paths_stack.pop();
                    project_settings_stack.pop();
                    continue;
                }
                break;
            }

            paths_stack.push(Some((*root_id, directory_path.as_ref())));
            let mut merged_local_settings = if let Some(deepest) = project_settings_stack.last() {
                (*deepest).clone()
            } else {
                self.merged_settings.as_ref().clone()
            };
            merged_local_settings.merge_from(local_settings);

            project_settings_stack.push(merged_local_settings);

            // If a local settings file changed, then avoid recomputing local
            // settings for any path outside of that directory.
            if changed_local_path.is_some_and(|(changed_root_id, changed_local_path)| {
                *root_id != changed_root_id || !directory_path.starts_with(changed_local_path)
            }) {
                continue;
            }

            for setting_value in self.setting_values.values_mut() {
                let value = setting_value.from_settings(&project_settings_stack.last().unwrap());
                setting_value.set_local_value(*root_id, directory_path.clone(), value);
            }
        }
    }
}
