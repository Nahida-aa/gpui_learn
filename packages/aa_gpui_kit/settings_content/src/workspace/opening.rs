use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    Debug,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum CliDefaultOpenBehavior {
    /// Open directories as a new workspace in the current Zed window's sidebar.
    #[default]
    #[strum(serialize = "Add to Existing Window")]
    ExistingWindow,
    /// Open paths in a new window unless they are subpaths of an existing project.
    #[strum(serialize = "Open a New Window")]
    NewWindow,
}

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    Debug,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DefaultOpenBehavior {
    /// Open projects in the current Zed window.
    #[default]
    #[strum(serialize = "Add to Existing Window")]
    ExistingWindow,
    /// Open projects in a new window.
    #[strum(serialize = "Open a New Window")]
    NewWindow,
}

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    Debug,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum RestoreOnStartupBehavior {
    /// Always start with an empty editor tab
    #[serde(alias = "none")]
    EmptyTab,
    /// Restore the workspace that was closed last.
    LastWorkspace,
    /// Restore all workspaces that were open when quitting Zed.
    #[default]
    LastSession,
    /// Show the launchpad with recent projects (no tabs).
    Launchpad,
}
