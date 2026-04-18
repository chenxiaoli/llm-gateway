pub mod pricing;

pub use pricing::PricingCalculator;
pub use llm_gateway_storage::{PricingPolicy, Usage};

use llm_gateway_storage::BillingType;

pub struct CostCalculation {
    pub cost: f64,
}

pub fn calculate_cost(
    billing_type: &BillingType,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    input_price: f64,   // per 1M tokens
    output_price: f64,  // per 1M tokens
    request_price: f64, // per request
) -> CostCalculation {
    calculate_cost_with_cache(input_tokens, output_tokens, input_price, output_price, request_price, 0.0, None, billing_type)
}

pub fn calculate_cost_with_cache(
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    input_price: f64,           // per 1M tokens
    output_price: f64,          // per 1M tokens
    request_price: f64,         // per request
    cache_read_price: f64,      // per 1M tokens (for cache reads, cheaper)
    cache_read_tokens: Option<i64>,
    billing_type: &BillingType,
) -> CostCalculation {
    let cost = match billing_type {
        BillingType::Token => {
            let cache = cache_read_tokens.unwrap_or(0);
            let thinking_input = input_tokens.unwrap_or(0).saturating_sub(cache);
            let cache_cost = (cache as f64 / 1_000_000.0) * cache_read_price;
            let thinking_cost = (thinking_input as f64 / 1_000_000.0) * input_price;
            let output_cost = (output_tokens.unwrap_or(0) as f64 / 1_000_000.0) * output_price;
            cache_cost + thinking_cost + output_cost
        }
        BillingType::Request => request_price,
    };
    CostCalculation { cost }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_billing() {
        let calc = calculate_cost(
            &BillingType::Token,
            Some(1_000_000),
            Some(500_000),
            3.0,  // $3/1M input
            15.0, // $15/1M output
            0.0,
        );
        assert!((calc.cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_token_billing_zero_tokens() {
        let calc = calculate_cost(&BillingType::Token, Some(0), Some(0), 3.0, 15.0, 0.0);
        assert!((calc.cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_token_billing_none_tokens() {
        let calc = calculate_cost(&BillingType::Token, None, None, 3.0, 15.0, 0.0);
        assert!((calc.cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_request_billing() {
        let calc = calculate_cost(&BillingType::Request, None, None, 0.0, 0.0, 0.05);
        assert!((calc.cost - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_request_billing_ignores_tokens() {
        let calc = calculate_cost(&BillingType::Request, Some(999), Some(999), 999.0, 999.0, 0.05);
        assert!((calc.cost - 0.05).abs() < 0.001);
    }
}
