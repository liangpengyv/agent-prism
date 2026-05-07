// src-tauri/src/data_source/codex/reconciler.rs

use crate::data_source::{SessionRecord, ThreadRecord};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReconcileResult {
    pub sqlite_total: i64,
    pub jsonl_total: i64,
    pub diff: i64,
    pub diff_rate: f64,
    pub warning: Option<String>,
}

pub fn reconcile(threads: &[ThreadRecord], sessions: &[SessionRecord]) -> ReconcileResult {
    let sqlite_total: i64 = threads.iter().map(|t| t.tokens_used).sum();
    let jsonl_total: i64 = sessions.iter().map(|s| s.total_tokens).sum();
    let diff = jsonl_total - sqlite_total;

    let diff_rate = if sqlite_total == 0 {
        if jsonl_total == 0 { 0.0 } else { 1.0 }
    } else {
        diff.abs() as f64 / sqlite_total as f64
    };

    let warning = if diff_rate > 0.05 {
        Some(format!(
            "对账差异率 {:.1}%，SQLite={} JSONL={}，部分数据可能不完整",
            diff_rate * 100.0, sqlite_total, jsonl_total
        ))
    } else {
        None
    };

    ReconcileResult { sqlite_total, jsonl_total, diff, diff_rate, warning }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_thread(tokens: i64) -> ThreadRecord {
        ThreadRecord {
            id: "t1".into(), title: "".into(), cwd: "".into(),
            model: "".into(), model_provider: "".into(),
            tokens_used: tokens,
            created_at: Utc::now(), updated_at: Utc::now(),
            source: "codex".into(),
        }
    }

    fn make_session(total: i64) -> SessionRecord {
        SessionRecord {
            session_id: "s1".into(), cwd: "".into(), model_provider: "".into(),
            input_tokens: 0, cached_input_tokens: 0, output_tokens: 0,
            reasoning_output_tokens: 0, total_tokens: total,
            source: "codex".into(),
        }
    }

    #[test]
    fn test_no_diff() {
        let threads = vec![make_thread(1000), make_thread(500)];
        let sessions = vec![make_session(1500)];
        let r = reconcile(&threads, &sessions);
        assert_eq!(r.sqlite_total, 1500);
        assert_eq!(r.jsonl_total, 1500);
        assert_eq!(r.diff, 0);
        assert!((r.diff_rate - 0.0).abs() < f64::EPSILON);
        assert!(r.warning.is_none());
    }

    #[test]
    fn test_large_diff_triggers_warning() {
        let threads = vec![make_thread(1000)];
        let sessions = vec![make_session(1100)];
        let r = reconcile(&threads, &sessions);
        assert!(r.diff_rate > 0.05);
        assert!(r.warning.is_some());
    }

    #[test]
    fn test_zero_sqlite_total() {
        let r = reconcile(&[], &[make_session(100)]);
        assert_eq!(r.sqlite_total, 0);
        assert_eq!(r.jsonl_total, 100);
        assert!(r.warning.is_some());
    }
}
