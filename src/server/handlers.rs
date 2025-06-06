use crate::server::schema::SchemaProcessor;
use crate::server::state::{AppState, ServerStatus};
use axum::{
    Json as RequestJson,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Response structure for API errors
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    /// Error code identifier
    pub error: String,
    /// Additional error details
    pub details: Option<String>,
}

/// Request structure for gRPC method calls
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CallRequest {
    /// Full method name (e.g., "package.service/method")
    pub method: String,
    /// JSON request data
    pub data: Value,
    /// Custom headers to include in the gRPC call
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Whether to emit default values in the response
    #[serde(default)]
    pub emit_defaults: bool,
}

/// Response structure for gRPC method calls
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CallResponse {
    /// Whether the call was successful
    pub success: bool,
    /// Response data from the gRPC call
    pub response: Vec<Value>,
    /// Error message if the call failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Service information for API responses
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service description
    pub description: Option<String>,
    /// List of methods in this service
    pub methods: Vec<MethodInfo>,
}

/// Method information for API responses
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MethodInfo {
    /// Method name
    pub name: String,
    /// Input message type
    pub input_type: String,
    /// Output message type
    pub output_type: String,
    /// Whether the method uses client streaming
    pub client_streaming: bool,
    /// Whether the method uses server streaming
    pub server_streaming: bool,
    /// Streaming type description
    pub streaming_type: String,
    /// Method description
    pub description: Option<String>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy")
    ),
    tag = "health"
)]
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "grpc-client-web-ui",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// List all configured servers with their connection status
#[utoipa::path(
    get,
    path = "/api/servers",
    responses(
        (status = 200, description = "List of configured servers", body = Vec<ServerStatus>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "servers"
)]
pub async fn list_servers(State(state): State<AppState>) -> Json<Vec<ServerStatus>> {
    Json(state.get_connection_status())
}

/// List services for a specific server
#[utoipa::path(
    get,
    path = "/api/servers/{server_id}/services",
    params(
        ("server_id" = String, Path, description = "Server identifier")
    ),
    responses(
        (status = 200, description = "List of services for the server"),
        (status = 404, description = "Server not found", body = ErrorResponse),
        (status = 400, description = "Failed to connect to server", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "services"
)]
pub async fn list_services(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify server exists
    if state.get_server_config(&server_id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Server configuration not found".to_string(),
                details: None,
            }),
        ));
    }

    // Get client for the server
    let client = match state.get_client(&server_id).await {
        Ok(client) => client,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Failed to connect to server".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    let ret = client.handle_service_list().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list services".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    Ok(Json(json!({
        "server_id": server_id,
        "services": ret,
    })))
}

/// Describe a specific service
#[utoipa::path(
    get,
    path = "/api/servers/{server_id}/services/{service_name}",
    params(
        ("server_id" = String, Path, description = "Server identifier"),
        ("service_name" = String, Path, description = "Service name")
    ),
    responses(
        (status = 200, description = "Service description with methods", body = ServiceInfo),
        (status = 404, description = "Server not found", body = ErrorResponse),
        (status = 400, description = "Failed to connect to server", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "services"
)]
pub async fn describe_service(
    State(state): State<AppState>,
    Path((server_id, service_name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify server exists
    if state.get_server_config(&server_id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Server configuration not found".to_string(),
                details: None,
            }),
        ));
    }

    // Get client for the server
    let client = match state.get_client(&server_id).await {
        Ok(client) => client,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Failed to connect to server".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    let ret = client.handle_describe(&service_name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to describe service".to_string(),
                details: Some(e.to_string()),
            }),
        )
    })?;

    Ok(Json(json!({
        "server_id": server_id,
        "service_name": service_name,
        "description": ret,
    })))
}

