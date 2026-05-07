# AgentPrism V0.1 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 AgentPrism V0.1 探针时代——本地 Codex 数据精准读取、双口径对账、计费估算，并通过系统托盘和主窗口展示结果。

**Architecture:** Rust 后端先行：依次实现 AgentSource trait 抽象、Codex SQLite 读取、JSONL 解析、对账和计费矩阵，通过 Tauri invoke 命令暴露给 Vue 前端；前端实现系统托盘悬浮面板和主窗口线程列表。

**Tech Stack:** Tauri 2, Vue 3, Rust, rusqlite, serde_json, tauri-plugin-tray, glob

---

## 文件结构

**新建文件：**
- `src-tauri/src/data_source/mod.rs` — AgentSource trait + ThreadRecord + SessionRecord + CommandResult
- `src-tauri/src/data_source/codex/mod.rs` — CodexSource 结构体，实现 AgentSource
- `src-tauri/src/data_source/codex/sqlite.rs` — 读取 state_*.sqlite，解析 threads 表
- `src-tauri/src/data_source/codex/jsonl.rs` — 扫描 sessions/**/*.jsonl，取最后一条 token_count
- `src-tauri/src/data_source/codex/reconciler.rs` — 双口径对账，计算差异率
- `src-tauri/src/data_source/claude/mod.rs` — V1.0 占位 stub
- `src-tauri/src/billing/mod.rs` — 计费矩阵，内置价格表，估算费用
- `src-tauri/src/store/mod.rs` — AgentPrism 缓存 SQLite（~/.agent-prism/cache.db）
- `src-tauri/src/commands.rs` — Tauri invoke 命令：get_summary, get_threads, refresh
- `src-tauri/tests/fixtures/sample.sqlite` — 测试用 SQLite fixture
- `src-tauri/tests/fixtures/sessions/sample.jsonl` — 测试用 JSONL fixture
- `src/components/TrayPanel.vue` — 托盘悬浮面板
- `src/components/ThreadList.vue` — 线程列表
- `src/views/Dashboard.vue` — 主窗口视图
- `src/composables/useStats.ts` — 封装 invoke 调用

**修改文件：**
- `src-tauri/src/lib.rs` — 注册所有模块和 Tauri commands
- `src-tauri/Cargo.toml` — 添加 rusqlite, glob, tauri-plugin-tray, dirs 依赖
- `src-tauri/tauri.conf.json` — 配置主窗口（无边框深色）、注册 tray 插件
- `src-tauri/capabilities/default.json` — 添加 tray 权限
- `src/App.vue` — 替换为主窗口路由入口
- `src/main.ts` — 无需改动（已是 Vue 入口）
- `package.json` — 添加 @tauri-apps/plugin-tray 前端依赖

---

## Task 1: 添加 Rust 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加依赖**

将 `src-tauri/Cargo.toml` 的 `[dependencies]` 替换为：

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-tray = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
glob = "0.3"
dirs = "5"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: 验证依赖可以编译**

```bash
cd src-tauri && cargo check
```

预期：无编译错误（只有 unused import 警告可以忽略）

- [ ] **Step 3: 提交**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: 添加 rusqlite/glob/dirs/chrono/tray 依赖"
```

---

## Task 2: 定义核心数据结构和 AgentSource trait

**Files:**
- Create: `src-tauri/src/data_source/mod.rs`
- Create: `src-tauri/src/data_source/claude/mod.rs`

- [ ] **Step 1: 创建 data_source/mod.rs**

```rust
// src-tauri/src/data_source/mod.rs
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

pub mod codex;
pub mod claude;

#[derive(Debug, Clone, Serialize)]
pub struct ThreadRecord {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model: String,
    pub model_provider: String,
    pub tokens_used: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub cwd: String,
    pub model_provider: String,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct CommandResult<T: Serialize> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

impl<T: Serialize> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self { data: Some(data), error: None, warnings: vec![] }
    }

    pub fn ok_with_warnings(data: T, warnings: Vec<String>) -> Self {
        Self { data: Some(data), error: None, warnings }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self { data: None, error: Some(msg.into()), warnings: vec![] }
    }
}

