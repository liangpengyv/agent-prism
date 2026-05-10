// src-tauri/src/commands.rs

use crate::billing::{BillingMatrix, ModelPrice};
use crate::data_source::codex::reconciler::{reconcile, ReconcileResult};
use crate::data_source::codex::CodexSource;
use crate::data_source::claude::ClaudeCodeSource;
use crate::data_source::{AgentSource, CommandResult, ThreadRecord};
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::HashMap;

fn load_matrix(agent: &str) -> BillingMatrix {
    use crate::store::AppStore;
    if let Ok(store) = AppStore::new() {
        if let Ok(Some(prices)) = store.get_prices(agent) {
            return BillingMatrix::with_prices(prices);
        }
    }
    BillingMatrix::new_for_agent(agent)
}

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

// ── Codex 内部实现 ──────────────────────────────────────────

fn get_summary_codex() -> CommandResult<SummaryData> {
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
    let matrix = load_matrix("codex");
    let cost = matrix.estimate(&sessions);
    let top_project = {
        let mut m: HashMap<String, i64> = Default::default();
        for t in &threads {
            *m.entry(t.cwd.clone()).or_insert(0) += t.tokens_used;
        }
        m.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
    };
    CommandResult::ok_with_warnings(
        SummaryData {
            total_tokens: reconcile_result.sqlite_total,
            thread_count: threads.len(),
            session_count: sessions.len(),
            estimated_cost_usd: cost.total_usd,
            top_project,
            reconcile: reconcile_result,
        },
        warnings,
    )
}

fn get_summary_claude_code() -> CommandResult<SummaryData> {
    let source = match ClaudeCodeSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.claude 目录"),
    };
    let mut warnings = Vec::new();
    let (sessions, mut s_warns) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };
    warnings.append(&mut s_warns);
    let matrix = load_matrix("claude-code");
    let cost = matrix.estimate(&sessions);
    let top_project = {
        let mut m: HashMap<String, i64> = Default::default();
        for s in &sessions {
            *m.entry(s.cwd.clone()).or_insert(0) += s.total_tokens;
        }
        m.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
    };
    let total_tokens: i64 = sessions.iter().map(|s| s.total_tokens).sum();
    // reconcile 不适用：diff_rate = -1.0 作为 sentinel
    let na_reconcile = ReconcileResult {
        sqlite_total: 0,
        jsonl_total: total_tokens,
        diff: 0,
        diff_rate: -1.0,
        warning: None,
    };
    CommandResult::ok_with_warnings(
        SummaryData {
            total_tokens,
            thread_count: 0,
            session_count: sessions.len(),
            estimated_cost_usd: cost.total_usd,
            top_project,
            reconcile: na_reconcile,
        },
        warnings,
    )
}

// ── Public Commands ─────────────────────────────────────────

#[tauri::command]
pub fn get_summary(agent: String) -> CommandResult<SummaryData> {
    match agent.as_str() {
        "codex" => get_summary_codex(),
        "claude-code" => get_summary_claude_code(),
        _ => CommandResult::err(format!("未知 Agent: {agent}")),
    }
}

