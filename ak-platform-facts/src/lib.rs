pub mod disk;
pub mod group;
pub mod hardware;
pub mod network;
pub mod os;
pub mod process;
mod query;
pub mod user;
mod util;
pub mod vendor;

pub use hardware::serial;
pub use process::ProcessCollection;

use std::collections::HashMap;

use authentik_client::models::DeviceFactsRequest;
use serde::{Deserialize, Serialize};
use util::attempt;

/// Controls which facts [`gather_with`] collects. The default collects
/// everything.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GatherOptions {
    pub processes: ProcessCollection,
}

pub fn hostname() -> String {
    sysinfo::System::host_name().unwrap_or_default()
}

/// Gathers every fact with the default [`GatherOptions`].
pub fn gather() -> DeviceFactsRequest {
    gather_with(&GatherOptions::default())
}

/// Each subsystem is attempted independently: a failure in one leaves
/// that section unset rather than failing the whole call. Subsystems
/// disabled in `opts` are left unset too.
pub fn gather_with(opts: &GatherOptions) -> DeviceFactsRequest {
    // Namespaced so other vendors/agents can write their own top-level
    // keys into the same shared map.
    let mut vendor = HashMap::new();
    vendor.insert(
        "goauthentik.io/platform".to_string(),
        serde_json::to_value(vendor::gather()).unwrap_or_default(),
    );

    DeviceFactsRequest {
        hardware: Some(attempt("hardware", hardware::gather)),
        os: Some(attempt("os", os::gather)),
        network: Some(attempt("network", network::gather)),
        disks: Some(attempt("disks", disk::gather)),
        processes: match opts.processes {
            ProcessCollection::Disabled => None,
            collection => Some(attempt("processes", || process::gather(collection))),
        },
        users: Some(attempt("users", user::gather)),
        groups: Some(attempt("groups", group::gather)),
        software: None,
        vendor: Some(vendor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_produces_serializable_facts() {
        let facts = gather();
        assert!(facts.hardware.is_some());
        assert!(facts.os.is_some());
        assert!(facts.network.is_some());
        assert!(serde_json::to_string(&facts).is_ok_and(|json| !json.is_empty()));
    }

    #[test]
    fn gather_with_processes_disabled_omits_processes() {
        let facts = gather_with(&GatherOptions {
            processes: ProcessCollection::Disabled,
        });
        assert!(facts.processes.is_none());
        assert!(facts.os.is_some());
        assert!(serde_json::to_value(&facts).is_ok_and(|v| v.get("processes").is_none()));
    }

    #[test]
    fn gather_options_default_when_unset() {
        assert!(
            serde_json::from_str::<GatherOptions>("{}")
                .is_ok_and(|o| o == GatherOptions::default())
        );
        assert_eq!(GatherOptions::default().processes, ProcessCollection::Full);
        assert!(
            serde_json::from_str::<GatherOptions>(r#"{"processes":"executable"}"#)
                .is_ok_and(|o| o.processes == ProcessCollection::Executable)
        );
    }
}
