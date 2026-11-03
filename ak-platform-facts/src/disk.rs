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
    #[cfg(any(target_os = "linux", test))]
    #[serde(default)]
    encryption_status: String,
    #[cfg(any(target_os = "linux", test))]
    #[serde(rename = "type", default)]
    encryption_type: String,
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

    let encryption_rows = query_named::<DiskEncryptionRow>("disk_encryption")?;
    #[cfg(target_os = "macos")]
    let encrypted_by_device: HashMap<String, bool> = encryption_rows
        .iter()
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
        #[cfg(target_os = "macos")]
        {
            disk.encryption_enabled = encrypted_by_device.get(&key).copied();
        }
        #[cfg(target_os = "linux")]
        {
            disk.encryption_enabled = linux_encryption(device, &encryption_rows);
        }
        disk.label = label_by_device.get(&key).cloned();
    }
    // A missing or failing `zfs` CLI must not cost the non-ZFS disk facts.
    crate::util::attempt("zfs", || apply_zfs(disks, &dataset_by_mount));
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_device(path: &str) -> Option<String> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.file_type().is_block_device() {
        return None;
    }
    let dev = metadata.rdev();
    // Linux's dev_t encoding, including extended major/minor bits.
    let major = ((dev >> 8) & 0xfff) | ((dev >> 32) & 0xfffff000);
    let minor = (dev & 0xff) | ((dev >> 12) & 0xffffff00);
    Some(format!("{major}:{minor}"))
}

#[cfg(target_os = "linux")]
fn linux_encryption(device: &str, rows: &[DiskEncryptionRow]) -> Option<bool> {
    let root = linux_device(device)?;
    let evidence: HashMap<_, _> = rows
        .iter()
        .filter_map(|row| Some((linux_device(&row.name)?, row)))
        .collect();
    linux_walk(
        &std::path::Path::new("/sys/dev/block").join(root),
        &evidence,
        &mut std::collections::HashSet::new(),
        &mut 4096,
        0,
    )
}

#[cfg(any(target_os = "linux", all(test, target_os = "macos")))]
fn linux_walk(
    node: &std::path::Path,
    evidence: &HashMap<String, &DiskEncryptionRow>,
    active: &mut std::collections::HashSet<std::path::PathBuf>,
    budget: &mut usize,
    depth: usize,
) -> Option<bool> {
    if depth >= 64 || *budget == 0 {
        return None;
    }
    *budget -= 1;
    let node = std::fs::canonicalize(node).ok()?;
    if !active.insert(node.clone()) {
        return None;
    }
    let result = (|| {
        let identity = std::fs::read_to_string(node.join("dev")).ok()?;
        let row = evidence.get(identity.trim()).copied();
        let uuid = std::fs::read_to_string(node.join("dm/uuid")).unwrap_or_default();
        if uuid.starts_with("CRYPT-LUKS1-") || uuid.starts_with("CRYPT-LUKS2-") {
            return row
                .filter(|r| {
                    r.encrypted == "1"
                        && r.encryption_status == "encrypted"
                        && (r.encryption_type.starts_with("LUKS1-")
                            || r.encryption_type.starts_with("LUKS2-"))
                        && !r.encryption_type.contains("null")
                })
                .map(|_| true);
        }
        // Partition sysfs directories may have no slaves directory.
        // Preserve explicit negative evidence without inferring encryption.
        if node.join("partition").is_file() && row.is_some_and(|r| r.encrypted == "0") {
            return Some(false);
        }
        let children = std::fs::read_dir(node.join("slaves"))
            .ok()?
            .collect::<std::io::Result<Vec<_>>>()
            .ok()?;
        if children.is_empty() {
            return row.filter(|r| r.encrypted == "0").map(|_| false);
        }
        for child in children {
            if linux_walk(&child.path(), evidence, active, budget, depth + 1) != Some(true) {
                return None;
            }
        }
        Some(true)
    })();
    active.remove(&node);
    result
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn linux_lvm_requires_verified_encryption_on_every_branch() -> Result<()> {
        use std::os::unix::fs::symlink;
        let fixture = tempfile::tempdir()?;
        let root = fixture.path().join("dm-1");
        let crypt = fixture.path().join("dm-0");
        std::fs::create_dir_all(root.join("slaves"))?;
        std::fs::create_dir_all(crypt.join("dm"))?;
        std::fs::write(root.join("dev"), "254:1")?;
        std::fs::write(crypt.join("dev"), "254:0")?;
        std::fs::write(crypt.join("dm/uuid"), "CRYPT-LUKS2-test")?;
        symlink(&crypt, root.join("slaves/dm-0"))?;
        let row = DiskEncryptionRow {
            name: String::new(),
            encrypted: "1".into(),
            encryption_status: "encrypted".into(),
            encryption_type: "LUKS2-aes-xts-plain64".into(),
        };
        let evidence = HashMap::from([("254:0".into(), &row)]);
        let check = |evidence: &HashMap<String, &DiskEncryptionRow>| {
            linux_walk(&root, evidence, &mut Default::default(), &mut 4096, 0)
        };
        assert_eq!(check(&evidence), Some(true));
        assert_eq!(check(&HashMap::new()), None);
        let plain = fixture.path().join("plain");
        std::fs::create_dir_all(plain.join("slaves"))?;
        std::fs::write(plain.join("dev"), "8:1")?;
        symlink(&plain, root.join("slaves/plain"))?;
        assert_eq!(check(&evidence), None);
        std::fs::remove_file(root.join("slaves/plain"))?;
        symlink(&root, root.join("slaves/cycle"))?;
        assert_eq!(check(&evidence), None);
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn linux_encryption_rejects_insufficient_evidence_and_limits() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let node = fixture.path();
        std::fs::create_dir(node.join("dm"))?;
        std::fs::write(node.join("dev"), "254:0")?;
        std::fs::write(node.join("dm/uuid"), "CRYPT-LUKS2-test")?;
        for (encrypted, status, kind) in [
            ("1", "encrypted", ""),
            ("1", "encrypted", "LUKS2-cipher_null-ecb"),
            ("1", "undefined", "LUKS2-aes-xts-plain64"),
            ("", "encrypted", "LUKS2-aes-xts-plain64"),
            ("0", "not encrypted", "LUKS2-aes-xts-plain64"),
        ] {
            let row = DiskEncryptionRow {
                name: String::new(),
                encrypted: encrypted.into(),
                encryption_status: status.into(),
                encryption_type: kind.into(),
            };
            let evidence = HashMap::from([("254:0".into(), &row)]);
            assert_eq!(
                linux_walk(node, &evidence, &mut Default::default(), &mut 4096, 0),
                None
            );
        }
        assert_eq!(
            linux_walk(node, &HashMap::new(), &mut Default::default(), &mut 0, 0),
            None
        );
        assert_eq!(
            linux_walk(
                node,
                &HashMap::new(),
                &mut Default::default(),
                &mut 4096,
                64
            ),
            None
        );
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn linux_partition_without_slaves_preserves_unencrypted_status() -> Result<()> {
        let fixture = tempfile::tempdir()?;
        let node = fixture.path();
        std::fs::write(node.join("dev"), "259:1")?;
        std::fs::write(node.join("partition"), "1")?;
        let row = DiskEncryptionRow {
            name: String::new(),
            encrypted: "0".into(),
            encryption_status: "not encrypted".into(),
            encryption_type: String::new(),
        };
        let evidence = HashMap::from([("259:1".into(), &row)]);
        assert_eq!(
            linux_walk(node, &evidence, &mut Default::default(), &mut 4096, 0),
            Some(false)
        );
        Ok(())
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
