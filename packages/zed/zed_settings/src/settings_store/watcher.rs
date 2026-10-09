use std::{path::PathBuf, sync::Arc};

use fs::Fs;
use futures::{FutureExt, StreamExt, channel::mpsc};
use gpui::{App, AppContext, BorrowAppContext, Task};

use crate::settings_store::parse_result::SettingsParseResult;

use super::{SettingsStore, file::SettingsFile};

impl SettingsStore {
    pub fn watch_settings_files(
        &mut self,
        fs: Arc<dyn Fs>,
        cx: &mut App,
        settings_changed: impl 'static + Fn(SettingsFile, SettingsParseResult, &mut App),
    ) {
        let (mut user_settings_file_rx, user_settings_watcher) = crate::watch_config_file(
            cx.background_executor(),
            fs.clone(),
            paths::settings_file().clone(),
        );
        let (mut global_settings_file_rx, global_settings_watcher) = crate::watch_config_file(
            cx.background_executor(),
            fs,
            paths::global_settings_file().clone(),
        );

        let global_content = cx
            .foreground_executor()
            .block_on(global_settings_file_rx.next())
            .unwrap();
        let user_content = cx
            .foreground_executor()
            .block_on(user_settings_file_rx.next())
            .unwrap();

        let result = self.set_user_settings(&user_content, cx);
        settings_changed(SettingsFile::User, result, cx);
        let result = self.set_global_settings(&global_content, cx);
        settings_changed(SettingsFile::Global, result, cx);

        self._settings_files_watcher = Some(cx.spawn(async move |cx| {
            let _user_settings_watcher = user_settings_watcher;
            let _global_settings_watcher = global_settings_watcher;
            let mut settings_streams = futures::stream::select(
                global_settings_file_rx.map(|content| (SettingsFile::Global, content)),
                user_settings_file_rx.map(|content| (SettingsFile::User, content)),
            );

            while let Some((settings_file, content)) = settings_streams.next().await {
                cx.update_global(|store: &mut SettingsStore, cx| {
                    let result = match settings_file {
                        SettingsFile::User => store.set_user_settings(&content, cx),
                        SettingsFile::Global => store.set_global_settings(&content, cx),
                        _ => return,
                    };
                    settings_changed(settings_file, result, cx);
                    cx.refresh_windows();
                });
            }
        }));
    }
}
