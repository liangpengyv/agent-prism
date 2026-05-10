use crate::data_source::{AgentSource, SessionRecord, ThreadRecord};
use glob::glob;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub struct ClaudeCodeSource {
    pub base_dir: PathBuf,
}

impl ClaudeCodeSource {
    pub fn new() -> Option<Self> {
        let base = dirs::home_dir()?.join(".claude");
        if base.exists() { Some(Self { base_dir: base }) } else { None }
    }
}

impl AgentSource for ClaudeCodeSource {
    fn name(&self) -> &str { "claude-code" }

    fn discover(&self) -> anyhow::Result<Vec<PathBuf>> {
        let pattern = self.base_dir
            .join("projects")
            .join("**")
            .join("*.jsonl")
            .to_string_lossy()
            .to_string();
        let files: Vec<PathBuf> = glob(&pattern)?.filter_map(|e| e.ok()).collect();
        Ok(files)
    }

    fn threads(&self) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)> {
        Ok((vec![], vec![]))
    }

    fn sessions(&self) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
        let files = self.discover()?;
        let mut all_sessions = Vec::new();
        let mut all_warnings = Vec::new();

        for file in files {
            match parse_session_file(&file) {
                Ok((Some(record), mut warns)) => {
                    all_sessions.push(record);
                    all_warnings.append(&mut warns);
                }
                Ok((None, mut warns)) => {
                    all_warnings.append(&mut warns);
                }
                Err(e) => {
                    all_warnings.push(format!("无法解析 {:?}: {e}", file));
                }
            }
        }

        Ok((all_sessions, all_warnings))
    }
}

/// 规范化模型名称
/// - "anthropic/claude-4.6-sonnet-20260217" → "claude-sonnet-4-6"
/// - "claude-sonnet-4-6" → "claude-sonnet-4-6"
/// - "claude-sonnet-4-6-bh" → "claude-sonnet-4-6-bh"
fn normalize_model_name(raw: &str) -> String {
    // 去除 provider 前缀
    let name = raw.strip_prefix("anthropic/").unwrap_or(raw);

    // 如果已经是短格式（不含日期），直接返回
    if !name.contains('-') || name.split('-').count() < 4 {
        return name.to_string();
    }

    // 尝试解析 "claude-X.Y-model-YYYYMMDD" 格式
    // 例如: "claude-4.6-sonnet-20260217" → "claude-sonnet-4-6"
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 4 && parts[0] == "claude" {
        // 检查第二部分是否是版本号（如 "4.6"）
        if parts[1].contains('.') {
            let version = parts[1].replace('.', "-");
            let model_type = parts[2];
            // 忽略日期部分（最后一个），拼接剩余后缀
            let suffix = if parts.len() > 4 {
                format!("-{}", parts[3..parts.len()-1].join("-"))
            } else {
                String::new()
            };
            return format!("claude-{}-{}{}", model_type, version, suffix);
        }
    }

    // 无法解析，返回原名
    name.to_string()
}

