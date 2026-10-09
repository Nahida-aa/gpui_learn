use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    #[serde(alias = "compact")]
    Compact,
    #[default]
    #[serde(alias = "default")]
    Default,
    #[serde(alias = "comfortable")]
    Comfortable,
}

impl UiDensity {
    pub fn spacing_ratio(self) -> f32 {
        match self {
            UiDensity::Compact => 0.75,
            UiDensity::Default => 1.0,
            UiDensity::Comfortable => 1.25,
        }
    }
}
