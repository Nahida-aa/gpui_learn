//! 设置内容定义（对齐 Zed `crates/settings_content`）。
//!
//! 存放从 settings JSON 反序列化的 Setting struct。
//! 按功能分子模块：language_model、agent、editor、terminal、theme ...
//!
//! settings crate（settings + settings_store）是**基础设施**，
//! 负责 RustEmbed + SettingsStore Global；
//! settings_content 是**内容层**，只放数据结构。
pub use merge_from::MergeFrom as MergeFromTrait;
mod action;
pub mod agent;

mod contents;
// zed 里这些内容 struct 直接写在 settings_content.rs 里、天然在 crate 根；
// 我们按功能拆进了 `contents/`，所以这里要把它们再导出到根，
// 否则 `settings::SettingsContent` 那层 `pub use ::settings_content::*` 带不出来。
pub use contents::*;
pub mod extension;
pub mod fallible_options;
mod feature_flags;
mod language;
pub mod language_model;
pub mod merge_from;
mod project;
mod serde_helper;
pub mod terminal;
pub mod theme;
mod title_bar;
mod ui;
pub use action::{ActionName, ActionWithArguments, CommandAliasTarget};
use collections::{HashMap, IndexMap};
pub use language::*;
pub use project::{GitHostingProviderConfig, SemanticTokenRules, *};
pub use serde_helper::{
    serialize_f32_with_two_decimal_places, serialize_optional_f32_with_two_decimal_places,
};
use settings_json::parse_json_with_comments;
// ---------- 通用工具 re-export ----------
pub use common::{
    DelayMs, DocumentFoldingRanges, DocumentSymbols, ExtendingSet, ExtendingVec, PixelSetting,
    SaturatingBool, SemanticTokens, SplicingVec,
};
pub use fallible_options::{FallibleOption, deserialize as deserialize_fallible, parse_json};
pub use merge_from::MergeFrom;

// ---------- 各子模块 re-export ----------

pub use agent::*;
pub use language_model::{Config, McpConfig, McpServerDef, ProviderConfig, ResolvedConfig};
pub use language_model_core::*;
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use settings_macros::{MergeFrom, with_fallible_options};

mod editor;

