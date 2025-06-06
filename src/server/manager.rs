use crate::ServiceName;
use crate::client::GrpcClient;
use crate::domain::OutputFormat;
use crate::reflection::{MethodDescriptor, ServiceDescriptor, create_reflection_client};
use crate::server::config::GrpcServerConfig;
use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Manager for gRPC client connections and operations
#[derive(Debug, Clone, Default)]
pub struct ServerManager {
    /// Shared clients for each server
    clients: Arc<DashMap<String, Arc<GrpcClient>>>,
    /// Cached service information
    service_cache: Arc<DashMap<String, Vec<ServiceDescriptor>>>,
    /// Cached method information per service
    method_cache: Arc<DashMap<String, Vec<MethodDescriptor>>>,
}

/// Wrapper for method call results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    pub success: bool,
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: CallMetadata,
}

/// Metadata about a method call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallMetadata {
    pub method: String,
    pub server_id: String,
    pub duration_ms: u64,
    pub response_size: usize,
    pub timestamp: String,
}

impl ServerManager {
    /// Create a new server manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(DashMap::new()),
            service_cache: Arc::new(DashMap::new()),
            method_cache: Arc::new(DashMap::new()),
        }
    }

    /// Get or create a gRPC client for the specified server
    pub async fn get_client(
        &self,
        server_id: &str,
        config: &GrpcServerConfig,
    ) -> Result<Arc<GrpcClient>> {
        // Check if client already exists
        if let Some(client) = self.clients.get(server_id) {
            return Ok(client.clone());
        }

        // Create new client
        let client = self
            .create_client(config)
            .await
            .with_context(|| format!("Failed to create client for server: {}", server_id))?;

        let client_arc = Arc::new(client);
        self.clients
            .insert(server_id.to_string(), client_arc.clone());

        Ok(client_arc)
    }

    /// Create a new gRPC client from configuration
    async fn create_client(&self, config: &GrpcServerConfig) -> Result<GrpcClient> {
        // Convert headers from HashMap to Vec<(String, String)>
        let headers: Vec<(String, String)> = config
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Parse endpoint string to create domain::Endpoint
        let endpoint = crate::domain::Endpoint::parse(&config.endpoint)
            .with_context(|| format!("Invalid endpoint: {}", config.endpoint))?;

        let client = GrpcClient {
            endpoint,
            headers,
            format: OutputFormat::Json {
                pretty: true,
                emit_defaults: false,
            },
            verbose: false,
            ca_cert_path: config.ca_cert.clone(),
            plaintext: config.plaintext,
            cache: crate::client::PerformanceCache::new(),
        };

        // Test connection
        client
            .get_or_create_channel()
            .await
            .with_context(|| format!("Failed to connect to server: {}", config.endpoint))?;

        Ok(client)
    }

    /// List services for a server with caching
    pub async fn list_services(
        &self,
        server_id: &str,
        client: Arc<GrpcClient>,
    ) -> Result<Vec<ServiceDescriptor>> {
        let cache_key = format!("services:{}", server_id);

        // Check cache first
        if let Some(cached) = self.service_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Get services from reflection
        let channel = client.get_or_create_channel().await?;
        let mut reflection_client = create_reflection_client(channel);
        let services = reflection_client.list_services().await?;

        // Convert to service descriptors
        let mut service_descriptors = Vec::new();
        for service_name in services {
            if let Ok(descriptor) = reflection_client.get_service(&service_name).await {
                service_descriptors.push(descriptor);
            }
        }

        // Cache the results
        self.service_cache
            .insert(cache_key, service_descriptors.clone());

        Ok(service_descriptors)
    }

    /// List methods for a service with caching
    pub async fn list_methods(
        &self,
        server_id: &str,
        service_name: &str,
        client: Arc<GrpcClient>,
    ) -> Result<Vec<MethodDescriptor>> {
        let cache_key = format!("methods:{}:{}", server_id, service_name);

        // Check cache first
        if let Some(cached) = self.method_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Get methods from reflection
        let channel = client.get_or_create_channel().await?;
        let mut reflection_client = create_reflection_client(channel);
        let service = ServiceName::new(service_name.to_string());
        let methods = reflection_client.list_methods(&service).await?;

        // Cache the results
        self.method_cache.insert(cache_key, methods.clone());

        Ok(methods)
    }

    /// Execute a gRPC method call with enhanced error handling and metadata
    pub async fn call_method(
        &self,
        server_id: &str,
        method: &str,
        data: Option<&str>,
        client: Arc<GrpcClient>,
        emit_defaults: Option<bool>,
    ) -> Result<CallResult> {
        let start_time = std::time::Instant::now();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Create output format
        let format = OutputFormat::Json {
            pretty: true,
            emit_defaults: emit_defaults.unwrap_or(false),
        };

        // Attempt the call
        match client.handle_call(method, data, format).await {
            Ok(_) => {
                let duration = start_time.elapsed();

                // Since handle_call doesn't return the response data directly,
                // we need to simulate success response for now
                // TODO: In a real implementation, we'd capture the output
                Ok(CallResult {
                    success: true,
                    response: Some(serde_json::json!({
                        "message": "Call completed successfully",
                        "note": "Response capture not yet implemented"
                    })),
                    error: None,
                    metadata: CallMetadata {
                        method: method.to_string(),
                        server_id: server_id.to_string(),
                        duration_ms: duration.as_millis() as u64,
                        response_size: 0, // TODO: Capture actual response size
                        timestamp,
                    },
                })
            }
            Err(e) => {
                let duration = start_time.elapsed();

                Ok(CallResult {
                    success: false,
                    response: None,
                    error: Some(e.to_string()),
                    metadata: CallMetadata {
                        method: method.to_string(),
                        server_id: server_id.to_string(),
                        duration_ms: duration.as_millis() as u64,
                        response_size: 0,
                        timestamp,
                    },
                })
            }
        }
    }

    /// Clear caches for a specific server
    pub fn clear_cache(&self, server_id: &str) {
        let service_key = format!("services:{}", server_id);
        self.service_cache.remove(&service_key);

        // Clear all method caches for this server
        let method_prefix = format!("methods:{}:", server_id);
        self.method_cache
            .retain(|key, _| !key.starts_with(&method_prefix));
    }

    /// Clear all caches
    pub fn clear_all_caches(&self) {
        self.service_cache.clear();
        self.method_cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "service_cache_size": self.service_cache.len(),
            "method_cache_size": self.method_cache.len(),
            "active_clients": self.clients.len()
        })
    }

    /// Remove a client connection
    pub fn remove_client(&self, server_id: &str) {
        self.clients.remove(server_id);
        self.clear_cache(server_id);
    }
}
