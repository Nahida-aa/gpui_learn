//! Agent 相关 settings — sidebar 位置等。
//!
//! 两个 enum 对齐 zed `crates/settings_content/src/agent.rs`:
//! - `SidebarDockPosition` — settings JSON 反序列化层（带 serde + JsonSchema）
//! - `SidebarSide` — 运行时层（简化的 Copy enum）
//!
//! Sidebar 是侧边栏概念，永远不会在 Bottom（那是 Dock 的位置）。

use std::sync::Arc;

use collections::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
pub mod agent_server;
pub mod auto_compact;
pub mod model_selection;
pub mod notification;
pub mod profile;
pub mod sandbox;
pub mod sidebar;
pub mod thinking;
pub mod tool_permissions;

pub use agent_server::{AgentConfigOptionValue, AllAgentServersSettings, CustomAgentServerSettings};
pub use auto_compact::{AutoCompactSettingsContent, AutoCompactThreshold};
pub use model_selection::{
    LanguageModelParameters, LanguageModelProviderSetting, LanguageModelSelection,
};
pub use notification::{NotifyWhenAgentWaiting, PlaySoundWhenAgentDone};
pub use profile::{AgentProfileContent, ContextServerPresetContent};
pub use sandbox::{GrantedWritePathContent, SandboxPermissionsContent};
pub use sidebar::{SidebarDockPosition, SidebarSide, ThreadsSidebarSettingsContent};
pub use thinking::ThinkingBlockDisplay;
pub use tool_permissions::{
    ToolPermissionMode, ToolPermissionsContent, ToolRegexRule, ToolRulesContent,
};
use crate::{DockPosition, common::ExtendingVec};

