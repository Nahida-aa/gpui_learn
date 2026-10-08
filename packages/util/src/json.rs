pub fn merge_json_lenient_value_into(
    source: serde_json_lenient::Value,
    target: &mut serde_json_lenient::Value,
) {
    match (source, target) {
        (serde_json_lenient::Value::Object(source), serde_json_lenient::Value::Object(target)) => {
            for (key, value) in source {
                if let Some(target) = target.get_mut(&key) {
                    merge_json_lenient_value_into(value, target);
                } else {
                    target.insert(key, value);
                }
            }
        }

        (serde_json_lenient::Value::Array(source), serde_json_lenient::Value::Array(target)) => {
            for value in source {
                target.push(value);
            }
        }

        (source, target) => *target = source,
    }
}

/// Merges `source` into `target`: objects are merged recursively, key by key;
/// any other colliding value in `target` — including arrays — is replaced by
/// `source`'s value wholesale.
pub fn merge_json_value_into(source: serde_json::Value, target: &mut serde_json::Value) {
    use serde_json::Value;

    match (source, target) {
        (Value::Object(source), Value::Object(target)) => {
            for (key, value) in source {
                if let Some(target) = target.get_mut(&key) {
                    merge_json_value_into(value, target);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (source, target) => *target = source,
    }
}

pub fn union_json_value_into(source: serde_json::Value, target: &mut serde_json::Value) {
    use serde_json::Value;

    match (source, target) {
        (Value::Object(source), Value::Object(target)) => {
            for (key, value) in source {
                if let Some(target) = target.get_mut(&key) {
                    union_json_value_into(value, target);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (Value::Array(source), Value::Array(target)) => {
            for value in source {
                if !target.contains(&value) {
                    target.push(value);
                }
            }
        }
        (source, target) => *target = source,
    }
}

pub fn merge_non_null_json_value_into(source: serde_json::Value, target: &mut serde_json::Value) {
    use serde_json::Value;
    if let Value::Object(source_object) = source {
        let target_object = if let Value::Object(target) = target {
            target
        } else {
            *target = Value::Object(Default::default());
            target.as_object_mut().unwrap()
        };
        for (key, value) in source_object {
            if let Some(target) = target_object.get_mut(&key) {
                merge_non_null_json_value_into(value, target);
            } else if !value.is_null() {
                target_object.insert(key, value);
            }
        }
    } else if !source.is_null() {
        *target = source
    }
}
