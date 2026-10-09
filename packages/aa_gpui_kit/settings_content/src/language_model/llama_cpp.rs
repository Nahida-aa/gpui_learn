use crate::merge_from::MergeFrom as MergeFromTrait;
use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::sync::Arc;

use super::common::{
    LanguageModelCacheConfiguration, ModelMode, OpenAiReasoningEffort, default_true,
};

#[with_fallible_options]
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct LlamaCppSettingsContent {
    pub api_url: Option<String>,
    /// Whether to automatically discover models served by the llama.cpp server.
    /// Defaults to true.
    pub auto_discover: Option<bool>,
    pub available_models: Option<Vec<LlamaCppAvailableModel>>,
    /// Overrides the context length reported for every llama.cpp model.
    pub context_window: Option<u64>,
    pub custom_headers: Option<HashMap<String, String>>,
}

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct LlamaCppAvailableModel {
    /// The model id reported by the llama.cpp server (its `--alias` or the model file path).
    pub name: String,
    /// The model's name in Zed's UI, such as in the model selector dropdown menu in the agent panel.
    pub display_name: Option<String>,
    /// The Context Length parameter to the model (aka n_ctx).
    pub max_tokens: u64,
    /// Whether the model supports tools.
    pub supports_tools: Option<bool>,
    /// Whether the model supports vision.
    pub supports_images: Option<bool>,
    /// Whether the model emits reasoning/thinking content.
    pub supports_thinking: Option<bool>,
}
