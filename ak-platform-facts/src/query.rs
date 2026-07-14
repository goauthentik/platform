use std::sync::OnceLock;

use eyre::Result;
use osquery::{OsqueryError, OsqueryInstance, Row};

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
];

/// Runs a query by name from the [`QUERIES`] registry against the embedded
/// osquery engine and returns its rows.
pub(crate) fn query_named(name: &str) -> Result<Vec<Row>> {
    let sql = QUERIES
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| eyre::eyre!("unknown named query: {name}"))?
        .sql;
    tracing::debug!("running osquery query {name:?}: {sql}");
    let instance = instance()?;
    let result = instance
        .query(sql)
        .map_err(|e| eyre::eyre!("osquery query {name:?} failed: {e}"))?;
    Ok(result.rows)
}
