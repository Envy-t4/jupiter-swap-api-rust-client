use anyhow::{anyhow, Result};
use quote::{QuoteRequest, QuoteResponse};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use swap::{SwapInstructionsResponse, SwapInstructionsResponseInternal, SwapRequest, SwapResponse};
use std::time::Duration;
use v2::build::{BuildRequest, BuildResponse};
use v2::order::{OrderRequest, OrderResponse};

pub mod quote;
pub mod route_plan_with_metadata;
mod serde_helpers;
pub mod swap;
pub mod transaction_config;
pub mod v2;

#[derive(Clone)]
pub struct JupiterSwapApiClient {
    pub base_path: String,
    pub api_key: Option<String>,
    // Reusable HTTP client for connection pooling and performance optimization
    client: Client,
}

async fn check_is_success(response: Response) -> Result<Response> {
    if !response.status().is_success() {
        return Err(anyhow!(
            "Request status not ok: {}, body: {:?}",
            response.status(),
            response.text().await
        ));
    }
    Ok(response)
}

async fn check_status_code_and_deserialize<T: DeserializeOwned>(response: Response) -> Result<T> {
    check_is_success(response)
        .await?
        .json::<T>()
        .await
        .map_err(Into::into)
}

impl JupiterSwapApiClient {
    pub fn new(base_path: String) -> Self {
        // Create optimized HTTP client once for connection reuse
        let client = Self::build_optimized_client(None);
        Self { 
            base_path,
            api_key: None,
            client,
        }
    }

    pub fn new_with_api_key(base_path: String, api_key: String) -> Self {
        // Create optimized HTTP client once with API key headers
        let client = Self::build_optimized_client(Some(&api_key));
        Self { 
            base_path,
            api_key: Some(api_key),
            client,
        }
    }

    // Build optimized HTTP client with performance settings for connection reuse
    fn build_optimized_client(api_key: Option<&str>) -> Client {
        // Read timeout values from environment variables with fallback to defaults
        let request_timeout = std::env::var("JUPITER_REQUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3);
            
        let connection_idle_timeout = std::env::var("JUPITER_CONNECTION_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1800);

        let mut client_builder = Client::builder()
            // Timeout settings to prevent hanging requests
            .timeout(Duration::from_secs(request_timeout))           // Overall request timeout
            .connect_timeout(Duration::from_secs(5))   // Connection establishment timeout
            
            // Connection pooling for reusing TCP/TLS connections
            .pool_max_idle_per_host(10)                 // Max 10 idle connections per host
            .pool_idle_timeout(Duration::from_secs(connection_idle_timeout)) // Keep connections alive for 90 seconds
            
            // HTTP/2 optimizations for request multiplexing
            .http2_adaptive_window(true)
            
            // Enable gzip compression to reduce bandwidth usage
            .gzip(true);
        
        // Add API key header if provided
        if let Some(api_key) = api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "x-api-key", 
                reqwest::header::HeaderValue::from_str(api_key).unwrap()
            );
            client_builder = client_builder.default_headers(headers);
        }
        
        client_builder.build().unwrap()
    }

    pub async fn quote(&self, quote_request: &QuoteRequest) -> Result<QuoteResponse> {
        let query = serde_qs::to_string(&quote_request)?;
        // Use reusable client instead of creating new one each time
        let response = self.client
            .get(format!("{}/quote?{query}", self.base_path))
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap(&self, swap_request: &SwapRequest) -> Result<SwapResponse> {
        // Use reusable client instead of creating new one each time
        let response = self.client
            .post(format!("{}/swap", self.base_path))
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap_instructions(
        &self,
        swap_request: &SwapRequest,
    ) -> Result<SwapInstructionsResponse> {
        // Use reusable client instead of creating new one each time
        let response = self.client
            .post(format!("{}/swap-instructions", self.base_path))
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize::<SwapInstructionsResponseInternal>(response)
            .await
            .map(Into::into)
    }

    /// `GET /order` (Jupiter Swap API v2).
    ///
    /// `base_path` must already include the v2 prefix (e.g.
    /// `https://lite-api.jup.ag/swap/v2`).
    pub async fn order(&self, request: &OrderRequest) -> Result<OrderResponse> {
        let query = serde_qs::to_string(request)?;
        let response = self
            .client
            .get(format!("{}/order?{query}", self.base_path))
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    /// `GET /build` (Jupiter Swap API v2, Metis-only). Returns raw swap
    /// instructions; use [`v2::build::assemble_transaction`] to compose a
    /// `VersionedTransaction`.
    pub async fn build(&self, request: &BuildRequest) -> Result<BuildResponse> {
        let query = serde_qs::to_string(request)?;
        let response = self
            .client
            .get(format!("{}/build?{query}", self.base_path))
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }
}