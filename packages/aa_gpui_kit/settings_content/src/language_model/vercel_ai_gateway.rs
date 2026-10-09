use super::common::{
    LanguageModelCacheConfiguration, ModelMode, OpenAiReasoningEffort, default_true,
};
use super::openai::OpenAiCompatibleModelCapabilities;
use crate::merge_from::MergeFrom as MergeFromTrait;
use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::sync::Arc;

#[with_fallible_options]
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema, MergeFrom)]
pub struct VercelAiGatewaySettingsContent {
    pub api_url: Option<String>,
    pub available_models: Option<Vec<VercelAiGatewayAvailableModel>>,
    pub custom_headers: Option<HashMap<String, String>>,
}

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct VercelAiGatewayAvailableModel {
    pub name: String,
    pub display_name: Option<String>,
    pub max_tokens: u64,
    pub max_output_tokens: Option<u64>,
    pub max_completion_tokens: Option<u64>,
    #[serde(default)]
    pub capabilities: OpenAiCompatibleModelCapabilities,
}
