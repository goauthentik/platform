use ak_platform::generated::ping::ping_client::PingClient;
use ak_platform::generated::sys_directory::system_directory_client::SystemDirectoryClient;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use eyre::{Result, bail};
use std::collections::{BTreeMap, HashMap};

pub struct CheckResult {
    pub category: String,
    pub message: String,
    pub success: bool,
}

fn result_from_error(category: &str, message: impl std::fmt::Display) -> CheckResult {
    CheckResult {
        category: category.to_string(),
        message: message.to_string(),
        success: false,
    }
}

/// Runs setup diagnostics and prints a pass/fail tree, mirroring Go's
/// `ak-sysd troubleshoot check` subcommand output
/// (`pkg/agent_system/check/{check.go,check_nss.go,check_pam.go}`).
pub async fn run_checks() -> Result<()> {
    let mut results = vec![
        check_nss_passwd(),
        check_nss_shadow(),
        check_nss_group(),
        check_pam_auth(),
        check_pam_session(),
    ];
    results.push(check_nss_direct().await);
    results.push(check_agent_connectivity().await);

    let mut by_category: BTreeMap<String, Vec<CheckResult>> = BTreeMap::new();
    let mut all_ok = true;
    for r in results {
        all_ok &= r.success;
        by_category.entry(r.category.clone()).or_default().push(r);
    }

    for (cat, results) in &by_category {
        println!("{cat}");
        for r in results {
            let mark = if r.success { "[OK]" } else { "[FAIL]" };
            println!("  {mark} {}", r.message);
        }
    }

    if !all_ok {
        bail!("one or more checks failed");
    }
    Ok(())
}

/// Parses `/etc/nsswitch.conf` into a map of database name -> configured
/// sources, mirroring Go's `_readNSSWitch` (`check/utils.go`).
fn read_nsswitch() -> Result<HashMap<String, String>> {
    let raw = std::fs::read_to_string("/etc/nsswitch.conf")?;
    let mut dbs = HashMap::new();
    for line in raw.split('\n') {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let Some((db, sources)) = line.split_once(':') else {
            continue;
        };
        dbs.insert(db.trim().to_string(), sources.trim().to_string());
    }
    Ok(dbs)
}

/// Reads `/etc/pam.d/<name>`, mirroring Go's `_readPAMConfig` (`check/utils.go`).
fn read_pam_config(name: &str) -> Result<String> {
    Ok(std::fs::read_to_string(
        std::path::Path::new("/etc/pam.d").join(name),
    )?)
}

fn check_nss_passwd() -> CheckResult {
    match read_nsswitch() {
        Ok(nss) => {
            if nss.get("passwd").is_some_and(|v| v.contains("authentik")) {
                CheckResult {
                    category: "NSS".to_string(),
                    message: "nsswitch uses authentik for passwd lookups".to_string(),
                    success: true,
                }
            } else {
                result_from_error("NSS", "nsswitch passwd not configured to use authentik")
            }
        }
        Err(e) => result_from_error("NSS", e),
    }
}

fn check_nss_shadow() -> CheckResult {
    match read_nsswitch() {
        Ok(nss) => {
            if nss.get("shadow").is_some_and(|v| v.contains("authentik")) {
                CheckResult {
                    category: "NSS".to_string(),
                    message: "nsswitch uses authentik for shadow lookups".to_string(),
                    success: true,
                }
            } else {
                result_from_error("NSS", "nsswitch shadow not configured to use authentik")
            }
        }
        Err(e) => result_from_error("NSS", e),
    }
}

fn check_nss_group() -> CheckResult {
    match read_nsswitch() {
        Ok(nss) => {
            if nss.get("group").is_some_and(|v| v.contains("authentik")) {
                CheckResult {
                    category: "NSS".to_string(),
                    message: "nsswitch uses authentik for group lookups".to_string(),
                    success: true,
                }
            } else {
                result_from_error("NSS", "nsswitch group not configured to use authentik")
            }
        }
        Err(e) => result_from_error("NSS", e),
    }
}

fn check_pam_auth() -> CheckResult {
    match read_pam_config("common-auth") {
        Ok(cfg) => {
            if cfg.contains("pam_authentik.so") {
                CheckResult {
                    category: "PAM".to_string(),
                    message: "PAM uses authentik for authentication".to_string(),
                    success: true,
                }
            } else {
                result_from_error("PAM", "PAM authentication not configured to use authentik")
            }
        }
        Err(e) => result_from_error("PAM", e),
    }
}

fn check_pam_session() -> CheckResult {
    match read_pam_config("common-session") {
        Ok(cfg) => {
            if cfg.contains("pam_authentik.so") {
                CheckResult {
                    category: "PAM".to_string(),
                    message: "PAM uses authentik for sessions".to_string(),
                    success: true,
                }
            } else {
                result_from_error("PAM", "PAM sessions not configured to use authentik")
            }
        }
        Err(e) => result_from_error("PAM", e),
    }
}

async fn check_nss_direct() -> CheckResult {
    let path = sysd_socket_path(SysdSocketID::Default).for_current();
    let attempt = async {
        let channel = ak_platform::grpc::grpc_endpoint(path).await?;
        let mut client = SystemDirectoryClient::new(channel);
        let users = client.list_users(()).await?.into_inner();
        Ok::<usize, eyre::Report>(users.users.len())
    };

    match attempt.await {
        Ok(count) if count >= 1 => CheckResult {
            category: "NSS".to_string(),
            message: "Successfully able to list authentik users".to_string(),
            success: true,
        },
        Ok(_) => CheckResult {
            category: "NSS".to_string(),
            message: "Failed to list authentik users".to_string(),
            success: false,
        },
        Err(e) => result_from_error("NSS", e),
    }
}

async fn check_agent_connectivity() -> CheckResult {
    let path = sysd_socket_path(SysdSocketID::Default).for_current();
    let attempt = async {
        let channel = ak_platform::grpc::grpc_endpoint(path).await?;
        let mut client = PingClient::new(channel);
        client.ping(()).await?;
        Ok::<(), eyre::Report>(())
    };

    match attempt.await {
        Ok(()) => CheckResult {
            category: "Agent".to_string(),
            message: "sysd is reachable on the default socket".to_string(),
            success: true,
        },
        Err(e) => CheckResult {
            category: "Agent".to_string(),
            message: format!("sysd is not reachable: {e}"),
            success: false,
        },
    }
}
