//! 字体相关 settings 类型。
//!
//! 对齐 Zed `settings_content::theme` 中的字体部分。

use collections::IndexMap;
use std::{borrow::Cow, fmt::Display, sync::Arc};

use crate::merge_from::MergeFrom as _;
use crate::serialize_f32_with_two_decimal_places;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use settings_macros::{MergeFrom, with_fallible_options};
/// 字体大小（像素）。
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    JsonSchema,
    MergeFrom,
    derive_more::FromStr,
)]
#[serde(transparent)]
pub struct FontSize(#[serde(serialize_with = "serialize_f32_with_two_decimal_places")] pub f32);

impl Display for FontSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl From<f32> for FontSize {
    fn from(v: f32) -> Self { Self(v) }
}

impl From<FontSize> for f32 {
    fn from(v: FontSize) -> Self { v.0 }
}

/// 字体族名称（包 `Arc<str>`）。
#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, MergeFrom, PartialEq, Eq)]
#[serde(transparent)]
pub struct FontFamilyName(pub Arc<str>);

impl AsRef<str> for FontFamilyName {
    fn as_ref(&self) -> &str { &self.0 }
}

impl From<String> for FontFamilyName {
    fn from(s: String) -> Self { Self(Arc::from(s)) }
}
impl From<FontFamilyName> for String {
    fn from(value: FontFamilyName) -> Self { value.0.to_string() }
}
impl From<&str> for FontFamilyName {
    fn from(s: &str) -> Self { Self(Arc::from(s)) }
}

// ---------- FontWeightContent ----------

/// 字体粗细（CSS 单位 100-900）。
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    MergeFrom,
    derive_more::FromStr,
)]
#[serde(transparent)]
pub struct FontWeightContent(pub f32);
impl Display for FontWeightContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl From<f32> for FontWeightContent {
    fn from(weight: f32) -> Self { FontWeightContent(weight) }
}

impl Default for FontWeightContent {
    fn default() -> Self { Self::NORMAL }
}
impl FontWeightContent {
    pub const THIN: Self = Self(100.0);
    pub const EXTRA_LIGHT: Self = Self(200.0);
    pub const LIGHT: Self = Self(300.0);
    pub const NORMAL: Self = Self(400.0);
    pub const MEDIUM: Self = Self(500.0);
    pub const SEMIBOLD: Self = Self(600.0);
    pub const BOLD: Self = Self(700.0);
    pub const EXTRA_BOLD: Self = Self(800.0);
    pub const BLACK: Self = Self(900.0);
}
impl schemars::JsonSchema for FontWeightContent {
    fn schema_name() -> std::borrow::Cow<'static, str> { "FontWeightContent".into() }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;
        json_schema!({
            "type": "number",
            "minimum": Self::THIN.0,
            "maximum": Self::BLACK.0,
            "default": Self::NORMAL.0,
            "description": "Font weight value between 100 (thin) and 900 (black)"
        })
    }
}
/// OpenType font features as a map of feature tag to value.
/// This is a content type that mirrors `gpui::FontFeatures` but without the Arc wrapper.
/// Values can be specified as booleans (true=1, false=0) or integers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, MergeFrom)]
#[serde(transparent)]
pub struct FontFeaturesContent(pub IndexMap<String, u32>);

impl FontFeaturesContent {
    pub fn new() -> Self { Self(IndexMap::default()) }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum FeatureValue {
    Bool(bool),
    Number(serde_json::Number),
}

fn is_valid_feature_tag(tag: &str) -> bool {
    tag.len() == 4 && tag.chars().all(|c| c.is_ascii_alphanumeric())
}

impl<'de> Deserialize<'de> for FontFeaturesContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct FontFeaturesVisitor;

        impl<'de> Visitor<'de> for FontFeaturesVisitor {
            type Value = FontFeaturesContent;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of font features")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut feature_map = IndexMap::default();

                while let Some((key, value)) =
                    access.next_entry::<String, Option<FeatureValue>>()?
                {
                    if !is_valid_feature_tag(&key) {
                        log::error!("Incorrect font feature tag: {}", key);
                        continue;
                    }
                    if let Some(value) = value {
                        match value {
                            FeatureValue::Bool(enable) => {
                                feature_map.insert(key, if enable { 1 } else { 0 });
                            }
                            FeatureValue::Number(value) => {
                                if value.is_u64() {
                                    feature_map.insert(key, value.as_u64().unwrap() as u32);
                                } else {
                                    log::error!(
                                        "Incorrect font feature value {} for feature tag {}",
                                        value,
                                        key
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }

                Ok(FontFeaturesContent(feature_map))
            }
        }

        deserializer.deserialize_map(FontFeaturesVisitor)
    }
}

impl JsonSchema for FontFeaturesContent {
    fn schema_name() -> Cow<'static, str> { "FontFeaturesContent".into() }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;
        json_schema!({
            "type": "object",
            "patternProperties": {
                "[0-9a-zA-Z]{4}$": {
                    "type": ["boolean", "integer"],
                    "minimum": 0,
                    "multipleOf": 1
                }
            },
            "additionalProperties": false
        })
    }
}

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
pub struct CodeFade(#[serde(serialize_with = "serialize_f32_with_two_decimal_places")] pub f32);

impl Display for CodeFade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl From<f32> for CodeFade {
    fn from(x: f32) -> Self { Self(x) }
}

/// 字体样式（serif / italic / oblique）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(rename_all = "snake_case")]
pub enum FontStyleContent {
    Normal,
    Italic,
    Oblique,
}
