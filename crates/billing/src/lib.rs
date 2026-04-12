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
    let cost = match billing_type {
        BillingType::Token => {
            let input_cost = input_tokens.unwrap_or(0) as f64 / 1_000_000.0 * input_price;
            let output_cost = output_tokens.unwrap_or(0) as f64 / 1_000_000.0 * output_price;
            input_cost + output_cost
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