#[with_fallible_options]
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, Default)]
pub struct AgentSettingsContent {
    /// Whether the Agent is enabled.
    ///
    /// Default: true
    pub enabled: Option<bool>,
    /// Whether to show the agent panel button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Where to dock the agent panel.
    ///
    /// Default: left (Agentic layout), right (Classic layout)
    pub dock: Option<DockPosition>,
    /// Whether the agent panel should use flexible (proportional) sizing when docked to the
    /// left or right.
    ///
    /// When enabled, `default_width` does not control the panel width, and resetting the panel
    /// restores the default proportion.
    ///
    /// Default: true
    pub flexible: Option<bool>,
    /// Settings for the sidebar holding the agent panel and the list of agent threads.
    pub threads_sidebar: Option<ThreadsSidebarSettingsContent>,
    /// The deprecated version of `threads_sidebar.position`.
    ///
    /// Don't use this field.
    #[schemars(skip)]
    pub sidebar_side: Option<SidebarDockPosition>,
    /// The deprecated version of `threads_sidebar.default_width`.
    ///
    /// Don't use this field.
    #[schemars(skip)]
    pub threads_sidebar_default_width: Option<crate::PixelSetting>,
    /// The deprecated version of `threads_sidebar.auto_open`.
    ///
    /// Don't use this field.
    #[schemars(skip)]
    pub threads_sidebar_auto_open: Option<bool>,
    /// Maximum number of idle threads with loadable sessions to retain in the
    /// agent panel. Set to 0 to unload every eligible idle thread when it is
    /// no longer active.
    ///
    /// Default: 5
    pub max_idle_retained_threads: Option<usize>,
    /// Default fixed width in pixels when the agent panel is docked to the left or right and
    /// `flexible` is false.
    ///
    /// Default: 640
    pub default_width: Option<crate::PixelSetting>,
    /// Default height in pixels when the agent panel is docked to the bottom.
    ///
    /// Default: 320
    pub default_height: Option<crate::PixelSetting>,
    /// Whether to limit the content width in the agent panel. When enabled,
    /// content will be constrained to `max_content_width` and centered when
    /// the panel is wider than that value, for optimal readability.
    ///
    /// Default: true
    pub limit_content_width: Option<bool>,
    /// Maximum content width in pixels for the agent panel. Content will be
    /// centered when the panel is wider than this value.
    ///
    /// Default: 850
    pub max_content_width: Option<crate::PixelSetting>,
    /// The default model to use when creating new chats and for other features when a specific model is not specified.
    pub default_model: Option<LanguageModelSelection>,
    /// The model to use for subagents spawned via the `spawn_agent` tool. Defaults to the parent agent's model when not specified.
    pub subagent_model: Option<LanguageModelSelection>,
    /// Favorite models to show at the top of the model selector.
    #[serde(default)]
    pub favorite_models: Vec<LanguageModelSelection>,
    /// Model to use for the inline assistant. Defaults to default_model when not specified.
    pub inline_assistant_model: Option<LanguageModelSelection>,
    /// Model to use for the inline assistant when streaming tools are enabled.
    ///
    /// Default: true
    pub inline_assistant_use_streaming_tools: Option<bool>,
    /// Model to use for generating git commit messages. Defaults to default_model when not specified.
    pub commit_message_model: Option<LanguageModelSelection>,
    /// Whether to include project rules files (AGENTS.md, CLAUDE.md, .rules, etc.)
    /// in the prompt when generating git commit messages.
    ///
    /// Default: true
    pub commit_message_include_project_rules: Option<bool>,
    /// Custom instructions to include in the prompt when generating git commit messages.
    /// Applied in addition to any project rules files (such as `.rules` or `AGENTS.md`).
    pub commit_message_instructions: Option<String>,
    /// Model to use for generating thread summaries. Defaults to default_model when not specified.
    pub thread_summary_model: Option<LanguageModelSelection>,
    /// Model to use for context compaction (`/compact` and auto-compaction).
    /// Falls back to the thread's currently selected model when not specified.
    /// If the configured model is unavailable (provider not registered, model
    /// not found), the thread's current model is used instead.
    pub compaction_model: Option<LanguageModelSelection>,
    /// Additional models with which to generate alternatives when performing inline assists.
    pub inline_alternatives: Option<Vec<LanguageModelSelection>>,
    /// The default profile to use in the Agent.
    ///
    /// Default: write
    pub default_profile: Option<Arc<str>>,
    /// The available agent profiles.
    pub profiles: Option<IndexMap<Arc<str>, AgentProfileContent>>,
    /// Where to show a popup notification when the agent is waiting for user input.
    ///
    /// Default: "primary_screen"
    pub notify_when_agent_waiting: Option<NotifyWhenAgentWaiting>,
    /// When to play a sound when the agent has either completed its response, or needs user input.
    ///
    /// Default: never
    pub play_sound_when_agent_done: Option<PlaySoundWhenAgentDone>,
    /// Whether to keep the system awake while agent threads are running.
    ///
    /// Default: true
    pub prevent_idle_sleep: Option<bool>,
    /// Whether to display agent edits in single-file editors in addition to the review multibuffer pane.
    ///
    /// Default: false
    pub single_file_review: Option<bool>,
    /// Additional parameters for language model requests. When making a request
    /// to a model, parameters will be taken from the last entry in this list
    /// that matches the model's provider and name. In each entry, both provider
    /// and model are optional, so that you can specify parameters for either
    /// one.
    ///
    /// Default: []
    #[serde(default)]
    pub model_parameters: Vec<LanguageModelParameters>,
    /// Settings for automatic agent context compaction, which summarizes
    /// earlier messages to free up room in the model's context window once the
    /// context grows too large.
    pub auto_compact: Option<AutoCompactSettingsContent>,
    /// Whether to show thumb buttons for feedback in the agent panel.
    ///
    /// Default: true
    pub enable_feedback: Option<bool>,
    /// Whether to have edit cards in the agent panel expanded, showing a preview of the full diff.
    ///
    /// Default: true
    pub expand_edit_card: Option<bool>,
    /// Whether to have terminal cards in the agent panel expanded, showing the whole command output.
    ///
    /// Default: true
    pub expand_terminal_card: Option<bool>,
    /// Command to automatically run when Zed creates a Terminal Thread shell in the agent panel.
    /// The command is sent to the shell as if typed, so it is interpreted by your
    /// configured shell (including on Windows and remote/WSL projects).
    /// An empty string disables this behavior.
    ///
    /// Default: ""
    pub terminal_init_command: Option<String>,
    /// How thinking blocks should be displayed by default in the agent panel.
    ///
    /// Default: automatic
    pub thinking_display: Option<ThinkingBlockDisplay>,
    /// Whether clicking the stop button on a running terminal tool should also cancel the agent's generation.
    /// Note that this only applies to the stop button, not to ctrl+c inside the terminal.
    ///
    /// Default: true
    pub cancel_generation_on_terminal_stop: Option<bool>,
    /// Whether to always use cmd-enter (or ctrl-enter on Linux or Windows) to send messages in the agent panel.
    ///
    /// Default: false
    pub use_modifier_to_send: Option<bool>,
    /// Minimum number of lines of height the agent message editor should have.
    ///
    /// Default: 4
    pub message_editor_min_lines: Option<usize>,
    /// Whether to show turn statistics (elapsed time during generation, final turn duration).
    ///
    /// Default: false
    pub show_turn_stats: Option<bool>,
    /// Whether to show the merge conflict indicator in the status bar
    /// that offers to resolve conflicts using the agent.
    ///
    /// Default: true
    pub show_merge_conflict_indicator: Option<bool>,
    /// Per-tool permission rules for granular control over which tool actions
    /// require confirmation.
    ///
    /// The global `default` applies when no tool-specific rules match.
    /// For external agent servers (e.g. Claude Agent) that define their own
    /// permission modes, "deny" and "confirm" still take precedence — the
    /// external agent's permission system is only used when Zed would allow
    /// the action. Per-tool regex patterns (`always_allow`, `always_deny`,
    /// `always_confirm`) match against the tool's text input (command, path,
    /// URL, etc.).
    pub tool_permissions: Option<ToolPermissionsContent>,