/// Execute a gRPC method call
#[utoipa::path(
    post,
    path = "/api/servers/{server_id}/call",
    params(
        ("server_id" = String, Path, description = "Server identifier")
    ),
    request_body = CallRequest,
    responses(
        (status = 200, description = "Method call successful", body = CallResponse),
        (status = 404, description = "Server not found", body = ErrorResponse),
        (status = 400, description = "Invalid request or connection failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "grpc"
)]
pub async fn call_method(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
    RequestJson(request): RequestJson<CallRequest>,
) -> Result<Json<CallResponse>, (StatusCode, Json<ErrorResponse>)> {
    if state.get_server_config(&server_id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Server configuration not found".to_string(),
                details: None,
            }),
        ));
    }

    let client = match state.get_client(&server_id).await {
        Ok(client) => client,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Failed to connect to server".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    let ret = client
        .handle_call(&request.method, request.data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to call method".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    Ok(Json(CallResponse {
        success: true,
        response: ret,
        error: None,
    }))
}

/// Generate JSON schema for a method's input type
#[utoipa::path(
    get,
    path = "/api/servers/{server_id}/services/{service_name}/methods/{method_name}/schema",
    params(
        ("server_id" = String, Path, description = "Server identifier"),
        ("service_name" = String, Path, description = "Service name"),
        ("method_name" = String, Path, description = "Method name")
    ),
    responses(
        (status = 200, description = "JSON schema for method input"),
        (status = 404, description = "Server, service, or method not found", body = ErrorResponse),
        (status = 400, description = "Failed to connect to server", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "schema"
)]
pub async fn describe_method(
    State(state): State<AppState>,
    Path((server_id, service_name, method_name)): Path<(String, String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify server exists
    if state.get_server_config(&server_id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Server configuration not found".to_string(),
                details: None,
            }),
        ));
    }

    // Get client for the server
    let client = match state.get_client(&server_id).await {
        Ok(client) => client,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Failed to connect to server".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    // Get channel and create reflection client
    let channel = match client.get_or_create_channel().await {
        Ok(channel) => channel,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create gRPC channel".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    let Ok(methods) = client.handle_method_list(&service_name).await else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list service methods".to_string(),
                details: None,
            }),
        ));
    };

    // Find the specific method
    let method = methods.iter().find(|m| m.name.as_str() == method_name);
    let Some(method) = method else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Method not found".to_string(),
                details: Some(format!(
                    "Method '{}' not found in service '{}'",
                    method_name, service_name
                )),
            }),
        ));
    };

    // Get descriptor pool for the input type
    let input_type = &method.input_type;
    let output_type = &method.output_type;

    let Ok(pool) = client
        .get_or_create_descriptor_pool(channel, input_type, output_type)
        .await
    else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to create descriptor pool".to_string(),
                details: None,
            }),
        ));
    };
    // Get message descriptor for input type
    let Some(input_descriptor) = pool.get_message_by_name(input_type) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to get input message descriptor".to_string(),
                details: Some(format!("Message type '{}' not found", input_type)),
            }),
        ));
    };

    // Generate JSON schema
    let mut schema_processor = SchemaProcessor::new();
    let schema = match schema_processor.generate_schema(&input_descriptor) {
        Ok(schema) => schema,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate JSON schema".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    // Generate validation rules
    let validation_rules = match schema_processor.generate_validation_rules(&input_descriptor) {
        Ok(rules) => rules,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate validation rules".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    Ok(Json(json!({
        "server_id": server_id,
        "service_name": service_name,
        "method_name": method_name,
        "input_type": input_type,
        "output_type": output_type,
        "schema": schema,
        "validation_rules": validation_rules,
        "streaming_type": match (method.client_streaming, method.server_streaming) {
            (false, false) => "unary",
            (false, true) => "server_streaming",
            (true, false) => "client_streaming",
            (true, true) => "bidirectional",
        },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::ServerConfig;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        let value = response.0;
        assert!(value["status"].as_str().unwrap() == "healthy");
        assert!(value["service"].as_str().unwrap() == "grpc-client-web-ui");
    }

    #[tokio::test]
    async fn test_list_servers() {
        let config = ServerConfig::default();
        let state = AppState::new(config).await.unwrap();

        let response = list_servers(State(state)).await;
        let servers = response.0;

        assert!(!servers.is_empty());
        assert!(servers.iter().any(|s| s.id == "local"));
    }
}
