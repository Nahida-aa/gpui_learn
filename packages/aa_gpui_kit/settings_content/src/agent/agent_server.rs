use std::path::PathBuf;

use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

#[with_fallible_options]
#[derive(Default, PartialEq, Deserialize, Serialize, Clone, JsonSchema, MergeFrom, Debug)]
#[serde(transparent)]
pub struct AllAgentServersSettings(pub HashMap<String, CustomAgentServerSettings>);

impl std::ops::Deref for AllAgentServersSettings {
    type Target = HashMap<String, CustomAgentServerSettings>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for AllAgentServersSettings {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

/// The value of a session config option.
///
/// Aligns Zed `crates/settings_content/src/agent.rs:705`.
#[derive(Deserialize, Serialize, Clone, JsonSchema, MergeFrom, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum AgentConfigOptionValue {
    ValueId(String),
    Boolean(bool),
}

impl AgentConfigOptionValue {
    pub fn as_value_id(&self) -> Option<&str> {
        match self {
            Self::ValueId(value) => Some(value),
            Self::Boolean(_) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::ValueId(_) => None,
        }
    }
}

impl std::fmt::Display for AgentConfigOptionValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValueId(value) => formatter.write_str(value),
            Self::Boolean(value) => value.fmt(formatter),
        }
    }
}

impl From<String> for AgentConfigOptionValue {
    fn from(value: String) -> Self { Self::ValueId(value) }
}

impl From<&str> for AgentConfigOptionValue {
    fn from(value: &str) -> Self { Self::ValueId(value.to_string()) }
}

impl From<bool> for AgentConfigOptionValue {
    fn from(value: bool) -> Self { Self::Boolean(value) }
}

#[with_fallible_options]
#[derive(Deserialize, Serialize, Clone, JsonSchema, MergeFrom, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CustomAgentServerSettings {
    Custom {
        #[serde(rename = "command")]
        path: PathBuf,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
        /// The default mode to use for this agent.
        ///
        /// Note: Not only all agents support modes.
        ///
        /// Default: None
        default_mode: Option<String>,
        /// Default values for session config options.
        ///
        /// This is a map from config option ID to the default value for that option.
        ///
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        default_config_options: HashMap<String, AgentConfigOptionValue>,
        /// Favorited values for session config options.
        ///
        /// This is a map from config option ID to a list of favorited value IDs.
        ///
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        favorite_config_option_values: HashMap<String, Vec<String>>,
    },
    // Used for the ACP extension migration
    #[serde(alias = "extension")]
    Registry {
        /// Additional environment variables to pass to the agent.
        ///
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
        /// The default mode to use for this agent.
        ///
        /// Note: Not only all agents support modes.
        ///
        /// Default: None
        default_mode: Option<String>,
        /// Default values for session config options.
        ///
        /// This is a map from config option ID to the default value for that option.
        ///
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        default_config_options: HashMap<String, AgentConfigOptionValue>,
        /// Favorited values for session config options.
        ///
        /// This is a map from config option ID to a list of favorited value IDs.
        ///
        /// Default: {}
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        favorite_config_option_values: HashMap<String, Vec<String>>,
    },
}
