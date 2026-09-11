use ak_platform::string::PlatformString;
use eyre::{Result, WrapErr};
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority",
    gen_blocking = false
)]
trait PolkitAuthority {
    async fn check_authorization(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        action_id: &str,
        details: HashMap<String, String>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

// Returns the process start time from /proc/self/stat (field 22, 1-indexed).
// Used alongside the PID to form a stable unix-process polkit subject that
// is resistant to PID reuse races.
fn proc_start_time() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // The comm field may contain spaces; find the last ')' to skip past it.
    let after_comm = stat.rfind(')')?;
    let fields: Vec<&str> = stat[after_comm + 2..].split(' ').collect();
    // Field 22 (1-indexed) = starttime, which is index 19 after the comm field.
    fields.get(19)?.parse().ok()
}

pub async fn prompt(_msg: PlatformString) -> Result<bool> {
    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("polkit: D-Bus system bus unavailable, allowing authorization: {e:?}");
            return Ok(true);
        }
    };

    let proxy = match PolkitAuthorityProxy::new(&conn).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                "polkit: could not connect to PolicyKit1 authority, allowing authorization: {e:?}"
            );
            return Ok(true);
        }
    };

    let pid: u32 = std::process::id();
    let mut subject_details: HashMap<String, OwnedValue> = HashMap::new();
    subject_details.insert("pid".to_string(), OwnedValue::from(pid));
    if let Some(start_time) = proc_start_time() {
        subject_details.insert("start-time".to_string(), OwnedValue::from(start_time));
    }

    let (is_authorized, _, _) = proxy
        .check_authorization(
            ("unix-process".to_string(), subject_details),
            "io.goauthentik.platform.authorize",
            HashMap::new(),
            1u32, // ALLOW_USER_INTERACTION
            "",
        )
        .await
        .wrap_err("polkit: check_authorization RPC failed")?;

    Ok(is_authorized)
}
