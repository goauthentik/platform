use eyre::Result;
use serde::Deserialize;
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
/// (`NetConnectionID`, the friendly name shown in Windows), then filters
/// `Win32_NetworkAdapterConfiguration` by that exact `InterfaceIndex`.
pub fn dns_servers(iface: &str) -> Vec<String> {
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

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NetFirewallProfile {
    enabled: Option<u32>,
}

pub fn firewall_enabled() -> Result<bool> {
    let con = WMIConnection::with_namespace_path("ROOT\\StandardCimv2", COMLibrary::new()?)?;
    let profiles: Vec<NetFirewallProfile> = con.query()?;
    Ok(profiles.iter().any(|p| p.enabled == Some(1)))
}
