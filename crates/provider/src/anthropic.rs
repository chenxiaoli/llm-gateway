use super::{DefaultAdapter, Provider, ProviderAdapter, ProxyResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct AnthropicProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

impl ProviderAdapter for AnthropicProvider {}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str { &self.name }
    fn base_url(&self) -> &str { &self.base_url }
    fn adapter(&self) -> &dyn ProviderAdapter { self }
    fn api_key(&self) -> &str { &self.api_key }

    async fn proxy_request(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>> {
        let (url, mut adapter_headers) = self.adapter().transform_request(self.base_url(), path);
        adapter_headers.push(("x-api-key".to_string(), self.api_key.clone()));
        adapter_headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
        adapter_headers.push(("Content-Type".to_string(), "application/json".to_string()));
        for (k, v) in extra_headers {
            adapter_headers.push((k, v));
        }

        let mut req = client.post(&url);
        for (k, v) in &adapter_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req = req.body(request_body);

        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;

        let (input_tokens, output_tokens) = extract_usage(&body);

        Ok(ProxyResult {
            status_code: status,
            response_body: body,
            input_tokens,
            output_tokens,
        })
    }
}

fn extract_usage(body: &str) -> (Option<i64>, Option<i64>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let usage = v.get("usage").and_then(|u| {
        Some((
            u.get("input_tokens")?.as_i64()?,
            u.get("output_tokens")?.as_i64()?,
        ))
    });
    usage.unwrap_or((None, None))
}
