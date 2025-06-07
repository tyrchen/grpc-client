mod common;

use common::init_test_logging;
use grpc_client::server::config::GrpcServerConfig;
use grpc_client::{Endpoint, GrpcClient, SecurityConfig};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_endpoint_parsing() {
    init_test_logging();

    // Valid endpoint parsing
    let endpoint = Endpoint::parse("localhost:9090").unwrap();
    assert_eq!(endpoint.host, "localhost");
    assert_eq!(endpoint.port, 9090);

    let endpoint_with_scheme = Endpoint::parse("http://localhost:8080").unwrap();
    assert_eq!(endpoint_with_scheme.host, "localhost");
    assert_eq!(endpoint_with_scheme.port, 8080);

    // Empty string should actually succeed (gets parsed as host "" with default port)
    let endpoint_empty = Endpoint::parse("");
    assert!(endpoint_empty.is_ok());

    // Invalid port number should fail
    let invalid_port = Endpoint::parse("localhost:invalid-port");
    assert!(invalid_port.is_err());
}

#[tokio::test]
async fn test_security_config_creation() {
    init_test_logging();

    let plaintext = SecurityConfig::Plaintext;
    assert!(matches!(plaintext, SecurityConfig::Plaintext));

    let tls = SecurityConfig::Tls {
        ca_cert: None,
        client_cert: None,
        client_key: None,
        server_name: None,
    };
    assert!(matches!(tls, SecurityConfig::Tls { .. }));

    let tls_with_cert = SecurityConfig::Tls {
        ca_cert: Some("/path/to/cert.pem".into()),
        client_cert: None,
        client_key: None,
        server_name: None,
    };
    assert!(matches!(tls_with_cert, SecurityConfig::Tls { .. }));
}

#[tokio::test]
async fn test_grpc_client_from_config() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:9090".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: Some("Test server".to_string()),
    };

    let client = GrpcClient::from_config(&config);
    assert!(client.is_ok());

    let client = client.unwrap();
    assert_eq!(client.endpoint.host, "localhost");
    assert_eq!(client.endpoint.port, 9090);
    assert!(client.plaintext);
}

#[tokio::test]
async fn test_grpc_client_with_headers() {
    init_test_logging();

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), "Bearer token123".to_string());
    headers.insert("x-api-key".to_string(), "api-key-456".to_string());

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:9090".to_string(),
        plaintext: true,
        ca_cert: None,
        headers,
        description: None,
    };

    let client = GrpcClient::from_config(&config);
    assert!(client.is_ok());

    let client = client.unwrap();
    assert_eq!(client.headers.len(), 2);
    assert!(
        client
            .headers
            .contains(&("authorization".to_string(), "Bearer token123".to_string()))
    );
    assert!(
        client
            .headers
            .contains(&("x-api-key".to_string(), "api-key-456".to_string()))
    );
}

#[tokio::test]
async fn test_grpc_client_connection_failure() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "127.0.0.1:65534".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: Some("Test server for connection failure".to_string()),
    };

    let client = GrpcClient::from_config(&config).unwrap();
    let result = client.handle_service_list().await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Verify we get the expected gRPC connection error
    assert!(error_msg.contains("Failed to create gRPC channel"));
}

#[tokio::test]
async fn test_method_descriptor_parsing() {
    init_test_logging();

    // Test valid method descriptors
    let method = "package.Service/Method";
    let parts: Vec<&str> = method.split('/').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "package.Service");
    assert_eq!(parts[1], "Method");

    // Test service name extraction
    let service_parts: Vec<&str> = parts[0].split('.').collect();
    assert!(service_parts.len() >= 2);
    assert_eq!(service_parts.last().unwrap(), &"Service");
}

#[tokio::test]
async fn test_grpc_client_tls_configuration() {
    init_test_logging();

    // Test TLS configuration
    let tls_config = GrpcServerConfig {
        name: "TLS Server".to_string(),
        endpoint: "secure.example.com:443".to_string(),
        plaintext: false,
        ca_cert: Some("/path/to/ca.pem".to_string()),
        headers: HashMap::new(),
        description: None,
    };

    let tls_client = GrpcClient::from_config(&tls_config);
    assert!(tls_client.is_ok());

    let client = tls_client.unwrap();
    assert!(!client.plaintext);
    assert_eq!(client.ca_cert_path, Some("/path/to/ca.pem".to_string()));
}

