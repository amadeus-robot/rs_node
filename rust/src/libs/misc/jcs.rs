use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub struct JCS;

impl JCS {
    /// Public: serialize a JSON object into canonical form
    pub fn serialize(map: &Value) -> String {
        let canonical = Self::serialize_value(map);
        serde_json::to_string(&canonical).unwrap()
    }

    /// Internal recursive canonicalization
    fn serialize_value(value: &Value) -> Value {
        match value {
            Value::Object(obj) => {
                let mut sorted = BTreeMap::new();
                for (k, v) in obj {
                    sorted.insert(k.clone(), Self::serialize_value(v));
                }
                let mut new_map = Map::new();
                for (k, v) in sorted {
                    new_map.insert(k, v);
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| Self::serialize_value(v)).collect())
            }
            _ => value.clone(),
        }
    }

    /// Validate a JSON string: returns Some(Value) if canonical, else None
    pub fn validate(binary: &str) -> Option<Value> {
        match serde_json::from_str::<Value>(binary) {
            Ok(value) => {
                let reserialized = Self::serialize(&value);
                if reserialized == binary {
                    Some(value)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
