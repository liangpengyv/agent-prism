// src-tauri/src/commands.rs

use crate::billing::BillingMatrix;
use crate::data_source::codex::reconciler::ReconcileResult;
use crate::data_source::codex::reconciler::reconcile;
use crate::data_source::codex::CodexSource;
use crate::data_source::{AgentSource, CommandResult, ThreadRecord};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ProjectStat {
    pub project: String,
    pub tokens: i64,
    pub cost_usd: f64,
}

#[derive(Serialize, Clone)]
pub struct ModelStat {
    pub model: String,
    pub tokens: i64,
    pub cost_usd: f64,
}

#[derive(Serialize, Clone)]
pub struct DayStat {
    pub date: String,
    pub tokens: i64,
    pub cost_usd: f64,
}

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

#[tauri::command]
pub fn get_by_project() -> CommandResult<Vec<ProjectStat>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (threads, warnings) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };

    let mut map: std::collections::HashMap<String, i64> = Default::default();
    for t in &threads {
        let key = t.cwd.split('/').last().unwrap_or(&t.cwd).to_string();
        *map.entry(key).or_insert(0) += t.tokens_used;
    }

    // 用 codex-mini 输入输出均价估算
    let avg_price = (1.5_f64 + 6.0_f64) / 2.0 / 1_000_000.0;

    let mut stats: Vec<ProjectStat> = map
        .into_iter()
        .map(|(project, tokens)| ProjectStat {
            cost_usd: tokens as f64 * avg_price,
            project,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    CommandResult::ok_with_warnings(stats, warnings)
}

#[tauri::command]
pub fn get_by_model() -> CommandResult<Vec<ModelStat>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (sessions, warnings) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };

    let matrix = BillingMatrix::new();

    let mut token_map: std::collections::HashMap<String, i64> = Default::default();
    let mut cost_map: std::collections::HashMap<String, f64> = Default::default();

    for s in &sessions {
        *token_map.entry(s.model_provider.clone()).or_insert(0) += s.total_tokens;
        let cost = matrix.estimate(std::slice::from_ref(s)).total_usd;
        *cost_map.entry(s.model_provider.clone()).or_insert(0.0) += cost;
    }

    let mut stats: Vec<ModelStat> = token_map
        .into_iter()
        .map(|(model, tokens)| ModelStat {
            cost_usd: *cost_map.get(&model).unwrap_or(&0.0),
            model,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    CommandResult::ok_with_warnings(stats, warnings)
}

#[tauri::command]
pub fn get_by_date() -> CommandResult<Vec<DayStat>> {
    use chrono::{Datelike, Duration, Utc};

    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };

    let (threads, warnings) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };

    let cutoff = Utc::now() - Duration::days(30);
    let avg_price = (1.5_f64 + 6.0_f64) / 2.0 / 1_000_000.0;

    let mut map: std::collections::BTreeMap<String, i64> = Default::default();
    for t in &threads {
        if t.updated_at < cutoff {
            continue;
        }
        let date = format!(
            "{:04}-{:02}-{:02}",
            t.updated_at.year(),
            t.updated_at.month(),
            t.updated_at.day()
        );
        *map.entry(date).or_insert(0) += t.tokens_used;
    }

    let stats: Vec<DayStat> = map
        .into_iter()
        .map(|(date, tokens)| DayStat {
            cost_usd: tokens as f64 * avg_price,
            date,
            tokens,
        })
        .collect();

    CommandResult::ok_with_warnings(stats, warnings)
}

#[tauri::command]
pub fn get_budget() -> CommandResult<Option<i64>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.get_budget_tokens() {
            Ok(v) => CommandResult::ok(v),
            Err(e) => CommandResult::err(format!("读取预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn set_budget(tokens: i64) -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.set_budget_tokens(tokens) {
            Ok(_) => CommandResult::ok("预算已保存".to_string()),
            Err(e) => CommandResult::err(format!("保存预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}