mod common;
mod macros;
mod overrides;
mod profiles;
mod workspace;
pub use crate::{
    agent::{AgentSettingsContent, AllAgentServersSettings},
    contents::{
        AudioSettingsContent, CallHierarchySettingsContent, CallSettingsContent,
        CommandPaletteSettingsContent, DebuggerSettingsContent, FileFinderSettingsContent,
        GitPanelSettingsContent, ImageViewerSettingsContent, InstrumentationSettingsContent,
        JournalSettingsContent, MarkdownPreviewSettingsContent, OutlinePanelSettingsContent,
        PanelSettingsContent, RemoteSettingsContent, ReplSettingsContent, SshConnection,
        SshPortForwardOption, TelemetrySettingsContent, VimSettingsContent,
        WhichKeySettingsContent, WslConnection  ,
    },
    extension::{ExtensionSettingsContent, ExtensionCapabilityContent},
    feature_flags::FeatureFlagsMap,
    language_model::{AllLanguageModelSettingsContent, OpenAiReasoningEffort, ollama::{KeepAlive, OllamaAvailableModel}, open_router::{DataCollection, OpenRouterAvailableModel, OpenRouterProvider},
        bedrock::{BedrockMantleProtocolContent,BedrockAuthMethodContent, BedrockMantleAvailableModel, BedrockAvailableModel},
        opencode::{OpenCodeModelSubscription, OpenCodeApiProtocol, OpenCodeAvailableModel},
        vercel_ai_gateway::VercelAiGatewayAvailableModel,
        x_ai::XaiAvailableModel,
        openai::{OpenAiCompatibleModelCapabilities, OpenAiAvailableModel, OpenAiCompatibleAvailableModel, OpenAiCompatibleSettingsContent},
        mistral::MistralAvailableModel,
        deepseek::DeepseekAvailableModel,
        lmstudio::LmStudioAvailableModel,
        llama_cpp::LlamaCppAvailableModel,
        google::GoogleAvailableModel,
        zed_dot_dev::{ZedDotDevAvailableModel, ZedDotDevAvailableProvider},
        anthropic::{AnthropicCompatibleAvailableModel, AnthropicAvailableModel, AnthropicCompatibleModelCapabilities, AnthropicCompatibleSettingsContent},
    },
    profiles::SettingsProfile,
    project::{
        ContextServerCommand, DiagnosticsSettingsContent, GitSettings, GlobalLspSettingsContent,
        InlineBlameSettings, NodeBinarySettings, SemanticTokenColorOverride,
        SemanticTokenFontStyle, SemanticTokenFontWeight, SemanticTokenRule, SessionSettingsContent,
        WorktreeSettingsContent,
    },
    title_bar::{TitleBarSettingsContent, WindowButtonLayoutContent, WindowButtonLayoutContentDiscriminants},
    ui::{HideMouseMode, LineIndicatorFormat, ReduceMotionMode, StatusStyle},
    workspace::{
        ActivePaneModifiers, AutosaveSetting, AutosaveSettingDiscriminants,
        CenteredLayoutSettings, CloseWindowWhenNoItems, CliDefaultOpenBehavior, FullscreenMode,
        OnLastWindowClosed, OnNewWindow, PaneSplitDirectionHorizontal,
        PaneSplitDirectionVertical, PreviewTabsSettingsContent, TextRenderingMode,
        WindowDecorations, WorkspaceSettingsContent,
        bar::{EncodingDisplayOptions, StatusBarSettingsContent, TabBarSettingsContent},
        item::{
            ActivateOnClose, ClosePosition, ItemSettingsContent, ShowCloseButton, ShowDiagnostics,
        },
        opening::{DefaultOpenBehavior, RestoreOnStartupBehavior},
        pane_split::BottomDockLayout,
        project_panel::{
            ProjectPanelEntrySpacing, ProjectPanelScrollbarSettingsContent,
            ProjectPanelSettingsContent, ProjectPanelSortMode, ProjectPanelSortOrder,
            ProjectPanelTitleTooltipDelay, ProjectPanelTitleTooltipDelayDiscriminants,
        },
    },
};
pub use common::ParseStatus;
pub use contents::BaseKeymapContent;
pub use editor::cursor::CursorShape;
// 对齐 zed settings_content.rs 的 `pub use editor::*;`：editor 那边是一整份 API，
// 逐个点名必漏（漏一个就让下游 `settings::Xxx` 报 E0432）。
pub use editor::*;
pub use overrides::{PlatformOverrides, ReleaseChannelOverrides};
pub use profiles::ProfileBase;
pub use project::{
    ContextServerSettingsContent, LspSettings, LspSettingsMap, ProjectSettingsContent,
};
pub use terminal::{
    ActivateScript, AlternateScroll, CondaManager, CursorShapeContent, PathHyperlinkRegex,
    ProjectTerminalSettingsContent, ScrollbarSettingsContent, Shell, ShellDiscriminants,
    ShowScrollbar, TerminalBell, TerminalBlink, TerminalDockPosition, TerminalLineHeight,
    TerminalSettingsContent, TerminalToolbarContent, VenvSettings, WorkingDirectory,
    WorkingDirectoryDiscriminants,
};
pub use theme::{
    AccentContent,
    BufferLineHeight,
    BufferLineHeightDiscriminants,
    CodeFade,
    DEFAULT_DARK_THEME,
    DEFAULT_LIGHT_THEME,
    FontFamilyName,
    FontFeaturesContent,
    FontSize,
    FontStyleContent,
    FontWeightContent,
    HighlightStyleContent,
    IconThemeName,
    IconThemeSelection,
    IconThemeSelectionDiscriminants,
    PlayerColorContent,
    StatusColorsContent,
    ThemeAppearanceMode,
    ThemeColor,
    ThemeColorsContent,
    ThemeName,
    ThemeSelection,
    ThemeSelectionDiscriminants,
    ThemeSettingsContent,
    ThemeStyleContent,
    UiDensity,
    WindowBackgroundContent,
    // font::FontSettingsContent,
};
pub use ui::{DockPosition, DockSide, ModalWidthContent, ShowIndentGuides};
pub use workspace::folder_indicator::FolderIndicator;

