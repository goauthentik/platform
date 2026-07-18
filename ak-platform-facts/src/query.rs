use std::sync::OnceLock;

use eyre::Result;
use osquery::{OsqueryError, OsqueryInstance};
use serde::de::DeserializeOwned;

/// `OsqueryInstance::start()` can only ever succeed once per process
/// (enforced by osquery-rs's own global guard), so this cell holds the
/// *outcome* of that one attempt: `Ok` is reused by every caller, `Err` is
/// a permanent, cached failure that every subsequent call short-circuits
/// on rather than retrying a `start()` that can only ever error again.
static INSTANCE: OnceLock<Result<OsqueryInstance, OsqueryError>> = OnceLock::new();

fn instance() -> Result<&'static OsqueryInstance> {
    match INSTANCE.get_or_init(OsqueryInstance::start) {
        Ok(instance) => Ok(instance),
        Err(e) => Err(eyre::eyre!("osquery instance unavailable: {e}")),
    }
}

struct NamedQuery {
    name: &'static str,
    sql: &'static str,
}

/// Central registry of osquery SQL used by this crate. Adding a new fact
/// later is just appending one entry here and calling [`query_named`] with
/// its name — no changes needed to instance lifecycle or execution
/// plumbing.
const QUERIES: &[NamedQuery] = &[
    NamedQuery {
        name: "system_info",
        sql: "SELECT hardware_vendor, hardware_model, hardware_serial, cpu_brand, \
              cpu_logical_cores, physical_memory FROM system_info",
    },
    NamedQuery {
        name: "os_version",
        sql: "SELECT name, version FROM os_version",
    },
    NamedQuery {
        name: "processes_with_user",
        sql: "SELECT p.pid, p.name, p.path, p.cmdline, u.username FROM processes p \
              LEFT JOIN users u ON p.uid = u.uid",
    },
    NamedQuery {
        name: "users",
        sql: "SELECT uid, username, description, directory FROM users",
    },
    NamedQuery {
        name: "groups",
        // Excludes `_`-prefixed system groups: the old macOS `dscl`
        // listing filtered these out, but osquery's `groups` table does
        // not, so this preserves prior behavior (a no-op on platforms
        // where underscore-prefixed groups aren't a real convention).
        sql: "SELECT gid, groupname FROM groups WHERE groupname NOT LIKE '\\_%' ESCAPE '\\'",
    },
    NamedQuery {
        name: "mounts",
        // Bridges sysinfo's mount_point()-keyed disk listing to osquery's
        // device-node-keyed disk_encryption/block_devices rows. Needed
        // because sysinfo's Disk::name() on macOS is a Finder volume
        // label ("Macintosh HD"), not a device node, so there's no way
        // to match it against those tables' `name` column directly.
        sql: "SELECT device, path FROM mounts",
    },
    NamedQuery {
        name: "disk_encryption",
        // Linux + macOS only (no Windows table). Linux's implementation
        // returns empty results entirely unless running as root/euid 0.
        sql: "SELECT name, encrypted FROM disk_encryption",
    },
    NamedQuery {
        name: "block_devices",
        sql: "SELECT name, label FROM block_devices",
    },
    NamedQuery {
        name: "bitlocker_info",
        // Backed by the same Win32_EncryptableVolume WMI class the old
        // native Windows code queried. protection_status/
        // conversion_status are documented INTEGER enums mirroring that
        // WMI class; encryption_method is TEXT with unverified exact
        // string values, so it isn't used in the boolean "is encrypted"
        // logic.
        sql: "SELECT drive_letter, protection_status, conversion_status, encryption_method \
              FROM bitlocker_info",
    },
    NamedQuery {
        name: "default_gateway",
        sql: "SELECT gateway, type, metric FROM routes WHERE destination = '0.0.0.0' AND netmask = 0",
    },
    NamedQuery {
        name: "macos_firewall",
        sql: "SELECT global_state FROM alf",
    },
    NamedQuery {
        name: "dns_resolvers",
        sql: "SELECT address FROM dns_resolvers WHERE type = 'nameserver'",
    },
];

/// Runs a query by name from the [`QUERIES`] registry against the embedded
/// osquery engine, deserializing each row into `T`.
pub(crate) fn query_named<T: DeserializeOwned>(name: &str) -> Result<Vec<T>> {
    let sql = QUERIES
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| eyre::eyre!("unknown named query: {name}"))?
        .sql;
    tracing::debug!("running osquery query {name:?}: {sql}");
    let instance = instance()?;
    let result = instance
        .query::<T>(sql)
        .map_err(|e| eyre::eyre!("osquery query {name:?} failed: {e}"))?;
    Ok(result.rows)
}

/// osquery's rows are fundamentally `map<string, string>` on the engine
/// side, so a SQL NULL always arrives as an empty string, never a missing
/// key or JSON null — this converts that convention into an
/// `Option<String>` for callers.
pub(crate) fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}
