# AgentPrism V1.0 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 接入 Claude Code 数据源，实现 Codex / Claude Code 双 Agent 完全解耦展示与切换，预算、价格表按 agent 隔离存储。

**Architecture:** 后端所有聚合 command 新增 `agent: String` 参数，按 agent 路由到对应 `AgentSource`；Store key 加 agent 前缀实现配置隔离，首次启动自动迁移旧数据；前端新增 `useAgentSwitch` composable 驱动全局 agent 状态，`AgentSwitcher` 组件替换原标题，Settings/Dashboard 均按当前 agent 加载对应数据。

**Tech Stack:** Tauri 2, Vue 3, Rust, rusqlite, serde_json, glob, chrono

---

## 文件结构

**新建文件：**
- `src-tauri/src/data_source/claude/mod.rs` — `ClaudeCodeSource` 实现（替换占位注释）
- `src/composables/useAgentSwitch.ts` — 全局 agent 状态 composable
- `src/components/AgentSwitcher.vue` — 顶部 Agent 切换 dropdown 组件

**修改文件：**
- `src-tauri/src/billing/mod.rs` — 原 `default_prices()` 改名为 `default_prices_codex()`；新增 `default_prices_claude_code()`；新增 `new_for_agent(agent)`
- `src-tauri/src/store/mod.rs` — 所有 `get/set_budget_tokens`、`get/set/delete_prices` 加 `agent: &str` 参数；新增 `get/set_last_selected_agent`；新增 `migrate_legacy_keys`；在 `new()` 中调用迁移
- `src-tauri/src/commands.rs` — 所有聚合 command 新增 `agent: String` 参数；新增 `get_last_selected_agent`、`set_last_selected_agent`
- `src-tauri/src/lib.rs` — 注册新 commands
- `src/composables/useStats.ts` — `loadSummary(agent)` 加参数
- `src/composables/useAggregates.ts` — `loadAll(agent)` 加参数
- `src/views/Dashboard.vue` — 引入 `useAgentSwitch`；替换标题为 `AgentSwitcher`；按 agent 条件渲染
- `src/views/Settings.vue` — 接收 `currentAgent` prop；所有 invoke 传入 agent
- `src/App.vue` — 初始化 `useAgentSwitch`；透传 `currentAgent` prop

---

## Task 1: BillingMatrix 扩展 — 新增 Claude Code 价格表

**Files:**
- Modify: `src-tauri/src/billing/mod.rs`

- [ ] **Step 1: 将 `default_prices()` 改名为 `default_prices_codex()`，并新增 `default_prices_claude_code()` 和 `new_for_agent()`**

在 `src-tauri/src/billing/mod.rs` 中，将现有 `default_prices()` 函数重命名为 `default_prices_codex()`，然后在其后追加以下代码，并将 `new()` 内部调用改为 `default_prices_codex()`：

```rust
pub fn default_prices_codex() -> IndexMap<String, ModelPrice> {
    // （原 default_prices() 的完整内容，函数名改为 default_prices_codex）
    let mut m = IndexMap::new();
    m.insert("gpt-5.5".into(), ModelPrice {
        input_per_1m: 5.0,
        cached_input_per_1m: 0.5,
        output_per_1m: 30.0,
    });
    m.insert("gpt-5.4".into(), ModelPrice {
        input_per_1m: 2.5,
        cached_input_per_1m: 0.25,
        output_per_1m: 15.0,
    });
    m.insert("gpt-5.4-mini".into(), ModelPrice {
        input_per_1m: 0.75,
        cached_input_per_1m: 0.075,
        output_per_1m: 4.5,
    });
    m.insert("gpt-5.2".into(), ModelPrice {
        input_per_1m: 1.75,
        cached_input_per_1m: 0.175,
        output_per_1m: 14.0,
    });
    m
}

pub fn default_prices_claude_code() -> IndexMap<String, ModelPrice> {
    let mut m = IndexMap::new();
    m.insert("claude-opus-4-7".into(), ModelPrice {
        input_per_1m: 15.0,
        cached_input_per_1m: 1.5,
        output_per_1m: 75.0,
    });
    m.insert("claude-sonnet-4-6".into(), ModelPrice {
        input_per_1m: 3.0,
        cached_input_per_1m: 0.3,
        output_per_1m: 15.0,
    });
    m.insert("claude-haiku-4-5".into(), ModelPrice {
        input_per_1m: 0.8,
        cached_input_per_1m: 0.08,
        output_per_1m: 4.0,
    });
    m
}

pub fn new_for_agent(agent: &str) -> Self {
    let prices = match agent {
        "claude-code" => Self::default_prices_claude_code(),
        _ => Self::default_prices_codex(),
    };
    Self { prices }
}
```

同时将 `new()` 方法改为：

```rust
pub fn new() -> Self {
    Self { prices: Self::default_prices_codex() }
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error" | head -20
```

