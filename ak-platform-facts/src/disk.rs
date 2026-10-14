#[cfg(any(target_os = "linux", target_os = "macos"))]
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
            let mountpoint = disk.mount_point().to_string_lossy().to_string();
            let mut name = disk.name().to_string_lossy().to_string();
            if name.is_empty() {
                name = mountpoint.clone();
            }
            DiskRequest {
                name,
                mountpoint,
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
    #[serde(rename = "type", default)]
    fs_type: String,
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
    let mounts: Vec<MountRow> = query_named::<MountRow>("mounts")?
        .into_iter()
        .filter(|row| !row.device.is_empty() && !row.path.is_empty())
        .collect();
    let device_by_mount: HashMap<String, String> = mounts
        .iter()
        .map(|row| (row.path.clone(), row.device.clone()))
        .collect();
    let dataset_by_mount: HashMap<String, String> = mounts
        .into_iter()
        .filter(|row| row.fs_type == "zfs")
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
    // A missing or failing `zfs` CLI must not cost the non-ZFS disk facts.
    crate::util::attempt("zfs", || apply_zfs(disks, &dataset_by_mount));
    Ok(())
}

/// The pool is the first component of a dataset name (`tank` for `tank/home`).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn zfs_pool(dataset: &str) -> &str {
    dataset.split('/').next().unwrap_or(dataset)
}

/// Parses `zfs get -Hp -o name,property,value` output, keyed by
/// `(name, property)`.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn parse_zfs_get(out: &str) -> HashMap<(String, String), String> {
    out.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next()?.to_string();
            let property = fields.next()?.to_string();
            Some(((name, property), fields.next()?.to_string()))
        })
        .collect()
}

/// ZFS native encryption is invisible to `disk_encryption`, which only knows
/// dm-crypt, so it is read from the mounted dataset. Capacity is the pool's:
/// datasets share the pool's free space, so a dataset's own usage reads as
/// near-empty when the data lives in sibling datasets.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn apply_zfs_props(
    disks: &mut [DiskRequest],
    dataset_by_mount: &HashMap<String, String>,
    props: &HashMap<(String, String), String>,
) {
    let get = |name: &str, property: &str| props.get(&(name.to_string(), property.to_string()));
    let bytes = |name: &str, property: &str| get(name, property)?.parse::<i64>().ok();
    for disk in disks.iter_mut() {
        let Some(dataset) = dataset_by_mount.get(&disk.mountpoint) else {
            continue;
        };
        if let Some(encryption) = get(dataset, "encryption") {
            disk.encryption_enabled = Some(encryption != "off");
        }
        let pool = zfs_pool(dataset);
        if let (Some(used), Some(available)) = (bytes(pool, "used"), bytes(pool, "available")) {
            disk.capacity_total_bytes = Some(used + available);
            disk.capacity_used_bytes = Some(used);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn apply_zfs(disks: &mut [DiskRequest], dataset_by_mount: &HashMap<String, String>) -> Result<()> {
    if dataset_by_mount.is_empty() {
        return Ok(());
    }
    let mut names: Vec<&str> = dataset_by_mount
        .values()
        .flat_map(|dataset| [dataset.as_str(), zfs_pool(dataset)])
        .collect();
    names.sort_unstable();
    names.dedup();
    let out = crate::util::run(
        std::process::Command::new("zfs")
            .args([
                "get",
                "-Hp",
                "-o",
                "name,property,value",
                "encryption,used,available",
            ])
            .args(&names),
    )?;
    apply_zfs_props(disks, dataset_by_mount, &parse_zfs_get(&out));
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn disk(mountpoint: &str) -> DiskRequest {
        DiskRequest {
            name: mountpoint.to_string(),
            mountpoint: mountpoint.to_string(),
            label: None,
            capacity_total_bytes: Some(100),
            capacity_used_bytes: Some(1),
            encryption_enabled: None,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn zfs_mounts_report_pool_capacity_and_dataset_encryption() {
        let out = "rpool\tencryption\taes-256-gcm\n\
                   rpool\tused\t600\n\
                   rpool\tavailable\t400\n\
                   rpool/root\tencryption\taes-256-gcm\n\
                   rpool/root\tused\t5\n\
                   rpool/root\tavailable\t400\n\
                   rpool/scratch\tencryption\toff\n";
        let dataset_by_mount = HashMap::from([
            ("/".to_string(), "rpool/root".to_string()),
            ("/scratch".to_string(), "rpool/scratch".to_string()),
        ]);
        let mut disks = [disk("/"), disk("/scratch"), disk("/boot")];
        apply_zfs_props(&mut disks, &dataset_by_mount, &parse_zfs_get(out));

        let [root, scratch, boot] = &disks;
        assert_eq!(root.encryption_enabled, Some(true));
        assert_eq!(root.capacity_total_bytes, Some(1000));
        assert_eq!(root.capacity_used_bytes, Some(600));
        assert_eq!(scratch.encryption_enabled, Some(false));
        assert_eq!(scratch.capacity_total_bytes, Some(1000));
        assert_eq!(boot.encryption_enabled, None);
        assert_eq!(boot.capacity_total_bytes, Some(100));
    }

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
