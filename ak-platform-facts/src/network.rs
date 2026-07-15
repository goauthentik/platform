use authentik_client::models::{NetworkInterfaceRequest, NetworkRequest};
use eyre::Result;
use serde::Deserialize;
use sysinfo::{InterfaceOperationalState, Networks};

use crate::query::query_named;

/// Only interfaces that are up, non-loopback, have a hardware address, and
/// have at least one non-loopback IP address are included.
fn interfaces_base() -> Vec<NetworkInterfaceRequest> {
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
                dns_servers: None,
            })
        })
        .collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));
    interfaces
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Deserialize)]
struct DnsResolverRow {
    #[serde(default)]
    address: String,
}

/// Linux + macOS: `dns_resolvers` is system-wide, not per-interface, so
/// every interface gets the same list. This is an intentional accuracy
/// trade-off on macOS specifically — the old implementation used
/// `networksetup -getdnsservers <iface>`, which was genuinely
/// per-interface; this collapses macOS to the same system-wide-list
/// limitation Linux already had via `/etc/resolv.conf`.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn apply_dns_servers(interfaces: &mut [NetworkInterfaceRequest]) {
    let nameservers: Vec<String> = query_named::<DnsResolverRow>("dns_resolvers")
        .map(|rows| {
            rows.into_iter()
                .map(|row| row.address)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for iface in interfaces.iter_mut() {
        iface.dns_servers = Some(nameservers.clone());
    }
}

/// Kept native: no osquery table covers per-interface DNS on Windows
/// (`dns_resolvers` is Linux/macOS-only).
#[cfg(target_os = "windows")]
fn apply_dns_servers(interfaces: &mut [NetworkInterfaceRequest]) {
    use wmi::{COMLibrary, WMIConnection};

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32NetworkAdapter {
        net_connection_id: Option<String>,
        index: Option<u32>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32NetworkAdapterConfiguration {
        interface_index: Option<u32>,
        dns_server_search_order: Option<Vec<String>>,
    }

    /// Looks up the adapter's numeric index by its connection name
    /// (`NetConnectionID`, the friendly name shown in Windows), then
    /// filters `Win32_NetworkAdapterConfiguration` by that exact
    /// `InterfaceIndex`.
    fn dns_servers_for(iface: &str) -> Vec<String> {
        (|| -> Result<Vec<String>> {
            let con = WMIConnection::new(COMLibrary::new()?)?;

            let adapters: Vec<Win32NetworkAdapter> = con.query()?;
            let Some(index) = adapters.iter().find_map(|a| {
                a.net_connection_id
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(iface))
                    .then_some(a.index)
                    .flatten()
            }) else {
                return Ok(Vec::new());
            };

            let configs: Vec<Win32NetworkAdapterConfiguration> = con.query()?;
            Ok(configs
                .into_iter()
                .find(|c| c.interface_index == Some(index))
                .and_then(|c| c.dns_server_search_order)
                .unwrap_or_default())
        })()
        .unwrap_or_default()
    }

    for iface in interfaces.iter_mut() {
        iface.dns_servers = Some(dns_servers_for(&iface.name));
    }
}

/// Kept native: osquery's `iptables` table only sees libiptc/legacy
/// netfilter rules and silently under-reports "no firewall" on
/// nftables-native systems (common modern Debian/Ubuntu/RHEL defaults).
#[cfg(target_os = "linux")]
fn firewall_enabled() -> Result<bool> {
    use crate::util::run;

    fn iptables_has_rules() -> bool {
        run(std::process::Command::new("iptables").arg("-L"))
            .is_ok_and(|out| out.contains("REJECT") || out.contains("DROP"))
    }
    fn ufw_active() -> bool {
        run(std::process::Command::new("ufw").arg("status"))
            .is_ok_and(|out| out.contains("Status: active"))
    }
    fn firewalld_active() -> bool {
        run(std::process::Command::new("systemctl").args(["is-active", "firewalld"]))
            .is_ok_and(|out| out.trim() == "active")
    }
    Ok(iptables_has_rules() || ufw_active() || firewalld_active())
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
struct AlfRow {
    #[serde(default)]
    global_state: String,
}

/// This is the actual macOS System Settings -> Network -> Firewall toggle
/// (reads `/Library/Preferences/com.apple.alf.plist`), more correct than
/// the old `pfctl -s info` check (`pf` is a lower-level packet filter, not
/// the user-facing toggle) — and removes the passwordless-sudo requirement
/// entirely.
#[cfg(target_os = "macos")]
fn firewall_enabled() -> Result<bool> {
    Ok(query_named::<AlfRow>("macos_firewall")?
        .iter()
        .any(|row| row.global_state.parse::<i64>().is_ok_and(|state| state != 0)))
}

/// Kept native: no real osquery table exposes profile-level
/// `NetFirewallProfile.Enabled` state (`windows_firewall_rules.enabled` is
/// per-rule, not per-profile).
#[cfg(target_os = "windows")]
fn firewall_enabled() -> Result<bool> {
    use wmi::{COMLibrary, WMIConnection};

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct NetFirewallProfile {
        enabled: Option<u32>,
    }

    let con = WMIConnection::with_namespace_path("ROOT\\StandardCimv2", COMLibrary::new()?)?;
    let profiles: Vec<NetFirewallProfile> = con.query()?;
    Ok(profiles.iter().any(|p| p.enabled == Some(1)))
}

#[derive(Deserialize)]
struct RouteRow {
    #[serde(default)]
    gateway: String,
    #[serde(rename = "type", default)]
    route_type: String,
    #[serde(default)]
    metric: String,
}

/// Brand new on all three platforms: `gateway` was hardcoded `None`
/// before this migration everywhere, so there's no native/osquery split
/// to preserve here.
fn gateway() -> Result<Option<String>> {
    let rows = query_named::<RouteRow>("default_gateway")?;
    if let Some(row) = rows.iter().find(|r| r.route_type == "gateway") {
        return Ok((!row.gateway.is_empty()).then(|| row.gateway.clone()));
    }
    Ok(rows
        .into_iter()
        .filter(|r| !r.gateway.is_empty())
        .min_by_key(|r| r.metric.parse::<i64>().unwrap_or(i64::MAX))
        .map(|r| r.gateway))
}

pub fn gather() -> Result<NetworkRequest> {
    let mut interfaces = interfaces_base();
    apply_dns_servers(&mut interfaces);
    Ok(NetworkRequest {
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        firewall_enabled: firewall_enabled().ok(),
        interfaces,
        gateway: gateway().ok().flatten(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_produces_hostname() {
        assert!(!gather().unwrap().hostname.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn gather_populates_gateway() {
        assert!(gateway().unwrap().is_some());
    }
}