    /// Persistent sandbox permission grants for agent-run terminal commands.
    /// These are populated when choosing "Allow always" from a sandbox
    /// escalation prompt.
    pub sandbox_permissions: Option<SandboxPermissionsContent>,
}

impl AgentSettingsContent {
    pub fn set_dock(&mut self, dock: DockPosition) { self.dock = Some(dock); }

    pub fn set_threads_sidebar_position(&mut self, position: Option<SidebarDockPosition>) {
        self.sidebar_side = None;
        self.threads_sidebar.get_or_insert_default().position = position;
    }

    pub fn set_threads_sidebar_default_width(
        &mut self,
        default_width: Option<crate::PixelSetting>,
    ) {
        self.threads_sidebar_default_width = None;
        self.threads_sidebar.get_or_insert_default().default_width = default_width;
    }

    pub fn set_threads_sidebar_auto_open(&mut self, auto_open: Option<bool>) {
        self.threads_sidebar_auto_open = None;
        self.threads_sidebar.get_or_insert_default().auto_open = auto_open;
    }

    pub fn set_flexible_size(&mut self, flexible: bool) { self.flexible = Some(flexible); }

    pub fn set_model(&mut self, language_model: LanguageModelSelection) {
        self.default_model = Some(language_model)
    }

    pub fn set_inline_assistant_model(&mut self, provider: String, model: String) {
        self.inline_assistant_model = Some(LanguageModelSelection {
            provider: provider.into(),
            model,
            enable_thinking: false,
            effort: None,
            speed: None,
        });
    }

    pub fn set_profile(&mut self, profile_id: Arc<str>) { self.default_profile = Some(profile_id); }

    pub fn add_favorite_model(&mut self, model: LanguageModelSelection) {
        // Note: this is intentional to not compare using `PartialEq`here.
        // Full equality would treat entries that differ just in thinking/effort/speed
        // as distinct and silently produce duplicates.
        if !self
            .favorite_models
            .iter()
            .any(|m| m.provider == model.provider && m.model == model.model)
        {
            self.favorite_models.push(model);
        }
    }

