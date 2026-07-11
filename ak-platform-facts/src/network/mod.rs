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

use authentik_client::models::{NetworkInterfaceRequest, NetworkRequest};
use eyre::Result;
use sysinfo::{InterfaceOperationalState, Networks};

/// Only interfaces that are up, non-loopback, have a hardware address, and
/// have at least one non-loopback IP address are included.
fn interfaces() -> Vec<NetworkInterfaceRequest> {
    let networks = Networks::new_with_refreshed_list();
    let mut interfaces: Vec<NetworkInterfaceRequest> = networks
        .list()
        .iter()
        .filter(|(_, data)| {
            !matches!(
                data.operational_state(),
                InterfaceOperationalState::Down
                    | InterfaceOperationalState::NotPresent
                    | InterfaceOperationalState::LowerLayerDown
            )
        })
        .filter_map(|(name, data)| {
            let mac = data.mac_address();
            if mac.is_unspecified() {
                return None;
            }
            let valid_addresses: Vec<String> = data
                .ip_networks()
                .iter()
                .filter(|ip| !ip.addr.is_loopback())
                .map(|ip| ip.addr.to_string())
                .collect();
            if valid_addresses.is_empty() {
                return None;
            }
            Some(NetworkInterfaceRequest {
                name: name.clone(),
                hardware_address: mac.to_string(),
                ip_addresses: Some(valid_addresses),
                dns_servers: Some(imp::dns_servers(name)),
            })
        })
        .collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));
    interfaces
}

pub fn gather() -> Result<NetworkRequest> {
    Ok(NetworkRequest {
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        firewall_enabled: imp::firewall_enabled().ok(),
        interfaces: interfaces(),
        gateway: None,
    })
}
