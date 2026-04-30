use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::money::usd_to_units;
use crate::types::{Model, PricingPolicy, Provider};

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