use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::sync::Arc;

use crate::ExtendingVec;

#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ToolPermissionsContent {
    /// Global default permission when no tool-specific rules match.
    /// Individual tools can override this with their own default.
    /// Default: confirm
    #[serde(alias = "default_mode")]
    pub default: Option<ToolPermissionMode>,

    /// Per-tool permission rules.
    /// Keys are tool names (e.g. terminal, edit_file, fetch) including MCP
    /// tools (e.g. mcp:server_name:tool_name). Any tool name is accepted;
    /// even tools without meaningful text input can have a `default` set.
    #[serde(default)]
    pub tools: HashMap<Arc<str>, ToolRulesContent>,
}

#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ToolRulesContent {
    /// Default mode when no regex rules match.
    /// When unset, inherits from the global `tool_permissions.default`.
    #[serde(alias = "default_mode")]
    pub default: Option<ToolPermissionMode>,

    /// Regexes for inputs to auto-approve.
    /// For terminal: matches command. For file tools: matches path. For fetch: matches URL.
    /// For `copy_path` and `move_path`, patterns are matched independently against each
    /// path (source and destination).
    /// Patterns accumulate across settings layers (user, project, profile) and cannot be
    /// removed by a higher-priority layer—only new patterns can be added.
    /// Default: []
    pub always_allow: Option<ExtendingVec<ToolRegexRule>>,

    /// Regexes for inputs to auto-reject.
    /// **SECURITY**: These take precedence over ALL other rules, across ALL settings layers.
    /// For `copy_path` and `move_path`, patterns are matched independently against each
    /// path (source and destination).
    /// Patterns accumulate across settings layers (user, project, profile) and cannot be
    /// removed by a higher-priority layer—only new patterns can be added.
    /// Default: []
    pub always_deny: Option<ExtendingVec<ToolRegexRule>>,

    /// Regexes for inputs that must always prompt.
    /// Takes precedence over always_allow but not always_deny.
    /// For `copy_path` and `move_path`, patterns are matched independently against each
    /// path (source and destination).
    /// Patterns accumulate across settings layers (user, project, profile) and cannot be
    /// removed by a higher-priority layer—only new patterns can be added.
    /// Default: []
    pub always_confirm: Option<ExtendingVec<ToolRegexRule>>,
}

#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ToolRegexRule {
    /// The regex pattern to match.
    #[serde(default)]
    pub pattern: String,

    /// Whether the regex is case-sensitive.
    /// Default: false (case-insensitive)
    pub case_sensitive: Option<bool>,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, MergeFrom,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionMode {
    /// Auto-approve without prompting.
    Allow,
    /// Auto-reject with an error.
    Deny,
    /// Always prompt for confirmation (default behavior).
    #[default]
    Confirm,
}

impl std::fmt::Display for ToolPermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolPermissionMode::Allow => write!(f, "Allow"),
            ToolPermissionMode::Deny => write!(f, "Deny"),
            ToolPermissionMode::Confirm => write!(f, "Confirm"),
        }
    }
}
