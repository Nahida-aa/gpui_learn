use std::num::NonZeroUsize;

use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
mod active_pane;
mod autosave;
pub mod bar;
mod centered_layout;
pub mod item;
pub mod pane_split;
mod preview_tabs;
pub use preview_tabs::PreviewTabsSettingsContent;

pub mod opening;
pub mod project_panel;
mod semantic_tokens;
mod text_rendering;
mod window; // 包含打开窗口

// Re-export commonly used types from private submodules.
pub use crate::{
    CommandAliasTarget, DockPosition, serialize_optional_f32_with_two_decimal_places,
    workspace::{
        centered_layout::CenteredLayoutSettings,
        focus_follows_mouse::FocusFollowsMouse,
        opening::{CliDefaultOpenBehavior, DefaultOpenBehavior, RestoreOnStartupBehavior},
        pane_split::{BottomDockLayout, PaneSplitDirectionHorizontal, PaneSplitDirectionVertical},
        text_rendering::TextRenderingMode,
        item::{ActivateOnClose, ClosePosition,  ShowCloseButton, ShowDiagnostics},
    },
};
pub use active_pane::ActivePaneModifiers;
pub use autosave::{AutosaveSetting, AutosaveSettingDiscriminants};
pub use window::{
    CloseWindowWhenNoItems, FullscreenMode, OnLastWindowClosed, OnNewWindow, WindowDecorations,
};
mod focus_follows_mouse;
pub mod folder_indicator;

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct WorkspaceSettingsContent {
    /// Active pane styling settings.
    pub active_pane_modifiers: Option<ActivePaneModifiers>,
    /// The text rendering mode to use.
    ///
    /// Default: platform_default
    pub text_rendering_mode: Option<TextRenderingMode>,
    /// Layout mode for the bottom dock
    ///
    /// Default: contained
    pub bottom_dock_layout: Option<BottomDockLayout>,
    /// Direction to split horizontally.
    ///
    /// Default: "up"
    pub pane_split_direction_horizontal: Option<PaneSplitDirectionHorizontal>,
    /// Direction to split vertically.
    ///
    /// Default: "left"
    pub pane_split_direction_vertical: Option<PaneSplitDirectionVertical>,
    /// Centered layout related settings.
    pub centered_layout: Option<CenteredLayoutSettings>,
    /// Whether or not to prompt the user to confirm before closing the application.
    ///
    /// Default: false
    pub confirm_quit: Option<bool>,
    /// Whether or not to show the call status icon in the status bar.
    ///
    /// Default: true
    pub show_call_status_icon: Option<bool>,
    /// When to automatically save edited buffers.
    ///
    /// Default: off
    pub autosave: Option<AutosaveSetting>,
    /// Controls previous session restoration in freshly launched Zed instance.
    /// Values: empty_tab, last_workspace, last_session, launchpad
    /// Default: last_session
    pub restore_on_startup: Option<RestoreOnStartupBehavior>,
    /// The default behavior when opening paths from the CLI without
    /// an explicit `-e` or `-n` flag.
    ///
    /// Default: existing_window
    pub cli_default_open_behavior: Option<CliDefaultOpenBehavior>,
    /// The default behavior when opening projects from the UI.
    ///
    /// Default: existing_window
    pub default_open_behavior: Option<DefaultOpenBehavior>,
    /// Whether to attempt to restore previous file's state when opening it again.
    /// The state is stored per pane.
    /// When disabled, defaults are applied instead of the state restoration.
    ///
    /// E.g. for editors, selections, folds and scroll positions are restored, if the same file is closed and, later, opened again in the same pane.
    /// When disabled, a single selection in the very beginning of the file, zero scroll position and no folds state is used as a default.
    ///
    /// Default: true
    pub restore_on_file_reopen: Option<bool>,
    /// Whether to reveal an already open file in an existing pane instead of opening it in the active pane.
    ///
    /// Default: false
    pub reveal_if_open: Option<bool>,
    /// The size of the workspace split drop targets on the outer edges.
    /// Given as a fraction that will be multiplied by the smaller dimension of the workspace.
    ///
    /// Default: `0.2` (20% of the smaller dimension of the workspace)
    #[serde(serialize_with = "serialize_optional_f32_with_two_decimal_places")]
    pub drop_target_size: Option<f32>,
    /// Whether to close the window when using 'close active item' on a workspace with no tabs
    ///
    /// Default: auto ("on" on macOS, "off" otherwise)
    pub when_closing_with_no_tabs: Option<CloseWindowWhenNoItems>,
    /// Whether to optimize Zed's interface for assistive technology such as
    /// screen readers.
    ///
    /// Default: false
    pub accessible_mode: Option<bool>,
    /// Whether to use the system provided dialogs for Open and Save As.
    /// When set to false, Zed will use the built-in keyboard-first pickers.
    ///
    /// Default: true
    pub use_system_path_prompts: Option<bool>,
    /// Whether to use the system provided prompts.
    /// When set to false, Zed will use the built-in prompts.
    /// Note that this setting has no effect on Linux, where Zed will always
    /// use the built-in prompts.
    ///
    /// Default: true
    pub use_system_prompts: Option<bool>,
    /// Aliases for the command palette. When you type a key in this map,
    /// it will be assumed to equal the value.
    ///
    /// Default: {}
    #[serde(default)]
    pub command_aliases: HashMap<String, CommandAliasTarget>,
    /// Maximum open tabs in a pane. Will not close an unsaved
    /// tab. Set to `None` for unlimited tabs.
    ///
    /// Default: none
    pub max_tabs: Option<NonZeroUsize>,
    /// What to show when opening a new window.
    /// Values: empty_tab, launchpad
    /// Default: launchpad
    pub on_new_window: Option<OnNewWindow>,
    /// What to do when the last window is closed
    ///
    /// Default: auto (nothing on macOS, "app quit" otherwise)
    pub on_last_window_closed: Option<OnLastWindowClosed>,
    /// Whether to resize all the panels in a dock when resizing the dock.
    ///
    /// Default: ["left"]
    pub resize_all_panels_in_dock: Option<Vec<DockPosition>>,
    /// Whether to automatically close files that have been deleted on disk.
    ///
    /// Default: false
    pub close_on_file_delete: Option<bool>,
    /// Whether to allow windows to tab together based on the user’s tabbing preference (macOS only).
    ///
    /// Default: false
    pub use_system_window_tabs: Option<bool>,
    /// Which fullscreen mode the `zed::ToggleFullScreen` action enters (macOS only).
    ///
    /// Default: native
    pub fullscreen_mode: Option<FullscreenMode>,
    /// Whether to show padding for zoomed panels.
    /// When enabled, zoomed bottom panels will have some top padding,
    /// while zoomed left/right panels will have padding to the right/left (respectively).
    ///
    /// Default: true
    pub zoomed_padding: Option<bool>,
    /// Whether invoking a panel's `ToggleFocus` action while the panel is
    /// already focused closes the panel, instead of just moving focus back
    /// to the editor. This only applies to a panel's focus-toggle action, not
    /// to its regular visibility-toggle action.
    ///
    /// Default: false
    pub close_panel_on_toggle: Option<bool>,
    /// Window title template.
    ///
    /// Available variables are `${projectName}`, `${fileName}`,
    /// `${filePath}`, `${relativePath}`, `${fileStem}`, `${remoteName}`,
    /// `${remoteHost}`, `${appName}`, `${branch}`,
    /// and `${separator}`.
    /// `${separator}` is omitted when adjacent variables are empty,
    /// but literal text is preserved.
    /// The collaboration indicator, when present, is appended after the
    /// rendered template.
    /// If the template renders to nothing, the default template is used instead.
    ///
    /// Default: `${projectName}${separator}${fileName}`
    pub window_title_format: Option<String>,
    /// String substituted for `${separator}` in the window title format.
    /// Include any surrounding whitespace in the value.
    ///
    /// Default: ` — `
    pub window_title_separator: Option<String>,
    /// Controls whether Zed or the window manager or compositor draws window decorations on Linux.
    ///
    /// Default: client
    pub window_decorations: Option<WindowDecorations>,
    /// Whether the focused panel follows the mouse location
    /// Default: false
    pub focus_follows_mouse: Option<FocusFollowsMouse>,
}
