//! 容错反序列化。
//!
//! 对齐 Zed `crates/settings_content/src/fallible_options.rs`，但做了大幅简化：
//!
//! Zed 版依赖 `thread_local!` + `anyhow` + `serde_json_lenient` + `serde_path_to_error`
//! 收集所有解析错误，最终返回 `(Option<T>, ParseStatus)`。
//!
//! **我们先只做核心容错** —— 单个字段反序列化失败时降级成 `Default::default()`（Option 的 default 就是 None）。
//! 以后 settings-store 升级到 Zed 风格时再改实现。
//!
//! 用法：配合 `#[with_fallible_options]` attribute，macro 自动给每个 Option<T> 字段加：
//! ```ignore
//! #[serde(default, skip_serializing_if = "Option::is_none",
//!           deserialize_with = "crate::fallible_options::deserialize")]
//! ```

use std::cell::RefCell;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::common::ParseStatus;

thread_local! {
    static ERRORS: RefCell<Option<Vec<anyhow::Error>>> = const { RefCell::new(None) };
}
pub fn parse_json<'de, T>(json: &'de str) -> (Option<T>, ParseStatus)
where
    T: Deserialize<'de>,
{
    ERRORS.with_borrow_mut(|errors| {
        errors.replace(Vec::default());
    });

    let mut deserializer = serde_json_lenient::Deserializer::from_str(json);
    let value = serde_path_to_error::deserialize::<_, T>(&mut deserializer);
    let value = match value {
        Ok(value) => value,
        Err(error) => {
            return (
                None,
                ParseStatus::Failed {
                    error: error.into_inner().to_string(),
                },
            );
        }
    };

    if let Some(errors) = ERRORS.with_borrow_mut(|errors| errors.take().filter(|e| !e.is_empty())) {
        let error = errors
            .into_iter()
            .map(|e| e.to_string())
            .flat_map(|e| ["\n".to_owned(), e])
            .skip(1)
            .collect::<String>();
        return (Some(value), ParseStatus::Failed { error });
    }

    (Some(value), ParseStatus::Success)
}

/// 容错反序列化器 —— 解析失败时返回 Default::default()。
///
/// 用作 serde 的 `deserialize_with` 值：
/// ```ignore
/// #[serde(deserialize_with = "crate::fallible_options::deserialize")]
/// field: Option<u32>,
/// ```
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + FallibleOption,
{
    match T::deserialize(deserializer) {
        Ok(value) => Ok(value),
        // 解析出错（比如字符串给了数字字段），降级为 None
        Err(_) => Ok(T::default()),
    }
}

/// 标记可容错的类型（现在只有 Option<T>）。
pub trait FallibleOption: Default {}
impl<T> FallibleOption for Option<T> {}

macro_rules! flattened_deserialize {
    ($type_name:ty {
        sections: { $($section:ident),* $(,)? },
        options: { $($option_field:ident),* $(,)? },
        defaults: { $($default_field:ident),* $(,)? } $(,)?
    }) => {
        impl $type_name {
            #[doc(hidden)]
            pub const NAMED_DESERIALIZE_KEYS: &'static [&'static str] = &[
                $(stringify!($option_field),)*
                $(stringify!($default_field),)*
            ];
        }

        impl<'de> serde::Deserialize<'de> for $type_name {
            fn deserialize<D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Self, D::Error> {
                let mut object =
                    serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
                (|| -> Result<Self, serde_json::Error> {
                    $(
                        let $option_field =
                            $crate::fallible_options::take_option_field(
                                &mut object,
                                stringify!($option_field),
                            )?;
                    )*
                    $(
                        let $default_field =
                            $crate::fallible_options::take_default_field(
                                &mut object,
                                stringify!($default_field),
                            )?;
                    )*
                    let rest = serde_json::Value::Object(object);
                    Ok(Self {
                        $($section: $crate::fallible_options::section(&rest)?,)*
                        $($option_field,)*
                        $($default_field,)*
                    })
                })()
                .map_err(serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use flattened_deserialize;

pub(crate) fn take_option_field<T>(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned + FallibleOption,
{
    match object.remove(key) {
        None => Ok(T::default()),
        Some(value) => deserialize(&value),
    }
}

pub(crate) fn take_default_field<T>(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned + Default,
{
    match object.remove(key) {
        None => Ok(T::default()),
        Some(value) => T::deserialize(&value),
    }
}

pub(crate) fn section<T>(rest: &serde_json::Value) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(rest)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use crate::fallible_options::deserialize as fallible_deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "fallible_deserialize"
        )]
        foo: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "fallible_deserialize"
        )]
        bar: Option<u32>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "fallible_deserialize"
        )]
        baz: Option<bool>,
    }

    use crate::merge_from::MergeFrom;

    #[test]
    fn fallible_option_serde_value() {
        let t: Test = serde_json::from_value(json!({
            "foo": "bar",
            "bar": "not-a-number",
            "baz": 3,
        }))
        .unwrap();
        assert_eq!(
            t,
            Test {
                foo: Some("bar".into()),
                bar: None,
                baz: None,
            }
        );
    }

    // MergeFrom impl for Test...
    impl MergeFrom for Test {
        fn merge_from(&mut self, other: &Self) {
            self.foo.merge_from(&other.foo);
            self.bar.merge_from(&other.bar);
            self.baz.merge_from(&other.baz);
        }
    }

    impl Clone for Test {
        fn clone(&self) -> Self {
            Test {
                foo: self.foo.clone(),
                bar: self.bar.clone(),
                baz: self.baz.clone(),
            }
        }
    }
}
