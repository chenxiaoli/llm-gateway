use llm_gateway_storage::{
    HybridConfig, PerCharacterConfig, PerRequestConfig, PerTokenConfig,
    PricingPolicy, TieredTokenConfig, Usage,
};

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> f64 {
        match policy.billing_type.as_str() {
            "per_token" => self.calculate_per_token(&policy.config, usage),
            "per_request" => self.calculate_per_request(&policy.config, usage),
            "per_character" => self.calculate_per_character(&policy.config, usage),
            "tiered_token" => self.calculate_tiered_token(&policy.config, usage),
            "hybrid" => self.calculate_hybrid(&policy.config, usage),
            _ => 0.0,
        }
    }

    fn calculate_per_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let cfg: PerTokenConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        // Detect divisor from raw JSON keys — _per_1k keys use per-1K divisor (1000),
        // _per_1m / _price_1m keys use per-1M divisor (1_000_000).
        let div = if config.get("input_per_1k").or_else(|| config.get("output_per_1k")).is_some() {
            1_000.0
        } else {
            1_000_000.0
        };
        let cache = usage.cache_read_tokens.unwrap_or(0);
        let thinking_input = usage.input_tokens.saturating_sub(cache);

        let input_cost = (thinking_input as f64 / div) * cfg.input_price();
        let cache_cost = (cache as f64 / div) * cfg.cache_read_price();
        let output_cost = (usage.output_tokens as f64 / div) * cfg.output_price();
        input_cost + cache_cost + output_cost
    }

    fn calculate_per_request(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let cfg: PerRequestConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        usage.request_count as f64 * cfg.price_per_call()
    }

    fn calculate_per_character(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let cfg: PerCharacterConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        let div = cfg.divisor();
        let input_chars = usage.input_chars.unwrap_or(0);
        let output_chars = usage.output_chars.unwrap_or(0);
        (input_chars as f64 / div) * cfg.input_price()
            + (output_chars as f64 / div) * cfg.output_price()
    }

    fn calculate_tiered_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let cfg: TieredTokenConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        if cfg.tiers.is_empty() {
            return 0.0;
        }

        let div = cfg.tier_divisor();
        let mut total_cost = 0.0;
        let mut remaining_input = usage.input_tokens;
        let mut remaining_output = usage.output_tokens;

        for tier in &cfg.tiers {
            let up_to = tier.up_to;
            let (tier_input, tier_output) = if let Some(limit) = up_to {
                let input_in_tier = remaining_input.min(limit);
                let output_in_tier = remaining_output.min(limit);
                remaining_input -= input_in_tier;
                remaining_output -= output_in_tier;
                (input_in_tier, output_in_tier)
            } else {
                let input_in_tier = remaining_input;
                let output_in_tier = remaining_output;
                remaining_input = 0;
                remaining_output = 0;
                (input_in_tier, output_in_tier)
            };

            total_cost +=
                (tier_input as f64 / div) * tier.input_price()
                + (tier_output as f64 / div) * tier.output_price();

            if remaining_input == 0 && remaining_output == 0 {
                break;
            }
        }

        total_cost
    }

    fn calculate_hybrid(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let cfg: HybridConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        let div = cfg.divisor();
        let cache = usage.cache_read_tokens.unwrap_or(0);
        let thinking_input = usage.input_tokens.saturating_sub(cache);

        let base = cfg.base_per_call.unwrap_or(0.0);
        let input_cost = (thinking_input as f64 / div) * cfg.input_price();
        let cache_cost = (cache as f64 / div) * cfg.cache_read_price();
        let output_cost = (usage.output_tokens as f64 / div) * cfg.output_price();

        (usage.request_count as f64 * base) + input_cost + cache_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_policy(billing_type: &str, config: serde_json::Value) -> PricingPolicy {
        PricingPolicy {
            id: "test".to_string(),
            name: "Test".to_string(),
            billing_type: billing_type.to_string(),
            config,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn usage() -> Usage {
        Usage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            input_chars: None,
            output_chars: None,
            request_count: 1,
            cache_read_tokens: None,
        }
    }

    fn usage_chars() -> Usage {
        Usage {
            input_tokens: 0,
            output_tokens: 0,
            input_chars: Some(1_000_000),
            output_chars: Some(500_000),
            request_count: 1,
            cache_read_tokens: None,
        }
    }

    fn usage_hybrid() -> Usage {
        Usage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            input_chars: None,
            output_chars: None,
            request_count: 10,
            cache_read_tokens: None,
        }
    }

    #[test]
    fn test_per_token() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_price_1m": 3.0, "output_price_1m": 15.0}));
        let cost = calc.calculate_cost(&policy, &usage());
        assert!((cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_per_token_with_cache() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_price_1m": 3.0, "output_price_1m": 15.0, "cache_read_price_1m": 1.0}));
        let usage_with_cache = Usage { input_tokens: 1_000_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: Some(200_000) };
        let cost = calc.calculate_cost(&policy, &usage_with_cache);
        // input_cost: (800k / 1M) * 3.0 = 2.4
        // cache_cost: (200k / 1M) * 1.0 = 0.2
        // output_cost: (500k / 1M) * 15.0 = 7.5
        // total = 10.1
        assert!((cost - 10.1).abs() < 0.001);
    }

    #[test]
    fn test_per_request() {
        let calc = PricingCalculator;
        let policy = make_policy("per_request", json!({"request_price": 0.05}));
        let usage_req = Usage { input_tokens: 0, output_tokens: 0, input_chars: None, output_chars: None, request_count: 100, cache_read_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_req);
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_per_character() {
        let calc = PricingCalculator;
        let policy = make_policy("per_character", json!({"input_price_1m": 1.0, "output_price_1m": 4.0}));
        let cost = calc.calculate_cost(&policy, &usage_chars());
        // input: 1M * 1.0 / 1M = 1.0
        // output: 500k * 4.0 / 1M = 2.0
        // total = 3.0
        assert!((cost - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_tiered_token() {
        let calc = PricingCalculator;
        let policy = make_policy("tiered_token", json!({
            "tiers": [
                {"up_to": 1_000_000, "input_price_1m": 5.0, "output_price_1m": 15.0},
                {"up_to": null, "input_price_1m": 4.0, "output_price_1m": 12.0}
            ]
        }));
        let usage_tier = Usage { input_tokens: 1_000_000, output_tokens: 0, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_tier);
        // 1M tokens at tier 1 (5.0 per 1M) = 5.0
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_hybrid() {
        let calc = PricingCalculator;
        let policy = make_policy("hybrid", json!({"base_per_call": 0.001, "input_price_1m": 1.0, "output_price_1m": 3.0}));
        let cost = calc.calculate_cost(&policy, &usage_hybrid());
        // base: 0.001 * 10 = 0.01
        // input: 1M * 1.0 / 1M = 1.0
        // output: 500k * 3.0 / 1M = 1.5
        // total = 2.51
        assert!((cost - 2.51).abs() < 0.001);
    }

    #[test]
    fn test_backward_compat_legacy_keys() {
        let calc = PricingCalculator;
        // Old-style keys from before the refactor
        let policy = make_policy("per_token", json!({"input_per_1m": 3.0, "output_per_1m": 15.0}));
        let cost = calc.calculate_cost(&policy, &usage());
        assert!((cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_backward_compat_per_1k_keys() {
        let calc = PricingCalculator;
        // Old _per_1k keys
        let policy = make_policy("per_token", json!({"input_per_1k": 3.0, "output_per_1k": 15.0}));
        let cost = calc.calculate_cost(&policy, &usage());
        // 1M input * 3.0 / 1000 = 3000
        // 500k output * 15.0 / 1000 = 7500
        // total = 10500
        assert!((cost - 10500.0).abs() < 0.001);
    }
}
