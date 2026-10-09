use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

#[derive(
    Copy,
    Clone,
    Default,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum NotifyWhenAgentWaiting {
    #[default]
    PrimaryScreen,
    AllScreens,
    Never,
}

#[derive(
    Copy,
    Clone,
    Default,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum PlaySoundWhenAgentDone {
    #[default]
    Never,
    WhenHidden,
    Always,
}

impl PlaySoundWhenAgentDone {
    pub fn should_play(&self, visible: bool) -> bool {
        match self {
            PlaySoundWhenAgentDone::Never => false,
            PlaySoundWhenAgentDone::WhenHidden => !visible,
            PlaySoundWhenAgentDone::Always => true,
        }
    }
}
