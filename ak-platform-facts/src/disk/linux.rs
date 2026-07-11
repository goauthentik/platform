use eyre::Result;

/// Only device-mapper devices (`dm-*`, used by LVM/LUKS) can be encrypted.
pub fn encryption_enabled(name: &str, _mountpoint: &str) -> Result<bool> {
    let basename = name.rsplit('/').next().unwrap_or(name);
    if !basename.starts_with("dm-") {
        return Ok(false);
    }
    let dm_name = std::fs::read_to_string(format!("/sys/block/{basename}/dm/name")).unwrap_or_default();
    Ok(dm_name.to_lowercase().contains("crypt"))
}
