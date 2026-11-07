//! Deserializes admin-enforced policy from the platform's management mechanism: forced
//! (MDM) managed preferences on macOS, a `HKEY_LOCAL_MACHINE` key on Windows and a
//! root-owned JSON file elsewhere.
//!
//! The registry has no booleans, so numbers deserialize as booleans. On macOS only the
//! target's top-level fields are looked up, so it can't use `#[serde(flatten)]`.
use eyre::Result;
use serde::de::value::{MapDeserializer, SeqDeserializer};
use serde::de::{self, DeserializeOwned, IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer, forward_to_deserialize_any};
use serde_json::Value;

#[cfg_attr(any(target_os = "macos", windows), allow(dead_code))]
mod file;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub struct PolicySource<'a> {
    /// Managed preferences domain, e.g. `io.goauthentik.platform.agent`.
    pub macos_domain: &'a str,
    /// Key below `HKEY_LOCAL_MACHINE`, e.g. `SOFTWARE\Policies\Vendor\Product`.
    pub windows_key: &'a str,
    pub file_path: &'a str,
}

/// `Ok(None)` when no policy is set.
pub fn load<T: DeserializeOwned>(source: &PolicySource) -> Result<Option<T>> {
    #[cfg(target_os = "macos")]
    let value = macos::read(source.macos_domain, struct_fields::<T>())?;
    #[cfg(windows)]
    let value = windows::read(source.windows_key)?;
    #[cfg(not(any(target_os = "macos", windows)))]
    let value = file::read(source.file_path)?;
    Ok(value.map(|v| T::deserialize(Lenient(v))).transpose()?)
}

/// Field names of struct `T`, empty if `T` isn't a plain struct.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn struct_fields<'de, T: Deserialize<'de>>() -> &'static [&'static str] {
    struct Fields<'a>(&'a mut &'static [&'static str]);

    impl<'de> Deserializer<'de> for Fields<'_> {
        type Error = de::value::Error;

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error> {
            *self.0 = fields;
            Err(de::Error::custom("fields collected"))
        }

        fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
            Err(de::Error::custom("not a struct"))
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
            byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map enum
            identifier ignored_any
        }
    }

    let mut fields: &'static [&'static str] = &[];
    let _ = T::deserialize(Fields(&mut fields));
    fields
}

/// A [`Value`] deserializer that also accepts numbers for booleans.
struct Lenient(Value);

impl<'de> IntoDeserializer<'de, serde_json::Error> for Lenient {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> Deserializer<'de> for Lenient {
    type Error = serde_json::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Array(a) => visitor.visit_seq(SeqDeserializer::new(a.into_iter().map(Lenient))),
            Value::Object(m) => visitor.visit_map(MapDeserializer::new(
                m.into_iter().map(|(k, v)| (k, Lenient(v))),
            )),
            other => other.deserialize_any(visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Number(n) => visitor.visit_bool(n.as_f64() != Some(0.0)),
            other => other.deserialize_bool(visitor),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            other => visitor.visit_some(Lenient(other)),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_enum(name, variants, visitor)
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit
        unit_struct newtype_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default)]
    pub(crate) struct Policy {
        pub(crate) enabled: Option<bool>,
        pub(crate) timeout: Option<u64>,
        pub(crate) hosts: Vec<String>,
        pub(crate) nested: Nested,
    }

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default)]
    pub(crate) struct Nested {
        pub(crate) allow: Option<bool>,
    }

    /// What every platform test loads: `enabled` and `nested.allow` given as numbers.
    pub(crate) fn expected() -> Policy {
        Policy {
            enabled: Some(true),
            timeout: Some(30),
            hosts: vec!["github.com".into()],
            nested: Nested { allow: Some(false) },
        }
    }

    #[test]
    fn collects_struct_fields() {
        assert_eq!(
            struct_fields::<Policy>(),
            ["enabled", "timeout", "hosts", "nested"]
        );
        assert!(struct_fields::<u32>().is_empty());
    }

    #[test]
    fn numbers_deserialize_as_booleans() {
        let v = json!({ "enabled": 1, "timeout": 30, "hosts": ["github.com"], "nested": { "allow": 0 } });
        assert_eq!(Policy::deserialize(Lenient(v)).unwrap(), expected());
    }

    #[test]
    fn plain_json_still_works() {
        let p = Policy::deserialize(Lenient(json!({ "enabled": false, "nested": {} }))).unwrap();
        assert_eq!(p.enabled, Some(false));
        assert_eq!(p.timeout, None);
        assert!(Policy::deserialize(Lenient(json!({ "timeout": "x" }))).is_err());
    }
}
