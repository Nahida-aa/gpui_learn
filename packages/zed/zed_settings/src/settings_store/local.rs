use super::SettingsStore;
use crate::{
    WorktreeId,
    settings_store::{
        error::InvalidSettingsError,
        file::{LocalSettingsKind, LocalSettingsPath, SettingsFile},
    },
};
use anyhow::{Context as _, Result};
use collections::{BTreeMap, HashMap, TypeIdHashMap, btree_map, hash_map};
use gpui::App;
use path::rel_path::RelPath;
use paths::{local_settings_file_relative_path, task_file_name};
use settings_content::{
    ExtensionsSettingsContent, MergeFrom, ParseStatus, ProjectSettingsContent, SettingsContent,
};
use std::sync::Arc;

impl SettingsStore {
    /// Add or remove a set of local settings via a JSON string.
    pub fn set_local_settings(
        &mut self,
        root_id: WorktreeId,
        path: LocalSettingsPath,
        kind: LocalSettingsKind,
        settings_content: Option<&str>,
        cx: &mut App,
    ) -> std::result::Result<(), InvalidSettingsError> {
        let content = settings_content
            .map(|content| content.trim())
            .filter(|content| !content.is_empty());
        let mut zed_settings_changed = false;
        match (path.clone(), kind, content) {
            (LocalSettingsPath::InWorktree(directory_path), LocalSettingsKind::Tasks, _) => {
                return Err(InvalidSettingsError::Tasks {
                    message: "Attempted to submit tasks into the settings store".to_string(),
                    path: directory_path
                        .join(RelPath::from_unix_str(task_file_name()).unwrap())
                        .as_std_path()
                        .to_path_buf(),
                });
            }
            (LocalSettingsPath::InWorktree(directory_path), LocalSettingsKind::Debug, _) => {
                return Err(InvalidSettingsError::Debug {
                    message: "Attempted to submit debugger config into the settings store"
                        .to_string(),
                    path: directory_path
                        .join(RelPath::from_unix_str(task_file_name()).unwrap())
                        .as_std_path()
                        .to_path_buf(),
                });
            }
            (LocalSettingsPath::InWorktree(directory_path), LocalSettingsKind::Settings, None) => {
                zed_settings_changed = self
                    .local_settings
                    .remove(&(root_id, directory_path.clone()))
                    .is_some();
                self.file_errors
                    .remove(&SettingsFile::Project((root_id, directory_path)));
            }
            (
                LocalSettingsPath::InWorktree(directory_path),
                LocalSettingsKind::Settings,
                Some(settings_contents),
            ) => {
                let (new_settings, parse_result) = self
                    .parse_and_migrate_zed_settings::<ProjectSettingsContent>(
                        settings_contents,
                        SettingsFile::Project((root_id, directory_path.clone())),
                    );
                match parse_result.parse_status {
                    ParseStatus::Success => Ok(()),
                    ParseStatus::Unchanged => Ok(()),
                    ParseStatus::Failed { error } => Err(InvalidSettingsError::LocalSettings {
                        path: directory_path
                            .join(local_settings_file_relative_path())
                            .into(),
                        message: error,
                    }),
                }?;
                if let Some(new_settings) = new_settings {
                    match self.local_settings.entry((root_id, directory_path)) {
                        btree_map::Entry::Vacant(v) => {
                            v.insert(SettingsContent {
                                project: new_settings,
                                ..Default::default()
                            });
                            zed_settings_changed = true;
                        }
                        btree_map::Entry::Occupied(mut o) => {
                            if &o.get().project != &new_settings {
                                o.insert(SettingsContent {
                                    project: new_settings,
                                    ..Default::default()
                                });
                                zed_settings_changed = true;
                            }
                        }
                    }
                }
            }
            (directory_path, LocalSettingsKind::Editorconfig, editorconfig_contents) => {
                self.editorconfig_store.update(cx, |store, _| {
                    store.set_configs(root_id, directory_path, editorconfig_contents)
                })?;
            }
            (LocalSettingsPath::OutsideWorktree(path), kind, _) => {
                log::error!(
                    "OutsideWorktree path {:?} with kind {:?} is only supported by editorconfig",
                    path,
                    kind
                );
                return Ok(());
            }
        }
        if let LocalSettingsPath::InWorktree(directory_path) = &path {
            if zed_settings_changed {
                self.recompute_values(Some((root_id, &directory_path)), cx);
            }
        }
        Ok(())
    }

    pub fn set_extension_settings(
        &mut self,
        content: ExtensionsSettingsContent,
        cx: &mut App,
    ) -> Result<()> {
        self.extension_settings = Some(Box::new(SettingsContent {
            project: ProjectSettingsContent {
                all_languages: content.all_languages,
                ..Default::default()
            },
            ..Default::default()
        }));
        self.recompute_values(None, cx);
        Ok(())
    }

    /// Add or remove a set of local settings via a JSON string.
    pub fn clear_local_settings(&mut self, root_id: WorktreeId, cx: &mut App) -> Result<()> {
        self.local_settings
            .retain(|(worktree_id, _), _| worktree_id != &root_id);

        self.editorconfig_store
            .update(cx, |store, _cx| store.remove_for_worktree(root_id));

        for setting_value in self.setting_values.values_mut() {
            setting_value.clear_local_values(root_id);
        }
        self.recompute_values(Some((root_id, RelPath::empty())), cx);
        Ok(())
    }

    pub fn local_settings(
        &self,
        root_id: WorktreeId,
    ) -> impl '_ + Iterator<Item = (Arc<RelPath>, &ProjectSettingsContent)> {
        self.local_settings
            .range(
                (root_id, RelPath::empty_arc())
                    ..(
                        WorktreeId::from_usize(root_id.to_usize() + 1),
                        RelPath::empty_arc(),
                    ),
            )
            .map(|((_, path), content)| (path.clone(), &content.project))
    }
}
