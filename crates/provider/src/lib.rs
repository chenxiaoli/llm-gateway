pub mod openai;
pub mod anthropic;

use async_trait::async_trait;
use reqwest::Client;

/// Result of a proxied request, containing response info for billing/audit.
pub struct ProxyResult {
    pub status_code: u16,
    pub response_body: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Optional per-provider request/response transformation.
pub trait ProviderAdapter: Send + Sync {
    fn transform_request(&self, base_url: &str, path: &str) -> (String, Vec<(String, String)>) {
        (format!("{}{}", base_url, path), vec![])
    }
}

/// A default adapter that does no transformation.
pub struct DefaultAdapter;

impl ProviderAdapter for DefaultAdapter {}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn base_url(&self) -> &str;
    fn adapter(&self) -> &dyn ProviderAdapter;
    fn api_key(&self) -> &str;

    /// Proxy a non-streaming request. Returns the full response body.
    async fn proxy_request(
        &self,
        client: &Client,
        path: &str,
        request_body: String,
        extra_headers: Vec<(String, String)>,
    ) -> Result<ProxyResult, Box<dyn std::error::Error + Send + Sync>>;
}