#[with_fallible_options]
#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, MergeFrom)]
pub struct SettingsContent {
    #[serde(flatten)]
    pub project: ProjectSettingsContent,

    #[serde(flatten)]
    pub theme: Box<ThemeSettingsContent>,

    #[serde(flatten)]
    pub extension: ExtensionSettingsContent,

    #[serde(flatten)]
    pub workspace: WorkspaceSettingsContent,

    #[serde(flatten)]
    pub editor: EditorSettingsContent,

    #[serde(flatten)]
    pub remote: RemoteSettingsContent,

    /// Settings related to the command palette.
    pub command_palette: Option<CommandPaletteSettingsContent>,

    /// Settings related to the file finder.
    pub file_finder: Option<FileFinderSettingsContent>,

    pub call_hierarchy: Option<CallHierarchySettingsContent>,

    pub git_panel: Option<GitPanelSettingsContent>,

    pub tabs: Option<ItemSettingsContent>,
    pub tab_bar: Option<TabBarSettingsContent>,
    pub status_bar: Option<StatusBarSettingsContent>,

    pub preview_tabs: Option<PreviewTabsSettingsContent>,

    pub agent: Option<AgentSettingsContent>,
    pub agent_servers: Option<AllAgentServersSettings>,

    /// Configuration of audio in Zed.
    pub audio: Option<AudioSettingsContent>,

    /// Whether or not to automatically check for updates.
    ///
    /// Default: true
    pub auto_update: Option<bool>,

    /// This base keymap settings adjusts the default keybindings in Zed to be similar
    /// to other common code editors. By default, Zed's keymap closely follows VSCode's
    /// keymap, with minor adjustments, this corresponds to the "VSCode" setting.
    ///
    /// Default: VSCode
    pub base_keymap: Option<BaseKeymapContent>,

    /// Configuration for the collab panel visual settings.
    pub collaboration_panel: Option<PanelSettingsContent>,

    pub debugger: Option<DebuggerSettingsContent>,

    /// Configuration for Diagnostics-related features.
    pub diagnostics: Option<DiagnosticsSettingsContent>,

    /// Configuration for Git-related features
    pub git: Option<GitSettings>,

    /// Common language server settings.
    pub global_lsp_settings: Option<GlobalLspSettingsContent>,

    /// The settings for the image viewer.
    pub image_viewer: Option<ImageViewerSettingsContent>,

    /// The settings for the markdown preview.
    pub markdown_preview: Option<MarkdownPreviewSettingsContent>,

    pub repl: Option<ReplSettingsContent>,

    /// Whether or not to enable Helix mode.
    ///
    /// Default: false
    pub helix_mode: Option<bool>,

    /// Determines when the mouse cursor should be hidden in response to
    /// keyboard input. Applies globally across all input surfaces (editors,
    /// terminals, palettes, etc.).
    ///
    /// Default: on_typing_and_action
    pub hide_mouse: Option<HideMouseMode>,

    pub journal: Option<JournalSettingsContent>,

    /// A map of log scopes to the desired log level.
    /// Useful for filtering out noisy logs or enabling more verbose logging.
    ///
    /// Example: {"log": {"client": "warn"}}
    pub log: Option<HashMap<String, String>>,

    pub line_indicator_format: Option<LineIndicatorFormat>,

    pub language_models: Option<AllLanguageModelSettingsContent>,

    pub outline_panel: Option<OutlinePanelSettingsContent>,

    pub project_panel: Option<ProjectPanelSettingsContent>,

    /// Configuration for Node-related features
    pub node: Option<NodeBinarySettings>,

    pub proxy: Option<String>,

    /// Whether to reduce non-essential motion in the UI, such as loading
    /// spinners and pulsating labels, by rendering them in a static state.
    ///
    /// Default: off
    pub reduce_motion: Option<ReduceMotionMode>,

    /// The URL of the Zed server to connect to.
    pub server_url: Option<String>,

    /// The URL used as the key for credential storage.
    ///
    /// When set, credentials are stored under this URL instead of `server_url`.
    /// This allows running multiple Zed instances side by side without them
    /// overwriting each other's keychain entries.
    pub credentials_url: Option<String>,

