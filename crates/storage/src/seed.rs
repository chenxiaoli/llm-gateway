use chrono::Utc;
use uuid::Uuid;

use crate::types::{Model, Provider};

/// Seed data for popular LLM providers
/// (name, base_url, openai_compatible, anthropic_compatible)
pub const SEED_PROVIDERS: &[(&str, &str, bool, bool)] = &[
    ("OpenAI", "https://api.openai.com/v1", true, false),
    ("Anthropic", "https://api.anthropic.com", false, true),
    ("MiniMax", "https://api.minimax.chat/v1", true, false),
    ("GLM", "https://open.bigmodel.cn/api/paas/v4", true, false),
];

/// Seed data for flagship models
/// (provider_name, model_name, billing_type, input_price, output_price)
pub const SEED_MODELS: &[(&str, &str, &str, f64, f64)] = &[
    // OpenAI models
    ("OpenAI", "gpt-4o", "per_token", 2.50, 10.00),
    ("OpenAI", "gpt-4o-mini", "per_token", 0.075, 0.30),
    ("OpenAI", "gpt-4-turbo", "per_token", 5.00, 15.00),
    ("OpenAI", "gpt-3.5-turbo", "per_token", 0.50, 1.50),
    // Anthropic models
    ("Anthropic", "claude-4-opus-20250514", "per_token", 15.00, 75.00),
    ("Anthropic", "claude-sonnet-4-20250514", "per_token", 3.00, 15.00),
    ("Anthropic", "claude-3-5-sonnet", "per_token", 1.50, 7.50),
    ("Anthropic", "claude-3-haiku", "per_token", 0.20, 1.00),
    // MiniMax models
    ("MiniMax", "MiniMax-M2.7", "per_token", 2.1, 8.4),
    ("MiniMax", "MiniMax-M2.7-highspeed", "per_token", 4.2, 16.8),
    // GLM models
    ("GLM", "glm-4", "per_token", 1.00, 10.00),
    ("GLM", "glm-4-flash", "per_token", 0.10, 0.10),
    ("GLM", "glm-4-plus", "per_token", 5.00, 50.00),
];

/// Generate seed providers with generated UUIDs and timestamps
pub fn get_seed_providers() -> Vec<Provider> {
    SEED_PROVIDERS
        .iter()
        .map(|(name, base_url, _, _)| Provider {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            base_url: Some(base_url.to_string()),
            // Store compatibility info in endpoints JSON
            endpoints: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .collect()
}

/// Generate seed models given provider IDs mapped by provider name
/// Returns a vector of models with generated UUIDs
pub fn get_seed_models(provider_ids: &[(String, String)]) -> Vec<Model> {
    let provider_map: std::collections::HashMap<&str, &str> = provider_ids
        .iter()
        .map(|(name, id)| (name.as_str(), id.as_str()))
        .collect();

    SEED_MODELS
        .iter()
        .filter_map(|(provider_name, model_name, billing_type, input_price, output_price)| {
            provider_map.get(provider_name).map(|provider_id| Model {
                id: Uuid::new_v4().to_string(),
                name: model_name.to_string(),
                provider_id: provider_id.to_string(),
                model_type: None,
                pricing_policy_id: None,
                billing_type: billing_type.to_string(),
                input_price: *input_price,
                output_price: *output_price,
                request_price: 0.0,
                enabled: true,
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Build provider-to-ID mapping from a list of providers
pub fn build_provider_id_map(providers: &[Provider]) -> Vec<(String, String)> {
    providers
        .iter()
        .map(|p| (p.name.clone(), p.id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_seed_providers() {
        let providers = get_seed_providers();
        assert_eq!(providers.len(), 4);

        let names: Vec<&str> = providers.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"OpenAI"));
        assert!(names.contains(&"Anthropic"));
        assert!(names.contains(&"MiniMax"));
        assert!(names.contains(&"GLM"));

        for provider in &providers {
            assert!(!provider.id.is_empty());
            assert!(provider.base_url.is_some());
            assert!(provider.enabled);
        }
    }

    #[test]
    fn test_get_seed_models() {
        let providers = get_seed_providers();
        let provider_ids = build_provider_id_map(&providers);
        let models = get_seed_models(&provider_ids);

        assert_eq!(models.len(), 13);

        // Check that models belong to correct providers
        let openai_id = providers.iter().find(|p| p.name == "OpenAI").unwrap().id.clone();
        let openai_models: Vec<_> = models.iter().filter(|m| m.provider_id == openai_id).collect();
        assert_eq!(openai_models.len(), 4);

        let anthropic_id = providers.iter().find(|p| p.name == "Anthropic").unwrap().id.clone();
        let anthropic_models: Vec<_> = models.iter().filter(|m| m.provider_id == anthropic_id).collect();
        assert_eq!(anthropic_models.len(), 4);
    }

    #[test]
    fn test_build_provider_id_map() {
        let providers = get_seed_providers();
        let map = build_provider_id_map(&providers);

        assert_eq!(map.len(), 4);
        for (name, id) in &map {
            assert!(!id.is_empty());
            assert!(!name.is_empty());
        }
    }
}