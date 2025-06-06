use anyhow::{Context, Result};
use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};

/// Simple newtypes for domain concepts
#[derive(Debug, Clone, PartialEq, Eq, Display, From, Into, Serialize, Deserialize)]
pub struct ServiceName(String);

impl ServiceName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Display, From, Into, Serialize, Deserialize)]
pub struct MethodName(String);

impl MethodName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Clean endpoint representation
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Endpoint {
    pub fn parse(address: &str) -> Result<Self> {
        if address.starts_with("https://") || address.starts_with("grpcs://") {
            let addr = address.split("://").nth(1).unwrap_or(address);
            Self::parse_host_port(addr, true)
        } else if address.starts_with("http://") || address.starts_with("grpc://") {
            let addr = address.split("://").nth(1).unwrap_or(address);
            Self::parse_host_port(addr, false)
        } else {
            Self::parse_host_port(address, true)
        }
    }

    fn parse_host_port(address: &str, tls: bool) -> Result<Self> {
        let parts: Vec<&str> = address.rsplitn(2, ':').collect();
        match parts.len() {
            2 => Ok(Self {
                host: parts[1].to_string(),
                port: parts[0]
                    .parse()
                    .with_context(|| format!("Invalid port number: {}", parts[0]))?,
                tls,
            }),
            1 => Ok(Self {
                host: parts[0].to_string(),
                port: if tls { 443 } else { 80 },
                tls,
            }),
            _ => anyhow::bail!("Invalid endpoint format: {}", address),
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Configuration types
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json { pretty: bool, emit_defaults: bool },
    Text { compact: bool },
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Json {
            pretty: true,
            emit_defaults: false,
        }
    }
}

/// Security configuration for connections
#[derive(Debug, Clone)]
pub enum SecurityConfig {
    Plaintext,
    Tls {
        ca_cert: Option<std::path::PathBuf>,
        client_cert: Option<std::path::PathBuf>,
        client_key: Option<std::path::PathBuf>,
        server_name: Option<String>,
    },
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::Tls {
            ca_cert: None,
            client_cert: None,
            client_key: None,
            server_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_parsing() {
        let endpoint = Endpoint::parse("localhost:9090").unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 9090);
        assert!(endpoint.tls);

        let endpoint = Endpoint::parse("http://localhost:8080").unwrap();
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 8080);
        assert!(!endpoint.tls);

        let endpoint = Endpoint::parse("https://api.example.com:443").unwrap();
        assert_eq!(endpoint.host, "api.example.com");
        assert_eq!(endpoint.port, 443);
        assert!(endpoint.tls);
    }

    #[test]
    fn test_endpoint_default_ports() {
        let endpoint = Endpoint::parse("localhost").unwrap();
        assert_eq!(endpoint.port, 443);

        let endpoint = Endpoint::parse("http://localhost").unwrap();
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn test_service_and_method_names() {
        let service = ServiceName::new("test.Service".to_string());
        assert_eq!(service.as_str(), "test.Service");

        let method = MethodName::new("GetUser".to_string());
        assert_eq!(method.as_str(), "GetUser");
    }
}
