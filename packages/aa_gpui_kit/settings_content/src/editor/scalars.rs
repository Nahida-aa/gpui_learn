use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::MergeFrom;

use crate::serialize_f32_with_two_decimal_places;

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    PartialOrd,
    derive_more::FromStr,
)]
#[serde(transparent)]
pub struct MinimumContrast(
    #[serde(serialize_with = "crate::serialize_f32_with_two_decimal_places")] pub f32,
);

impl Display for MinimumContrast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}
impl From<f32> for MinimumContrast {
    fn from(x: f32) -> Self { Self(x) }
}

// InactiveOpacity 同理
/// Opacity of the inactive panes. 0 means transparent, 1 means opaque.
///
/// Valid range: 0.0 to 1.0
/// Default: 1.0
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    PartialEq,
    PartialOrd,
    derive_more::FromStr,
)]
#[serde(transparent)]
pub struct InactiveOpacity(
    #[serde(serialize_with = "serialize_f32_with_two_decimal_places")] pub f32,
);

impl Display for InactiveOpacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}

impl From<f32> for InactiveOpacity {
    fn from(x: f32) -> Self { Self(x) }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    MergeFrom,
    PartialEq,
    PartialOrd,
    derive_more::FromStr,
)]
#[serde(transparent)]
pub struct CenteredPaddingSettings(
    #[serde(serialize_with = "crate::serialize_f32_with_two_decimal_places")] pub f32,
);

impl CenteredPaddingSettings {
    pub const MIN_PADDING: f32 = 0.0;
    // This is an f64 so serde_json can give a type hint without random numbers in the back
    pub const DEFAULT_PADDING: f64 = 0.2;
    pub const MAX_PADDING: f32 = 0.4;
}
impl Display for CenteredPaddingSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl From<f32> for CenteredPaddingSettings {
    fn from(x: f32) -> Self { Self(x) }
}

impl Default for CenteredPaddingSettings {
    fn default() -> Self { Self(Self::DEFAULT_PADDING as f32) }
}

impl schemars::JsonSchema for CenteredPaddingSettings {
    fn schema_name() -> std::borrow::Cow<'static, str> { "CenteredPaddingSettings".into() }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;
        json_schema!({
            "type": "number",
            "minimum": Self::MIN_PADDING,
            "maximum": Self::MAX_PADDING,
            "default": Self::DEFAULT_PADDING,
            "description": "Centered layout related setting (left/right)."
        })
    }
}
