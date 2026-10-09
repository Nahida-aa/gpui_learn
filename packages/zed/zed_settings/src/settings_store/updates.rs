use anyhow::{Context as _, Result};
use fs::Fs;
use futures::{FutureExt, channel::oneshot};
use gpui::{App, AsyncApp, BorrowAppContext};
use gpui_util::ResultExt as _;
use settings_content::{
    MergeFrom, ParseStatus, RootUserSettings, SettingsContent, UserSettingsContent,
};
use settings_json::{infer_json_indent_size, update_value_in_json_text};
use std::{ops::Range, sync::Arc};

use crate::VsCodeSettings;

use super::SettingsStore;

impl SettingsStore {
    fn update_settings_file_inner(
        &self,
        fs: Arc<dyn Fs>,
        update: Box<dyn Send + FnOnce(String, AsyncApp) -> Result<String>>,
    ) -> oneshot::Receiver<Result<()>> {
        let (tx, rx) = oneshot::channel::<Result<()>>();
        self.setting_file_updates_tx
            .unbounded_send(Box::new(move |cx: AsyncApp| {
                async move {
                    let res = async move {
                        let old_text = Self::load_settings(&fs).await?;
                        let new_text = update(old_text, cx.clone())?;

                        let settings_path = paths::settings_file().as_path();
                        if fs.is_file(settings_path).await {
                            let resolved_path =
                                fs.canonicalize(settings_path).await.with_context(|| {
                                    format!(
                                        "Failed to canonicalize settings path {:?}",
                                        settings_path
                                    )
                                })?;

                            fs.atomic_write(resolved_path.clone(), new_text.clone())
                                .await
                                .with_context(|| {
                                    format!("Failed to write settings to file {:?}", resolved_path)
                                })?;
                        } else {
                            fs.atomic_write(settings_path.to_path_buf(), new_text.clone())
                                .await
                                .with_context(|| {
                                    format!("Failed to write settings to file {:?}", settings_path)
                                })?;
                        }

                        cx.update_global(|store: &mut SettingsStore, cx| {
                            store.set_user_settings(&new_text, cx).result().map(|_| ())
                        })
                    }
                    .await;

                    let new_res = match &res {
                        Ok(_) => anyhow::Ok(()),
                        Err(e) => Err(anyhow::anyhow!("{:?}", e)),
                    };

                    _ = tx.send(new_res);
                    res
                }
                .boxed_local()
            }))
            .map_err(|err| anyhow::format_err!("Failed to update settings file: {}", err))
            .log_with_level(log::Level::Warn);
        return rx;
    }
    pub fn update_settings_file(
        &self,
        fs: Arc<dyn Fs>,
        update: impl 'static + Send + FnOnce(&mut SettingsContent, &App),
    ) {
        _ = self.update_settings_file_with_completion(fs, update);
    }

    pub fn update_settings_file_with_completion(
        &self,
        fs: Arc<dyn Fs>,
        update: impl 'static + Send + FnOnce(&mut SettingsContent, &App),
    ) -> oneshot::Receiver<Result<()>> {
        let mut update = Some(update);
        self.update_settings_file_inner(
            fs,
            Box::new(move |old_text: String, cx: AsyncApp| {
                cx.read_global(|store: &SettingsStore, cx| {
                    store.new_text_for_update_inner(old_text, &mut |content| {
                        (update.take().expect("called once"))(content, cx)
                    })
                })
            }),
        )
    }

    pub fn import_vscode_settings(
        &self,
        fs: Arc<dyn Fs>,
        vscode_settings: VsCodeSettings,
    ) -> oneshot::Receiver<Result<()>> {
        self.update_settings_file_inner(
            fs,
            Box::new(move |old_text: String, cx: AsyncApp| {
                cx.read_global(|store: &SettingsStore, _cx| {
                    store.get_vscode_edits(old_text, &vscode_settings)
                })
            }),
        )
    }
    /// Updates the value of a setting in a JSON file, returning the new text
    /// for that JSON file.
    pub fn new_text_for_update(
        &self,
        old_text: String,
        update: impl FnOnce(&mut SettingsContent),
    ) -> Result<String> {
        let mut update = Some(update);
        self.new_text_for_update_inner(old_text, &mut |content| {
            (update.take().expect("called once"))(content)
        })
    }

    fn new_text_for_update_inner(
        &self,
        old_text: String,
        update: &mut dyn FnMut(&mut SettingsContent),
    ) -> Result<String> {
        let edits = self.edits_for_update_inner(&old_text, update)?;
        let mut new_text = old_text;
        for (range, replacement) in edits.into_iter() {
            new_text.replace_range(range, &replacement);
        }
        Ok(new_text)
    }

    pub fn get_vscode_edits(&self, old_text: String, vscode: &VsCodeSettings) -> Result<String> {
        self.new_text_for_update(old_text, |content| {
            content.merge_from(&vscode.settings_content())
        })
    }

    /// Updates the value of a setting in a JSON file, returning a list
    /// of edits to apply to the JSON file.
    pub fn edits_for_update(
        &self,
        text: &str,
        update: impl FnOnce(&mut SettingsContent),
    ) -> Result<Vec<(Range<usize>, String)>> {
        let mut update = Some(update);
        self.edits_for_update_inner(text, &mut |content| {
            (update.take().expect("called once"))(content)
        })
    }

    fn edits_for_update_inner(
        &self,
        text: &str,
        update: &mut dyn FnMut(&mut SettingsContent),
    ) -> Result<Vec<(Range<usize>, String)>> {
        let old_content = if text.trim().is_empty() {
            UserSettingsContent::default()
        } else {
            let (old_content, parse_status) = UserSettingsContent::parse_json(text);
            if let ParseStatus::Failed { error } = &parse_status {
                log::error!("Failed to parse settings for update: {error}");
            }
            old_content
                .context("Settings file could not be parsed. Fix syntax errors before updating.")?
        };
        let mut new_content = old_content.clone();
        update(&mut new_content.content);

        let old_value = serde_json::to_value(&old_content).unwrap();
        let new_value = serde_json::to_value(new_content).unwrap();

        let mut key_path = Vec::new();
        let mut edits = Vec::new();
        let tab_size = infer_json_indent_size(&text);
        let mut text = text.to_string();
        update_value_in_json_text(
            &mut text,
            &mut key_path,
            tab_size,
            &old_value,
            &new_value,
            &mut edits,
        );
        Ok(edits)
    }
}