pub trait AgentSource {
    fn name(&self) -> &str;
    fn discover(&self) -> anyhow::Result<Vec<PathBuf>>;
    fn threads(&self) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)>;
    fn sessions(&self) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)>;
}
```

- [ ] **Step 2: 创建 claude 占位 stub**

```rust
// src-tauri/src/data_source/claude/mod.rs
// V1.0 占位：Claude Code AgentSource 实现
```

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/data_source/
git commit -m "feat: 定义 AgentSource trait 和核心数据结构"
```

---

## Task 3: 实现 Codex SQLite 读取

**Files:**
- Create: `src-tauri/src/data_source/codex/sqlite.rs`
- Create: `src-tauri/src/data_source/codex/mod.rs`
- Create: `src-tauri/tests/fixtures/sample.sqlite`（通过测试代码动态创建）

- [ ] **Step 1: 编写 SQLite 读取测试**

在 `src-tauri/src/data_source/codex/sqlite.rs` 底部添加测试模块（先写测试）：

```rust
// src-tauri/src/data_source/codex/sqlite.rs

use crate::data_source::ThreadRecord;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

pub fn read_threads(db_path: &Path) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)> {
    todo!()
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
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test data_source::codex::sqlite 2>&1 | head -30
```

预期：编译失败（todo!() panic 或 tempfile 未找到）

- [ ] **Step 3: 添加 tempfile 依赖**

在 `Cargo.toml` 的 `[dev-dependencies]` 中添加：

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: 实现 read_threads**

将 `sqlite.rs` 中的 `read_threads` 函数替换为：

```rust
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
```

- [ ] **Step 5: 创建 codex/mod.rs**

```rust
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
```

- [ ] **Step 6: 运行测试，确认通过**

```bash
cd src-tauri && cargo test data_source::codex::sqlite
```

预期：3 个测试全部 PASS

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/data_source/codex/ src-tauri/Cargo.toml
git commit -m "feat: 实现 Codex SQLite 读取（read_threads）"
```

---

## Task 4: 实现 Codex JSONL 解析

**Files:**
- Create: `src-tauri/src/data_source/codex/jsonl.rs`

- [ ] **Step 1: 编写 JSONL 解析测试**

```rust
// src-tauri/src/data_source/codex/jsonl.rs

use crate::data_source::SessionRecord;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn read_sessions(codex_dir: &Path) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
    todo!()
}

fn parse_session_file(path: &Path) -> anyhow::Result<(Option<SessionRecord>, Vec<String>)> {
    todo!()
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
{"type":"session_meta","id":"s1","timestamp":1714000000,"cwd":"/proj","model_provider":"openai"}
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
{"type":"session_meta","id":"s2","timestamp":1714000000,"cwd":"/proj","model_provider":"openai"}
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
{"type":"session_meta","id":"s3","timestamp":1714000000,"cwd":"/proj","model_provider":"openai"}
{"type":"event_msg","payload":{"type":"other_event"}}
"#);
        let (sessions, _) = read_sessions(dir.path()).unwrap();
        assert!(sessions.is_empty());
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test data_source::codex::jsonl
```

预期：todo!() panic

- [ ] **Step 3: 实现 parse_session_file 和 read_sessions**

将 `jsonl.rs` 中两个 `todo!()` 替换为完整实现：

```rust
pub fn read_sessions(codex_dir: &Path) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)> {
    let pattern = codex_dir.join("sessions").join("**").join("*.jsonl")
        .to_string_lossy().to_string();

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

    let mut session_id = path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut cwd = String::new();
    let mut model_provider = String::new();
    let mut last_token_count: Option<Value> = None;
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

        match v.get("type").and_then(|t| t.as_str()) {
            Some("session_meta") => {
                if let Some(id) = v.get("id").and_then(|s| s.as_str()) {
                    session_id = id.to_string();
                }
                if let Some(c) = v.get("cwd").and_then(|s| s.as_str()) {
                    cwd = c.to_string();
                }
                if let Some(mp) = v.get("model_provider").and_then(|s| s.as_str()) {
                    model_provider = mp.to_string();
                }
            }
            Some("event_msg") => {
                if v.pointer("/payload/type").and_then(|t| t.as_str()) == Some("token_count") {
                    last_token_count = v.get("payload").cloned();
                }
            }
            _ => {}
        }
    }

    let record = last_token_count.map(|tc| SessionRecord {
        session_id,
        cwd,
        model_provider,
        input_tokens: tc.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        cached_input_tokens: tc.get("cached_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        output_tokens: tc.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        reasoning_output_tokens: tc.get("reasoning_output_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        total_tokens: tc.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
        source: "codex".to_string(),
    });

    Ok((record, warnings))
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd src-tauri && cargo test data_source::codex::jsonl
```

预期：3 个测试全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/data_source/codex/jsonl.rs
git commit -m "feat: 实现 Codex JSONL 解析（取最后一条 token_count）"
```

---

## Task 5: 实现对账器（reconciler）

**Files:**
- Create: `src-tauri/src/data_source/codex/reconciler.rs`

- [ ] **Step 1: 编写对账测试**

```rust
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
    todo!()
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
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test data_source::codex::reconciler
```

预期：todo!() panic

- [ ] **Step 3: 实现 reconcile**

```rust
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
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd src-tauri && cargo test data_source::codex::reconciler
```

预期：3 个测试全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/data_source/codex/reconciler.rs
git commit -m "feat: 实现双口径对账，差异率 > 5% 触发 warning"
```

---

## Task 6: 实现计费矩阵

**Files:**
- Create: `src-tauri/src/billing/mod.rs`

- [ ] **Step 1: 编写计费测试**

```rust
// src-tauri/src/billing/mod.rs

use crate::data_source::SessionRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_1m: f64,
    pub cached_input_per_1m: f64,
    pub output_per_1m: f64,
}

