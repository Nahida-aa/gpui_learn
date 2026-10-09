use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::PixelSetting;

/// Where to position the threads sidebar.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum SidebarDockPosition {
    /// Always show the sidebar on the left side.
    #[default]
    Left,
    /// Always show the sidebar on the right side.
    Right,
}
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

// 双向转换 — settings 层 ↔ 运行时层
impl From<SidebarDockPosition> for SidebarSide {
    fn from(p: SidebarDockPosition) -> Self {
        match p {
            SidebarDockPosition::Left => Self::Left,
            SidebarDockPosition::Right => Self::Right,
        }
    }
}

impl From<SidebarSide> for SidebarDockPosition {
    fn from(s: SidebarSide) -> Self {
        match s {
            SidebarSide::Left => Self::Left,
            SidebarSide::Right => Self::Right,
        }
    }
}

impl SidebarSide {
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

#[with_fallible_options]
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, Default)]
pub struct ThreadsSidebarSettingsContent {
    /// Whether opening a folder in an existing window automatically opens the
    /// Threads Sidebar. Applies when `default_open_behavior` or
    /// `cli_default_open_behavior` is set to `existing_window`.
    ///
    /// Default: true
    pub auto_open: Option<bool>,
    /// Where to position the threads sidebar.
    ///
    /// Default: left
    pub position: Option<SidebarDockPosition>,
    /// Default width of the threads sidebar in pixels.
    ///
    /// Values range from 200 to 800, matching the widths the sidebar can be
    /// dragged to. Values outside that range are clamped into it.
    ///
    /// Default: 300
    #[schemars(range(min = 200, max = 800))]
    pub default_width: Option<PixelSetting>,
}
