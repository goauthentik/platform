#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use other as imp;

use authentik_client::models::DiskRequest;
use eyre::Result;
use sysinfo::Disks;

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

pub fn gather() -> Result<Vec<DiskRequest>> {
    let mut disks = disks_base();
    for disk in &mut disks {
        disk.encryption_enabled = imp::encryption_enabled(&disk.name, &disk.mountpoint).ok();
    }
    Ok(disks)
}
