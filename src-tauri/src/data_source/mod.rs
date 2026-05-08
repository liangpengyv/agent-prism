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
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn discover(&self) -> anyhow::Result<Vec<PathBuf>>;
    fn threads(&self) -> anyhow::Result<(Vec<ThreadRecord>, Vec<String>)>;
    fn sessions(&self) -> anyhow::Result<(Vec<SessionRecord>, Vec<String>)>;
}