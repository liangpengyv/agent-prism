// src-tauri/src/data_source/codex/jsonl.rs
use crate::data_source::SessionRecord;
use std::path::Path;

pub fn read_sessions(_codex_dir: &Path) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
    Ok((vec![], vec![]))
}
