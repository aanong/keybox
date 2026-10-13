use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppResult;

pub struct RawEntry {
    pub id: String,
    pub blob: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value BLOB NOT NULL);
             CREATE TABLE IF NOT EXISTS entries (
                 id TEXT PRIMARY KEY,
                 data BLOB NOT NULL,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );",
        )?;
        Ok(Self { conn })
    }

    pub fn get_meta(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &[u8]) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn list(&self) -> AppResult<Vec<RawEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, data, created_at, updated_at FROM entries")?;
        let rows = stmt.query_map([], |r| {
            Ok(RawEntry { id: r.get(0)?, blob: r.get(1)?, created_at: r.get(2)?, updated_at: r.get(3)? })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn upsert(&self, e: &RawEntry) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO entries(id, data, created_at, updated_at) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            params![e.id, e.blob, e.created_at, e.updated_at],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<usize> {
        Ok(self.conn.execute("DELETE FROM entries WHERE id = ?1", [id])?)
    }

    pub fn transaction<T>(&mut self, f: impl FnOnce(&Store) -> AppResult<T>) -> AppResult<T> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(v) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}