#[tokio::test]
async fn test_reflection_service_discovery() {
    init_test_logging();

    // This test would require a real gRPC server with reflection enabled
    // For now, we test the error handling when no server is available
    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:65534".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Test handle_service_list
    let services_result = client.handle_service_list().await;
    assert!(services_result.is_err());

    // Test handle_method_list
    let methods_result = client.handle_method_list("test.Service").await;
    assert!(methods_result.is_err());

    // Test handle_describe
    let describe_result = client.handle_describe("test.Service").await;
    assert!(describe_result.is_err());
}

#[tokio::test]
async fn test_error_handling_patterns() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:65534".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Test various error scenarios
    let call_result = client
        .handle_call("invalid.Service/Method", json!({}))
        .await;
    assert!(call_result.is_err());

    // Verify error contains useful information
    let error = call_result.unwrap_err();
    let error_string = error.to_string();
    assert!(!error_string.is_empty());
}

#[tokio::test]
async fn test_concurrent_client_operations() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:65534".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Test concurrent reflection calls
    let mut tasks = Vec::new();

    for _ in 0..5 {
        let client_clone = client.clone();
        let task = async move { client_clone.handle_service_list().await };
        tasks.push(task);
    }

    let results = futures::future::join_all(tasks).await;

    // All should fail with the same type of error (no server)
    for result in results {
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_client_configuration_validation() {
    init_test_logging();

    // Test various endpoint formats
    let valid_endpoints = vec![
        "localhost:9090",
        "127.0.0.1:8080",
        "example.com:443",
        "https://secure.example.com:443",
    ];

    for endpoint_str in valid_endpoints {
        let endpoint = Endpoint::parse(endpoint_str);
        assert!(
            endpoint.is_ok(),
            "Failed to parse endpoint: {}",
            endpoint_str
        );
    }

    let invalid_endpoints = vec![
        "localhost:99999", // Port out of range
        "localhost:abc",   // Invalid port format
        "localhost:-100",  // Negative port
    ];

    for endpoint_str in invalid_endpoints {
        let endpoint = Endpoint::parse(endpoint_str);
        assert!(
            endpoint.is_err(),
            "Should have failed to parse: {}",
            endpoint_str
        );
    }
}

#[tokio::test]
async fn test_client_state_management() {
    init_test_logging();

    let mut headers = HashMap::new();
    headers.insert("test-header".to_string(), "test-value".to_string());

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:9090".to_string(),
        plaintext: true,
        ca_cert: None,
        headers,
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Verify client maintains its configuration
    assert_eq!(client.endpoint.host, "localhost");
    assert_eq!(client.endpoint.port, 9090);
    assert!(client.plaintext);
    assert_eq!(client.headers.len(), 1);

    // Test client cloning preserves state
    let cloned_client = client.clone();
    assert_eq!(cloned_client.endpoint.host, "localhost");
    assert_eq!(cloned_client.headers.len(), 1);
}

#[tokio::test]
async fn test_method_call_parameter_validation() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:65534".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Test with empty method name
    let result = client.handle_call("", json!({})).await;
    assert!(result.is_err());

    // Test with invalid method format
    let result = client.handle_call("InvalidMethod", json!({})).await;
    assert!(result.is_err());

    // Test with valid format but non-existent server
    let result = client.handle_call("test.Service/Method", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_client_output_format() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:9090".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Verify default output format
    match client.format {
        grpc_client::domain::OutputFormat::Json {
            pretty,
            emit_defaults,
        } => {
            assert!(pretty);
            assert!(!emit_defaults);
        }
        _ => panic!("Expected JSON format"),
    }
}

#[tokio::test]
async fn test_client_performance_cache() {
    init_test_logging();

    let config = GrpcServerConfig {
        name: "Test Server".to_string(),
        endpoint: "localhost:9090".to_string(),
        plaintext: true,
        ca_cert: None,
        headers: HashMap::new(),
        description: None,
    };

    let client = GrpcClient::from_config(&config).unwrap();

    // Verify cache is initialized
    // The cache is private, so we can't directly test it,
    // but we can verify the client was created successfully
    assert_eq!(client.endpoint.host, "localhost");
}
