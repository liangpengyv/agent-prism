// src-tauri/src/commands.rs

use crate::billing::BillingMatrix;
use crate::data_source::codex::reconciler::ReconcileResult;
use crate::data_source::codex::reconciler::reconcile;
use crate::data_source::codex::CodexSource;
use crate::data_source::{AgentSource, CommandResult, ThreadRecord};
use serde::Serialize;

#[derive(Serialize)]
pub struct SummaryData {
    pub total_tokens: i64,
    pub thread_count: usize,
    pub session_count: usize,
    pub estimated_cost_usd: f64,
    pub top_project: Option<String>,
    pub reconcile: ReconcileResult,
}

#[tauri::command]
pub fn get_summary() -> CommandResult<SummaryData> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let mut warnings = Vec::new();

    let (threads, mut t_warns) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };
    warnings.append(&mut t_warns);

    let (sessions, mut s_warns) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };
    warnings.append(&mut s_warns);

    let reconcile_result = reconcile(&threads, &sessions);
    if let Some(ref w) = reconcile_result.warning {
        warnings.push(w.clone());
    }

    let matrix = BillingMatrix::new();
    let cost = matrix.estimate(&sessions);

    let top_project = {
        let mut project_tokens: std::collections::HashMap<String, i64> = Default::default();
        for t in &threads {
            *project_tokens.entry(t.cwd.clone()).or_insert(0) += t.tokens_used;
        }
        project_tokens.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
    };

    let data = SummaryData {
        total_tokens: reconcile_result.sqlite_total,
        thread_count: threads.len(),
        session_count: sessions.len(),
        estimated_cost_usd: cost.total_usd,
        top_project,
        reconcile: reconcile_result,
    };

    CommandResult::ok_with_warnings(data, warnings)
}

#[tauri::command]
pub fn get_threads() -> CommandResult<Vec<ThreadRecord>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    match source.threads() {
        Ok((threads, warnings)) => CommandResult::ok_with_warnings(threads, warnings),
        Err(e) => CommandResult::err(format!("读取 threads 失败: {e}")),
    }
}

#[tauri::command]
pub fn refresh() -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => {
            let ts = chrono::Utc::now().timestamp();
            match store.set_last_refresh(ts) {
                Ok(_) => CommandResult::ok("刷新完成".to_string()),
                Err(e) => CommandResult::err(format!("写入缓存失败: {e}")),
            }
        }
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}
