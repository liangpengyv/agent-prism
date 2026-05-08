use crate::data_source::SessionRecord;
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
    pub prices: HashMap<String, ModelPrice>,
}

impl BillingMatrix {
    pub fn default_prices() -> HashMap<String, ModelPrice> {
        let mut m = HashMap::new();
        // Codex 系列
        m.insert("codex-mini".into(), ModelPrice {
            input_per_1m: 1.5,
            cached_input_per_1m: 0.375,
            output_per_1m: 6.0,
        });
        // GPT-4.1 系列
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
        // GPT-5 系列（价格为估算，以 OpenAI 官方公布为准）
        m.insert("gpt-5.5".into(), ModelPrice {
            input_per_1m: 10.0,
            cached_input_per_1m: 2.5,
            output_per_1m: 40.0,
        });
        m.insert("gpt-5.4".into(), ModelPrice {
            input_per_1m: 5.0,
            cached_input_per_1m: 1.25,
            output_per_1m: 20.0,
        });
        m.insert("gpt-5.4-mini".into(), ModelPrice {
            input_per_1m: 1.0,
            cached_input_per_1m: 0.25,
            output_per_1m: 4.0,
        });
        m.insert("gpt-5.3-codex".into(), ModelPrice {
            input_per_1m: 3.0,
            cached_input_per_1m: 0.75,
            output_per_1m: 12.0,
        });
        m.insert("gpt-5.2".into(), ModelPrice {
            input_per_1m: 2.5,
            cached_input_per_1m: 0.625,
            output_per_1m: 10.0,
        });
        m
    }

    pub fn new() -> Self {
        Self { prices: Self::default_prices() }
    }

    pub fn with_prices(prices: HashMap<String, ModelPrice>) -> Self {
        Self { prices }
    }

    /// 全局 fallback 均价：取已知模型中最低输入+输出均价
    /// 用于模型未知时提供非零的估算
    pub fn fallback_avg_per_token(&self) -> f64 {
        self.prices.values()
            .map(|p| p.avg_per_token())
            .fold(f64::MAX, f64::min)
            .min(3.75 / 1_000_000.0) // 上限 codex-mini 均价
    }

    pub fn estimate(&self, sessions: &[SessionRecord]) -> CostEstimate {
        let mut total_usd = 0.0;
        let mut breakdown: HashMap<String, f64> = HashMap::new();
        let fallback = self.fallback_avg_per_token();

        for session in sessions {
            let cost = if let Some(price) = self.prices.get(&session.model_provider) {
                let uncached = (session.input_tokens - session.cached_input_tokens).max(0);
                uncached as f64 / 1_000_000.0 * price.input_per_1m
                    + session.cached_input_tokens as f64 / 1_000_000.0 * price.cached_input_per_1m
                    + session.output_tokens as f64 / 1_000_000.0 * price.output_per_1m
            } else {
                // 未知模型用 fallback 均价估算
                session.total_tokens as f64 * fallback
            };

            total_usd += cost;
            *breakdown.entry(session.model_provider.clone()).or_insert(0.0) += cost;
        }

        CostEstimate { total_usd, breakdown, is_estimate: true }
    }

    /// 按 token 数直接估算（用于无 SessionRecord 的场合，如按项目/日期聚合）
    pub fn estimate_by_tokens(&self, model_provider: &str, total_tokens: i64) -> f64 {
        let price_per_token = if let Some(p) = self.prices.get(model_provider) {
            p.avg_per_token()
        } else {
            self.fallback_avg_per_token()
        };
        total_tokens as f64 * price_per_token
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
        // 1M uncached input + 0 cached + 1M output = 1.5 + 0 + 6.0 = 7.5
        let sessions = vec![make_session("codex-mini", 1_000_000, 0, 1_000_000)];
        let estimate = matrix.estimate(&sessions);
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
        let sessions_cached = vec![make_session("codex-mini", 1_000_000, 1_000_000, 0)];
        let sessions_uncached = vec![make_session("codex-mini", 1_000_000, 0, 0)];
        let cost_cached = matrix.estimate(&sessions_cached).total_usd;
        let cost_uncached = matrix.estimate(&sessions_uncached).total_usd;
        assert!(cost_cached < cost_uncached);
    }
}
