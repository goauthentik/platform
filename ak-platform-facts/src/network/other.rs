use eyre::Result;

use crate::util::unsupported_platform;

pub fn dns_servers(_iface: &str) -> Vec<String> {
    Vec::new()
}

pub fn firewall_enabled() -> Result<bool> {
    unsupported_platform("network firewall")
}
