pub mod disk;
pub mod group;
pub mod hardware;
pub mod network;
pub mod os;
pub mod process;
mod util;
pub mod user;
pub mod vendor;

pub use hardware::serial;

use authentik_client::models::DeviceFactsRequest;
use util::attempt;

/// Hostname of the current host, also used for enrollment.
pub fn hostname() -> String {
    sysinfo::System::host_name().unwrap_or_default()
}

/// Gathers all device facts for the current host, matching the shape of
/// Go's `facts.Gather`. Each subsystem is attempted independently: a
/// failure in one (e.g. an unreadable file, a missing command) is logged
/// and leaves that section unset rather than failing the whole call.
pub fn gather() -> DeviceFactsRequest {
    DeviceFactsRequest {
        hardware: Some(attempt("hardware", hardware::gather)),
        os: Some(attempt("os", os::gather)),
        network: Some(attempt("network", network::gather)),
        disks: Some(attempt("disks", disk::gather)),
        processes: Some(attempt("processes", process::gather)),
        users: Some(attempt("users", user::gather)),
        groups: Some(attempt("groups", group::gather)),
        software: None,
        vendor: Some(vendor::gather()),
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
}
