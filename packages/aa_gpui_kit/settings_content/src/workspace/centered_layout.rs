use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::CenteredPaddingSettings;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
#[with_fallible_options]
pub struct CenteredLayoutSettings {
    /// The relative width of the left padding of the central pane from the
    /// workspace when the centered layout is used.
    ///
    /// Default: 0.2
    pub left_padding: Option<CenteredPaddingSettings>,
    // The relative width of the right padding of the central pane from the
    // workspace when the centered layout is used.
    ///
    /// Default: 0.2
    pub right_padding: Option<CenteredPaddingSettings>,
}
