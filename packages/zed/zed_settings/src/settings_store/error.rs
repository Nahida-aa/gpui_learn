use std::{path::PathBuf, sync::Arc};

use path::rel_path::RelPath;

use super::file::LocalSettingsPath;

#[derive(Debug, Clone, PartialEq)]
pub enum InvalidSettingsError {
    LocalSettings {
        path: Arc<RelPath>,
        message: String,
    },
    UserSettings {
        message: String,
    },
    ServerSettings {
        message: String,
    },
    DefaultSettings {
        message: String,
    },
    Editorconfig {
        path: LocalSettingsPath,
        message: String,
    },
    Tasks {
        path: PathBuf,
        message: String,
    },
    Debug {
        path: PathBuf,
        message: String,
    },
}

impl std::fmt::Display for InvalidSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidSettingsError::LocalSettings { message, .. }
            | InvalidSettingsError::UserSettings { message }
            | InvalidSettingsError::ServerSettings { message }
            | InvalidSettingsError::DefaultSettings { message }
            | InvalidSettingsError::Tasks { message, .. }
            | InvalidSettingsError::Editorconfig { message, .. }
            | InvalidSettingsError::Debug { message, .. } => write!(f, "{message}"),
        }
    }
}
impl std::error::Error for InvalidSettingsError {}
