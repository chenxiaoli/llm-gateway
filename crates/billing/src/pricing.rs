use llm_gateway_storage::{
    ContextTieredTokenConfig, HybridConfig, PerCharacterConfig, PerRequestConfig,
    PerTokenConfig, PricingPolicy, TieredTokenConfig, Usage,
};

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> i64 {
        match policy.billing_type.as_str() {
            "per_token" => self.calculate_per_token(&policy.config, usage),
            "per_request" => self.calculate_per_request(&policy.config, usage),
            "per_character" => self.calculate_per_character(&policy.config, usage),
            "tiered_token" => self.calculate_tiered_token(&policy.config, usage),
            "context_tiered" => self.calculate_context_tiered(&policy.config, usage),
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

    fn calculate_context_tiered(&self, config: &serde_json::Value, usage: &Usage) -> i64 {
        let cfg: ContextTieredTokenConfig = match serde_json::from_value(config.clone()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        if cfg.tiers.is_empty() {
            return 0;
        }

        // Find the tier matching the total input token count.
        // Tiers are ordered by up_to (ascending); null = final catch-all.
        let input = usage.input_tokens;
        let tier = cfg.tiers.iter().find(|t| {
            match t.up_to {
                Some(limit) => input < limit,
                None => true,
            }
        });

        let tier = match tier {
            Some(t) => t,
            None => return 0,
        };

        let div = cfg.divisor();
        let mut cost: i64 = 0;
        if usage.input_tokens > 0 {
            cost += (usage.input_tokens * tier.input_price()) / div;
        }
        if usage.output_tokens > 0 {
            cost += (usage.output_tokens * tier.output_price()) / div;
        }
        if let Some(t) = usage.cache_read_tokens {
            if t > 0 {
                cost += (t * tier.cache_read_price()) / div;
            }
        }
        if let Some(t) = usage.cache_creation_tokens {
            if t > 0 {
                cost += (t * tier.cache_creation_price()) / div;
            }
        }
        cost
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
        // Prices in integer subunits (100M per USD): $3/M = 300_000_000, $15/M = 1_500_000_000
        let policy = make_policy("per_token", json!({"input_price_1m": 300_000_000, "output_price_1m": 1_500_000_000}));
        let cost = calc.calculate_cost(&policy, &usage());
        // input: (1M * 300M) / 1M = 300_000_000 subunits ($3.00)
        // output: (500k * 1500M) / 1M = 750_000_000 subunits ($7.50)
        // total = 1_050_000_000 subunits ($10.50)
        assert_eq!(cost, 1_050_000_000);
    }

    #[test]
    fn test_per_token_with_cache() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_price_1m": 300_000_000, "output_price_1m": 1_500_000_000, "cache_read_price_1m": 100_000_000}));
        // input_tokens = non-cache input (800k), cache_read_tokens = 200k
        let usage_with_cache = Usage { input_tokens: 800_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: Some(200_000), cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_with_cache);
        // input_cost: (800k * 300M) / 1M = 240_000_000 ($2.40)
        // cache_cost: (200k * 100M) / 1M = 20_000_000 ($0.20)
        // output_cost: (500k * 1500M) / 1M = 750_000_000 ($7.50)
        // total = 1_010_000_000 ($10.10)
        assert_eq!(cost, 1_010_000_000);
    }

    #[test]
    fn test_per_request() {
        let calc = PricingCalculator;
        // request_price = 5_000_000 subunits ($0.05)
        let policy = make_policy("per_request", json!({"request_price": 5_000_000}));
        let usage_req = Usage { input_tokens: 0, output_tokens: 0, input_chars: None, output_chars: None, request_count: 100, cache_read_tokens: None, cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_req);
        // 100 * 5_000_000 = 500_000_000 ($5.00)
        assert_eq!(cost, 500_000_000);
    }

    #[test]
    fn test_per_character() {
        let calc = PricingCalculator;
        // input_price_1m = 100_000_000 ($1/M chars), output_price_1m = 400_000_000 ($4/M chars)
        let policy = make_policy("per_character", json!({"input_price_1m": 100_000_000, "output_price_1m": 400_000_000}));
        let cost = calc.calculate_cost(&policy, &usage_chars());
        // input: (1M * 100M) / 1M = 100_000_000 ($1.00)
        // output: (500k * 400M) / 1M = 200_000_000 ($2.00)
        // total = 300_000_000 ($3.00)
        assert_eq!(cost, 300_000_000);
    }

    #[test]
    fn test_tiered_token() {
        let calc = PricingCalculator;
        let policy = make_policy("tiered_token", json!({
            "tiers": [
                {"up_to": 1_000_000, "input_price_1m": 500_000_000, "output_price_1m": 1_500_000_000},
                {"up_to": null, "input_price_1m": 400_000_000, "output_price_1m": 1_200_000_000}
            ]
        }));
        let usage_tier = Usage { input_tokens: 1_000_000, output_tokens: 0, input_chars: None, output_chars: None, request_count: 1, cache_read_tokens: None, cache_creation_tokens: None };
        let cost = calc.calculate_cost(&policy, &usage_tier);
        // 1M tokens at tier 1 ($5/M) = (1M * 500M) / 1M = 500_000_000 ($5.00)
        assert_eq!(cost, 500_000_000);
    }

    #[test]
    fn test_hybrid() {
        let calc = PricingCalculator;
        // base_per_call = 100_000 subunits ($0.001), input_price_1m = 100_000_000 ($1/M), output_price_1m = 300_000_000 ($3/M)
        let policy = make_policy("hybrid", json!({"base_per_call": 100_000, "input_price_1m": 100_000_000, "output_price_1m": 300_000_000}));
        let cost = calc.calculate_cost(&policy, &usage_hybrid());
        // base: 10 * 100_000 = 1_000_000 ($0.01)
        // input: (1M * 100M) / 1M = 100_000_000 ($1.00)
        // output: (500k * 300M) / 1M = 150_000_000 ($1.50)
        // total = 251_000_000 ($2.51)
        assert_eq!(cost, 251_000_000);
    }

    #[test]
    fn test_context_tiered_first_tier() {
        let calc = PricingCalculator;
        // Tiers: <32K → ¥6/M input, ¥24/M output; >=32K → ¥8/M input, ¥28/M output
        // Using CNY subunits (100M per ¥)
        let policy = make_policy("context_tiered", json!({
            "tiers": [
                {"up_to": 32000, "input_price_1m": 600_000_000, "output_price_1m": 2_400_000_000_i64,
                 "cache_read_price_1m": 130_000_000, "cache_creation_price_1m": 0},
                {"up_to": null, "input_price_1m": 800_000_000, "output_price_1m": 2_800_000_000_i64,
                 "cache_read_price_1m": 200_000_000, "cache_creation_price_1m": 0}
            ]
        }));
        // 20K input + 10K output → falls in tier 1 (< 32K)
        let usage_t1 = Usage {
            input_tokens: 20_000, output_tokens: 10_000, input_chars: None, output_chars: None,
            request_count: 1, cache_read_tokens: None, cache_creation_tokens: None,
        };
        let cost = calc.calculate_cost(&policy, &usage_t1);
        // input: (20K * 600M) / 1M = 12_000_000 subunits
        // output: (10K * 2400M) / 1M = 24_000_000 subunits
        // total = 36_000_000
        assert_eq!(cost, 36_000_000);
    }

    #[test]
    fn test_context_tiered_second_tier() {
        let calc = PricingCalculator;
        let policy = make_policy("context_tiered", json!({
            "tiers": [
                {"up_to": 32000, "input_price_1m": 600_000_000, "output_price_1m": 2_400_000_000_i64,
                 "cache_read_price_1m": 130_000_000, "cache_creation_price_1m": 0},
                {"up_to": null, "input_price_1m": 800_000_000, "output_price_1m": 2_800_000_000_i64,
                 "cache_read_price_1m": 200_000_000, "cache_creation_price_1m": 0}
            ]
        }));
        // 40K input + 10K output → falls in tier 2 (>= 32K), ALL tokens at tier 2 prices
        let usage_t2 = Usage {
            input_tokens: 40_000, output_tokens: 10_000, input_chars: None, output_chars: None,
            request_count: 1, cache_read_tokens: Some(5_000), cache_creation_tokens: None,
        };
        let cost = calc.calculate_cost(&policy, &usage_t2);
        // input: (40K * 800M) / 1M = 32_000_000
        // output: (10K * 2800M) / 1M = 28_000_000
        // cache_read: (5K * 200M) / 1M = 1_000_000
        // total = 61_000_000
        assert_eq!(cost, 61_000_000);
    }

    #[test]
    fn test_context_tiered_boundary() {
        let calc = PricingCalculator;
        let policy = make_policy("context_tiered", json!({
            "tiers": [
                {"up_to": 32000, "input_price_1m": 600_000_000, "output_price_1m": 2_400_000_000_i64},
                {"up_to": null, "input_price_1m": 800_000_000, "output_price_1m": 2_800_000_000_i64}
            ]
        }));
        // Exactly 32K → NOT < 32000, so tier 2
        let usage = Usage {
            input_tokens: 32_000, output_tokens: 0, input_chars: None, output_chars: None,
            request_count: 1, cache_read_tokens: None, cache_creation_tokens: None,
        };
        let cost = calc.calculate_cost(&policy, &usage);
        // (32K * 800M) / 1M = 25_600_000
        assert_eq!(cost, 25_600_000);
    }

    }
