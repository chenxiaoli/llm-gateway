use llm_gateway_storage::{
    HybridConfig, PerCharacterConfig, PerRequestConfig, PerTokenConfig,
    PricingPolicy, TieredTokenConfig, Usage,
};

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> i64 {
        match policy.billing_type.as_str() {
            "per_token" => self.calculate_per_token(&policy.config, usage),
            "per_request" => self.calculate_per_request(&policy.config, usage),
            "per_character" => self.calculate_per_character(&policy.config, usage),
            "tiered_token" => self.calculate_tiered_token(&policy.config, usage),
            "hybrid" => self.calculate_hybrid(&policy.config, usage),
            _ => 0,
        }
    }

    fn calculate_per_token(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: PerTokenConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let div: i64 = cfg.divisor();
        let mut cost: i64 = 0;
        if usage.input_tokens > 0 {
            cost += (usage.input_tokens * cfg.input_price()) / div;
        }
        if usage.output_tokens > 0 {
            cost += (usage.output_tokens * cfg.output_price()) / div;
        }
        if let Some(t) = usage.cache_read_tokens {
            if t > 0 {
                cost += (t * cfg.cache_read_price()) / div;
            }
        }
        if let Some(t) = usage.cache_creation_tokens {
            if t > 0 {
                cost += (t * cfg.cache_creation_price()) / div;
            }
        }
        cost
    }

    fn calculate_per_request(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: PerRequestConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        usage.request_count * cfg.price_per_call()
    }

    fn calculate_per_character(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: PerCharacterConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let div: i64 = cfg.divisor();
        let input_chars = usage.input_chars.unwrap_or(0);
        let output_chars = usage.output_chars.unwrap_or(0);
        let mut cost: i64 = 0;
        if input_chars > 0 {
            cost += (input_chars * cfg.input_price()) / div;
        }
        if output_chars > 0 {
            cost += (output_chars * cfg.output_price()) / div;
        }
        cost
    }

    fn calculate_tiered_token(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: TieredTokenConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        if cfg.tiers.is_empty() {
            return 0;
        }

        let div = cfg.tier_divisor();
        let mut total_cost: i64 = 0;
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

            if tier_input > 0 {
                total_cost += (tier_input * tier.input_price()) / div;
            }
            if tier_output > 0 {
                total_cost += (tier_output * tier.output_price()) / div;
            }

            if remaining_input == 0 && remaining_output == 0 {
                break;
            }
        }

        total_cost
    }

    fn calculate_hybrid(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: HybridConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let div: i64 = cfg.divisor();
        let base: i64 = cfg.base_per_call.unwrap_or(0).max(0);
        let mut cost: i64 = usage.request_count * base;
        if usage.input_tokens > 0 {
            cost += (usage.input_tokens * cfg.input_price()) / div;
        }
        if usage.output_tokens > 0 {
            cost += (usage.output_tokens * cfg.output_price()) / div;
        }
        if let Some(t) = usage.cache_read_tokens {
            if t > 0 {
                cost += (t * cfg.cache_read_price()) / div;
            }
        }
        if let Some(t) = usage.cache_creation_tokens {
            if t > 0 {
                cost += (t * cfg.cache_creation_price()) / div;
            }
        }
        cost
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
            cache_creation_tokens: None,
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
            cache_creation_tokens: None,
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
            cache_creation_tokens: None,
        }
    }

    #[test]
    fn test_per_token() {
        let calc = PricingCalculator;
        // input_price_1m = 3_000_000 subunits ($3/M), output_price_1m = 15_000_000 subunits ($15/M)
        let policy = make_policy("per_token", json!({"input_price_1m": 3_000_000, "output_price_1m": 15_000_000}));
        let cost = calc.calculate_cost(&policy, &usage());
        // input: (1M * 3M) / 1M = 3_000_000
        // output: (500k * 15M) / 1M = 7_500_000
        // total = 10_500_000
        assert_eq!(cost, 10_500_000);
    }

    #[test]
    fn test_per_token_with_cache() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_price_1m": 3_000_000, "output_price_1m": 15_000_000, "cache_read_price_1m": 1_000_000}));
        // input_tokens = non-cache input (800k), cache_read_tokens = 200k
        let usage_with_cache = Usage { input_tokens: 800_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: Some(200_000), cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_with_cache);
        // input_cost: (800k * 3M) / 1M = 2_400_000
        // cache_cost: (200k * 1M) / 1M = 200_000
        // output_cost: (500k * 15M) / 1M = 7_500_000
        // total = 10_100_000
        assert_eq!(cost, 10_100_000);
    }

    #[test]
    fn test_per_request() {
        let calc = PricingCalculator;
        // request_price = 50_000 subunits ($0.05)
        let policy = make_policy("per_request", json!({"request_price": 50_000}));
        let usage_req = Usage { input_tokens: 0, output_tokens: 0, input_chars: None, output_chars: None, request_count: 100, cache_read_tokens: None, cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_req);
        // 100 * 50_000 = 5_000_000
        assert_eq!(cost, 5_000_000);
    }

    #[test]
    fn test_per_character() {
        let calc = PricingCalculator;
        // input_price_1m = 1_000_000 ($1/M chars), output_price_1m = 4_000_000 ($4/M chars)
        let policy = make_policy("per_character", json!({"input_price_1m": 1_000_000, "output_price_1m": 4_000_000}));
        let cost = calc.calculate_cost(&policy, &usage_chars());
        // input: (1M * 1M) / 1M = 1_000_000
        // output: (500k * 4M) / 1M = 2_000_000
        // total = 3_000_000
        assert_eq!(cost, 3_000_000);
    }

    #[test]
    fn test_tiered_token() {
        let calc = PricingCalculator;
        let policy = make_policy("tiered_token", json!({
            "tiers": [
                {"up_to": 1_000_000, "input_price_1m": 5_000_000, "output_price_1m": 15_000_000},
                {"up_to": null, "input_price_1m": 4_000_000, "output_price_1m": 12_000_000}
            ]
        }));
        let usage_tier = Usage { input_tokens: 1_000_000, output_tokens: 0, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: None, cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_tier);
        // 1M tokens at tier 1 ($5/M) = (1M * 5M) / 1M = 5_000_000
        assert_eq!(cost, 5_000_000);
    }

    #[test]
    fn test_hybrid() {
        let calc = PricingCalculator;
        // base_per_call = 1_000 subunits ($0.001), input_price_1m = 1_000_000 ($1/M), output_price_1m = 3_000_000 ($3/M)
        let policy = make_policy("hybrid", json!({"base_per_call": 1_000, "input_price_1m": 1_000_000, "output_price_1m": 3_000_000}));
        let cost = calc.calculate_cost(&policy, &usage_hybrid());
        // base: 10 * 1_000 = 10_000
        // input: (1M * 1M) / 1M = 1_000_000
        // output: (500k * 3M) / 1M = 1_500_000
        // total = 2_510_000
        assert_eq!(cost, 2_510_000);
    }

    }
