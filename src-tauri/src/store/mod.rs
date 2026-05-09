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

    pub fn set_budget_tokens(&self, tokens: i64) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('budget_tokens', ?1)",
            params![tokens.to_string()],
        )?;
        Ok(())
    }

    pub fn get_budget_tokens(&self) -> Result<Option<i64>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'budget_tokens'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(s.parse().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 将自定义价格表序列化为 JSON 存入 meta 表
    pub fn set_prices(&self, prices: &IndexMap<String, ModelPrice>) -> Result<()> {
        let json = serde_json::to_string(prices)?;
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('custom_prices', ?1)",
            params![json],
        )?;
        Ok(())
    }

    /// 读取自定义价格表，不存在则返回 None（由调用方决定是否回退到默认）
    pub fn get_prices(&self) -> Result<Option<IndexMap<String, ModelPrice>>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'custom_prices'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(serde_json::from_str(&s).ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 删除自定义价格表，恢复为预设值
    pub fn delete_prices(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "DELETE FROM meta WHERE key = 'custom_prices'",
            [],
        )?;
        Ok(())
    }
}