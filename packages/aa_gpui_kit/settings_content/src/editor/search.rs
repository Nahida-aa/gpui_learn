use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

/// Default options for buffer and project search items.
#[with_fallible_options]
#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Eq)]
pub struct SearchSettingsContent {
    /// Whether to show the project search button in the status bar.
    pub button: Option<bool>,
    /// Whether to only match on whole words.
    pub whole_word: Option<bool>,
    /// Whether to match case sensitively.
    pub case_sensitive: Option<bool>,
    /// Whether to include gitignored files in search results.
    pub include_ignored: Option<bool>,
    /// Whether to interpret the search query as a regular expression.
    pub regex: Option<bool>,
    /// Whether to center the cursor on each search match when navigating.
    pub center_on_match: Option<bool>,
    /// Start searching as you type in project search, without pressing Enter.
    pub search_on_type: Option<bool>,
}
