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
