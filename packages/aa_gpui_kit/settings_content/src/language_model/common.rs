use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

pub use language_model_core::ModelMode;
pub use language_model_core::ReasoningEffort as OpenAiReasoningEffort;

use crate::merge_from::MergeFrom as MergeFromTrait;

impl MergeFromTrait for ModelMode {
    fn merge_from(&mut self, other: &Self) { *self = *other; }
}

impl MergeFromTrait for OpenAiReasoningEffort {
    fn merge_from(&mut self, other: &Self) { *self = *other; }
}

pub(crate) fn default_true() -> bool { true }

/// Configuration for caching language model messages.
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct LanguageModelCacheConfiguration {
    pub max_cache_anchors: usize,
    pub should_speculate: bool,
    pub min_total_token: u64,
}