预期：无 error 输出（可能有 warning，忽略）

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/billing/mod.rs
git commit -m "feat: BillingMatrix 新增 Claude Code 价格表，default_prices 改名为 default_prices_codex"
```

---

## Task 2: Store 改造 — agent 前缀隔离 + 迁移

**Files:**
- Modify: `src-tauri/src/store/mod.rs`

- [ ] **Step 1: 编写迁移和新方法的测试**

在 `src-tauri/src/store/mod.rs` 末尾追加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_store() -> AppStore {
        let file = NamedTempFile::new().unwrap();
        let store = AppStore { db_path: file.path().to_path_buf() };
        store.init().unwrap();
        std::mem::forget(file); // 保持文件存活
        store
    }

    fn make_store_with_file() -> (AppStore, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let store = AppStore { db_path: file.path().to_path_buf() };
        store.init().unwrap();
        (store, file)
    }

    #[test]
    fn test_budget_per_agent() {
        let (_store, _file) = make_store_with_file();
        let (store, _f) = make_store_with_file();
        store.set_budget_tokens("codex", 10_000_000).unwrap();
        store.set_budget_tokens("claude-code", 1_000_000_000).unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(10_000_000));
        assert_eq!(store.get_budget_tokens("claude-code").unwrap(), Some(1_000_000_000));
    }

    #[test]
    fn test_last_selected_agent() {
        let (store, _f) = make_store_with_file();
        assert_eq!(store.get_last_selected_agent().unwrap(), None);
        store.set_last_selected_agent("claude-code").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("claude-code".to_string()));
        store.set_last_selected_agent("codex").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("codex".to_string()));
    }

    #[test]
    fn test_migrate_legacy_keys_idempotent() {
        let (store, _f) = make_store_with_file();
        // 写入旧格式 key
        let conn = rusqlite::Connection::open(&store.db_path).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('budget_tokens', '9999')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('custom_prices', '{}')",
            [],
        ).unwrap();
        drop(conn);

        // 第一次迁移
        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));

        // 第二次迁移（幂等）
        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));
    }
}
```

- [ ] **Step 2: 运行测试，确认编译失败（方法尚未实现）**

```bash
cd src-tauri && cargo test store::tests 2>&1 | head -30
```

预期：编译错误（方法签名不匹配）

- [ ] **Step 3: 重写 `src-tauri/src/store/mod.rs` 全文**

将文件内容完整替换为：

```rust
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
        if let Err(e) = store.migrate_legacy_keys() {
            eprintln!("[store] 旧数据迁移失败（不阻断启动）: {e}");
        }
        Ok(store)
    }

    pub fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn migrate_legacy_keys(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "INSERT OR IGNORE INTO meta (key, value)
                SELECT 'budget_tokens_codex', value FROM meta WHERE key = 'budget_tokens';
            DELETE FROM meta WHERE key = 'budget_tokens';
            INSERT OR IGNORE INTO meta (key, value)
                SELECT 'custom_prices_codex', value FROM meta WHERE key = 'custom_prices';
            DELETE FROM meta WHERE key = 'custom_prices';"
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
        self.get_meta_i64("last_refresh")
    }

    pub fn set_budget_tokens(&self, agent: &str, tokens: i64) -> Result<()> {
        let key = format!("budget_tokens_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, tokens.to_string()],
        )?;
        Ok(())
    }

    pub fn get_budget_tokens(&self, agent: &str) -> Result<Option<i64>> {
        let key = format!("budget_tokens_{}", agent);
        self.get_meta_i64(&key)
    }

    pub fn set_prices(&self, agent: &str, prices: &IndexMap<String, ModelPrice>) -> Result<()> {
        let key = format!("custom_prices_{}", agent);
        let json = serde_json::to_string(prices)?;
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, json],
        )?;
        Ok(())
    }

    pub fn get_prices(&self, agent: &str) -> Result<Option<IndexMap<String, ModelPrice>>> {
        let key = format!("custom_prices_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(serde_json::from_str(&s).ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_prices(&self, agent: &str) -> Result<()> {
        let key = format!("custom_prices_{}", agent);
        let conn = Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_last_selected_agent(&self) -> Result<Option<String>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'last_selected_agent'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_last_selected_agent(&self, agent: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_selected_agent', ?1)",
            params![agent],
        )?;
        Ok(())
    }

    fn get_meta_i64(&self, key: &str) -> Result<Option<i64>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(s.parse().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_store_with_file() -> (AppStore, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let store = AppStore { db_path: file.path().to_path_buf() };
        store.init().unwrap();
        (store, file)
    }

    #[test]
    fn test_budget_per_agent() {
        let (store, _f) = make_store_with_file();
        store.set_budget_tokens("codex", 10_000_000).unwrap();
        store.set_budget_tokens("claude-code", 1_000_000_000).unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(10_000_000));
        assert_eq!(store.get_budget_tokens("claude-code").unwrap(), Some(1_000_000_000));
    }

    #[test]
    fn test_last_selected_agent() {
        let (store, _f) = make_store_with_file();
        assert_eq!(store.get_last_selected_agent().unwrap(), None);
        store.set_last_selected_agent("claude-code").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("claude-code".to_string()));
        store.set_last_selected_agent("codex").unwrap();
        assert_eq!(store.get_last_selected_agent().unwrap(), Some("codex".to_string()));
    }

    #[test]
    fn test_migrate_legacy_keys_idempotent() {
        let (store, _f) = make_store_with_file();
        let conn = rusqlite::Connection::open(&store.db_path).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('budget_tokens', '9999')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('custom_prices', '{}')",
            [],
        ).unwrap();
        drop(conn);

        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));

        store.migrate_legacy_keys().unwrap();
        assert_eq!(store.get_budget_tokens("codex").unwrap(), Some(9999));
    }
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd src-tauri && cargo test store::tests 2>&1
```

