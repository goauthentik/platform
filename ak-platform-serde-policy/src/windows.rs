use std::io::ErrorKind;

use eyre::Result;
use serde_json::{Map, Value};
use winreg::RegKey;
use winreg::enums::{
    HKEY_LOCAL_MACHINE, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_QWORD, REG_SZ,
};
use winreg::types::FromRegValue;

pub fn read(key: &str) -> Result<Option<Value>> {
    match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(key) {
        Ok(k) => Ok(Some(to_json(&k)?)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Subkeys become objects; value types without a JSON equivalent are skipped.
fn to_json(key: &RegKey) -> Result<Value> {
    let mut map = Map::new();
    for value in key.enum_values() {
        let (name, value) = value?;
        let json = match value.vtype {
            REG_SZ | REG_EXPAND_SZ => Value::String(String::from_reg_value(&value)?),
            REG_MULTI_SZ => Value::from(Vec::<String>::from_reg_value(&value)?),
            REG_DWORD => Value::from(u32::from_reg_value(&value)?),
            REG_QWORD => Value::from(u64::from_reg_value(&value)?),
            _ => continue,
        };
        map.insert(name, json);
    }
    for name in key.enum_keys() {
        let name = name?;
        map.insert(name.clone(), to_json(&key.open_subkey(&name)?)?);
    }
    Ok(Value::Object(map))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::borrow::Cow;

    use serde_json::json;
    use winreg::RegValue;
    use winreg::enums::REG_BINARY;

    use super::*;
    use crate::tests::{Policy, expected};
    use crate::{PolicySource, load};

    const TEST_ROOT: &str = r"SOFTWARE\authentik Security Inc.\Platform-testing";

    /// Writing below `HKEY_LOCAL_MACHINE` needs admin rights, which CI has. Deleted on drop.
    struct TestKey {
        path: String,
        key: RegKey,
    }

    impl TestKey {
        fn new(name: &str) -> Self {
            let path = format!(r"{TEST_ROOT}\serde-policy-{name}-{}", std::process::id());
            let (key, _) = RegKey::predef(HKEY_LOCAL_MACHINE)
                .create_subkey(&path)
                .unwrap();
            TestKey { path, key }
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_LOCAL_MACHINE).delete_subkey_all(&self.path);
        }
    }

    #[test]
    fn missing_key_is_no_policy() {
        assert!(
            read(&format!(r"{TEST_ROOT}\does-not-exist"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn empty_key_is_empty_object() {
        let t = TestKey::new("empty");
        assert_eq!(read(&t.path).unwrap(), Some(json!({})));
    }

    #[test]
    fn converts_values_and_subkeys() {
        let t = TestKey::new("convert");
        t.key.set_value("string", &"value").unwrap();
        t.key.set_value("dword", &1u32).unwrap();
        t.key
            .set_value("qword", &(u64::from(u32::MAX) + 1))
            .unwrap();
        t.key
            .set_value("list", &vec!["a".to_string(), "b".to_string()])
            .unwrap();
        t.key
            .set_raw_value(
                "binary",
                &RegValue {
                    bytes: Cow::Borrowed(&[1, 2, 3]),
                    vtype: REG_BINARY,
                },
            )
            .unwrap();
        let (nested, _) = t.key.create_subkey(r"outer\inner").unwrap();
        nested.set_value("deny", &0u32).unwrap();

        assert_eq!(
            read(&t.path).unwrap(),
            Some(json!({
                "string": "value",
                "dword": 1,
                "qword": u64::from(u32::MAX) + 1,
                "list": ["a", "b"],
                "outer": { "inner": { "deny": 0 } },
            }))
        );
    }

    #[test]
    fn loads_policy_with_dwords_as_booleans() {
        let t = TestKey::new("deserialize");
        t.key.set_value("enabled", &1u32).unwrap();
        t.key.set_value("timeout", &30u32).unwrap();
        t.key
            .set_value("hosts", &vec!["github.com".to_string()])
            .unwrap();
        let (nested, _) = t.key.create_subkey("nested").unwrap();
        nested.set_value("allow", &0u32).unwrap();

        let source = PolicySource {
            macos_domain: "",
            windows_key: &t.path,
            file_path: "",
        };
        assert_eq!(load::<Policy>(&source).unwrap(), Some(expected()));
    }
}
