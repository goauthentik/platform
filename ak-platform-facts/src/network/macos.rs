use eyre::Result;

use crate::util::run;

pub fn dns_servers(iface: &str) -> Vec<String> {
    run(std::process::Command::new("networksetup").args(["-getdnsservers", iface]))
        .map(|out| {
            out.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("There aren't any"))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Matches Go's own approach (`sudo pfctl -s info`) — inherits the same
/// requirement that the caller has passwordless sudo for `pfctl`.
pub fn firewall_enabled() -> Result<bool> {
    let out = run(std::process::Command::new("sudo").args(["pfctl", "-s", "info"]))?;
    Ok(out.contains("Status: Enabled"))
}
