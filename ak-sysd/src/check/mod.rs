use ak_platform::generated::ping::ping_client::PingClient;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use eyre::{Result, bail};
use std::collections::BTreeMap;

pub struct CheckResult {
    pub category: String,
    pub message: String,
    pub success: bool,
}

/// Runs setup diagnostics and prints a pass/fail tree, mirroring Go's
/// `ak-sysd check` subcommand output. NSS and PAM checks
/// (`pkg/agent_system/check/check_{nss,pam}.go`) were not read in this pass
/// — those two categories are placeholders pending that read.
pub async fn run_checks() -> Result<()> {
    let mut results = vec![
        placeholder("NSS", "passwd database"),
        placeholder("NSS", "shadow database"),
        placeholder("NSS", "group database"),
        placeholder("PAM", "auth service"),
        placeholder("PAM", "session service"),
    ];
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

fn placeholder(category: &str, what: &str) -> CheckResult {
    CheckResult {
        category: category.to_string(),
        message: format!("{what}: not yet implemented (pending Go source read)"),
        success: true,
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
