use crate::merge_from::MergeFrom as MergeFromTrait;
use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::sync::Arc;

use super::common::{
    LanguageModelCacheConfiguration, ModelMode, OpenAiReasoningEffort, default_true,
};
use language_model_core::ReasoningEffort;

#[with_fallible_options]
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct OpenCodeSettingsContent {
    pub api_url: Option<String>,
    pub available_models: Option<Vec<OpenCodeAvailableModel>>,
    pub custom_headers: Option<HashMap<String, String>>,
    /// Whether to show OpenCode Zen models. Defaults to true.
    pub show_zen_models: Option<bool>,
    /// Whether to show OpenCode Go models. Defaults to true.
    pub show_go_models: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub enum OpenCodeApiProtocol {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai_responses", alias = "open_ai_responses")]
    OpenAiResponses,
    #[serde(rename = "openai_chat", alias = "open_ai_chat")]
    OpenAiChat,
    #[serde(rename = "google")]
    Google,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(rename_all = "snake_case")]
pub enum OpenCodeModelSubscription {
    Zen,
    Go,
}

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct OpenCodeAvailableModel {
    pub name: String,
    pub display_name: Option<String>,
    pub max_tokens: u64,
    pub max_output_tokens: Option<u64>,
    /// The API protocol to use for this model: "anthropic", "openai_responses", "openai_chat", or "google". Defaults to "openai_chat".
    pub protocol: Option<OpenCodeApiProtocol>,
    /// The subscription for this model: "zen" or "go". Defaults to Zen.
    pub subscription: Option<OpenCodeModelSubscription>,
    /// Custom Model API URL to use for this model.
    pub custom_model_api_url: Option<String>,
    /// Supported reasoning effort levels, for example `["low", "medium", "high"].
    pub reasoning_effort_levels: Option<Vec<ReasoningEffort>>,
    /// When using OpenAiChat protocol, whether thinking tokens are sent as a dedicated `reasoning_content` field or inline in message text.
    #[serde(default)]
    pub interleaved_reasoning: bool,
}
