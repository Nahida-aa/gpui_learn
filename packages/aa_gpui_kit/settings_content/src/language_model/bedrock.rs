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
pub struct AmazonBedrockSettingsContent {
    pub available_models: Option<Vec<BedrockAvailableModel>>,
    /// Custom models served through the `bedrock-mantle` endpoint's
    /// OpenAI-compatible APIs, in addition to the built-in Mantle models
    /// (GPT-5.6 Sol, GPT-5.6 Terra, GPT-5.6 Luna, GPT-5.5, GPT-5.4, Grok 4.3).
    pub mantle_available_models: Option<Vec<BedrockMantleAvailableModel>>,
    pub custom_headers: Option<HashMap<String, String>>,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
    pub profile: Option<String>,
    pub authentication_method: Option<BedrockAuthMethodContent>,
    pub allow_global: Option<bool>,
    /// The guardrail identifier (ARN or ID) to apply to Bedrock API requests.
    pub guardrail_identifier: Option<String>,
    /// The guardrail version to use. Defaults to "DRAFT" if not specified.
    pub guardrail_version: Option<String>,
}

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct BedrockAvailableModel {
    pub name: String,
    pub display_name: Option<String>,
    pub max_tokens: u64,
    pub cache_configuration: Option<LanguageModelCacheConfiguration>,
    pub max_output_tokens: Option<u64>,
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub default_temperature: Option<f32>,
    pub mode: Option<ModelMode>,
}

#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct BedrockMantleAvailableModel {
    /// The model id as expected in Bedrock Mantle request bodies, e.g. `openai.gpt-5.5`.
    pub name: String,
    pub display_name: Option<String>,
    pub max_tokens: u64,
    pub max_output_tokens: Option<u64>,
    /// The OpenAI-compatible API this model must be called through.
    pub protocol: BedrockMantleProtocolContent,
    pub supports_tools: Option<bool>,
    pub supports_images: Option<bool>,
    /// Whether this custom Mantle model supports OpenAI reasoning effort parameters.
    pub supports_thinking: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub enum BedrockMantleProtocolContent {
    /// The OpenAI Chat Completions API (`/chat/completions`).
    #[serde(rename = "chat_completions")]
    ChatCompletions,
    /// The OpenAI Responses API (`/responses`).
    #[serde(rename = "responses")]
    Responses,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub enum BedrockAuthMethodContent {
    #[serde(rename = "named_profile")]
    NamedProfile,
    #[serde(rename = "sso")]
    SingleSignOn,
    #[serde(rename = "api_key")]
    ApiKey,
    /// IMDSv2, PodIdentity, env vars, etc.
    #[serde(rename = "default")]
    Automatic,
}