fn parse_session_file(path: &std::path::Path) -> anyhow::Result<(Option<SessionRecord>, Vec<String>)> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let session_id = path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut cwd = String::new();

    let mut model = String::new();
    let mut input_tokens: i64 = 0;
    let mut cached_input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut has_usage = false;
    let mut warnings = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(e) => { warnings.push(format!("读取行失败: {e}")); continue; }
        };

        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => { warnings.push(format!("JSON 解析失败: {e}")); continue; }
        };

        // 提取第一条 user 消息的 cwd 字段作为项目路径
        if cwd.is_empty() && v.get("type").and_then(|t| t.as_str()) == Some("user") {
            if let Some(c) = v.get("cwd").and_then(|s| s.as_str()) {
                // 取路径最后一段作为项目名
                cwd = c.rsplit('/').next().unwrap_or(c).to_string();
            }
        }

        if v.get("type").and_then(|t| t.as_str()) == Some("assistant") {
            if let Some(usage) = v.pointer("/message/usage") {
                let inp = usage.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let cache_create = usage.get("cache_creation_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let out = usage.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);

                // cache_creation 归入 input（按输入价格计费）
                input_tokens += inp + cache_create;
                cached_input_tokens += cache_read;
                output_tokens += out;
                has_usage = true;

                // 取最后一条 assistant 消息的模型
                if let Some(m) = v.pointer("/message/model").and_then(|v| v.as_str()) {
                    model = normalize_model_name(m);
                }
            }
        }
    }

    if !has_usage {
        return Ok((None, warnings));
    }

    let total_tokens = input_tokens + cached_input_tokens + output_tokens;
    let record = SessionRecord {
        session_id,
        cwd,
        model,
        model_provider: "anthropic".to_string(),
        input_tokens,
        cached_input_tokens,
        output_tokens,
        reasoning_output_tokens: 0,
        total_tokens,
        source: "claude-code".to_string(),
    };

    Ok((Some(record), warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_session(dir: &TempDir, project: &str, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join("projects").join(project);
        fs::create_dir_all(&path).unwrap();
        let file = path.join(name);
        fs::write(&file, content).unwrap();
        file
    }

    #[test]
    fn test_normalize_model_name() {
        assert_eq!(normalize_model_name("anthropic/claude-4.6-sonnet-20260217"), "claude-sonnet-4-6");
        assert_eq!(normalize_model_name("claude-4.6-sonnet-20260217"), "claude-sonnet-4-6");
        assert_eq!(normalize_model_name("claude-sonnet-4-6"), "claude-sonnet-4-6");
        assert_eq!(normalize_model_name("claude-sonnet-4-6-bh"), "claude-sonnet-4-6-bh");
        assert_eq!(normalize_model_name("claude-4.5-haiku-20260217"), "claude-haiku-4-5");
        assert_eq!(normalize_model_name("glm-5.1"), "glm-5.1");
    }

    #[test]
    fn test_cache_creation_merged_into_input() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s1.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s1","cwd":"/Users/liang/repos/myapp","message":{"role":"user","content":[]}}
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"anthropic/claude-4.6-sonnet-20260217","usage":{"input_tokens":100,"cache_creation_input_tokens":500,"cache_read_input_tokens":200,"output_tokens":50}}}
"#);
        let source = ClaudeCodeSource { base_dir: dir.path().to_path_buf() };
        let (sessions, warnings) = source.sessions().unwrap();
        assert!(warnings.is_empty(), "应无警告: {:?}", warnings);
        assert_eq!(sessions.len(), 1);
        // input = 100 + 500 (cache_creation 归入 input)
        assert_eq!(sessions[0].input_tokens, 600);
        // cached = 200 (cache_read)
        assert_eq!(sessions[0].cached_input_tokens, 200);
        assert_eq!(sessions[0].output_tokens, 50);
        assert_eq!(sessions[0].total_tokens, 850);
        assert_eq!(sessions[0].model, "claude-sonnet-4-6");
        assert_eq!(sessions[0].model_provider, "anthropic");
    }

    #[test]
    fn test_multi_message_accumulation() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s2.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s2","cwd":"/Users/liang","message":{"role":"user","content":[]}}
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"anthropic/claude-4.6-sonnet-20260217","usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}}}
{"type":"assistant","timestamp":"2026-05-01T10:00:02Z","message":{"model":"anthropic/claude-4.5-haiku-20260217","usage":{"input_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":1000,"output_tokens":30}}}
"#);
        let source = ClaudeCodeSource { base_dir: dir.path().to_path_buf() };
        let (sessions, _) = source.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].input_tokens, 100 + 0 + 50 + 200);  // 350
        assert_eq!(sessions[0].cached_input_tokens, 0 + 1000);      // 1000
        assert_eq!(sessions[0].output_tokens, 50 + 30);             // 80
        assert_eq!(sessions[0].total_tokens, 350 + 1000 + 80);      // 1430
        // 取最后一条 assistant 消息的模型
        assert_eq!(sessions[0].model, "claude-haiku-4-5");
    }

    #[test]
    fn test_empty_session_skipped() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s3.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s3","cwd":"/Users/liang","message":{"role":"user","content":[]}}
{"type":"user","timestamp":"2026-05-01T10:00:01Z","message":{"role":"user","content":[]}}
"#);
        let source = ClaudeCodeSource { base_dir: dir.path().to_path_buf() };
        let (sessions, _) = source.sessions().unwrap();
        assert_eq!(sessions.len(), 0, "无 assistant usage 的 session 应跳过");
    }

    #[test]
    fn test_invalid_json_skipped() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s4.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s4","cwd":"/Users/liang","message":{"role":"user","content":[]}}
not valid json at all
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"anthropic/claude-4.6-sonnet-20260217","usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}
"#);
        let source = ClaudeCodeSource { base_dir: dir.path().to_path_buf() };
        let (sessions, warnings) = source.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(warnings.len(), 1, "无效 JSON 行应产生 1 条 warning");
    }
}
