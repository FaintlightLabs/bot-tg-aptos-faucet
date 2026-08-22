use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let parent = path
            .as_ref()
            .parent()
            .context("db path has no parent directory")?;
        std::fs::create_dir_all(parent)?;

        let conn = Connection::open(&path)
            .with_context(|| format!("cannot open db at {:?}", path.as_ref()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_calls (
                user_id INTEGER PRIMARY KEY,
                last_call TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_locales (
                user_id INTEGER PRIMARY KEY,
                locale TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_last_call(&self, user_id: u64) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT last_call FROM user_calls WHERE user_id = ?")?;
        let row: Option<String> = stmt
            .query_row(params![user_id as i64], |row| row.get(0))
            .optional()?;
        row.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .context("invalid last_call timestamp")
        })
        .transpose()
    }

    pub fn set_last_call(&self, user_id: u64, date: DateTime<Utc>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO user_calls (user_id, last_call) VALUES (?, ?)",
            params![user_id as i64, date.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_locale(&self, user_id: u64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT locale FROM user_locales WHERE user_id = ?")?;
        stmt.query_row(params![user_id as i64], |row| row.get(0))
            .optional()
            .context("failed to get locale")
    }

    pub fn set_locale(&self, user_id: u64, locale: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO user_locales (user_id, locale) VALUES (?, ?)",
            params![user_id as i64, locale],
        )?;
        Ok(())
    }
}
