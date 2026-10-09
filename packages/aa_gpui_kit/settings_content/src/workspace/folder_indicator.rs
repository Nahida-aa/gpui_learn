use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum FolderIndicator {
    /// Show a folder icon.
    #[default]
    Icon,
    /// Show a disclosure chevron.
    Chevron,
    /// Show a disclosure chevron followed by a folder icon.
    Both,
}

impl FolderIndicator {
    pub fn shows_chevron(self) -> bool { matches!(self, Self::Chevron | Self::Both) }

    pub fn shows_icon(self) -> bool { matches!(self, Self::Icon | Self::Both) }
}