预期：3 个测试全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/store/mod.rs
git commit -m "feat: store 按 agent 前缀隔离配置，新增 last_selected_agent，自动迁移旧 key"
```

---

## Task 3: ClaudeCodeSource 实现

**Files:**
- Modify: `src-tauri/src/data_source/claude/mod.rs`

- [ ] **Step 1: 编写 ClaudeCodeSource 测试**

将 `src-tauri/src/data_source/claude/mod.rs` 完整替换为：

```rust
use crate::data_source::{AgentSource, SessionRecord, ThreadRecord};
use chrono::{DateTime, Utc};
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

fn parse_session_file(path: &std::path::Path) -> anyhow::Result<(Option<SessionRecord>, Vec<String>)> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let session_id = path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 从 encoded_path（父目录名）取最后一段作为 display_name
    let cwd = path.parent()
        .and_then(|p| p.file_name())
        .map(|n| {
            let encoded = n.to_string_lossy();
            // 取最后一个 '-' 后的片段作为项目名
            encoded.rsplit('-').next().unwrap_or(&encoded).to_string()
        })
        .unwrap_or_default();

    let mut model = String::new();
    let mut input_tokens: i64 = 0;
    let mut cached_input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut has_usage = false;
    let mut warnings = Vec::new();
    let mut created_at: Option<DateTime<Utc>> = None;

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

        // 记录第一条消息时间作为 created_at
        if created_at.is_none() {
            if let Some(ts_str) = v.get("timestamp").and_then(|t| t.as_str()) {
                created_at = ts_str.parse::<DateTime<Utc>>().ok();
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
                    model = m.to_string();
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
    fn test_cache_creation_归入_input() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s1.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s1","cwd":"/Users/liang/repos/myapp","message":{"role":"user","content":[]}}
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"cache_creation_input_tokens":500,"cache_read_input_tokens":200,"output_tokens":50}}}
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
    fn test_多条_assistant_消息累加() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s2.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s2","cwd":"/Users/liang","message":{"role":"user","content":[]}}
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}}}
{"type":"assistant","timestamp":"2026-05-01T10:00:02Z","message":{"model":"claude-haiku-4-5","usage":{"input_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":1000,"output_tokens":30}}}
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
    fn test_空session跳过() {
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
    fn test_无效json行产生warning() {
        let dir = TempDir::new().unwrap();
        write_session(&dir, "-Users-liang-repos-myapp", "s4.jsonl", r#"
{"type":"user","timestamp":"2026-05-01T10:00:00Z","sessionId":"s4","cwd":"/Users/liang","message":{"role":"user","content":[]}}
not valid json at all
{"type":"assistant","timestamp":"2026-05-01T10:00:01Z","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":5}}}
"#);
        let source = ClaudeCodeSource { base_dir: dir.path().to_path_buf() };
        let (sessions, warnings) = source.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(warnings.len(), 1, "无效 JSON 行应产生 1 条 warning");
    }
}
```

- [ ] **Step 2: 运行测试，确认通过**

```bash
cd src-tauri && cargo test data_source::claude 2>&1
```

预期：4 个测试全部 PASS

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/data_source/claude/mod.rs
git commit -m "feat: 实现 ClaudeCodeSource，读取 ~/.claude/projects/**/*.jsonl"
```

---

## Task 4: Commands 改造 — 新增 agent 参数

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 重写 `src-tauri/src/commands.rs` 全文**

