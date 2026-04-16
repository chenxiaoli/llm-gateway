use llm_gateway_storage::{PricingPolicy, Usage};

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> f64 {
        let cfg = &policy.config;
        match policy.billing_type.as_str() {
            "per_token" => self.calculate_per_token(cfg, usage),
            "per_request" => self.calculate_per_request(cfg, usage),
            "per_character" => self.calculate_per_character(cfg, usage),
            "tiered_token" => self.calculate_tiered_token(cfg, usage),
            "hybrid" => self.calculate_hybrid(cfg, usage),
            _ => 0.0, // Unknown billing type, safe default
        }
    }

    fn calculate_per_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        // Support both _per_1k and _per_1m naming conventions
        // If both exist, prefer _per_1m (per million)
        let has_per_1m = config.get("input_per_1m").is_some() || config.get("output_per_1m").is_some();
        let divisor = if has_per_1m { 1_000_000.0 } else { 1000.0 };

        let input_price = config.get("input_per_1m")
            .and_then(|v| v.as_f64())
            .or_else(|| config.get("input_per_1k").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let output_price = config.get("output_per_1m")
            .and_then(|v| v.as_f64())
            .or_else(|| config.get("output_per_1k").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        let input_cost = (usage.input_tokens as f64 / divisor) * input_price;
        let output_cost = (usage.output_tokens as f64 / divisor) * output_price;
        input_cost + output_cost
    }

    fn calculate_per_request(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let price_per_call = config.get("price_per_call")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        usage.request_count as f64 * price_per_call
    }

    fn calculate_per_character(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        // Support both _per_1k and _per_1m naming conventions
        let has_per_1m = config.get("input_per_1m").is_some() || config.get("output_per_1m").is_some();
        let divisor = if has_per_1m { 1_000_000.0 } else { 1000.0 };

        let input_price = config.get("input_per_1m")
            .and_then(|v| v.as_f64())
            .or_else(|| config.get("input_per_1k").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let output_price = config.get("output_per_1m")
            .and_then(|v| v.as_f64())
            .or_else(|| config.get("output_per_1k").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        let input_chars = usage.input_chars.unwrap_or(0);
        let output_chars = usage.output_chars.unwrap_or(0);

        (input_chars as f64 / divisor) * input_price + (output_chars as f64 / divisor) * output_price
    }

    fn calculate_tiered_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let tiers = config.get("tiers").and_then(|v| v.as_array());
        if tiers.is_none() {
            return 0.0;
        }

        // Support both _per_1k and _per_1m
        let has_per_1m = config.get("tiers")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().any(|t| t.get("input_per_1m").is_some() || t.get("output_per_1m").is_some()))
            .unwrap_or(false);
        let divisor = if has_per_1m { 1_000_000.0 } else { 1000.0 };

        let tiers = tiers.unwrap();
        let mut total_cost = 0.0;
        let mut remaining_input = usage.input_tokens;
        let mut remaining_output = usage.output_tokens;

        for tier in tiers {
            let up_to = tier.get("up_to").and_then(|v| v.as_i64());
            let input_rate = tier.get("input_per_1m")
                .and_then(|v| v.as_f64())
                .or_else(|| tier.get("input_per_1k").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let output_rate = tier.get("output_per_1m")
                .and_then(|v| v.as_f64())
                .or_else(|| tier.get("output_per_1k").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);

            // Calculate how much usage applies to this tier
            let (tier_input, tier_output) = if let Some(limit) = up_to {
                let input_in_tier = remaining_input.min(limit as i64);
                let output_in_tier = remaining_output.min(limit as i64);

                // Subtract what's been accounted for
                remaining_input -= input_in_tier;
                remaining_output -= output_in_tier;

                (input_in_tier, output_in_tier)
            } else {
                // Final tier - gets all remaining
                let input_in_tier = remaining_input;
                let output_in_tier = remaining_output;
                remaining_input = 0;
                remaining_output = 0;
                (input_in_tier, output_in_tier)
            };

            total_cost += (tier_input as f64 / divisor) * input_rate + (tier_output as f64 / divisor) * output_rate;

            // If no more usage, stop processing
            if remaining_input == 0 && remaining_output == 0 {
                break;
            }
        }

        total_cost
    }

    fn calculate_hybrid(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let base = config.get("base_per_call")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let usage_cost = self.calculate_per_token(config, usage);
        (usage.request_count as f64 * base) + usage_cost
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

    #[test]
    fn test_per_token() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_per_1m": 3.0, "output_per_1m": 15.0}));
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 1 };
        let cost = calc.calculate_cost(&policy, &usage);
        assert!((cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_per_request() {
        let calc = PricingCalculator;
        let policy = make_policy("per_request", json!({"price_per_call": 0.05}));
        let usage = Usage { input_tokens: 0, output_tokens: 0, input_chars: None, output_chars: None, request_count: 100 };
        let cost = calc.calculate_cost(&policy, &usage);
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_tiered_token() {
        let calc = PricingCalculator;
        let policy = make_policy("tiered_token", json!({
            "tiers": [
                {"up_to": 1000000, "input_per_1k": 5.0, "output_per_1k": 15.0},
                {"up_to": null, "input_per_1k": 4.0, "output_per_1k": 12.0}
            ]
        }));
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 0, input_chars: None, output_chars: None, request_count: 1 };
        let cost = calc.calculate_cost(&policy, &usage);
        // 1M tokens at tier 1 (5.0 per 1k) = 1000 * 5.0 = 5000
        assert!((cost - 5000.0).abs() < 0.001);
    }

    #[test]
    fn test_hybrid() {
        let calc = PricingCalculator;
        // Using _per_1k pricing for hybrid
        let policy = make_policy("hybrid", json!({"base_per_call": 0.001, "input_per_1k": 1.0, "output_per_1k": 3.0}));
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 10 };
        let cost = calc.calculate_cost(&policy, &usage);
        // base: 0.001 * 10 = 0.01
        // usage: 1M * 1.0 / 1000 + 500k * 3.0 / 1000 = 1000 + 1500 = 2500
        // total = 2500.01
        assert!((cost - 2500.01).abs() < 0.001);
    }

    #[test]
    fn test_per_character() {
        let calc = PricingCalculator;
        let policy = make_policy("per_character", json!({"input_per_1k": 0.001, "output_per_1k": 0.004}));
        let usage = Usage { input_tokens: 0, output_tokens: 0, input_chars: Some(1_000_000), output_chars: Some(500_000), request_count: 1 };
        let cost = calc.calculate_cost(&policy, &usage);
        // 1M input chars * 0.001/1k = 1.0
        // 500k output chars * 0.004/1k = 2.0
        assert!((cost - 3.0).abs() < 0.001);
    }
}