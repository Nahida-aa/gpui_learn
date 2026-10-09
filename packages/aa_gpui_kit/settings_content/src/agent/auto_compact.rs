use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::borrow::Cow;

// ... AutoCompactThreshold + AutoCompactSettingsContent
/// Threshold at which agent auto-compaction runs. See
/// [`AutoCompactSettingsContent::threshold`] for the accepted formats.
///
/// The canonical textual form is stored verbatim so it can round-trip through
/// the settings UI; it is serialized back as a JSON string for percentages and
/// as a JSON integer for token counts.
#[derive(Clone, Debug, PartialEq, Eq, MergeFrom)]
pub struct AutoCompactThreshold(pub String);

impl From<String> for AutoCompactThreshold {
    fn from(value: String) -> Self { Self(value) }
}

impl From<AutoCompactThreshold> for String {
    fn from(value: AutoCompactThreshold) -> Self { value.0 }
}

impl AsRef<str> for AutoCompactThreshold {
    fn as_ref(&self) -> &str { &self.0 }
}
impl Serialize for AutoCompactThreshold {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.ends_with('%') {
            serializer.serialize_str(&self.0)
        } else if let Ok(tokens) = self.0.parse::<i64>() {
            serializer.serialize_i64(tokens)
        } else {
            serializer.serialize_str(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for AutoCompactThreshold {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ThresholdVisitor;

        impl serde::de::Visitor<'_> for ThresholdVisitor {
            type Value = AutoCompactThreshold;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a percentage string like \"90%\" or an integer number of tokens")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(AutoCompactThreshold(value.to_owned()))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(AutoCompactThreshold(value.to_string()))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(AutoCompactThreshold(value.to_string()))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Ok(AutoCompactThreshold(value.to_string()))
            }
        }

        deserializer.deserialize_any(ThresholdVisitor)
    }
}

impl JsonSchema for AutoCompactThreshold {
    fn schema_name() -> Cow<'static, str> { "AutoCompactThreshold".into() }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "oneOf": [
                {
                    "type": "string",
                    "pattern": "^\\d+(\\.\\d+)?%$"
                },
                {
                    "type": "integer"
                }
            ]
        })
    }
}

#[with_fallible_options]
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, Default)]
pub struct AutoCompactSettingsContent {
    /// Whether to automatically compact the agent's context when it grows too
    /// large, summarizing earlier messages to free up room in the model's
    /// context window.
    ///
    /// Default: true
    pub enabled: Option<bool>,
    /// The threshold at which auto-compaction runs. This is one of:
    ///
    /// - A percentage string ending in `%`, e.g. `"90%"`, measured against the
    ///   model's context window. `"90%"` compacts once the context is 90% full.
    /// - A positive integer: compaction runs once that many tokens have been
    ///   used. For example, `100000` compacts after 100,000 tokens are used.
    /// - A negative integer: compaction runs once that many tokens remain in
    ///   the context window. For example, `-20000` compacts once there are
    ///   fewer than 20,000 tokens of headroom left in the context window.
    ///
    /// `0` is not a valid threshold.
    ///
    /// Default: "90%"
    pub threshold: Option<AutoCompactThreshold>,
}