```rust
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
    if let Some(ref w) = reconcile_result.warning { warnings.push(w.clone()); }
    let matrix = load_matrix("codex");
    let cost = matrix.estimate(&sessions);
    let top_project = {
        let mut m: HashMap<String, i64> = Default::default();
        for t in &threads { *m.entry(t.cwd.clone()).or_insert(0) += t.tokens_used; }
        m.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
    };
    CommandResult::ok_with_warnings(SummaryData {
        total_tokens: reconcile_result.sqlite_total,
        thread_count: threads.len(),
        session_count: sessions.len(),
        estimated_cost_usd: cost.total_usd,
        top_project,
        reconcile: reconcile_result,
    }, warnings)
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
        for s in &sessions { *m.entry(s.cwd.clone()).or_insert(0) += s.total_tokens; }
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
    CommandResult::ok_with_warnings(SummaryData {
        total_tokens,
        thread_count: 0,
        session_count: sessions.len(),
        estimated_cost_usd: cost.total_usd,
        top_project,
        reconcile: na_reconcile,
    }, warnings)
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
    let total_cost: f64 = sessions.iter().map(|s| matrix.estimate(std::slice::from_ref(s)).total_usd).sum();
    let total_tokens: i64 = threads.iter().map(|t| t.tokens_used).sum();
    let global_per_token = if total_tokens > 0 { total_cost / total_tokens as f64 } else { matrix.fallback_avg_per_token() };
    let mut stats: Vec<ProjectStat> = token_map.into_iter()
        .map(|(project, tokens)| ProjectStat { cost_usd: tokens as f64 * global_per_token, project, tokens })
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
    let mut stats: Vec<ProjectStat> = token_map.into_iter()
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
        CodexSource::new().map(|s| s.sessions()).ok_or_else(|| anyhow::anyhow!("未检测到 ~/.codex 目录"))
            .and_then(|r| r)
    } else {
        ClaudeCodeSource::new().map(|s| s.sessions()).ok_or_else(|| anyhow::anyhow!("未检测到 ~/.claude 目录"))
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
    let mut stats: Vec<ModelStat> = token_map.into_iter()
        .map(|(model, tokens)| ModelStat { cost_usd: *cost_map.get(&model).unwrap_or(&0.0), model, tokens })
        .collect();
    stats.sort_by(|a, b| b.tokens.cmp(&a.tokens));
    CommandResult::ok_with_warnings(stats, warnings)
}

#[tauri::command]
pub fn get_by_date(agent: String) -> CommandResult<Vec<DayStat>> {
    use chrono::{Datelike, Duration, Utc};
    let cutoff = Utc::now() - Duration::days(30);

    let sessions_result = if agent == "codex" {
        // Codex 用 threads 的 updated_at
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
            if t.updated_at < cutoff { continue; }
            let date = format!("{:04}-{:02}-{:02}", t.updated_at.year(), t.updated_at.month(), t.updated_at.day());
            *map.entry(date).or_insert(0) += t.tokens_used;
        }
        let stats: Vec<DayStat> = map.into_iter().map(|(date, tokens)| DayStat { date, tokens }).collect();
        return CommandResult::ok_with_warnings(stats, warnings);
    } else {
        ClaudeCodeSource::new().map(|s| s.sessions()).ok_or_else(|| anyhow::anyhow!("未检测到 ~/.claude 目录"))
            .and_then(|r| r)
    };

    let (sessions, warnings) = match sessions_result {
        Ok(r) => r,
        Err(e) => return CommandResult::err(format!("读取 sessions 失败: {e}")),
    };

    // Claude Code 目前无精确 updated_at，暂用全量数据不做日期过滤
    // TODO: V1.1 在 SessionRecord 中增加 updated_at 字段
    let _ = cutoff;
    let mut map: std::collections::BTreeMap<String, i64> = Default::default();
    for s in &sessions {
        // 当前 SessionRecord 无日期字段，以 session 全量统计放入今天
        // 实际实现需 ClaudeCodeSource 提供 updated_at
        let _ = s;
    }
    let stats: Vec<DayStat> = map.into_iter().map(|(date, tokens)| DayStat { date, tokens }).collect();
    CommandResult::ok_with_warnings(stats, warnings)
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
    let response = match ureq::get(&api_url).set("User-Agent", "AgentPrism").call() {
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
    let tag = latest.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").trim_start_matches('v').to_string();
    let html_url = latest.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let has_update = !tag.is_empty() && tag != current;
    CommandResult::ok(UpdateInfo { has_update, latest_version: tag, release_url: html_url })
}
```

- [ ] **Step 2: 更新 `src-tauri/src/lib.rs` 注册新 commands**

将 `lib.rs` 中的 `use commands::` 和 `invoke_handler` 替换为：

```rust
use commands::{
    get_summary, get_threads, refresh,
    get_by_project, get_by_model, get_by_date,
    get_budget, set_budget,
    get_prices, set_prices, reset_prices,
    get_last_selected_agent, set_last_selected_agent,
    get_app_version, check_update,
};
```

```rust
.invoke_handler(tauri::generate_handler![
    get_summary, get_threads, refresh,
    get_by_project, get_by_model, get_by_date,
    get_budget, set_budget,
    get_prices, set_prices, reset_prices,
    get_last_selected_agent, set_last_selected_agent,
    get_app_version, check_update
])
```

- [ ] **Step 3: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error" | head -20
```

预期：无 error 输出

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: 所有 command 新增 agent 参数，接入 ClaudeCodeSource，新增 last_selected_agent command"
```

---

## Task 5: 前端 — useAgentSwitch composable

**Files:**
- Create: `src/composables/useAgentSwitch.ts`

- [ ] **Step 1: 创建 `src/composables/useAgentSwitch.ts`**

