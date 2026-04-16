use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::{Model, Provider};

/// Seed data format from JSON
#[derive(Debug, Deserialize)]
struct SeedData {
    providers: Vec<SeedProvider>,
    models: Vec<SeedModel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedProvider {
    name: String,
    base_url: Option<String>,
    #[serde(default)]
    endpoints: Option<HashMap<String, String>>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedModel {
    provider: String,
    name: String,
    #[serde(default)]
    billing_type: Option<String>,
    #[serde(default)]
    input_price: Option<f64>,
    #[serde(default)]
    output_price: Option<f64>,
}

const SEED_JSON: &str = include_str!("../seed_providers.json");

/// Load seed providers from JSON
pub fn get_seed_providers() -> Vec<Provider> {
    let data: SeedData = match serde_json::from_str(SEED_JSON) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to parse seed_providers.json: {}", e);
            return Vec::new();
        }
    };

    data.providers
        .into_iter()
        .map(|p| {
            let endpoints = p.endpoints.as_ref().map(|e| serde_json::to_string(e).ok()).flatten();
            Provider {
                id: Uuid::new_v4().to_string(),
                name: p.name.clone(),
                base_url: p.base_url,
                endpoints,
                enabled: p.enabled.unwrap_or(true),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        })
        .collect()
}

/// Load seed models given provider IDs mapped by provider name
pub fn get_seed_models(provider_ids: &[(String, String)]) -> Vec<Model> {
    let data: SeedData = match serde_json::from_str(SEED_JSON) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let provider_map: HashMap<&str, &str> = provider_ids
        .iter()
        .map(|(name, id)| (name.as_str(), id.as_str()))
        .collect();

    data.models
        .into_iter()
        .filter_map(|m| {
            provider_map.get(m.provider.as_str()).map(|provider_id| Model {
                id: Uuid::new_v4().to_string(),
                name: m.name,
                provider_id: provider_id.to_string(),
                model_type: None,
                pricing_policy_id: None,
                billing_type: m.billing_type.unwrap_or_else(|| "per_token".to_string()),
                input_price: m.input_price.unwrap_or(0.0),
                output_price: m.output_price.unwrap_or(0.0),
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
    fn test_load_seed_providers() {
        let providers = get_seed_providers();
        assert!(!providers.is_empty());
        println!("Loaded {} providers", providers.len());
    }

    #[test]
    fn test_load_seed_models() {
        let providers = get_seed_providers();
        let provider_ids: Vec<_> = providers.iter().map(|p| (p.name.clone(), p.id.clone())).collect();
        let models = get_seed_models(&provider_ids);
        assert!(!models.is_empty());
        println!("Loaded {} models", models.len());
    }
}