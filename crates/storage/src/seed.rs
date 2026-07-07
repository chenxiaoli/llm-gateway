use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::money::usd_to_units;
use crate::types::{Model, PricingPolicy, Provider, ProviderModel};

/// Seed data format from JSON (public for API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedData {
    pub providers: Vec<SeedProvider>,
    pub models: Vec<SeedModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedProvider {
    pub name: String,
    #[serde(default)]
    pub endpoints: Option<HashMap<String, String>>,
    #[serde(default)]
    pub proxy_url: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedModel {
    pub name: String,
    #[serde(default)]
    pub billing_type: Option<String>,
    #[serde(default)]
    pub input_price: Option<f64>,
    #[serde(default)]
    pub output_price: Option<f64>,
    #[serde(default)]
    pub cache_read_price: Option<f64>,
    #[serde(default)]
    pub cache_creation_price: Option<f64>,
    #[serde(default)]
    pub tiers: Vec<SeedTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedTier {
    pub up_to: Option<i64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    #[serde(default)]
    pub cache_read_price: Option<f64>,
    #[serde(default)]
    pub cache_creation_price: Option<f64>,
}

const SEED_JSON: &str = include_str!("../seed_providers.json");

/// Load seed JSON data from file
pub fn load_seed_data() -> Result<SeedData, String> {
    serde_json::from_str(SEED_JSON).map_err(|e| e.to_string())
}

/// Get available seed providers (for selection UI)
pub fn get_available_providers() -> Vec<SeedProvider> {
    load_seed_data().map(|d| d.providers).unwrap_or_default()
}

/// Get available seed models
pub fn get_available_models() -> Vec<SeedModel> {
    load_seed_data().map(|d| d.models).unwrap_or_default()
}

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
            let slug = p.name.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            Provider {
                id: Uuid::new_v4().to_string(),
                name: p.name.clone(),
                slug,
                endpoints,
                proxy_url: None,
                enabled: p.enabled.unwrap_or(true),
                owner_org_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        })
        .collect()
}