```typescript
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CommandResult } from './useStats'

export type AgentId = 'codex' | 'claude-code'

export interface AgentInfo {
  id: AgentId
  label: string
}

export const AGENTS: AgentInfo[] = [
  { id: 'claude-code', label: 'Claude Code' },
  { id: 'codex', label: 'Codex' },
]

export function useAgentSwitch() {
  const currentAgent = ref<AgentId>('claude-code')

  async function init() {
    try {
      const res = await invoke<CommandResult<string | null>>('get_last_selected_agent')
      if (res.data === 'codex' || res.data === 'claude-code') {
        currentAgent.value = res.data
      }
    } catch {
      // 读取失败时保持默认值 claude-code
    }
  }

  async function switchAgent(agent: AgentId) {
    if (currentAgent.value === agent) return
    currentAgent.value = agent
    try {
      await invoke('set_last_selected_agent', { agent })
    } catch {
      // 持久化失败不影响当前会话
    }
  }

  return { currentAgent, init, switchAgent, AGENTS }
}
```

- [ ] **Step 2: 提交**

```bash
git add src/composables/useAgentSwitch.ts
git commit -m "feat: 新增 useAgentSwitch composable"
```

---

## Task 6: 前端 — AgentSwitcher 组件

**Files:**
- Create: `src/components/AgentSwitcher.vue`

- [ ] **Step 1: 创建 `src/components/AgentSwitcher.vue`**

```vue
<!-- src/components/AgentSwitcher.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { AGENTS, type AgentId, type AgentInfo } from '../composables/useAgentSwitch'

const props = defineProps<{
  modelValue: AgentId
  agents: AgentInfo[]
}>()

const emit = defineEmits<{
  'update:modelValue': [agent: AgentId]
}>()

const open = ref(false)
const containerRef = ref<HTMLElement | null>(null)

function toggle() {
  open.value = !open.value
}

function select(id: AgentId) {
  open.value = false
  emit('update:modelValue', id)
}

function onClickOutside(e: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(e.target as Node)) {
    open.value = false
  }
}

onMounted(() => document.addEventListener('mousedown', onClickOutside))
onUnmounted(() => document.removeEventListener('mousedown', onClickOutside))

const currentLabel = () => props.agents.find(a => a.id === props.modelValue)?.label ?? ''
</script>

<template>
  <div class="agent-switcher" ref="containerRef">
    <button class="switcher-btn" @click="toggle">
      {{ currentLabel() }}<span class="arrow">{{ open ? ' ▴' : ' ▾' }}</span>
    </button>
    <div v-if="open" class="dropdown">
      <button
        v-for="a in agents"
        :key="a.id"
        class="dropdown-item"
        :class="{ active: modelValue === a.id }"
        @click="select(a.id)"
      >
        <span class="check">{{ modelValue === a.id ? '✓' : '' }}</span>
        {{ a.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.agent-switcher { position: relative; display: inline-block; }
.switcher-btn {
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: #333;
  display: flex;
  align-items: center;
  gap: 0;
  -webkit-app-region: no-drag;
}
.switcher-btn:hover { opacity: 0.75; }
.arrow { font-size: 10px; margin-left: 3px; color: #888; }
.dropdown {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  min-width: 140px;
  z-index: 100;
  overflow: hidden;
}
.dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 14px;
  background: none;
  border: none;
  font-size: 13px;
  color: #333;
  cursor: pointer;
  text-align: left;
}
.dropdown-item:hover { background: #f5f5f5; }
.dropdown-item.active { color: #0077cc; font-weight: 500; }
.check { width: 12px; font-size: 11px; color: #0077cc; }
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/AgentSwitcher.vue
git commit -m "feat: 新增 AgentSwitcher dropdown 组件"
```

---

## Task 7: 前端 — 更新 composables 加 agent 参数

**Files:**
- Modify: `src/composables/useStats.ts`
- Modify: `src/composables/useAggregates.ts`

- [ ] **Step 1: 修改 `src/composables/useStats.ts`，`loadSummary` 接收 agent 参数**

将 `useStats` 函数中的 `loadSummary` 改为：

