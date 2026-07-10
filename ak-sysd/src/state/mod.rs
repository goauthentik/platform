use eyre::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub mod sessions;

pub use sessions::{SessionRecord, SessionStore};

/// Persistent state for ak-sysd, replacing Go's bbolt-backed `*state.State`.
///
/// bbolt's nested-bucket model has no direct SQLite equivalent, so state is
/// stored in flat tables instead. `inspect()` (used by `troubleshoot_inspect`)
/// documents the resulting shape difference.
pub struct StateStore {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct TroubleshootNode {
    pub bucket: String,
    pub kv: Vec<(String, String)>,
    pub children: Vec<TroubleshootNode>,
}

impl StateStore {
    pub fn open(path: &str) -> Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id           TEXT PRIMARY KEY,
                username     TEXT NOT NULL,
                token_hash   TEXT NOT NULL,
                expires_at   INTEGER,
                created_at   INTEGER NOT NULL,
                pid          INTEGER,
                ppid         INTEGER,
                local_socket TEXT,
                opened       INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS domain_cache (
                domain            TEXT PRIMARY KEY,
                agent_config_json TEXT,
                brand_json        TEXT,
                last_updated      INTEGER
            );
            CREATE TABLE IF NOT EXISTS component_state (
                component TEXT NOT NULL,
                key       TEXT NOT NULL,
                value     TEXT NOT NULL,
                PRIMARY KEY (component, key)
            );
            ",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn sessions(&self) -> SessionStore {
        SessionStore::new(Arc::clone(&self.conn))
    }

    pub async fn domain_cache_get(&self, domain: &str) -> Result<Option<(String, String, i64)>> {
        let conn = Arc::clone(&self.conn);
        let domain = domain.to_string();
        tokio::task::spawn_blocking(move || -> Result<_> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT agent_config_json, brand_json, last_updated FROM domain_cache WHERE domain = ?1",
            )?;
            let mut rows = stmt.query(rusqlite::params![domain])?;
            if let Some(row) = rows.next()? {
                let cfg: Option<String> = row.get(0)?;
                let brand: Option<String> = row.get(1)?;
                let updated: Option<i64> = row.get(2)?;
                Ok(Some((
                    cfg.unwrap_or_default(),
                    brand.unwrap_or_default(),
                    updated.unwrap_or(0),
                )))
            } else {
                Ok(None)
            }
        })
        .await?
    }

    pub async fn domain_cache_set(
        &self,
        domain: &str,
        agent_config_json: &str,
        brand_json: &str,
        last_updated: i64,
    ) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let domain = domain.to_string();
        let agent_config_json = agent_config_json.to_string();
        let brand_json = brand_json.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT INTO domain_cache (domain, agent_config_json, brand_json, last_updated)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(domain) DO UPDATE SET
                    agent_config_json = excluded.agent_config_json,
                    brand_json = excluded.brand_json,
                    last_updated = excluded.last_updated",
                rusqlite::params![domain, agent_config_json, brand_json, last_updated],
            )?;
            Ok(())
        })
        .await?
    }

    pub fn component_kv(&self, component: &str) -> ComponentKv {
        ComponentKv {
            conn: Arc::clone(&self.conn),
            component: component.to_string(),
        }
    }

    /// Adapts bbolt's nested-bucket tree to SQLite's flat-table model:
    /// root -> one child per table -> one child per row (bucket label
    /// "<table>#<primary-key-value>", kv = that row's columns).
    pub async fn inspect(&self) -> Result<TroubleshootNode> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> Result<TroubleshootNode> {
            let conn = conn.lock().unwrap();
            let mut tables_stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            )?;
            let table_names: Vec<String> = tables_stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();

            let mut root = TroubleshootNode {
                bucket: "root".to_string(),
                kv: vec![],
                children: vec![],
            };

            for table in table_names {
                let mut table_node = TroubleshootNode {
                    bucket: table.clone(),
                    kv: vec![],
                    children: vec![],
                };

                let query = format!("SELECT * FROM {table}");
                let mut stmt = conn.prepare(&query)?;
                let col_names: Vec<String> =
                    stmt.column_names().iter().map(|s| s.to_string()).collect();
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let mut kv = vec![];
                    for (i, name) in col_names.iter().enumerate() {
                        let value: String = match row.get_ref(i)? {
                            rusqlite::types::ValueRef::Null => "NULL".to_string(),
                            rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                            rusqlite::types::ValueRef::Real(v) => v.to_string(),
                            rusqlite::types::ValueRef::Text(v) => {
                                String::from_utf8_lossy(v).to_string()
                            }
                            rusqlite::types::ValueRef::Blob(_) => "<blob>".to_string(),
                        };
                        kv.push((name.clone(), value));
                    }
                    let pk = kv.first().map(|(_, v)| v.clone()).unwrap_or_default();
                    table_node.children.push(TroubleshootNode {
                        bucket: format!("{table}#{pk}"),
                        kv,
                        children: vec![],
                    });
                }
                root.children.push(table_node);
            }

            Ok(root)
        })
        .await?
    }
}

pub struct ComponentKv {
    conn: Arc<Mutex<Connection>>,
    component: String,
}

impl ComponentKv {
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = Arc::clone(&self.conn);
        let component = self.component.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<String>> {
            let conn = conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT value FROM component_state WHERE component = ?1 AND key = ?2")?;
            let mut rows = stmt.query(rusqlite::params![component, key])?;
            if let Some(row) = rows.next()? {
                Ok(Some(row.get(0)?))
            } else {
                Ok(None)
            }
        })
        .await?
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let component = self.component.clone();
        let key = key.to_string();
        let value = value.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT INTO component_state (component, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT(component, key) DO UPDATE SET value = excluded.value",
                rusqlite::params![component, key, value],
            )?;
            Ok(())
        })
        .await?
    }
}
