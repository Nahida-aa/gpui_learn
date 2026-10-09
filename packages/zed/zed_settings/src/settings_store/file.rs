use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use path::rel_path::RelPath;

use crate::WorktreeId;
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SettingsFile {
    Default,
    Global,
    User,
    Server,
    /// Represents project settings in ssh projects as well as local projects
    Project((WorktreeId, Arc<RelPath>)),
}

impl PartialOrd for SettingsFile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

/// Sorted in order of precedence
impl Ord for SettingsFile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use SettingsFile::*;
        use std::cmp::Ordering;
        match (self, other) {
            (User, User) => Ordering::Equal,
            (Server, Server) => Ordering::Equal,
            (Default, Default) => Ordering::Equal,
            (Project((id1, rel_path1)), Project((id2, rel_path2))) => id1
                .cmp(id2)
                .then_with(|| rel_path1.cmp(rel_path2).reverse()),
            (Project(_), _) => Ordering::Less,
            (_, Project(_)) => Ordering::Greater,
            (Server, _) => Ordering::Less,
            (_, Server) => Ordering::Greater,
            (User, _) => Ordering::Less,
            (_, User) => Ordering::Greater,
            (Global, _) => Ordering::Less,
            (_, Global) => Ordering::Greater,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocalSettingsKind {
    Settings,
    Tasks,
    Editorconfig,
    Debug,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocalSettingsPath {
    InWorktree(Arc<RelPath>),
    OutsideWorktree(Arc<Path>),
}

impl LocalSettingsPath {
    pub fn is_outside_worktree(&self) -> bool { matches!(self, Self::OutsideWorktree(_)) }

    pub fn to_proto(&self) -> String {
        match self {
            Self::InWorktree(path) => path.as_unix_str().to_owned(),
            Self::OutsideWorktree(path) => path.to_string_lossy().to_string(),
        }
    }

    pub fn from_proto(path: &str, is_outside_worktree: bool) -> anyhow::Result<Self> {
        if is_outside_worktree {
            Ok(Self::OutsideWorktree(PathBuf::from(path).into()))
        } else {
            Ok(Self::InWorktree(RelPath::from_unix_str(path)?.into()))
        }
    }
}