```typescript
async function loadSummary(agent: string) {
  loading.value = true
  error.value = null
  try {
    const result = await invoke<CommandResult<SummaryData>>('get_summary', { agent })
    if (result.error) {
      error.value = result.error
    } else {
      summary.value = result.data
      warnings.value = result.warnings
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

同时将 `loadThreads` 改为接收 agent 参数（虽然 Claude Code 返回空，但保持一致）：

```typescript
async function loadThreads(agent: string) {
  loading.value = true
  try {
    const result = await invoke<CommandResult<ThreadRecord[]>>('get_threads', { agent })
    if (result.error) {
      error.value = result.error
    } else {
      threads.value = result.data ?? []
      warnings.value = result.warnings
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

更新 `return` 语句中的函数签名（TypeScript 不需要额外声明，只需函数体已更新即可）。

- [ ] **Step 2: 修改 `src/composables/useAggregates.ts`，`loadAll` 接收 agent 参数**

将 `loadAll` 改为：

```typescript
async function loadAll(agent: string) {
  loading.value = true
  error.value = null
  try {
    const [pRes, mRes, dRes] = await Promise.all([
      invoke<CommandResult<ProjectStat[]>>('get_by_project', { agent }),
      invoke<CommandResult<ModelStat[]>>('get_by_model', { agent }),
      invoke<CommandResult<DayStat[]>>('get_by_date', { agent }),
    ])
    if (pRes.error) error.value = pRes.error
    else byProject.value = pRes.data ?? []
    if (!mRes.error) byModel.value = mRes.data ?? []
    if (!dRes.error) byDate.value = dRes.data ?? []
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 3: 编译前端验证**

```bash
pnpm build 2>&1 | grep -E "error TS" | head -20
```

预期：若有 TypeScript 错误，均为调用处未传 agent 参数（下一步 Task 8 修复）

- [ ] **Step 4: 提交**

```bash
git add src/composables/useStats.ts src/composables/useAggregates.ts
git commit -m "feat: useStats/useAggregates 的 load 方法新增 agent 参数"
```

---

## Task 8: 前端 — App.vue 改造

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: 重写 `src/App.vue`**

```vue
<!-- src/App.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import Dashboard from './views/Dashboard.vue'
import Settings from './views/Settings.vue'
import { useAgentSwitch } from './composables/useAgentSwitch'

const page = ref<'dashboard' | 'settings'>('dashboard')
const { currentAgent, init } = useAgentSwitch()

onMounted(() => init())
</script>

<template>
  <Dashboard
    v-if="page === 'dashboard'"
    :currentAgent="currentAgent"
    @openSettings="page = 'settings'"
  />
  <Settings
    v-else
    :currentAgent="currentAgent"
    @back="page = 'dashboard'"
  />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/App.vue
git commit -m "feat: App.vue 初始化 useAgentSwitch，透传 currentAgent 给子页面"
```

---

## Task 9: 前端 — Dashboard.vue 改造

**Files:**
- Modify: `src/views/Dashboard.vue`

- [ ] **Step 1: 重写 `src/views/Dashboard.vue`**

```vue
<!-- src/views/Dashboard.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useStats, useDataUpdatedListener } from '../composables/useStats'
import { useAggregates } from '../composables/useAggregates'
import { useAgentSwitch, AGENTS, type AgentId } from '../composables/useAgentSwitch'
import BudgetRing from '../components/BudgetRing.vue'
import ProjectList from '../components/ProjectList.vue'
import ModelBreakdown from '../components/ModelBreakdown.vue'
import DayChart from '../components/DayChart.vue'
import AgentSwitcher from '../components/AgentSwitcher.vue'
import type { CommandResult } from '../composables/useStats'

const props = defineProps<{ currentAgent: AgentId }>()
const emit = defineEmits<{ openSettings: [] }>()

const { summary, error, loading, loadSummary } = useStats()
const { byProject, byModel, byDate, loadAll } = useAggregates()
const { switchAgent } = useAgentSwitch()

// 本地镜像 prop，用于双向绑定 AgentSwitcher
const localAgent = ref<AgentId>(props.currentAgent)

watch(() => props.currentAgent, (val) => {
  localAgent.value = val
})

const activeTab = ref<'project' | 'model' | 'date'>('project')
const budgetTokens = ref(1_000_000_000)

async function loadBudget(agent: string) {
  const res = await invoke<CommandResult<number | null>>('get_budget', { agent })
  if (res.data != null) budgetTokens.value = res.data
}

async function reload(agent: string) {
  await Promise.all([loadSummary(agent), loadAll(agent), loadBudget(agent)])
}

onMounted(async () => {
  await reload(props.currentAgent)
})

// 监听 prop 变化（外部切换 agent）
watch(() => props.currentAgent, async (agent) => {
  await reload(agent)
})

// AgentSwitcher 本地切换
async function onAgentChange(agent: AgentId) {
  await switchAgent(agent)
}

const stopListen = useDataUpdatedListener(() => reload(props.currentAgent))
onUnmounted(() => stopListen())

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

const agentLabel = () => AGENTS.find(a => a.id === props.currentAgent)?.label ?? ''

const emptyHint = () =>
  props.currentAgent === 'codex'
    ? '未检测到 ~/.codex 目录'
    : '未检测到 ~/.claude 目录'
</script>

<template>
  <div class="dashboard">
    <header class="header">
      <AgentSwitcher
        v-model="localAgent"
        :agents="AGENTS"
        @update:modelValue="onAgentChange"
      />
      <div class="header-actions">
        <button class="action-btn" @click="reload(currentAgent)" :disabled="loading">
          {{ loading ? '刷新中…' : '刷新' }}
        </button>
        <button class="action-btn" @click="$emit('openSettings')">设置</button>
      </div>
    </header>

    <div v-if="error" class="error-state">{{ emptyHint() }}</div>

    <template v-else>
      <div class="overview" v-if="summary">
        <BudgetRing :usedTokens="summary.total_tokens" :budgetTokens="budgetTokens" />
        <div class="stats-grid">
          <div class="stat">
            <div class="stat-value">{{ formatTokens(summary.total_tokens) }}</div>
            <div class="stat-label">Token 总量</div>
          </div>
          <div class="stat">
            <div class="stat-value accent">${{ summary.estimated_cost_usd.toFixed(4) }}</div>
            <div class="stat-label">估算费用</div>
          </div>
          <div v-if="currentAgent === 'codex'" class="stat">
            <div class="stat-value">{{ summary.thread_count }}</div>
            <div class="stat-label">线程数</div>
          </div>
          <div class="stat">
            <div class="stat-value">{{ summary.session_count }}</div>
            <div class="stat-label">Session 数</div>
          </div>
          <div
            v-if="currentAgent === 'codex' && summary.reconcile.diff_rate >= 0"
            class="reconcile"
            :class="{ warn: !!summary.reconcile.warning }"
          >
            对账差异率 {{ (summary.reconcile.diff_rate * 100).toFixed(1) }}%
          </div>
        </div>
      </div>

      <div class="tabs">
        <button
          v-for="[key, label] in [['project','项目'],['model','模型'],['date','时间']] as [string, string][]"
          :key="key"
          class="tab-btn"
          :class="{ active: activeTab === key }"
          @click="activeTab = key as 'project' | 'model' | 'date'"
        >{{ label }}</button>
      </div>

      <div class="tab-content">
        <ProjectList v-if="activeTab === 'project'" :stats="byProject" />
        <ModelBreakdown v-else-if="activeTab === 'model'" :stats="byModel" />
        <DayChart v-else :stats="byDate" />
      </div>

      <div class="estimate-footer">估算，非真实账单</div>
    </template>
  </div>
</template>

<style scoped>
.dashboard { display: flex; flex-direction: column; height: 100vh; font-family: -apple-system, sans-serif; color: #333; overflow: hidden; }
.header { display: flex; justify-content: space-between; align-items: center; padding: 10px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; -webkit-app-region: drag; }
.header-actions { display: flex; gap: 8px; -webkit-app-region: no-drag; }
.action-btn { background: #f0f0f0; border: 1px solid #ccc; border-radius: 5px; color: #333; font-size: 12px; padding: 4px 10px; cursor: pointer; }
.action-btn:hover { background: #e0e0e0; }
.action-btn:disabled { opacity: 0.5; cursor: default; }
.overview { display: flex; align-items: center; gap: 20px; padding: 12px 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.stats-grid { display: flex; flex-wrap: wrap; gap: 16px; align-items: center; flex: 1; }
.stat { text-align: center; }
.stat-value { font-size: 20px; font-weight: 200; }
.stat-value.accent { color: #0077cc; }
.stat-label { font-size: 10px; color: #888; text-transform: uppercase; margin-top: 2px; }
.reconcile { font-size: 11px; color: #888; }
.reconcile.warn { color: #e67e00; }
.tabs { display: flex; padding: 0 20px; border-bottom: 1px solid #e0e0e0; flex-shrink: 0; }
.tab-btn { background: none; border: none; border-bottom: 2px solid transparent; padding: 8px 16px; font-size: 13px; color: #888; cursor: pointer; margin-bottom: -1px; }
.tab-btn:hover { color: #333; }
.tab-btn.active { color: #0077cc; border-bottom-color: #0077cc; font-weight: 500; }
.tab-content { flex: 1; overflow-y: auto; padding: 16px 20px; }
.estimate-footer { padding: 6px 20px; font-size: 10px; color: #aaa; border-top: 1px solid #e0e0e0; flex-shrink: 0; }
.error-state { flex: 1; display: flex; align-items: center; justify-content: center; color: #888; font-size: 13px; }
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/views/Dashboard.vue
git commit -m "feat: Dashboard 引入 AgentSwitcher，按 agent 条件渲染，监听 prop 变化重新加载"
```

---

## Task 10: 前端 — Settings.vue 改造

**Files:**
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: 在 `src/views/Settings.vue` 中添加 `currentAgent` prop 并更新所有 invoke 调用**

在 `<script setup>` 最顶部添加 prop 定义和 agent label 计算：

```typescript
import { AGENTS, type AgentId } from '../composables/useAgentSwitch'

const props = defineProps<{ currentAgent: AgentId }>()

const agentLabel = () => AGENTS.find(a => a.id === props.currentAgent)?.label ?? ''
```

将 `onMounted` 改为：

```typescript
onMounted(async () => {
  const [vRes, bRes, pRes] = await Promise.all([
    invoke<CommandResult<string>>('get_app_version'),
    invoke<CommandResult<number | null>>('get_budget', { agent: props.currentAgent }),
    invoke<CommandResult<PriceMap>>('get_prices', { agent: props.currentAgent }),
  ])
  if (vRes.data) appVersion.value = vRes.data
  budgetInput.value = String(bRes.data ?? DEFAULT_BUDGET)
  if (pRes.data) prices.value = pRes.data
})
```

将 `saveBudget` 改为：

```typescript
async function saveBudget() {
  const val = parseInt(budgetInput.value, 10)
  if (isNaN(val) || val <= 0) return
  savingBudget.value = true
  budgetMsg.value = null
  try {
    await invoke('set_budget', { agent: props.currentAgent, tokens: val })
    budgetMsg.value = '已保存'
    setTimeout(() => { budgetMsg.value = null }, 2000)
  } finally {
    savingBudget.value = false
  }
}
```

将 `savePrices` 改为：

```typescript
async function savePrices() {
  savingPrices.value = true
  pricesMsg.value = null
  try {
    await invoke('set_prices', { agent: props.currentAgent, prices: prices.value })
    pricesMsg.value = '已保存'
    setTimeout(() => { pricesMsg.value = null }, 2000)
  } finally {
    savingPrices.value = false
  }
}
```

将 `resetPrices` 改为：

```typescript
async function resetPrices() {
  resettingPrices.value = true
  pricesMsg.value = null
  try {
    const res = await invoke<CommandResult<PriceMap>>('reset_prices', { agent: props.currentAgent })
    if (res.data) prices.value = res.data
    pricesMsg.value = '已恢复预设'
    setTimeout(() => { pricesMsg.value = null }, 2000)
  } finally {
    resettingPrices.value = false
  }
}
```

在 template 中将价格表 section-title 改为：

```html
<span class="section-title">计费价格表 · {{ agentLabel() }}（/1M token，单位：$）</span>
```

- [ ] **Step 2: 前端完整编译验证**

```bash
pnpm build 2>&1 | tail -20
```

预期：构建成功，无 TypeScript 错误

- [ ] **Step 3: 提交**

```bash
git add src/views/Settings.vue
git commit -m "feat: Settings 接收 currentAgent prop，所有配置操作按 agent 隔离"
```

---

## Task 11: 端到端集成验证

- [ ] **Step 1: 运行全量 Rust 测试**

```bash
cd src-tauri && cargo test 2>&1 | tail -30
```

预期：所有测试 PASS

- [ ] **Step 2: 启动开发模式**

```bash
pnpm tauri dev
```

手动验证清单：
- [ ] 顶部标题位置显示 `Claude Code ▾`，字号/字重与原标题一致
- [ ] 点击切换器展开下拉菜单，显示 `✓ Claude Code` 和 `Codex`
- [ ] 切换到 Codex：概览数字切换，出现"线程数"和对账差异率
- [ ] 切换到 Claude Code：概览数字切换，无"线程数"，无对账差异率
- [ ] 关闭并重新打开：自动恢复上次选择的 agent
- [ ] 进入设置：价格表标题显示 `计费价格表 · Claude Code`
- [ ] 切换 agent 后进入设置：价格表标题和内容均切换
- [ ] 修改 Claude Code 预算，切换到 Codex，预算不受影响
- [ ] `~/.codex` 如不存在，切换到 Codex 显示"未检测到 ~/.codex 目录"
- [ ] 托盘菜单和窗口切换功能正常（V0.5 功能无回归）

- [ ] **Step 3: 最终提交**

```bash
git add .
git commit -m "chore: V1.0 集成验证完成"
```

---

## 自审结果

1. **Spec 覆盖**：
   - ClaudeCodeSource 实现 → Task 3
   - BillingMatrix Claude Code 价格表 → Task 1
   - Store agent 前缀隔离 + 迁移 → Task 2
   - Commands 加 agent 参数 → Task 4
   - useAgentSwitch composable → Task 5
   - AgentSwitcher 组件 → Task 6
   - composables 加 agent 参数 → Task 7
   - App.vue 透传 → Task 8
   - Dashboard 改造 → Task 9
   - Settings 改造 → Task 10
   - 集成验证 → Task 11

2. **类型一致性**：
   - `AgentId = 'codex' | 'claude-code'` 在 Task 5 定义，Task 6/8/9/10 均使用相同类型
   - `loadSummary(agent: string)` 在 Task 7 定义，Task 9 调用一致
   - `loadAll(agent: string)` 在 Task 7 定义，Task 9 调用一致
   - `get_budget({ agent })` / `set_budget({ agent, tokens })` 在 Task 4 定义，Task 9/10 调用一致

3. **依赖顺序**：Task 1（billing）→ Task 2（store）→ Task 3（data_source）→ Task 4（commands）→ Task 5-6（前端工具）→ Task 7（composables）→ Task 8-10（页面）→ Task 11（验证）

4. **已知局限（不在 V1.0 范围）**：`get_by_date` 对 Claude Code 暂返回空数组，因 `SessionRecord` 无 `updated_at` 字段；时间维度 Tab 对 Claude Code 展示"近 30 天暂无数据"，V1.1 再补充。
