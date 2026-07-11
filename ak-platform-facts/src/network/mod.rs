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

/// Enumerates non-loopback, operational interfaces (name/MAC/IPs) via
/// `sysinfo`, matching Go's `net.Interfaces()` filtering, and attaches
/// per-interface DNS servers via the platform-specific lookup.
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
        .filter(|(_, data)| {
            let ips = data.ip_networks();
            ips.is_empty() || ips.iter().any(|ip| !ip.addr.is_loopback())
        })
        .map(|(name, data)| NetworkInterfaceRequest {
            name: name.clone(),
            hardware_address: data.mac_address().to_string(),
            ip_addresses: Some(
                data.ip_networks()
                    .iter()
                    .map(|ip| ip.addr.to_string())
                    .collect(),
            ),
            dns_servers: Some(imp::dns_servers(name)),
        })
        .collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));
    interfaces
}

/// Gathers hostname, interfaces, and firewall status for the current host.
pub fn gather() -> Result<NetworkRequest> {
    Ok(NetworkRequest {
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        firewall_enabled: imp::firewall_enabled().ok(),
        interfaces: interfaces(),
        gateway: None,
    })
}
