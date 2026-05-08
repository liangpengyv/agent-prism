// src-tauri/src/data_source/codex/jsonl.rs

use crate::data_source::SessionRecord;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn read_sessions(codex_dir: &Path) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
    let pattern = codex_dir
        .join("sessions")
        .join("**")
        .join("*.jsonl")
        .to_string_lossy()
        .to_string();

    let mut all_sessions = Vec::new();
    let mut all_warnings = Vec::new();

    for entry in glob::glob(&pattern)?.filter_map(|e| e.ok()) {
        match parse_session_file(&entry) {
            Ok((Some(record), mut warns)) => {
                all_sessions.push(record);
                all_warnings.append(&mut warns);
            }
            Ok((None, mut warns)) => {
                all_warnings.append(&mut warns);
            }
            Err(e) => {
                all_warnings.push(format!("无法解析 {:?}: {e}", entry));
            }
        }
    }

    Ok((all_sessions, all_warnings))
}

fn parse_session_file(path: &Path) -> anyhow::Result<(Option<SessionRecord>, Vec<String>)> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut cwd = String::new();
    let mut model_provider = String::new();
    let mut last_token_count: Option<Value> = None;
    let mut warnings = Vec::new();
    let mut line_count = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(e) => {
                warnings.push(format!("读取行失败: {e}"));
                continue;
            }
        };

        line_count += 1;

        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!("JSON 解析失败: {e}"));
                continue;
            }
        };

        let msg_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        // 只在前5行打印原始内容，便于诊断字段名
        if line_count <= 5 {
            eprintln!("[JSONL] {:?} line#{} type={} keys={:?}",
                path.file_name().unwrap_or_default(),
                line_count,
                msg_type,
                v.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
            );
            if msg_type == "session_meta" {
                eprintln!("  session_meta full: {}", &line[..line.len().min(300)]);
            }
            if v.pointer("/payload/type").and_then(|t| t.as_str()) == Some("token_count") {
                eprintln!("  token_count payload keys={:?}",
                    v.pointer("/payload").and_then(|p| p.as_object())
                        .map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
                );
                eprintln!("  token_count payload: {}", v.get("payload").map(|p| p.to_string()).unwrap_or_default());
            }
        }

        match msg_type {
            "session_meta" => {
                // session_meta 的字段在 payload 子对象里
                if let Some(payload) = v.get("payload") {
                    if let Some(id) = payload.get("id").and_then(|s| s.as_str()) {
                        session_id = id.to_string();
                    }
                    if let Some(c) = payload.get("cwd").and_then(|s| s.as_str()) {
                        cwd = c.to_string();
                    }
                    // 尝试多个可能的字段名
                    if let Some(mp) = payload.get("model_provider").and_then(|s| s.as_str())
                        .or_else(|| payload.get("model").and_then(|s| s.as_str()))
                        .or_else(|| payload.get("provider").and_then(|s| s.as_str()))
                    {
                        model_provider = mp.to_string();
                    }
                }
            }
            "event_msg" => {
                if v.pointer("/payload/type").and_then(|t| t.as_str()) == Some("token_count") {
                    last_token_count = v.get("payload").cloned();
                }
            }
            _ => {}
        }
    }

    let record = last_token_count.map(|tc| {
        // 尝试多个可能的 total_tokens 字段名
        let total_tokens = tc.get("total_tokens").and_then(|v| v.as_i64())
            .or_else(|| tc.get("total").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let input_tokens = tc.get("input_tokens").and_then(|v| v.as_i64())
            .or_else(|| tc.get("input").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let output_tokens = tc.get("output_tokens").and_then(|v| v.as_i64())
            .or_else(|| tc.get("output").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let cached_input_tokens = tc.get("cached_input_tokens").and_then(|v| v.as_i64())
            .or_else(|| tc.get("cached_tokens").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let reasoning_output_tokens = tc.get("reasoning_output_tokens").and_then(|v| v.as_i64())
            .unwrap_or(0);

        eprintln!("[JSONL] parsed session={} model={} input={} output={} total={}",
            session_id, model_provider, input_tokens, output_tokens, total_tokens);

        SessionRecord {
            session_id,
            cwd,
            model_provider,
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_output_tokens,
            total_tokens,
            source: "codex".to_string(),
        }
    });

    Ok((record, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_session_file(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join("sessions").join("proj");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(name), content).unwrap();
    }

    #[test]
    fn test_takes_last_token_count() {
        let dir = TempDir::new().unwrap();
        make_session_file(&dir, "s1.jsonl", r#"
{"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":"s1","cwd":"/proj","model_provider":"openai"}}
{"type":"event_msg","payload":{"type":"token_count","input_tokens":100,"cached_input_tokens":20,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}}
{"type":"event_msg","payload":{"type":"token_count","input_tokens":200,"cached_input_tokens":40,"output_tokens":80,"reasoning_output_tokens":10,"total_tokens":280}}
"#);
        let (sessions, warnings) = read_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].total_tokens, 280);
        assert_eq!(sessions[0].input_tokens, 200);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_skips_invalid_lines() {
        let dir = TempDir::new().unwrap();
        make_session_file(&dir, "s2.jsonl", r#"
{"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":"s2","cwd":"/proj","model_provider":"openai"}}
not valid json
{"type":"event_msg","payload":{"type":"token_count","input_tokens":100,"cached_input_tokens":0,"output_tokens":50,"reasoning_output_tokens":0,"total_tokens":150}}
"#);
        let (sessions, warnings) = read_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_no_token_count_returns_none() {
        let dir = TempDir::new().unwrap();
        make_session_file(&dir, "s3.jsonl", r#"
{"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":"s3","cwd":"/proj","model_provider":"openai"}}
{"type":"event_msg","payload":{"type":"other_event"}}
"#);
        let (sessions, _) = read_sessions(dir.path()).unwrap();
        assert!(sessions.is_empty());
    }
}
