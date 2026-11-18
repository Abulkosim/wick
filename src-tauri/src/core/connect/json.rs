//! JSON that keeps keys in file order, duplicates included, so rewriting an agent's settings
//! changes only what Wick changed. serde_json's `preserve_order` would do it for the whole app.

use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Number;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Json>),
    Object(Object),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object(Vec<(String, Json)>);

impl Object {
    /// The last value for `key`, which is the one JavaScript and serde_json read.
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.0
            .iter()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        self.0
            .iter_mut()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// Replaces the value in place, or appends the key.
    pub fn insert(&mut self, key: &str, value: Json) {
        match self.get_mut(key) {
            Some(slot) => *slot = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.0.retain(|(name, _)| name != key);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Json)> {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?.get(key)
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Json::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Json::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Number(number) => number.as_i64(),
            _ => None,
        }
    }
}

impl From<&str> for Json {
    fn from(text: &str) -> Self {
        Json::String(text.to_string())
    }
}

impl From<i64> for Json {
    fn from(number: i64) -> Self {
        Json::Number(number.into())
    }
}

impl Serialize for Json {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Json::Null => serializer.serialize_unit(),
            Json::Bool(value) => serializer.serialize_bool(*value),
            Json::Number(number) => number.serialize(serializer),
            Json::String(text) => serializer.serialize_str(text),
            Json::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Json::Object(object) => {
                let mut map = serializer.serialize_map(Some(object.0.len()))?;
                for (key, value) in &object.0 {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_none<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Json, E> {
        Ok(Json::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
        Ok(Json::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Json, E> {
        Number::from_f64(value)
            .map(Json::Number)
            .ok_or_else(|| E::custom("number out of range"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Json, E> {
        Ok(Json::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Json, E> {
        Ok(Json::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(Json::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry()? {
            entries.push(entry);
        }
        Ok(Json::Object(Object(entries)))
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

/// Two-space indent and a trailing newline, like Claude Code and Codex write.
pub fn pretty(value: &Json) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
    text.push('\n');
    text
}

pub fn object(entries: Vec<(&str, Json)>) -> Json {
    Json::Object(Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_order_duplicates_numbers_and_text() {
        let text = r#"{"z":1,"a":{"y":[1,-2,3.5,1e300,true,null],"b":"Wïck \"q\" \u001f"},"z":2}"#;
        let value: Json = serde_json::from_str(text).unwrap();
        let compact = serde_json::to_string(&value).unwrap();
        assert_eq!(
            compact,
            r#"{"z":1,"a":{"y":[1,-2,3.5,1e+300,true,null],"b":"Wïck \"q\" \u001f"},"z":2}"#
        );
        assert_eq!(value.get("z"), Some(&Json::from(2)));
        assert!(pretty(&value).ends_with("}\n"));
        assert!(pretty(&value).starts_with("{\n  \"z\": 1,\n"));
    }

    #[test]
    fn insert_keeps_position_and_remove_drops_every_copy() {
        let mut value = object(vec![
            ("b", Json::Null),
            ("a", Json::Null),
            ("b", Json::Null),
        ]);
        let object = value.as_object_mut().unwrap();
        object.insert("a", "x".into());
        object.insert("c", "y".into());
        object.remove("b");
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"a":"x","c":"y"}"#
        );
    }
}
