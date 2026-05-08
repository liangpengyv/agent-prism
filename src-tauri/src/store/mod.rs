use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

pub struct AppStore {
    pub db_path: PathBuf,
}

impl AppStore {
    pub fn new() -> Result<Self> {
        let dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("无法找到 home 目录"))?
            .join(".agent-prism");
        std::fs::create_dir_all(&dir)?;
        let db_path = dir.join("cache.db");
        let store = Self { db_path };
        store.init()?;
        Ok(store)
    }

    fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn set_last_refresh(&self, ts: i64) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_refresh', ?1)",
            params![ts.to_string()],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_last_refresh(&self) -> Result<Option<i64>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'last_refresh'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(s.parse().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}