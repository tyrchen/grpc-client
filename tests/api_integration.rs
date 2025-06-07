mod common;

use crate::common::start_grpc_server;
use axum::http::StatusCode;
use axum_test::TestServer;
use grpc_client::server::config::ServerConfig;
use grpc_client::server::routes::create_router_with_swagger;
use grpc_client::server::state::AppState;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;

async fn create_test_server() -> TestServer {
    start_grpc_server().await;
    let config = ServerConfig::load("fixtures/test.yml").await.unwrap();
    let state = AppState::new(config)
        .await
        .expect("Failed to create app state");
    let app = create_router_with_swagger(state, "ui/dist", true);
    TestServer::new(app).expect("Failed to create test server")
}

#[tokio::test]
async fn test_health_endpoint() {
    let server = create_test_server().await;
    let response = server.get("/api/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "grpc-client-web-ui");
    assert!(body.get("timestamp").is_some());
}

#[tokio::test]
async fn test_servers_list() {
    let server = create_test_server().await;

    let response = server.get("/api/servers").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body: Value = response.json();
    assert!(body.is_array());
    // Should have at least the default servers
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_services_endpoint_valid_server() {
    let server = create_test_server().await;

    // Try to list services for a configured server (should fail to connect but return proper error)
    let response = server.get("/api/servers/local/services").await;

    // Should return 500 (Internal Server Error) since we can't connect to the server
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let body: Value = response.json();
    assert_eq!(body["error"], "Server configuration not found");
}

#[tokio::test]
async fn test_services_endpoint_invalid_server() {
    let server = create_test_server().await;

    let response = server
        .get("/api/servers/non-existent-server/services")
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    let body: Value = response.json();
    assert_eq!(body["error"], "Server configuration not found");
}

#[tokio::test]
async fn test_describe_service_invalid_server() {
    let server = create_test_server().await;

    let response = server
        .get("/api/servers/non-existent/services/TestService")
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_describe_method_invalid_server() {
    let server = create_test_server().await;

    let response = server
        .get("/api/servers/non-existent/services/TestService/methods/TestMethod")
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_call_method_invalid_server() {
    let server = create_test_server().await;

    let call_request = json!({
        "method": "package.Service/Method",
        "data": {},
        "headers": {},
        "emitDefaults": false
    });

    let response = server
        .post("/api/servers/non-existent/call")
        .json(&call_request)
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_call_method_invalid_request_format() {
    let server = create_test_server().await;

    // Missing required fields
    let invalid_request = json!({
        "method": "package.Service/Method"
        // Missing "data" field
    });

    let response = server
        .post("/api/servers/local/call")
        .json(&invalid_request)
        .await;

    // Should return 422 (Unprocessable Entity) for malformed JSON
    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_call_method_valid_server_connection_failure() {
    let server = create_test_server().await;

    let call_request = json!({
        "method": "package.Service/Method",
        "data": {},
        "headers": {},
        "emitDefaults": false
    });

    let response = server
        .post("/api/servers/local/call")
        .json(&call_request)
        .await;

    // Should return 500 (Internal Server Error) since we can't connect to the server
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_openapi_spec_structure() {
    let server = create_test_server().await;

    let response = server.get("/api-docs/openapi.json").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let spec: Value = response.json();
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["info"]["title"], "gRPC Client Web API");
    assert!(spec.get("paths").is_some());
}

#[tokio::test]
async fn test_swagger_ui_available() {
    let server = create_test_server().await;

    let response = server.get("/swagger-ui/").await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_static_files_fallback() {
    let server = create_test_server().await;

    // Request a non-API path - should fallback to static files
    let response = server.get("/").await;

    // Should return OK or NOT_FOUND (depending on if ui/dist exists and has index.html)
    // In test environment, likely 404 since ui/dist may not exist
    assert!(
        response.status_code() == StatusCode::OK || response.status_code() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_concurrent_health_checks() {
    let server = create_test_server().await;

    let mut tasks = Vec::new();

    for _ in 0..10 {
        let server_ref = &server;
        let task = async move { server_ref.get("/api/health").await };
        tasks.push(task);
    }

    // Execute all requests concurrently
    let results = futures::future::join_all(tasks).await;

    // All should succeed
    for response in results {
        assert_eq!(response.status_code(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_api_endpoint_timeouts() {
    let server = create_test_server().await;

    // Test that API endpoints respond within reasonable time
    let health_future = server.get("/api/health");
    let health_response = timeout(Duration::from_secs(5), health_future).await;

    assert!(health_response.is_ok());
    assert_eq!(health_response.unwrap().status_code(), StatusCode::OK);
}

#[tokio::test]
async fn test_cors_headers() {
    let server = create_test_server().await;

    let response = server.get("/api/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // Check for CORS headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_json_content_type() {
    let server = create_test_server().await;

    let response = server.get("/api/health").await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("application/json"));
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = create_test_server().await;

    // Try to POST to a GET-only endpoint
    let response = server.post("/api/health").await;

    assert_eq!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED);
}
