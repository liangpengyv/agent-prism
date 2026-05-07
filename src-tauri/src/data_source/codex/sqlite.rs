// src-tauri/src/data_source/codex/sqlite.rs

use crate::data_source::ThreadRecord;
use chrono::{TimeZone, Utc};
use rusqlite::Connection;
use std::path::Path;

pub fn read_threads(db_path: &Path) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, title, cwd, model, model_provider, tokens_used,
                created_at, updated_at
         FROM threads
         ORDER BY updated_at DESC"
    )?;

    let mut threads = Vec::new();
    let mut warnings = Vec::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<i64>>(6)?,
            row.get::<_, Option<i64>>(7)?,
        ))
    })?;

    for row in rows {
        match row {
            Ok((id, title, cwd, model, model_provider, tokens_used, created_at, updated_at)) => {
                threads.push(ThreadRecord {
                    id,
                    title: title.unwrap_or_default(),
                    cwd: cwd.unwrap_or_default(),
                    model: model.unwrap_or_default(),
                    model_provider: model_provider.unwrap_or_default(),
                    tokens_used: tokens_used.unwrap_or(0),
                    created_at: created_at
                        .map(|ts| Utc.timestamp_opt(ts, 0).single().unwrap_or_default())
                        .unwrap_or_default(),
                    updated_at: updated_at
                        .map(|ts| Utc.timestamp_opt(ts, 0).single().unwrap_or_default())
                        .unwrap_or_default(),
                    source: "codex".to_string(),
                });
            }
            Err(e) => {
                warnings.push(format!("跳过一条记录: {e}"));
            }
        }
    }

    Ok((threads, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn make_test_db() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY,
                title TEXT,
                cwd TEXT,
                model TEXT,
                model_provider TEXT,
                tokens_used INTEGER,
                created_at INTEGER,
                updated_at INTEGER,
                source TEXT
            );
            INSERT INTO threads VALUES (
                'thread-001', 'Test Thread', '/home/user/project',
                'codex-mini', 'openai', 1500,
                1714000000, 1714003600, 'cli'
            );",
        ).unwrap();
        file
    }

    #[test]
    fn test_read_threads_normal() {
        let db = make_test_db();
        let (threads, warnings) = read_threads(db.path()).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, "thread-001");
        assert_eq!(threads[0].tokens_used, 1500);
        assert_eq!(threads[0].model, "codex-mini");
        assert_eq!(threads[0].source, "codex");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_read_threads_empty_db() {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (
                id TEXT PRIMARY KEY, title TEXT, cwd TEXT,
                model TEXT, model_provider TEXT, tokens_used INTEGER,
                created_at INTEGER, updated_at INTEGER, source TEXT
            );"
        ).unwrap();
        let (threads, warnings) = read_threads(file.path()).unwrap();
        assert!(threads.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_read_threads_missing_file() {
        let result = read_threads(Path::new("/nonexistent/path/state.sqlite"));
        assert!(result.is_err());
    }
}