#[tauri::command]
pub fn get_threads(agent: String) -> CommandResult<Vec<ThreadRecord>> {
    if agent != "codex" {
        return CommandResult::ok(vec![]);
    }
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
pub fn get_by_project(agent: String) -> CommandResult<Vec<ProjectStat>> {
    match agent.as_str() {
        "codex" => get_by_project_codex(),
        "claude-code" => get_by_project_claude_code(),
        _ => CommandResult::err(format!("未知 Agent: {agent}")),
    }
}

fn get_by_project_codex() -> CommandResult<Vec<ProjectStat>> {
    let source = match CodexSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.codex 目录"),
    };
    let (threads, mut warnings) = match source.threads() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
    };
    let (sessions, mut s_warns) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };
    warnings.append(&mut s_warns);
    let matrix = load_matrix("codex");
    let mut token_map: HashMap<String, i64> = Default::default();
    for t in &threads {
        let key = t.cwd.split('/').last().unwrap_or(&t.cwd).to_string();
        *token_map.entry(key).or_insert(0) += t.tokens_used;
    }
    let total_cost: f64 = sessions
        .iter()
        .map(|s| matrix.estimate(std::slice::from_ref(s)).total_usd)
        .sum();
    let total_tokens: i64 = threads.iter().map(|t| t.tokens_used).sum();
    let global_per_token = if total_tokens > 0 {
        total_cost / total_tokens as f64
    } else {
        matrix.fallback_avg_per_token()
    };
    let mut stats: Vec<ProjectStat> = token_map
        .into_iter()
        .map(|(project, tokens)| ProjectStat {
            cost_usd: tokens as f64 * global_per_token,
            project,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    CommandResult::ok_with_warnings(stats, warnings)
}

fn get_by_project_claude_code() -> CommandResult<Vec<ProjectStat>> {
    let source = match ClaudeCodeSource::new() {
        Some(s) => s,
        None => return CommandResult::err("未检测到 ~/.claude 目录"),
    };
    let (sessions, warnings) = match source.sessions() {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };
    let matrix = load_matrix("claude-code");
    let mut token_map: HashMap<String, i64> = Default::default();
    let mut cost_map: HashMap<String, f64> = Default::default();
    for s in &sessions {
        *token_map.entry(s.cwd.clone()).or_insert(0) += s.total_tokens;
        let cost = matrix.estimate(std::slice::from_ref(s)).total_usd;
        *cost_map.entry(s.cwd.clone()).or_insert(0.0) += cost;
    }
    let mut stats: Vec<ProjectStat> = token_map
        .into_iter()
        .map(|(project, tokens)| ProjectStat {
            cost_usd: *cost_map.get(&project).unwrap_or(&0.0),
            project,
            tokens,
        })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    CommandResult::ok_with_warnings(stats, warnings)
}

#[tauri::command]
pub fn get_by_model(agent: String) -> CommandResult<Vec<ModelStat>> {
    match agent.as_str() {
        "codex" => get_by_model_impl("codex"),
        "claude-code" => get_by_model_impl("claude-code"),
        _ => CommandResult::err(format!("未知 Agent: {agent}")),
    }
}

fn get_by_model_impl(agent: &str) -> CommandResult<Vec<ModelStat>> {
    let sessions_result = if agent == "codex" {
        CodexSource::new()
            .map(|s| s.sessions())
            .ok_or_else(|| anyhow::anyhow!("未检测到 ~/.codex 目录"))
            .and_then(|r| r)
    } else {
        ClaudeCodeSource::new()
            .map(|s| s.sessions())
            .ok_or_else(|| anyhow::anyhow!("未检测到 ~/.claude 目录"))
            .and_then(|r| r)
    };
    let (sessions, warnings) = match sessions_result {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };
    let matrix = load_matrix(agent);
    let mut token_map: HashMap<String, i64> = Default::default();
    let mut cost_map: HashMap<String, f64> = Default::default();
    for s in &sessions {
        *token_map.entry(s.model.clone()).or_insert(0) += s.total_tokens;
        let cost = matrix.estimate(std::slice::from_ref(s)).total_usd;
        *cost_map.entry(s.model.clone()).or_insert(0.0) += cost;
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
pub fn get_by_date(agent: String) -> CommandResult<Vec<DayStat>> {
    use chrono::{Datelike, Duration, Utc};
    let cutoff = Utc::now() - Duration::days(30);

    if agent == "codex" {
        let source = match CodexSource::new() {
            Some(s) => s,
            None => return CommandResult::err("未检测到 ~/.codex 目录"),
        };
        let (threads, warnings) = match source.threads() {
            Ok(r) => r,
            Err(e) => return CommandResult::err(format!("读取 threads 失败: {e}")),
        };
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
            .map(|(date, tokens)| DayStat { date, tokens })
            .collect();
        return CommandResult::ok_with_warnings(stats, warnings);
    }

    if agent == "claude-code" {
        use chrono::Datelike;
        let source = match ClaudeCodeSource::new() {
            Some(s) => s,
            None => return CommandResult::err("未检测到 ~/.claude 目录"),
        };
        let (sessions, warnings) = match source.sessions() {
            Ok(r) => r,
            Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
        };
        let mut map: std::collections::BTreeMap<String, i64> = Default::default();
        for s in &sessions {
            if s.created_at < cutoff {
                continue;
            }
            let date = format!(
                "{:04}-{:02}-{:02}",
                s.created_at.year(),
                s.created_at.month(),
                s.created_at.day()
            );
            *map.entry(date).or_insert(0) += s.total_tokens;
        }
        let stats: Vec<DayStat> = map
            .into_iter()
            .map(|(date, tokens)| DayStat { date, tokens })
            .collect();
        return CommandResult::ok_with_warnings(stats, warnings);
    }

    CommandResult::err(format!("未知 Agent: {agent}"))
}

#[tauri::command]
pub fn get_budget(agent: String) -> CommandResult<Option<i64>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.get_budget_tokens(&agent) {
            Ok(v) => CommandResult::ok(v),
            Err(e) => CommandResult::err(format!("读取预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn set_budget(agent: String, tokens: i64) -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.set_budget_tokens(&agent, tokens) {
            Ok(_) => CommandResult::ok("预算已保存".to_string()),
            Err(e) => CommandResult::err(format!("保存预算失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn get_prices(agent: String) -> CommandResult<IndexMap<String, ModelPrice>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.get_prices(&agent) {
            Ok(Some(prices)) => CommandResult::ok(prices),
            Ok(None) => CommandResult::ok(BillingMatrix::new_for_agent(&agent).prices),
            Err(e) => CommandResult::err(format!("读取价格表失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn set_prices(agent: String, prices: IndexMap<String, ModelPrice>) -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.set_prices(&agent, &prices) {
            Ok(_) => CommandResult::ok("价格表已保存".to_string()),
            Err(e) => CommandResult::err(format!("保存价格表失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn reset_prices(agent: String) -> CommandResult<IndexMap<String, ModelPrice>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.delete_prices(&agent) {
            Ok(_) => CommandResult::ok(BillingMatrix::new_for_agent(&agent).prices),
            Err(e) => CommandResult::err(format!("重置价格表失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn get_last_selected_agent() -> CommandResult<Option<String>> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.get_last_selected_agent() {
            Ok(v) => CommandResult::ok(v),
            Err(e) => CommandResult::err(format!("读取 agent 失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn set_last_selected_agent(agent: String) -> CommandResult<String> {
    use crate::store::AppStore;
    match AppStore::new() {
        Ok(store) => match store.set_last_selected_agent(&agent) {
            Ok(_) => CommandResult::ok("已保存".to_string()),
            Err(e) => CommandResult::err(format!("保存 agent 失败: {e}")),
        },
        Err(e) => CommandResult::err(format!("初始化 store 失败: {e}")),
    }
}

#[tauri::command]
pub fn get_app_version() -> CommandResult<String> {
    CommandResult::ok(env!("CARGO_PKG_VERSION").to_string())
}

#[derive(Serialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub latest_version: String,
    pub release_url: String,
}

#[tauri::command]
pub fn check_update() -> CommandResult<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION");
    let repo = "liangpengyv/agent-prism";
    let api_url = format!("https://api.github.com/repos/{repo}/releases");

    let response = match ureq::get(&api_url)
        .set("User-Agent", "AgentPrism")
        .call()
    {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("请求失败: {e}")),
    };

    let json: serde_json::Value = match response.into_json() {
        Ok(v) => v,
        Err(e) => return CommandResult::err(format!("解析响应失败: {e}")),
    };

    let latest = match json.as_array().and_then(|arr| arr.first()) {
        Some(r) => r,
        None => return CommandResult::err("暂无发布版本".to_string()),
    };

    let tag = latest.get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    let html_url = latest.get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let has_update = !tag.is_empty() && tag != current;

    CommandResult::ok(UpdateInfo {
        has_update,
        latest_version: tag,
        release_url: html_url,
    })
}
