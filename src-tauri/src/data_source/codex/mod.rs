// src-tauri/src/data_source/codex/mod.rs
pub mod sqlite;
pub mod jsonl;
pub mod reconciler;

use crate::data_source::{AgentSource, SessionRecord, ThreadRecord};
use glob::glob;
use std::path::PathBuf;

pub struct CodexSource {
    pub base_dir: PathBuf,
}

impl CodexSource {
    pub fn new() -> Option<Self> {
        let base = dirs::home_dir()?.join(".codex");
        if base.exists() { Some(Self { base_dir: base }) } else { None }
    }
}

impl AgentSource for CodexSource {
    fn name(&self) -> &str { "codex" }

    fn discover(&self) -> anyhow::Result<Vec<PathBuf>> {
        let pattern = self.base_dir.join("state_*.sqlite").to_string_lossy().to_string();
        let files: Vec<PathBuf> = glob(&pattern)?.filter_map(|e| e.ok()).collect();
        Ok(files)
    }

    fn threads(&self) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)> {
        let db_files = self.discover()?;
        let mut all_threads = Vec::new();
        let mut all_warnings = Vec::new();

        for db in db_files {
            match sqlite::read_threads(&db) {
                Ok((mut threads, mut warnings)) => {
                    all_threads.append(&mut threads);
                    all_warnings.append(&mut warnings);
                }
                Err(e) => {
                    all_warnings.push(format!("无法读取 {:?}: {e}", db));
                }
            }
        }

        Ok((all_threads, all_warnings))
    }

    fn sessions(&self) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
        jsonl::read_sessions(&self.base_dir)
    }
}
