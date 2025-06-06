use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;

/// Configuration for the web server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Map of server ID to server configuration
    pub servers: HashMap<String, GrpcServerConfig>,
}

/// Configuration for a single gRPC server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerConfig {
    /// Human-readable name for the server
    pub name: String,
    /// Server endpoint (host:port)
    pub endpoint: String,
    /// Whether to use plaintext connection (no TLS)
    #[serde(default)]
    pub plaintext: bool,
    /// Path to CA certificate file for TLS verification
    pub ca_cert: Option<String>,
    /// Default headers to include with requests
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional description of the server
    pub description: Option<String>,
}

impl ServerConfig {
    /// Load configuration from a YAML file
    pub async fn load(config_path: &str) -> Result<Self> {
        // Try to load existing config file
        match fs::read_to_string(config_path).await {
            Ok(content) => serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config file: {}", config_path)),
            Err(_) => {
                // If file doesn't exist, create a default config
                println!(
                    "📝 Config file '{}' not found, creating default configuration",
                    config_path
                );
                let default_config = Self::default();
                default_config.save(config_path).await?;
                Ok(default_config)
            }
        }
    }

    /// Save configuration to a YAML file
    pub async fn save(&self, config_path: &str) -> Result<()> {
        let yaml_content =
            serde_yaml::to_string(self).context("Failed to serialize config to YAML")?;

        fs::write(config_path, yaml_content)
            .await
            .with_context(|| format!("Failed to write config file: {}", config_path))?;

        println!("✅ Configuration saved to: {}", config_path);
        Ok(())
    }

    /// Get server configuration by ID
    pub fn get_server(&self, server_id: &str) -> Option<&GrpcServerConfig> {
        self.servers.get(server_id)
    }

    /// List all server IDs
    pub fn server_ids(&self) -> Vec<&String> {
        self.servers.keys().collect()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        let mut servers = HashMap::new();

        // Add some example server configurations
        servers.insert(
            "local".to_string(),
            GrpcServerConfig {
                name: "Local gRPC Server".to_string(),
                endpoint: "localhost:9090".to_string(),
                plaintext: true,
                ca_cert: None,
                headers: HashMap::new(),
                description: Some("Local development gRPC server".to_string()),
            },
        );

        servers.insert(
            "reflection-demo".to_string(),
            GrpcServerConfig {
                name: "gRPC Reflection Demo".to_string(),
                endpoint: "grpcb.in:9000".to_string(),
                plaintext: false,
                ca_cert: None,
                headers: HashMap::new(),
                description: Some("Public gRPC server with reflection enabled".to_string()),
            },
        );

        Self { servers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_default() {
        let config = ServerConfig::default();
        assert!(config.servers.contains_key("local"));
        assert!(config.servers.contains_key("reflection-demo"));
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let config = ServerConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_str().unwrap();

        // Save config
        config.save(config_path).await.unwrap();

        // Load config
        let loaded_config = ServerConfig::load(config_path).await.unwrap();

        assert_eq!(config.servers.len(), loaded_config.servers.len());
        assert!(loaded_config.servers.contains_key("local"));
    }
}
