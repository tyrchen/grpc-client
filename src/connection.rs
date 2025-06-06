use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint as TonicEndpoint};

use crate::domain::{Endpoint, SecurityConfig};

pub struct ConnectionBuilder {
    endpoint: Option<Endpoint>,
    security: Option<SecurityConfig>,
    timeout: Option<Duration>,
    headers: Vec<(String, String)>,
}

impl ConnectionBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            security: None,
            timeout: None,
            headers: Vec::new(),
        }
    }

    pub fn endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn security(mut self, config: SecurityConfig) -> Self {
        self.security = Some(config);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn header(mut self, name: String, value: String) -> Self {
        self.headers.push((name, value));
        self
    }

    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub async fn build(self) -> Result<Channel> {
        let endpoint = self
            .endpoint
            .ok_or_else(|| anyhow::anyhow!("Endpoint is required"))?;

        let security = self.security.unwrap_or_default();
        let timeout = self.timeout.unwrap_or(Duration::from_secs(10));

        let uri = format!("http://{}:{}", endpoint.host, endpoint.port);

        let mut tonic_endpoint = TonicEndpoint::from_shared(uri).context("Invalid endpoint URI")?;

        tonic_endpoint = tonic_endpoint.timeout(timeout).connect_timeout(timeout);

        match security {
            SecurityConfig::Tls { .. } => {
                let uri = format!("https://{}:{}", endpoint.host, endpoint.port);
                tonic_endpoint =
                    TonicEndpoint::from_shared(uri).context("Invalid TLS endpoint URI")?;
                tonic_endpoint = tonic_endpoint.timeout(timeout).connect_timeout(timeout);

                let tls_config = match &security {
                    SecurityConfig::Tls { ca_cert, .. } => {
                        let mut config = ClientTlsConfig::new();

                        if let Some(ca_path) = ca_cert {
                            match std::fs::read(ca_path) {
                                Ok(ca_pem) => {
                                    let cert = tonic::transport::Certificate::from_pem(ca_pem);
                                    config = config.ca_certificate(cert);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to read CA certificate file {}: {}",
                                        ca_path.display(),
                                        e
                                    );
                                }
                            }
                        }

                        config
                    }
                    _ => ClientTlsConfig::new(),
                };

                tonic_endpoint = tonic_endpoint
                    .tls_config(tls_config)
                    .context("Failed to configure TLS")?;
            }
            SecurityConfig::Plaintext => {}
        }

        let channel = tonic_endpoint
            .connect()
            .await
            .with_context(|| format!("Failed to connect to {}", endpoint))?;

        Ok(channel)
    }
}

impl Default for ConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn connect() -> ConnectionBuilder {
    ConnectionBuilder::new()
}

pub async fn create_channel(
    endpoint: &Endpoint,
    plaintext: bool,
    ca_cert_path: &Option<String>,
) -> Result<Channel> {
    let security = if plaintext {
        SecurityConfig::Plaintext
    } else if endpoint.tls {
        if let Some(ca_path) = ca_cert_path {
            SecurityConfig::Tls {
                ca_cert: Some(std::path::PathBuf::from(ca_path)),
                client_cert: None,
                client_key: None,
                server_name: None,
            }
        } else {
            SecurityConfig::default()
        }
    } else {
        SecurityConfig::Plaintext
    };

    connect()
        .endpoint(endpoint.clone())
        .security(security)
        .build()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Endpoint;

    #[test]
    fn test_connection_builder() {
        let builder = ConnectionBuilder::new();

        assert!(builder.endpoint.is_none());
        assert!(builder.security.is_none());
        assert!(builder.timeout.is_none());
        assert_eq!(builder.headers.len(), 0);
    }

    #[test]
    fn test_connection_builder_with_options() {
        let endpoint = Endpoint::parse("https://api.example.com:8443").unwrap();
        let builder = ConnectionBuilder::new()
            .endpoint(endpoint.clone())
            .security(SecurityConfig::default())
            .timeout(Duration::from_secs(30))
            .header("Authorization".to_string(), "Bearer token".to_string());

        assert!(builder.endpoint.is_some());
        assert!(builder.security.is_some());
        assert!(builder.timeout.is_some());
        assert_eq!(builder.headers.len(), 1);
    }

    #[test]
    fn test_connect_helper() {
        let builder = connect();
        assert!(builder.endpoint.is_none());
    }

    #[tokio::test]
    async fn test_create_channel_config() {
        let endpoint = Endpoint::parse("localhost:9090").unwrap();

        let result = create_channel(&endpoint, true, &None).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to connect") || error_msg.contains("Connection"));
    }
}