/// Build pricing policies from seed models.
/// Creates policies for both per_token (with optional cache prices) and context_tiered models.
/// Returns (policy, model_name) pairs so the caller can link them.
pub fn get_seed_pricing_policies() -> Vec<(PricingPolicy, String)> {
    let data: SeedData = match serde_json::from_str(SEED_JSON) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    data.models
        .iter()
        .filter_map(|m| {
            let billing_type = m.billing_type.as_deref().unwrap_or("per_token");

            match billing_type {
                "context_tiered" if !m.tiers.is_empty() => {
                    let tiers: Vec<serde_json::Value> = m.tiers.iter().map(|t| {
                        let mut obj = serde_json::Map::new();
                        if let Some(up_to) = t.up_to {
                            obj.insert("up_to".into(), serde_json::Value::from(up_to));
                        }
                        if let Some(p) = t.input_price {
                            obj.insert("input_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                        }
                        if let Some(p) = t.output_price {
                            obj.insert("output_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                        }
                        if let Some(p) = t.cache_read_price {
                            obj.insert("cache_read_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                        }
                        if let Some(p) = t.cache_creation_price {
                            obj.insert("cache_creation_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                        }
                        serde_json::Value::Object(obj)
                    }).collect();

                    let config = serde_json::json!({ "tiers": tiers });
                    Some((build_policy(&m.name, billing_type, config), m.name.clone()))
                }
                "per_token" if m.input_price.is_some() || m.output_price.is_some() => {
                    let mut obj = serde_json::Map::new();
                    if let Some(p) = m.input_price {
                        obj.insert("input_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                    }
                    if let Some(p) = m.output_price {
                        obj.insert("output_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                    }
                    if let Some(p) = m.cache_read_price {
                        obj.insert("cache_read_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                    }
                    if let Some(p) = m.cache_creation_price {
                        obj.insert("cache_creation_price_1m".into(), serde_json::Value::from(usd_to_units(p)));
                    }
                    let config = serde_json::Value::Object(obj);
                    Some((build_policy(&m.name, billing_type, config), m.name.clone()))
                }
                _ => None,
            }
        })
        .collect()
}

fn build_policy(model_name: &str, billing_type: &str, config: serde_json::Value) -> PricingPolicy {
    PricingPolicy {
        id: Uuid::new_v4().to_string(),
        name: format!("{} Pricing", model_name),
        billing_type: billing_type.to_string(),
        config,
        owner_org_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Load seed models given provider IDs mapped by provider name
pub fn get_seed_models(_provider_ids: &[(String, String)]) -> Vec<Model> {
    let data: SeedData = match serde_json::from_str(SEED_JSON) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    // For N:N architecture, models no longer belong to a single provider
    // We still load models but without provider_id - they will be linked via model_providers
    data.models
        .into_iter()
        .map(|m| Model {
            id: Uuid::new_v4().to_string(),
            name: m.name,
            model_type: None,
            pricing_policy_id: None,
            owner_org_id: None,
            created_at: Utc::now(),
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

/// Provider-to-model name mapping for seed data
const PROVIDER_MODEL_MAP: &[(&str, &[&str])] = &[
    ("OpenAI", &["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"]),
    ("Anthropic", &["claude-4-opus-20250514", "claude-sonnet-4-20250514", "claude-3-5-sonnet", "claude-3-haiku"]),
    ("MiniMax", &["minimax-m2.7", "minimax-m2.7-highspeed", "minimax-m2.5"]),
    ("GLM", &["glm-4", "glm-4-flash", "glm-4-plus", "glm-5.1"]),
    ("Alibaba", &["qwen3.6-plus", "kimi-k2.5"]),
];

/// Build provider_models entries for seeding.
/// `provider_id_map` = [(provider_name, provider_id)], `model_id_map` = [(model_name, model_id)]
pub fn get_seed_provider_models(
    provider_id_map: &[(String, String)],
    model_id_map: &[(String, String)],
) -> Vec<ProviderModel> {
    let provider_map: HashMap<&str, &str> = provider_id_map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let model_map: HashMap<&str, &str> = model_id_map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    let mut result = Vec::new();
    for (provider_name, model_names) in PROVIDER_MODEL_MAP {
        if let Some(provider_id) = provider_map.get(provider_name) {
            for model_name in *model_names {
                if let Some(model_id) = model_map.get(model_name) {
                    result.push(ProviderModel {
                        provider_id: provider_id.to_string(),
                        model_id: model_id.to_string(),
                        upstream_name: Some(model_name.to_string()),
                        pricing_policy_id: None,
                        owner_org_id: None,
                        created_at: Utc::now(),
                    });
                }
            }
        }
    }
    result
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

    #[test]
    fn test_seed_pricing_policies_not_empty() {
        let policies = get_seed_pricing_policies();
        println!("Generated {} pricing policies", policies.len());
        for (p, model_name) in &policies {
            println!("  {} -> {} ({})", model_name, p.name, p.billing_type);
        }
        assert!(!policies.is_empty(), "get_seed_pricing_policies should return policies for seed models");
    }

    #[test]
    fn test_seed_model_deserialization() {
        let data = load_seed_data().unwrap();
        // Verify models with pricing data deserialize correctly
        let minimax_m27 = data.models.iter().find(|m| m.name == "minimax-m2.7").expect("minimax-m2.7 should exist");
        assert_eq!(minimax_m27.billing_type.as_deref(), Some("per_token"));
        assert!(minimax_m27.input_price.is_some());
        assert!(minimax_m27.output_price.is_some());
        assert!(minimax_m27.cache_read_price.is_some());
        assert!(minimax_m27.cache_creation_price.is_some());

        // Verify models without pricing data deserialize with None defaults
        let gpt4o = data.models.iter().find(|m| m.name == "gpt-4o").expect("gpt-4o should exist");
        assert!(gpt4o.billing_type.is_none());
        assert!(gpt4o.input_price.is_none());
    }
}