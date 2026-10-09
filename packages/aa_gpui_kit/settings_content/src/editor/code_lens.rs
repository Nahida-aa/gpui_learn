use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

/// Whether to display code lenses from language servers above code elements.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    MergeFrom,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeLens {
    /// Do not query and display code lenses.
    #[default]
    Off,
    /// Display code lenses from language servers above code elements.
    On,
    /// Display code lenses in the code action menu.
    Menu,
}

impl CodeLens {
    pub fn enabled(&self) -> bool { self != &Self::Off }

    pub fn inline(&self) -> bool { *self == Self::On }

    pub fn show_in_menu(&self) -> bool { *self == Self::Menu }
}
