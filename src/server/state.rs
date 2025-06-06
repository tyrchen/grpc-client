use crate::client::GrpcClient;
use crate::server::config::{GrpcServerConfig, ServerConfig};
use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use tracing::info;
use utoipa::ToSchema;

/// Application state shared across all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Configuration loaded from YAML file
    pub config: ServerConfig,
    /// Map of server ID to initialized gRPC clients
    pub clients: Arc<DashMap<String, Arc<GrpcClient>>>,
}

impl AppState {
    /// Create new application state from configuration
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let clients = Arc::new(DashMap::new());

        // Pre-initialize clients for all configured servers
        for (server_id, conf) in &config.servers {
            match GrpcClient::from_config(conf) {
                Ok(client) => {
                    clients.insert(server_id.clone(), Arc::new(client));
                    println!(
                        "✅ Initialized client for server: {} ({})",
                        server_id, conf.name
                    );
                }
                Err(e) => {
                    println!(
                        "⚠️  Failed to initialize client for server '{}': {}",
                        server_id, e
                    );
                    // Continue with other servers even if one fails
                }
            }
        }

        Ok(Self { config, clients })
    }

    /// Get or create a gRPC client for the specified server
    pub async fn get_client(&self, server_id: &str) -> Result<Arc<GrpcClient>> {
        // Try to get existing client
        if let Some(client) = self.clients.get(server_id) {
            return Ok(client.clone());
        }

        // Get server configuration
        let conf = self
            .config
            .get_server(server_id)
            .with_context(|| format!("Server '{}' not found in configuration", server_id))?;

        info!("Creating client for server: {:?}", conf);
        // Create new client
        let client = GrpcClient::from_config(conf)
            .with_context(|| format!("Failed to create client for server '{}'", server_id))?;

        let client_arc = Arc::new(client);

        // Cache the client
        self.clients
            .insert(server_id.to_string(), client_arc.clone());

        Ok(client_arc)
    }

    /// Get all configured server IDs
    pub fn get_server_ids(&self) -> Vec<String> {
        self.config.server_ids().into_iter().cloned().collect()
    }

    /// Get server configuration by ID
    pub fn get_server_config(&self, server_id: &str) -> Option<&GrpcServerConfig> {
        self.config.get_server(server_id)
    }

    /// Check if a server client is initialized and available
    pub fn is_client_available(&self, server_id: &str) -> bool {
        self.clients.contains_key(server_id)
    }

    /// Get connection status for all servers
    pub fn get_connection_status(&self) -> Vec<ServerStatus> {
        self.config
            .servers
            .iter()
            .map(|(id, config)| ServerStatus {
                id: id.clone(),
                name: config.name.clone(),
                endpoint: config.endpoint.clone(),
                connected: self.clients.contains_key(id),
                description: config.description.clone(),
            })
            .collect()
    }
}

/// Server connection status for API responses
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    /// Server identifier
    pub id: String,
    /// Human-readable server name
    pub name: String,
    /// Server endpoint (host:port)
    pub endpoint: String,
    /// Whether the server is currently connected
    pub connected: bool,
    /// Optional server description
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> ServerConfig {
        let mut servers = HashMap::new();
        servers.insert(
            "test".to_string(),
            GrpcServerConfig {
                name: "Test Server".to_string(),
                endpoint: "localhost:9090".to_string(),
                plaintext: true,
                ca_cert: None,
                headers: HashMap::new(),
                description: Some("Test server".to_string()),
            },
        );
        ServerConfig { servers }
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let config = create_test_config();
        let state = AppState::new(config).await.unwrap();

        assert_eq!(state.get_server_ids().len(), 1);
        assert!(state.get_server_ids().contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_server_status() {
        let config = create_test_config();
        let state = AppState::new(config).await.unwrap();

        let statuses = state.get_connection_status();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].id, "test");
        assert_eq!(statuses[0].name, "Test Server");
    }
}
