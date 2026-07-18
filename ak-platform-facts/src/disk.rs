use std::collections::HashMap;

use authentik_client::models::DiskRequest;
use eyre::Result;
use serde::Deserialize;
use sysinfo::Disks;

use crate::query::query_named;

fn disks_base() -> Vec<DiskRequest> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            DiskRequest {
                name: disk.name().to_string_lossy().to_string(),
                mountpoint: disk.mount_point().to_string_lossy().to_string(),
                label: None,
                capacity_total_bytes: Some(total as i64),
                capacity_used_bytes: Some(total.saturating_sub(available) as i64),
                encryption_enabled: None,
            }
        })
        .collect()
}

/// osquery's `disk_encryption`/`block_devices` tables key rows by a bare
/// device node (e.g. `disk1s1`), while `mounts.device` is `/dev/`-style
/// (e.g. `/dev/disk1s1`) — this bridges the two so they can be joined.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn normalize_device_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).to_lowercase()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Deserialize)]
struct MountRow {
    #[serde(default)]
    device: String,
    #[serde(default)]
    path: String,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Deserialize)]
struct DiskEncryptionRow {
    #[serde(default)]
    name: String,
    #[serde(default)]
    encrypted: String,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Deserialize)]
struct BlockDeviceRow {
    #[serde(default)]
    name: String,
    #[serde(default)]
    label: String,
}

/// Bridges sysinfo's disk listing (keyed by `mount_point()` — sysinfo's
/// `name()` is a Finder volume label on macOS, not a device node, so it
/// can't be matched against these tables directly) to osquery's
/// device-node-keyed `disk_encryption`/`block_devices` rows.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn apply_unix_encryption_and_label(disks: &mut [DiskRequest]) -> Result<()> {
    let device_by_mount: HashMap<String, String> = query_named::<MountRow>("mounts")?
        .into_iter()
        .filter(|row| !row.device.is_empty() && !row.path.is_empty())
        .map(|row| (row.path, row.device))
        .collect();

    let encrypted_by_device: HashMap<String, bool> =
        query_named::<DiskEncryptionRow>("disk_encryption")?
            .into_iter()
            .filter(|row| !row.name.is_empty())
            .map(|row| (normalize_device_name(&row.name), row.encrypted == "1"))
            .collect();

    let label_by_device: HashMap<String, String> = query_named::<BlockDeviceRow>("block_devices")?
        .into_iter()
        .filter(|row| !row.name.is_empty() && !row.label.is_empty())
        .map(|row| (normalize_device_name(&row.name), row.label))
        .collect();

    for disk in disks.iter_mut() {
        let Some(device) = device_by_mount.get(&disk.mountpoint) else {
            continue;
        };
        let key = normalize_device_name(device);
        disk.encryption_enabled = encrypted_by_device.get(&key).copied();
        disk.label = label_by_device.get(&key).cloned();
    }
    Ok(())
}

#[cfg(target_os = "windows")]
#[derive(Deserialize)]
struct BitlockerRow {
    #[serde(default)]
    drive_letter: String,
    #[serde(default)]
    protection_status: String,
    #[serde(default)]
    conversion_status: String,
}

/// `label` is left `None` here — no regression, it was always `None` on
/// Windows before this migration too (no osquery table exposes a Windows
/// volume label equivalent).
#[cfg(target_os = "windows")]
fn apply_windows_encryption(disks: &mut [DiskRequest]) -> Result<()> {
    let rows = query_named::<BitlockerRow>("bitlocker_info")?;
    for disk in disks.iter_mut() {
        let mountpoint = disk.mountpoint.trim_end_matches('\\');
        disk.encryption_enabled = rows
            .iter()
            .find(|row| {
                row.drive_letter
                    .trim_end_matches('\\')
                    .eq_ignore_ascii_case(mountpoint)
            })
            .map(|row| {
                // Relies on the documented Win32_EncryptableVolume-mirrored
                // INTEGER enums, not the unverified `encryption_method`
                // TEXT string (see query.rs).
                row.protection_status == "1" && row.conversion_status != "0"
            });
    }
    Ok(())
}

pub fn gather() -> Result<Vec<DiskRequest>> {
    let mut disks = disks_base();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    apply_unix_encryption_and_label(&mut disks)?;

    #[cfg(target_os = "windows")]
    apply_windows_encryption(&mut disks)?;

    Ok(disks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_succeeds_and_has_at_least_one_disk() {
        let disks = gather().unwrap();
        assert!(!disks.is_empty());
        assert!(
            disks
                .iter()
                .all(|d| !d.name.is_empty() && !d.mountpoint.is_empty())
        );
    }
}