    pub fn remove_favorite_model(&mut self, model: &LanguageModelSelection) {
        self.favorite_models
            .retain(|m| !(m.provider == model.provider && m.model == model.model));
    }

    pub fn update_favorite_model<F>(&mut self, provider: &str, model: &str, f: F)
    where
        F: FnOnce(&mut LanguageModelSelection),
    {
        if let Some(entry) = self
            .favorite_models
            .iter_mut()
            .find(|m| m.provider.0 == provider && m.model == model)
        {
            f(entry);
        }
    }

    pub fn set_tool_default_permission(&mut self, tool_id: &str, mode: ToolPermissionMode) {
        let tool_permissions = self.tool_permissions.get_or_insert_default();
        let tool_rules = tool_permissions
            .tools
            .entry(Arc::from(tool_id))
            .or_default();
        tool_rules.default = Some(mode);
    }

    pub fn add_tool_allow_pattern(&mut self, tool_name: &str, pattern: String) {
        let tool_permissions = self.tool_permissions.get_or_insert_default();
        let tool_rules = tool_permissions
            .tools
            .entry(Arc::from(tool_name))
            .or_default();
        let always_allow = tool_rules.always_allow.get_or_insert_default();
        if !always_allow.0.iter().any(|r| r.pattern == pattern) {
            always_allow.0.push(ToolRegexRule {
                pattern,
                case_sensitive: None,
            });
        }
    }

    pub fn add_tool_deny_pattern(&mut self, tool_name: &str, pattern: String) {
        let tool_permissions = self.tool_permissions.get_or_insert_default();
        let tool_rules = tool_permissions
            .tools
            .entry(Arc::from(tool_name))
            .or_default();
        let always_deny = tool_rules.always_deny.get_or_insert_default();
        if !always_deny.0.iter().any(|r| r.pattern == pattern) {
            always_deny.0.push(ToolRegexRule {
                pattern,
                case_sensitive: None,
            });
        }
    }

    pub fn allow_sandbox_all_hosts(&mut self) {
        self.sandbox_permissions
            .get_or_insert_default()
            .allow_all_hosts = Some(true);
    }

    /// The persisted sandbox network host patterns, as written (callers own
    /// parsing/validation).
    pub fn sandbox_network_hosts(&self) -> &[String] {
        self.sandbox_permissions
            .as_ref()
            .and_then(|permissions| permissions.network_hosts.as_ref())
            .map(|hosts| hosts.0.as_slice())
            .unwrap_or_default()
    }

    /// Replace the persisted sandbox network host patterns. Callers compute
    /// the new list (typically the old list plus newly granted hosts, pruned
    /// of entries subsumed by wildcards) rather than appending blindly.
    pub fn set_sandbox_network_hosts(&mut self, hosts: Vec<String>) {
        self.sandbox_permissions
            .get_or_insert_default()
            .network_hosts = Some(ExtendingVec(hosts));
    }

    pub fn allow_sandbox_fs_write_all(&mut self) {
        self.sandbox_permissions
            .get_or_insert_default()
            .allow_fs_write_all = Some(true);
    }

    pub fn allow_sandbox_unsandboxed(&mut self) {
        self.sandbox_permissions
            .get_or_insert_default()
            .allow_unsandboxed = Some(true);
    }

    pub fn add_sandbox_write_path(&mut self, granted: GrantedWritePathContent) {
        let write_paths = &mut self
            .sandbox_permissions
            .get_or_insert_default()
            .write_paths
            .get_or_insert_default()
            .0;

        // Mirror `util::paths::insert_subtree`, keeping the grant set minimal,
        // but compare by each entry's canonical (resolved) grant path.
        let canonical = granted.canonical_or_requested().to_path_buf();
        if write_paths
            .iter()
            .any(|existing| canonical.starts_with(existing.canonical_or_requested()))
        {
            return;
        }
        write_paths.retain(|existing| !existing.canonical_or_requested().starts_with(&canonical));
        write_paths.push(granted);
    }
}
