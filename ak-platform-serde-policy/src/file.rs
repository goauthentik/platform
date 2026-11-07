use std::fs::File;
use std::io::ErrorKind;

use eyre::Result;
use serde_json::Value;

pub fn read(path: &str) -> Result<Option<Value>> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;

        let meta = f.metadata()?;
        if meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            eyre::bail!(
                "policy file {path} must be owned by root and not writable by others (is {:o}/{})",
                meta.mode() & 0o777,
                meta.uid(),
            );
        }
    }
    Ok(Some(serde_json::from_reader(f)?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_no_policy() {
        assert!(read("/nonexistent/policy.json").unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_file_not_owned_by_root() {
        let f = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(f.path(), "{}").unwrap();
        let err = read(f.path().to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("owned by root"));
    }
}
