use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::propertylist::CFPropertyList;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::preferences::{CFPreferencesAppValueIsForced, CFPreferencesCopyAppValue};
use eyre::Result;
use serde_json::{Map, Number, Value};

/// Only forced values come from a configuration profile; anything else the user can set.
pub fn read(domain: &str, fields: &[&str]) -> Result<Option<Value>> {
    read_values(domain, fields, true)
}

/// Without `forced_only`, also returns values from `defaults write`, for tests.
fn read_values(domain: &str, fields: &[&str], forced_only: bool) -> Result<Option<Value>> {
    let domain = CFString::new(domain);
    let mut map = Map::new();
    for field in fields {
        let key = CFString::new(field);
        // SAFETY: valid CFStrings; Copy* returns a +1 reference the property list takes over.
        let value = unsafe {
            if forced_only
                && CFPreferencesAppValueIsForced(
                    key.as_concrete_TypeRef(),
                    domain.as_concrete_TypeRef(),
                ) == 0
            {
                continue;
            }
            let value =
                CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), domain.as_concrete_TypeRef());
            if value.is_null() {
                continue;
            }
            CFPropertyList::wrap_under_create_rule(value)
        };
        if let Some(v) = to_json(&value) {
            map.insert((*field).to_owned(), v);
        }
    }
    Ok((!map.is_empty()).then_some(Value::Object(map)))
}

/// Dates and data have no JSON equivalent and are dropped.
fn to_json(p: &CFPropertyList) -> Option<Value> {
    if let Some(b) = p.downcast::<CFBoolean>() {
        return Some(Value::Bool(b.into()));
    }
    if let Some(n) = p.downcast::<CFNumber>() {
        return match n.to_i64() {
            Some(i) => Some(i.into()),
            None => n.to_f64().and_then(Number::from_f64).map(Value::Number),
        };
    }
    if let Some(s) = p.downcast::<CFString>() {
        return Some(Value::String(s.to_string()));
    }
    if let Some(a) = p.downcast::<CFArray>() {
        return Some(Value::Array(
            a.get_all_values()
                .into_iter()
                // SAFETY: items are property lists retained by the array.
                .filter_map(|item| to_json(&unsafe { CFPropertyList::wrap_under_get_rule(item) }))
                .collect(),
        ));
    }
    if let Some(d) = p.downcast::<CFDictionary>() {
        let (keys, values) = d.get_keys_and_values();
        let map = keys
            .into_iter()
            .zip(values)
            .filter_map(|(k, v)| {
                // SAFETY: plist keys are CFStrings, values property lists, both retained by
                // the dictionary.
                let (k, v) = unsafe {
                    (
                        CFString::wrap_under_get_rule(k as CFStringRef),
                        CFPropertyList::wrap_under_get_rule(v),
                    )
                };
                Some((k.to_string(), to_json(&v)?))
            })
            .collect();
        return Some(Value::Object(map));
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::process::Command;

    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::tests::{Policy, expected};
    use crate::{Lenient, struct_fields};

    /// Deleted again on drop.
    struct TestDomain(String);

    impl TestDomain {
        fn new(name: &str) -> Self {
            TestDomain(format!(
                "io.goauthentik.platform.test-serde-policy-{name}-{}",
                std::process::id()
            ))
        }

        fn write(&self, key: &str, args: &[&str]) -> &Self {
            let status = Command::new("defaults")
                .args(["write", &self.0, key])
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "defaults write {key} failed");
            self
        }
    }

    impl Drop for TestDomain {
        fn drop(&mut self) {
            let _ = Command::new("defaults").args(["delete", &self.0]).output();
            // cfprefsd leaves an empty plist behind.
            if let Some(home) = std::env::home_dir() {
                let _ = std::fs::remove_file(
                    home.join(format!("Library/Preferences/{}.plist", self.0)),
                );
            }
        }
    }

    #[test]
    fn converts_property_lists() {
        let dict = CFDictionary::from_CFType_pairs(&[
            (CFString::new("on"), CFBoolean::true_value().as_CFType()),
            (CFString::new("n"), CFNumber::from(30i64).as_CFType()),
            (
                CFString::new("list"),
                CFArray::from_CFTypes(&[CFString::new("a").as_CFType()]).as_CFType(),
            ),
        ]);
        let plist = unsafe { CFPropertyList::wrap_under_get_rule(dict.as_CFTypeRef()) };
        assert_eq!(
            to_json(&plist).unwrap(),
            json!({ "on": true, "n": 30, "list": ["a"] })
        );
    }

    #[test]
    fn missing_domain_is_no_policy() {
        let d = TestDomain::new("missing");
        assert!(read_values(&d.0, &["a"], false).unwrap().is_none());
        assert!(read(&d.0, &["a"]).unwrap().is_none());
    }

    #[test]
    fn reads_values_written_by_defaults() {
        let d = TestDomain::new("types");
        d.write("enabled", &["-bool", "true"])
            .write("timeout", &["-int", "30"])
            .write("ratio", &["-float", "1.5"])
            .write("name", &["-string", "value"])
            .write("hosts", &["-array", "a", "b"])
            .write(
                "nested",
                &["<dict><key>allow</key><false/><key>port</key><integer>22</integer></dict>"],
            )
            .write("not_a_field", &["-string", "ignored"]);

        let fields = [
            "enabled", "timeout", "ratio", "name", "hosts", "nested", "unset",
        ];
        assert_eq!(
            read_values(&d.0, &fields, false).unwrap(),
            Some(json!({
                "enabled": true,
                "timeout": 30,
                "ratio": 1.5,
                "name": "value",
                "hosts": ["a", "b"],
                "nested": { "allow": false, "port": 22 },
            }))
        );
    }

    #[test]
    fn read_ignores_values_written_by_defaults() {
        let d = TestDomain::new("unforced");
        d.write("enabled", &["-bool", "true"]);
        assert!(read_values(&d.0, &["enabled"], false).unwrap().is_some());
        assert!(read(&d.0, &["enabled"]).unwrap().is_none());
    }

    #[test]
    fn deserializes_values_written_by_defaults() {
        let d = TestDomain::new("deserialize");
        d.write("enabled", &["-int", "1"])
            .write("timeout", &["-int", "30"])
            .write("hosts", &["-array", "github.com"])
            .write(
                "nested",
                &["<dict><key>allow</key><integer>0</integer></dict>"],
            );

        let value = read_values(&d.0, struct_fields::<Policy>(), false)
            .unwrap()
            .unwrap();
        assert_eq!(Policy::deserialize(Lenient(value)).unwrap(), expected());
    }
}