#[derive(Debug, Serialize)]
pub struct CostEstimate {
    pub total_usd: f64,
    pub breakdown: HashMap<String, f64>,
    pub is_estimate: bool,
}

pub struct BillingMatrix {
    prices: HashMap<String, ModelPrice>,
}

impl BillingMatrix {
    pub fn default_prices() -> HashMap<String, ModelPrice> {
        todo!()
    }

    pub fn new() -> Self {
        todo!()
    }

    pub fn estimate(&self, sessions: &[SessionRecord]) -> CostEstimate {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(model_provider: &str, input: i64, cached: i64, output: i64) -> SessionRecord {
        SessionRecord {
            session_id: "s1".into(),
            cwd: "".into(),
            model_provider: model_provider.into(),
            input_tokens: input,
            cached_input_tokens: cached,
            output_tokens: output,
            reasoning_output_tokens: 0,
            total_tokens: input + output,
            source: "codex".into(),
        }
    }

    #[test]
    fn test_cost_calculation() {
        let matrix = BillingMatrix::new();
        // codex-mini: input=1.5, cached=0.375, output=6.0 per 1M
        // 1M uncached input = $1.5, 0 cached, 1M output = $6.0
        let sessions = vec![make_session("codex-mini", 1_000_000, 0, 1_000_000)];
        let estimate = matrix.estimate(&sessions);
        // uncached_input=1M * 1.5/1M + output=1M * 6.0/1M = 7.5
        assert!((estimate.total_usd - 7.5).abs() < 0.001);
        assert!(estimate.is_estimate);
    }

    #[test]
    fn test_unknown_model_costs_zero() {
        let matrix = BillingMatrix::new();
        let sessions = vec![make_session("unknown-model-xyz", 1_000_000, 0, 1_000_000)];
        let estimate = matrix.estimate(&sessions);
        assert_eq!(estimate.total_usd, 0.0);
    }

    #[test]
    fn test_cached_input_cheaper() {
        let matrix = BillingMatrix::new();
        // 全部 cached: cached=1M, uncached=0
        let sessions_cached = vec![make_session("codex-mini", 1_000_000, 1_000_000, 0)];
        // 全部 uncached: cached=0, uncached=1M
        let sessions_uncached = vec![make_session("codex-mini", 1_000_000, 0, 0)];
        let cost_cached = matrix.estimate(&sessions_cached).total_usd;
        let cost_uncached = matrix.estimate(&sessions_uncached).total_usd;
        assert!(cost_cached < cost_uncached);
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cd src-tauri && cargo test billing
```

预期：todo!() panic

- [ ] **Step 3: 实现 BillingMatrix**

```rust
impl BillingMatrix {
    pub fn default_prices() -> HashMap<String, ModelPrice> {
        let mut m = HashMap::new();
        m.insert("codex-mini".into(), ModelPrice {
            input_per_1m: 1.5,
            cached_input_per_1m: 0.375,
            output_per_1m: 6.0,
        });
        m.insert("gpt-4.1".into(), ModelPrice {
            input_per_1m: 2.0,
            cached_input_per_1m: 0.5,
            output_per_1m: 8.0,
        });
        m.insert("gpt-4.1-mini".into(), ModelPrice {
            input_per_1m: 0.4,
            cached_input_per_1m: 0.1,
            output_per_1m: 1.6,
        });
        m
    }

    pub fn new() -> Self {
        Self { prices: Self::default_prices() }
    }

    pub fn estimate(&self, sessions: &[SessionRecord]) -> CostEstimate {
        let mut total_usd = 0.0;
        let mut breakdown: HashMap<String, f64> = HashMap::new();

        for session in sessions {
            let Some(price) = self.prices.get(&session.model_provider) else { continue };

            let uncached = (session.input_tokens - session.cached_input_tokens).max(0);
            let cost = uncached as f64 / 1_000_000.0 * price.input_per_1m
                + session.cached_input_tokens as f64 / 1_000_000.0 * price.cached_input_per_1m
                + session.output_tokens as f64 / 1_000_000.0 * price.output_per_1m;

            total_usd += cost;
            *breakdown.entry(session.model_provider.clone()).or_insert(0.0) += cost;
        }

        CostEstimate { total_usd, breakdown, is_estimate: true }
    }
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cd src-tauri && cargo test billing
```

预期：3 个测试全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/billing/
git commit -m "feat: 实现计费矩阵，内置 codex-mini/gpt-4.1 价格表"
```

---

## Task 7: 实现缓存 Store

**Files:**
- Create: `src-tauri/src/store/mod.rs`

- [ ] **Step 1: 实现 Store**

（Store 逻辑较直接，不需要 TDD，直接实现后手动验证）

```rust
// src-tauri/src/store/mod.rs

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

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
        Ok(store)
    }

    fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
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

    pub fn get_last_refresh(&self) -> Result<Option<i64>> {
        let conn = Connection::open(&self.db_path)?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value FROM meta WHERE key = 'last_refresh'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(s.parse().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add src-tauri/src/store/
git commit -m "feat: 实现 AppStore（~/.agent-prism/cache.db）"
```

---

## Task 8: 实现 Tauri Commands

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 定义 SummaryData 结构体，实现三个 command**

```rust
// src-tauri/src/commands.rs

use crate::billing::BillingMatrix;
use crate::data_source::codex::CodexSource;
use crate::data_source::codex::reconciler::reconcile;
use crate::data_source::{AgentSource, CommandResult, ThreadRecord};
use crate::data_source::codex::reconciler::ReconcileResult;
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
```

- [ ] **Step 2: 更新 lib.rs，注册所有模块和 commands**

将 `src-tauri/src/lib.rs` 替换为：

```rust
mod billing;
mod commands;
mod data_source;
mod store;

use commands::{get_summary, get_threads, refresh};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_summary, get_threads, refresh])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

预期：编译成功，无错误

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: 实现 get_summary/get_threads/refresh Tauri commands"
```

---

## Task 9: 配置 Tauri 窗口与托盘插件

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json`

- [ ] **Step 1: 更新 tauri.conf.json**

将 `src-tauri/tauri.conf.json` 替换为：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "agent-prism",
  "version": "0.1.0",
  "identifier": "ink.laoliang.agent-prism",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "AgentPrism",
        "width": 900,
        "height": 620,
        "decorations": false,
        "vibrancy": "under-window",
        "visible": false
      }
    ],
    "trayIcon": {
      "iconPath": "icons/32x32.png",
      "iconAsTemplate": true
    },
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "tray": {}
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 2: 更新 capabilities/default.json**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "tray:default"
  ]
}
```

- [ ] **Step 3: 添加前端 tray 依赖**

```bash
pnpm add @tauri-apps/plugin-tray
```

- [ ] **Step 4: 提交**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json package.json pnpm-lock.yaml
git commit -m "feat: 配置无边框主窗口、vibrancy 毛玻璃、tray 插件"
```

---

## Task 10: 实现前端 Vue 结构

**Files:**
- Create: `src/composables/useStats.ts`
- Create: `src/components/TrayPanel.vue`
- Create: `src/components/ThreadList.vue`
- Create: `src/views/Dashboard.vue`
- Modify: `src/App.vue`

- [ ] **Step 1: 实现 useStats composable**

```typescript
// src/composables/useStats.ts
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ReconcileResult {
  sqlite_total: number
  jsonl_total: number
  diff: number
  diff_rate: number
  warning: string | null
}

export interface SummaryData {
  total_tokens: number
  thread_count: number
  session_count: number
  estimated_cost_usd: number
  top_project: string | null
  reconcile: ReconcileResult
}

export interface ThreadRecord {
  id: string
  title: string
  cwd: string
  model: string
  model_provider: string
  tokens_used: number
  created_at: string
  updated_at: string
  source: string
}

export interface CommandResult<T> {
  data: T | null
  error: string | null
  warnings: string[]
}

export function useStats() {
  const summary = ref<SummaryData | null>(null)
  const threads = ref<ThreadRecord[]>([])
  const warnings = ref<string[]>([])
  const error = ref<string | null>(null)
  const loading = ref(false)

  async function loadSummary() {
    loading.value = true
    error.value = null
    try {
      const result = await invoke<CommandResult<SummaryData>>('get_summary')
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

  async function loadThreads() {
    loading.value = true
    try {
      const result = await invoke<CommandResult<ThreadRecord[]>>('get_threads')
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

  async function refresh() {
    await invoke('refresh')
    await loadSummary()
    await loadThreads()
  }

  return { summary, threads, warnings, error, loading, loadSummary, loadThreads, refresh }
}
```

- [ ] **Step 2: 实现 TrayPanel.vue**

```vue
<!-- src/components/TrayPanel.vue -->
<script setup lang="ts">
import { onMounted } from 'vue'
import { useStats } from '../composables/useStats'

const { summary, warnings, error, loading, loadSummary } = useStats()

onMounted(() => loadSummary())

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

function formatCost(usd: number): string {
  return '$' + usd.toFixed(4)
}

function openDashboard() {
  // TODO Task 11: 通过 tray 事件打开主窗口
}
</script>

<template>
  <div class="tray-panel">
    <div v-if="loading" class="state-empty">加载中…</div>
    <div v-else-if="error" class="state-empty">{{ error }}</div>
    <div v-else-if="!summary" class="state-empty">未检测到 Codex 数据</div>
    <template v-else>
      <div class="metric-row">
        <span class="metric-label">Token 总量</span>
        <span class="metric-value">{{ formatTokens(summary.total_tokens) }}</span>
      </div>
      <div class="metric-row">
        <span class="metric-label">估算费用</span>
        <span class="metric-value accent">{{ formatCost(summary.estimated_cost_usd) }}</span>
      </div>
      <div class="metric-row" v-if="summary.top_project">
        <span class="metric-label">最活跃项目</span>
        <span class="metric-value project">{{ summary.top_project.split('/').at(-1) }}</span>
      </div>
      <div class="estimate-note">估算，非真实账单</div>
      <div v-if="warnings.length > 0" class="warning-note">⚠ 部分数据可能不完整</div>
    </template>
    <button class="open-btn" @click="openDashboard">打开看板</button>
  </div>
</template>

<style scoped>
.tray-panel {
  padding: 16px;
  min-width: 220px;
  font-family: -apple-system, sans-serif;
  color: #f0f0f0;
}
.metric-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}
.metric-label {
  font-size: 11px;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.metric-value {
  font-size: 20px;
  font-weight: 200;
  color: #f0f0f0;
}
.metric-value.accent { color: #4FC3F7; }
.metric-value.project { font-size: 13px; font-weight: 400; color: #ccc; }
.estimate-note {
  font-size: 10px;
  color: #555;
  margin-top: 4px;
}
.warning-note {
  font-size: 11px;
  color: #FFB74D;
  margin-top: 6px;
}
.open-btn {
  margin-top: 12px;
  width: 100%;
  padding: 6px;
  background: rgba(255,255,255,0.08);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 6px;
  color: #f0f0f0;
  font-size: 12px;
  cursor: pointer;
}
.open-btn:hover { background: rgba(255,255,255,0.14); }
.state-empty { font-size: 13px; color: #666; padding: 8px 0; }
</style>
```

- [ ] **Step 3: 实现 ThreadList.vue**

```vue
<!-- src/components/ThreadList.vue -->
<script setup lang="ts">
import type { ThreadRecord } from '../composables/useStats'

defineProps<{ threads: ThreadRecord[] }>()

function shortPath(cwd: string): string {
  const parts = cwd.split('/')
  return parts.at(-1) || cwd
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="thread-list">
    <div v-if="threads.length === 0" class="empty">暂无线程数据</div>
    <div v-for="t in threads" :key="t.id" class="thread-item">
      <div class="thread-title">{{ t.title || '(无标题)' }}</div>
      <div class="thread-meta">
        <span class="project">{{ shortPath(t.cwd) }}</span>
        <span class="model">{{ t.model }}</span>
        <span class="tokens">{{ formatTokens(t.tokens_used) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.thread-list { display: flex; flex-direction: column; gap: 1px; }
.empty { color: #555; font-size: 13px; padding: 16px; text-align: center; }
.thread-item {
  padding: 10px 16px;
  border-bottom: 1px solid rgba(255,255,255,0.05);
}
.thread-item:hover { background: rgba(255,255,255,0.04); }
.thread-title { font-size: 13px; color: #e0e0e0; margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.thread-meta { display: flex; gap: 10px; font-size: 11px; color: #555; }
.project { color: #4FC3F7; }
.tokens { margin-left: auto; color: #aaa; }
</style>
```

- [ ] **Step 4: 实现 Dashboard.vue**

```vue
<!-- src/views/Dashboard.vue -->
<script setup lang="ts">
import { onMounted } from 'vue'
import { useStats } from '../composables/useStats'
import ThreadList from '../components/ThreadList.vue'

const { summary, threads, warnings, error, loading, loadSummary, loadThreads, refresh } = useStats()

onMounted(async () => {
  await loadSummary()
  await loadThreads()
})

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="dashboard">
    <header class="header">
      <span class="logo">AgentPrism</span>
      <button class="refresh-btn" @click="refresh" :disabled="loading">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </header>

    <div v-if="error" class="error-state">{{ error }}</div>

    <template v-else>
      <div class="summary-bar" v-if="summary">
        <div class="stat">
          <div class="stat-value">{{ formatTokens(summary.total_tokens) }}</div>
          <div class="stat-label">Token 总量</div>
        </div>
        <div class="stat">
          <div class="stat-value accent">${{ summary.estimated_cost_usd.toFixed(4) }}</div>
          <div class="stat-label">估算费用</div>
        </div>
        <div class="stat">
          <div class="stat-value">{{ summary.thread_count }}</div>
          <div class="stat-label">线程数</div>
        </div>
        <div class="stat">
          <div class="stat-value">{{ summary.session_count }}</div>
          <div class="stat-label">Session 数</div>
        </div>
        <div class="reconcile" :class="{ warn: summary.reconcile.warning }">
          对账差异率 {{ (summary.reconcile.diff_rate * 100).toFixed(1) }}%
        </div>
      </div>

      <div v-if="warnings.length > 0" class="warnings">
        <span v-for="w in warnings" :key="w" class="warning-item">⚠ {{ w }}</span>
      </div>

      <div class="section-title">线程列表</div>
      <ThreadList :threads="threads" />

      <div class="estimate-footer">估算，非真实账单</div>
    </template>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  height: 100vh;
  font-family: -apple-system, sans-serif;
  color: #e0e0e0;
  overflow: hidden;
}
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  -webkit-app-region: drag;
}
.logo { font-size: 14px; font-weight: 500; letter-spacing: 0.08em; color: #fff; }
.refresh-btn {
  -webkit-app-region: no-drag;
  background: rgba(255,255,255,0.07);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 5px;
  color: #ccc;
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
}
.refresh-btn:hover { background: rgba(255,255,255,0.12); }
.summary-bar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
}
.stat { text-align: center; }
.stat-value { font-size: 22px; font-weight: 200; }
.stat-value.accent { color: #4FC3F7; }
.stat-label { font-size: 10px; color: #555; text-transform: uppercase; margin-top: 2px; }
.reconcile { margin-left: auto; font-size: 11px; color: #555; }
.reconcile.warn { color: #FFB74D; }
.warnings { padding: 8px 20px; display: flex; flex-direction: column; gap: 2px; }
.warning-item { font-size: 11px; color: #FFB74D; }
.section-title { padding: 10px 20px 4px; font-size: 11px; color: #444; text-transform: uppercase; letter-spacing: 0.06em; }
.estimate-footer { padding: 8px 20px; font-size: 10px; color: #444; border-top: 1px solid rgba(255,255,255,0.04); }
.error-state { padding: 40px 20px; text-align: center; color: #666; }
</style>
```

- [ ] **Step 5: 更新 App.vue**

```vue
<!-- src/App.vue -->
<script setup lang="ts">
import Dashboard from './views/Dashboard.vue'
</script>

<template>
  <Dashboard />
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body {
  background: transparent;
  height: 100%;
  overflow: hidden;
}
</style>
```

- [ ] **Step 6: 提交**

```bash
git add src/
git commit -m "feat: 实现前端 Vue 组件（TrayPanel、ThreadList、Dashboard）"
```

---

## Task 11: 配置系统托盘逻辑

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 更新 lib.rs 添加托盘初始化**

将 `src-tauri/src/lib.rs` 替换为：

```rust
mod billing;
mod commands;
mod data_source;
mod store;

use commands::{get_summary, get_threads, refresh};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_tray::init())
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "退出 AgentPrism", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "打开看板", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_summary, get_threads, refresh])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 更新 TrayPanel.vue 中的 openDashboard**

将 `src/components/TrayPanel.vue` 中的 `openDashboard` 函数替换为：

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window'

async function openDashboard() {
  // 通过主窗口展示（托盘面板与主窗口共用 main window）
  const win = getCurrentWindow()
  await win.show()
  await win.setFocus()
}
```

- [ ] **Step 3: 编译验证**

```bash
cd src-tauri && cargo build 2>&1 | tail -30
```

预期：编译成功

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/lib.rs src/components/TrayPanel.vue
git commit -m "feat: 配置系统托盘，左键点击切换主窗口显示/隐藏"
```

---

## Task 12: 运行全量测试 + 端到端集成验证

- [ ] **Step 1: 运行所有 Rust 单元测试**

```bash
cd src-tauri && cargo test 2>&1
```

预期：所有测试 PASS，无编译错误

- [ ] **Step 2: 启动开发模式**

```bash
pnpm tauri dev
```

预期：App 启动，托盘图标出现，点击图标显示主窗口，主窗口加载 Dashboard

- [ ] **Step 3: 集成验证清单**

手动核对：
- [ ] 托盘图标出现在菜单栏
- [ ] 左键点击托盘图标，主窗口切换显示/隐藏
- [ ] 右键菜单显示"打开看板"和"退出 AgentPrism"
- [ ] Dashboard 显示 Token 总量、估算费用、线程数、Session 数
- [ ] 估算费用旁边标注估算字样
- [ ] 线程列表可以滚动
- [ ] 若 `~/.codex/` 不存在，显示"未检测到 Codex 数据"
- [ ] 若对账差异率 > 5%，显示橙色警告

- [ ] **Step 4: 与 Codex 统计数字人工对比**

```bash
# 查看 Codex 本地 SQLite 总 token（用于对比）
sqlite3 ~/.codex/state_*.sqlite "SELECT sum(tokens_used) FROM threads;"
```

记录差异率，确认在合理范围内。

- [ ] **Step 5: 最终提交**

```bash
git add .
git commit -m "chore: V0.1 集成验证完成"
```

---

## 自审结果

1. **Spec 覆盖**：所有 V0.1 需求均有对应 Task：SQLite 读取（Task 3）、JSONL 解析（Task 4）、对账（Task 5）、计费矩阵（Task 6）、缓存 Store（Task 7）、Tauri Commands（Task 8）、窗口配置（Task 9）、前端 UI（Task 10）、托盘逻辑（Task 11）。

2. **占位符检查**：无 TBD/TODO（TrayPanel.vue 中 openDashboard 已在 Task 11 中完整实现，注释只是提醒顺序）。

3. **类型一致性**：`ThreadRecord`、`SessionRecord`、`CommandResult<T>`、`ReconcileResult` 在 Task 2 定义，后续 Task 使用相同字段名。前端 TypeScript 接口与 Rust 结构体字段一一对应（snake_case）。

4. **依赖顺序**：Task 1（依赖）→ Task 2（类型）→ Task 3-6（实现）→ Task 7（Store）→ Task 8（Commands）→ Task 9（配置）→ Task 10（前端）→ Task 11（托盘）→ Task 12（验证）。
