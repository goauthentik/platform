use eyre::Result;
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32NetworkAdapterConfiguration {
    description: Option<String>,
    dns_server_search_order: Option<Vec<String>>,
}

pub fn dns_servers(iface: &str) -> Vec<String> {
    (|| -> Result<Vec<String>> {
        let con = WMIConnection::new(COMLibrary::new()?)?;
        let configs: Vec<Win32NetworkAdapterConfiguration> = con.query()?;
        let matched = configs.iter().find(|c| {
            c.description
                .as_deref()
                .is_some_and(|d| d.eq_ignore_ascii_case(iface) || d.contains(iface))
        });
        Ok(matched
            .or_else(|| configs.iter().find(|c| c.dns_server_search_order.is_some()))
            .and_then(|c| c.dns_server_search_order.clone())
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
