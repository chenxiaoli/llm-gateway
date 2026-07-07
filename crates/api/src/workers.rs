use llm_gateway_storage::Protocol;

/// Parse usage from response bytes.
/// Returns (input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens).
/// For streaming (SSE): extract from last JSON chunk before "data: [DONE]"
/// For non-streaming (JSON): extract from "usage" field.
pub fn parse_usage(bytes: &[u8], stream: bool, proto: Protocol) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    let parse_value = |usage: Option<&serde_json::Value>| -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
        match proto {
            Protocol::Openai => {
                let prompt_tokens = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_i64());
                let output = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_i64());
                let cache_read = usage
                    .and_then(|u| u.get("prompt_tokens_details"))
                    .and_then(|d| d.get("cache_read_tokens"))
                    .and_then(|t| t.as_i64());
                let cache_creation = usage
                    .and_then(|u| u.get("prompt_tokens_details"))
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(|t| t.as_i64());
                // OpenAI prompt_tokens includes cache; store non-cache input only
                let input = prompt_tokens.unwrap_or(0) - cache_read.unwrap_or(0) - cache_creation.unwrap_or(0);
                (Some(input), output, cache_read, cache_creation)
            }
            Protocol::Anthropic => {
                // Anthropic input_tokens already excludes cache; store as-is
                let input = usage.and_then(|u| u.get("input_tokens")).and_then(|t| t.as_i64());
                let output = usage.and_then(|u| u.get("output_tokens")).and_then(|t| t.as_i64());
                let cache_read = usage
                    .and_then(|u| u.get("cache_read_input_tokens"))
                    .and_then(|t| t.as_i64());
                let cache_creation = usage
                    .and_then(|u| u.get("cache_creation_input_tokens"))
                    .and_then(|t| t.as_i64());
                (input, output, cache_read, cache_creation)
            }
        }
    };

    if stream {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None, None),
        };
        let mut last_usage: Option<&str> = None;
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let json_str = &line["data: ".len()..];
            if json_str == "[DONE]" {
                break;
            }
            if json_str.contains("\"usage\"") {
                last_usage = Some(json_str);
            }
        }
        if let Some(json_str) = last_usage {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                return parse_value(v.get("usage"));
            }
        }
        (None, None, None, None)
    } else {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return (None, None, None, None),
        };
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return (None, None, None, None),
        };
        parse_value(v.get("usage"))
    }
}

/// Calculate cost from token usage and pricing policy.
pub fn calculate_cost(
    pricing_policy_config: &Option<serde_json::Value>,
    pricing_policy_billing_type: &str,
    markup_ratio: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
) -> i64 {
    use llm_gateway_billing::PricingCalculator;
    use llm_gateway_storage::{PricingPolicy, Usage};

    if let Some(config) = pricing_policy_config {
        let policy = PricingPolicy {
            id: String::new(),
            owner_org_id: None,
            name: String::new(),
            billing_type: pricing_policy_billing_type.to_string(),
            config: config.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let usage = Usage {
            input_tokens: input_tokens.unwrap_or(0),
            output_tokens: output_tokens.unwrap_or(0),
            input_chars: None,
            output_chars: None,
            request_count: 1,
            cache_read_tokens,
            cache_creation_tokens,
        };
        let raw_cost = PricingCalculator.calculate_cost(&policy, &usage);
        raw_cost * markup_ratio / 10_000
    } else {
        0
    }
}

/// Calculate weighted tokens from pricing policy.
/// Base: input token weight = 1. Other weights are relative to input_price.
pub fn calculate_weighted_tokens(
    pricing_policy_config: &Option<serde_json::Value>,
    pricing_policy_billing_type: &str,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
) -> i64 {
    let input = input_tokens.unwrap_or(0);
    let output = output_tokens.unwrap_or(0);
    let cache_read = cache_read_tokens.unwrap_or(0);
    let cache_creation = cache_creation_tokens.unwrap_or(0);

    let Some(config) = pricing_policy_config else {
        return input + output + cache_read + cache_creation;
    };

    match pricing_policy_billing_type {
        "per_token" | "hybrid" => {
            weighted_from_config(config, input, output, cache_read, cache_creation)
        }
        "tiered_token" => {
            weighted_from_tiered(config, input, output)
        }
        "context_tiered" => {
            weighted_from_context_tiered(config, input, output, cache_read, cache_creation)
        }
        "per_character" => {
            weighted_from_config(config, input, output, 0, 0)
        }
        _ => input + output, // per_request etc.
    }
}

fn price_weight(config: &serde_json::Value, field: &str, base_price: f64) -> f64 {
    let price = config.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base_price > 0.0 { price / base_price } else { 0.0 }
}

fn weighted_from_config(
    config: &serde_json::Value,
    input: i64, output: i64, cache_read: i64, cache_creation: i64,
) -> i64 {
    let base = config.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output + cache_read + cache_creation;
    }
    let w_out = price_weight(config, "output_price_1m", base);
    let w_cr = price_weight(config, "cache_read_price_1m", base);
    let w_cc = price_weight(config, "cache_creation_price_1m", base);
    (input as f64
        + output as f64 * w_out
        + cache_read as f64 * w_cr
        + cache_creation as f64 * w_cc
    ).round() as i64
}

fn weighted_from_tiered(config: &serde_json::Value, input: i64, output: i64) -> i64 {
    let tiers = match config.get("tiers").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return input + output,
    };
    let first = match tiers.first() {
        Some(t) => t,
        None => return input + output,
    };
    let base = first.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output;
    }
    let w_out = price_weight(first, "output_price_1m", base);
    (input as f64 + output as f64 * w_out).round() as i64
}

fn weighted_from_context_tiered(
    config: &serde_json::Value,
    input: i64, output: i64, cache_read: i64, cache_creation: i64,
) -> i64 {
    let tiers = match config.get("tiers").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return input + output + cache_read + cache_creation,
    };
    let first = match tiers.first() {
        Some(t) => t,
        None => return input + output + cache_read + cache_creation,
    };
    let base = first.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output + cache_read + cache_creation;
    }
    let w_out = price_weight(first, "output_price_1m", base);
    let w_cr = price_weight(first, "cache_read_price_1m", base);
    let w_cc = price_weight(first, "cache_creation_price_1m", base);
    (input as f64
        + output as f64 * w_out
        + cache_read as f64 * w_cr
        + cache_creation as f64 * w_cc
    ).round() as i64
}
