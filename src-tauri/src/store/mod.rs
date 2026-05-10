use anyhow::Result;
use indexmap::IndexMap;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::billing::ModelPrice;

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
        if let Err(e) = store.migrate_legacy_keys() {
            eprintln!("[store] 旧数据迁移失败（不阻断启动）: {e}");
        }
        Ok(store)
    }

    pub fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn migrate_legacy_keys(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "INSERT OR IGNORE INTO meta (key, value)
                SELECT 'budget_tokens_codex', value FROM meta WHERE key = 'budget_tokens';
            DELETE FROM meta WHERE key = 'budget_tokens';
            INSERT OR IGNORE INTO meta (key, value)
                SELECT 'custom_prices_codex', value FROM meta WHERE key = 'custom_prices';
            DELETE FROM meta WHERE key = 'custom_prices';"
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
        self.get_meta_i64("last_refresh")
    }

    pub fn set_budget_tokens(&self, agent: &str, tokens: i64) -> Result<()> {
        let key = format!("budget_tokens_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, tokens.to_string()],
        )?;
        Ok(())
    }

    pub fn get_budget_tokens(&self, agent: &str) -> Result<Option<i64>> {
        let key = format!("budget_tokens_{}", agent);
        self.get_meta_i64(&key)
    }

    pub fn set_prices(&self, agent: &str, prices: &IndexMap<String, ModelPrice>) -> Result<()> {
        let key = format!("custom_prices_{}", agent);
        let json = serde_json::to_string(prices)?;
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, json],
        )?;
        Ok(())
    }

    pub fn get_prices(&self, agent: &str) -> Result<Option<IndexMap<String, ModelPrice>>> {
        let key = format!("custom_prices_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(serde_json::from_str(&s).ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_prices(&self, agent: &str) -> Result<()> {
        let key = format!("custom_prices_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_last_selected_agent(&self) -> Result<Option<String>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'last_selected_agent'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_last_selected_agent(&self, agent: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_selected_agent', ?1)",
            params![agent],
        )?;
        Ok(())
    }

    fn get_meta_i64(&self, key: &str) -> Result<Option<i64>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(s.parse().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_store_with_file() -> (AppStore, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let store = AppStore { db_path: file.path().to_path_buf() };
        store.init().unwrap();
        (store, file)
    }

    #[test]
    fn test_budget_per_agent() {
        let (store, _f) = make_store_with_file();
        store.set_budget_tokens("codex", 10_000_000).unwrap();
        store.set_budget_tokens("claude-code", 1_000_000_000).unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(10_000_000));
        assert_eq!(store.get_budget_tokens("claude-code").unwrap(), Some(1_000_000_000));
    }

    #[test]
    fn test_last_selected_agent() {
        let (store, _f) = make_store_with_file();
        assert_eq!(store.get_last_selected_agent().unwrap(), None);
        store.set_last_selected_agent("claude-code").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("claude-code".to_string()));
        store.set_last_selected_agent("codex").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("codex".to_string()));
    }

    #[test]
    fn test_migrate_legacy_keys_idempotent() {
        let (store, _f) = make_store_with_file();
        let conn = rusqlite::Connection::open(&store.db_path).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('budget_tokens', '9999')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('custom_prices', '{}')",
            [],
        ).unwrap();
        drop(conn);

        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));

        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));
    }
}
