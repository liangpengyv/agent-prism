use crate::data_source::SessionRecord;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_1m: f64,
    pub cached_input_per_1m: f64,
    pub output_per_1m: f64,
}

impl ModelPrice {
    pub fn avg_per_token(&self) -> f64 {
        (self.input_per_1m + self.output_per_1m) / 2.0 / 1_000_000.0
    }
}

#[derive(Debug, Serialize)]
pub struct CostEstimate {
    pub total_usd: f64,
    pub breakdown: HashMap<String, f64>,
    pub is_estimate: bool,
}

pub struct BillingMatrix {
    pub prices: IndexMap<String, ModelPrice>,
}

impl BillingMatrix {
    pub fn default_prices() -> IndexMap<String, ModelPrice> {
        let mut m = IndexMap::new();
        // GPT-5 系列（价格来源：OpenAI API Pricing https://openai.com/api/pricing/）
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

    pub fn new() -> Self {
        Self { prices: Self::default_prices() }
    }

    pub fn with_prices(prices: IndexMap<String, ModelPrice>) -> Self {
        Self { prices }
    }

    /// 全局 fallback 均价：取已知模型中最低输入+输出均价
    /// 用于模型未知时提供非零的估算
    pub fn fallback_avg_per_token(&self) -> f64 {
        self.prices.values()
            .map(|p| p.avg_per_token())
            .fold(f64::MAX, f64::min)
            .min(2.625 / 1_000_000.0) // 上限 gpt-5.4-mini 均价
    }

    pub fn estimate(&self, sessions: &[SessionRecord]) -> CostEstimate {
        let mut total_usd = 0.0;
        let mut breakdown: HashMap<String, f64> = HashMap::new();
        let fallback = self.fallback_avg_per_token();

        for session in sessions {
            let cost = if let Some(price) = self.prices.get(&session.model) {
                let uncached = (session.input_tokens - session.cached_input_tokens).max(0);
                uncached as f64 / 1_000_000.0 * price.input_per_1m
                    + session.cached_input_tokens as f64 / 1_000_000.0 * price.cached_input_per_1m
                    + session.output_tokens as f64 / 1_000_000.0 * price.output_per_1m
            } else {
                session.total_tokens as f64 * fallback
            };

            total_usd += cost;
            *breakdown.entry(session.model.clone()).or_insert(0.0) += cost;
        }

        CostEstimate { total_usd, breakdown, is_estimate: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(model: &str, input: i64, cached: i64, output: i64) -> SessionRecord {
        SessionRecord {
            session_id: "s1".into(),
            cwd: "".into(),
            model: model.into(),
            model_provider: "openai".into(),
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
        // gpt-5.4-mini: input=0.75, cached=0.075, output=4.5 per 1M
        // 1M uncached input + 0 cached + 1M output = 0.75 + 0 + 4.5 = 5.25
        let sessions = vec![make_session("gpt-5.4-mini", 1_000_000, 0, 1_000_000)];
        let estimate = matrix.estimate(&sessions);
        assert!((estimate.total_usd - 5.25).abs() < 0.001);
        assert!(estimate.is_estimate);
    }

    #[test]
    fn test_unknown_model_costs_zero() {
        let matrix = BillingMatrix::new();
        let sessions = vec![make_session("unknown-model-xyz", 1_000_000, 0, 1_000_000)];
        let estimate = matrix.estimate(&sessions);
        // 未知模型现在用 fallback 均价估算，不再是 0
        assert!(estimate.total_usd > 0.0);
    }

    #[test]
    fn test_cached_input_cheaper() {
        let matrix = BillingMatrix::new();
        let sessions_cached = vec![make_session("gpt-5.4-mini", 1_000_000, 1_000_000, 0)];
        let sessions_uncached = vec![make_session("gpt-5.4-mini", 1_000_000, 0, 0)];
        let cost_cached = matrix.estimate(&sessions_cached).total_usd;
        let cost_uncached = matrix.estimate(&sessions_uncached).total_usd;
        assert!(cost_cached < cost_uncached);
    }
}
