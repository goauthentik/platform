use eyre::Result;

use crate::util::run;

fn resolv_conf_nameservers() -> Vec<String> {
    std::fs::read_to_string("/etc/resolv.conf")
        .map(|content| {
            content
                .lines()
                .filter_map(|line| line.trim().strip_prefix("nameserver"))
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// `/etc/resolv.conf` is system-wide on Linux, so every interface gets the
/// same list, matching Go's behavior.
pub fn dns_servers(_iface: &str) -> Vec<String> {
    resolv_conf_nameservers()
}

fn iptables_has_rules() -> bool {
    run(std::process::Command::new("iptables").arg("-L"))
        .is_ok_and(|out| out.contains("REJECT") || out.contains("DROP"))
}

fn ufw_active() -> bool {
    run(std::process::Command::new("ufw").arg("status")).is_ok_and(|out| out.contains("Status: active"))
}

fn firewalld_active() -> bool {
    run(std::process::Command::new("systemctl").args(["is-active", "firewalld"]))
        .is_ok_and(|out| out.trim() == "active")
}

pub fn firewall_enabled() -> Result<bool> {
    Ok(iptables_has_rules() || ufw_active() || firewalld_active())
}