    /// Configuration for session-related features
    pub session: Option<SessionSettingsContent>,
    /// Control what info is collected by Zed.
    pub telemetry: Option<TelemetrySettingsContent>,

    /// Configuration of the terminal in Zed.
    pub terminal: Option<TerminalSettingsContent>,

    pub title_bar: Option<TitleBarSettingsContent>,

    /// Whether or not to enable Vim mode.
    ///
    /// Default: false
    pub vim_mode: Option<bool>,

    // Settings related to calls in Zed
    pub calls: Option<CallSettingsContent>,

    /// Settings for the which-key popup.
    pub which_key: Option<WhichKeySettingsContent>,

    /// Settings related to Vim mode in Zed.
    pub vim: Option<VimSettingsContent>,

    /// Number of lines to search for modelines at the beginning and end of files.
    /// Modelines contain editor directives (e.g., vim/emacs settings) that configure
    /// the editor behavior for specific files.
    ///
    /// Default: 5
    pub modeline_lines: Option<usize>,

    /// Local overrides for feature flags, keyed by flag name.
    pub feature_flags: Option<FeatureFlagsMap>,

    /// Settings for developer-oriented instrumentation tools (profilers,
    /// tracers, etc.) that can be toggled at runtime.
    pub instrumentation: Option<InstrumentationSettingsContent>,
}

#[with_fallible_options]
#[derive(Debug, Default, PartialEq, Clone, Serialize, JsonSchema, MergeFrom)]
pub struct UserSettingsContent {
    #[serde(flatten)]
    pub content: Box<SettingsContent>,

    #[serde(flatten)]
    pub release_channel_overrides: ReleaseChannelOverrides,

    #[serde(flatten)]
    pub platform_overrides: PlatformOverrides,

    #[serde(default)]
    pub profiles: IndexMap<String, SettingsProfile>,
}

pub struct ExtensionsSettingsContent {
    pub all_languages: AllLanguageSettingsContent,
}

// These impls are there to optimize builds by avoiding monomorphization downstream. Yes, they're repetitive, but using default impls
// break the optimization, for whatever reason.
pub trait RootUserSettings: Sized + DeserializeOwned {
    fn parse_json(json: &str) -> (Option<Self>, ParseStatus);
    fn parse_json_with_comments(json: &str) -> anyhow::Result<Self>;
}

impl RootUserSettings for SettingsContent {
    fn parse_json(json: &str) -> (Option<Self>, ParseStatus) { fallible_options::parse_json(json) }
    fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
        parse_json_with_comments(json)
    }
}
// Explicit opt-in instead of blanket impl to avoid monomorphizing downstream. Just a hunch though.
impl RootUserSettings for Option<SettingsContent> {
    fn parse_json(json: &str) -> (Option<Self>, ParseStatus) { fallible_options::parse_json(json) }
    fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
        parse_json_with_comments(json)
    }
}
impl RootUserSettings for UserSettingsContent {
    fn parse_json(json: &str) -> (Option<Self>, ParseStatus) { fallible_options::parse_json(json) }
    fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
        parse_json_with_comments(json)
    }
}

fallible_options::flattened_deserialize!(SettingsContent {
    sections: { project, theme, extension, workspace, editor, remote },
    options: {
        call_hierarchy, command_palette, file_finder, git_panel, tabs, tab_bar, status_bar, preview_tabs, agent,
        agent_servers, audio, auto_update, base_keymap, collaboration_panel, debugger, diagnostics,
        git,
        global_lsp_settings, image_viewer, markdown_preview, repl, helix_mode, hide_mouse,
        journal, log, line_indicator_format, language_models, outline_panel, project_panel,
        node, proxy, reduce_motion, server_url, credentials_url, session, telemetry, terminal,
        title_bar, vim_mode, calls, which_key, vim, modeline_lines, feature_flags,
        instrumentation,
    },
    defaults: {},
});

fallible_options::flattened_deserialize!(UserSettingsContent {
    sections: { content, release_channel_overrides, platform_overrides },
    options: {},
    defaults: { profiles },
});
