pub mod pricing;

pub use pricing::PricingCalculator;
pub use llm_gateway_storage::{PricingPolicy, Usage};

use llm_gateway_storage::BillingType;

pub struct CostCalculation {
    pub cost: i64,
}

pub fn calculate_cost(
    billing_type: &BillingType,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    input_price: i64,   // per 1M tokens (subunits)
    output_price: i64,  // per 1M tokens (subunits)
    request_price: i64, // per request (subunits)
) -> CostCalculation {
    calculate_cost_with_cache(input_tokens, output_tokens, input_price, output_price, request_price, 0, 0, None, None, billing_type)
}

pub fn calculate_cost_with_cache(
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    input_price: i64,           // per 1M tokens (subunits)
    output_price: i64,          // per 1M tokens (subunits)
    request_price: i64,         // per request (subunits)
    cache_read_price: i64,      // per 1M tokens (subunits, for cache reads)
    cache_creation_price: i64,  // per 1M tokens (subunits, for cache creation)
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
    billing_type: &BillingType,
) -> CostCalculation {
    let div: i64 = 1_000_000;
    let cost = match billing_type {
        BillingType::Token => {
            let cache = cache_read_tokens.unwrap_or(0);
            let thinking_input = input_tokens.unwrap_or(0).saturating_sub(cache);
            let cache_cost = if cache > 0 && cache_read_price > 0 {
                (cache * cache_read_price) / div
            } else { 0 };
            let cache_creation_cost = if let Some(t) = cache_creation_tokens {
                if t > 0 && cache_creation_price > 0 {
                    (t * cache_creation_price) / div
                } else { 0 }
            } else { 0 };
            let thinking_cost = if thinking_input > 0 && input_price > 0 {
                (thinking_input * input_price) / div
            } else { 0 };
            let output_cost = if output_tokens.unwrap_or(0) > 0 && output_price > 0 {
                (output_tokens.unwrap_or(0) * output_price) / div
            } else { 0 };
            cache_cost + cache_creation_cost + thinking_cost + output_cost
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
            3_000_000,  // $3/1M input in subunits
            15_000_000, // $15/1M output in subunits
            0,
        );
        // input: (1M * 3M) / 1M = 3M
        // output: (500k * 15M) / 1M = 7_500_000
        // total = 10_500_000
        assert_eq!(calc.cost, 10_500_000);
    }

    #[test]
    fn test_token_billing_zero_tokens() {
        let calc = calculate_cost(&BillingType::Token, Some(0), Some(0), 3_000_000, 15_000_000, 0);
        assert_eq!(calc.cost, 0);
    }

    #[test]
    fn test_token_billing_none_tokens() {
        let calc = calculate_cost(&BillingType::Token, None, None, 3_000_000, 15_000_000, 0);
        assert_eq!(calc.cost, 0);
    }

    #[test]
    fn test_request_billing() {
        let calc = calculate_cost(&BillingType::Request, None, None, 0, 0, 50_000);
        // 50_000 subunits = $0.05
        assert_eq!(calc.cost, 50_000);
    }

    #[test]
    fn test_request_billing_ignores_tokens() {
        let calc = calculate_cost(&BillingType::Request, Some(999), Some(999), 999_000_000, 999_000_000, 50_000);
        assert_eq!(calc.cost, 50_000);
    }
}
