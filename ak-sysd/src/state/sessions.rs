use eyre::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

/// Mirrors the generated `StateSession` proto message shape (field names
/// match exactly): id, username, token_hash, expires_at, created_at, pid,
/// ppid, local_socket, opened.
#[derive(Debug, Clone, Default)]
pub struct SessionRecord {
    pub id: String,
    pub username: String,
    pub token_hash: String,
    /// Unix seconds; `None` means never expires.
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub pid: Option<u32>,
    pub ppid: Option<u32>,
    pub local_socket: Option<String>,
    pub opened: bool,
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        id: row.get(0)?,
        username: row.get(1)?,
        token_hash: row.get(2)?,
        expires_at: row.get(3)?,
        created_at: row.get(4)?,
        pid: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
        ppid: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
        local_socket: row.get(7)?,
        opened: row.get::<_, i64>(8)? != 0,
    })
}

const SELECT_COLS: &str =
    "id, username, token_hash, expires_at, created_at, pid, ppid, local_socket, opened";

pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub(super) fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub async fn insert(&self, s: &SessionRecord) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let s = s.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT INTO sessions (id, username, token_hash, expires_at, created_at, pid, ppid, local_socket, opened)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    s.id,
                    s.username,
                    s.token_hash,
                    s.expires_at,
                    s.created_at,
                    s.pid.map(|v| v as i64),
                    s.ppid.map(|v| v as i64),
                    s.local_socket,
                    s.opened as i64,
                ],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn get(&self, id: &str) -> Result<Option<SessionRecord>> {
        let conn = Arc::clone(&self.conn);
        let id = id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<SessionRecord>> {
            let conn = conn.lock().unwrap();
            let query = format!("SELECT {SELECT_COLS} FROM sessions WHERE id = ?1");
            Ok(conn
                .query_row(&query, params![id], row_to_record)
                .optional()?)
        })
        .await?
    }

    pub async fn update(&self, s: &SessionRecord) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let s = s.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT INTO sessions (id, username, token_hash, expires_at, created_at, pid, ppid, local_socket, opened)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    username = excluded.username,
                    token_hash = excluded.token_hash,
                    expires_at = excluded.expires_at,
                    pid = excluded.pid,
                    ppid = excluded.ppid,
                    local_socket = excluded.local_socket,
                    opened = excluded.opened",
                params![
                    s.id,
                    s.username,
                    s.token_hash,
                    s.expires_at,
                    s.created_at,
                    s.pid.map(|v| v as i64),
                    s.ppid.map(|v| v as i64),
                    s.local_socket,
                    s.opened as i64,
                ],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let id = id.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().unwrap();
            conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
            Ok(())
        })
        .await?
    }

    pub async fn all_opened(&self) -> Result<Vec<SessionRecord>> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || -> Result<Vec<SessionRecord>> {
            let conn = conn.lock().unwrap();
            let query = format!("SELECT {SELECT_COLS} FROM sessions WHERE opened != 0");
            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], row_to_record)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .await?
    }
}
