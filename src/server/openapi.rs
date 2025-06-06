use utoipa::OpenApi;

use crate::server::handlers::{CallRequest, ErrorResponse, MethodInfo, ServiceInfo};
// Note: JsonSchema and JsonSchemaProperty are excluded from OpenAPI due to recursive structure
use crate::server::state::ServerStatus;

/// OpenAPI documentation for the gRPC Client Web API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "gRPC Client Web API",
        version = "0.1.0",
        description = "A modern gRPC client with web interface for testing and exploring gRPC services",
        contact(
            name = "gRPC Client",
            url = "https://github.com/tyrchen/grpc-client"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    paths(
        crate::server::handlers::health_check,
        crate::server::handlers::list_servers,
        crate::server::handlers::list_services,
        crate::server::handlers::describe_service,
        crate::server::handlers::call_method,
        crate::server::handlers::describe_method
    ),
    components(
        schemas(
            ErrorResponse,
            CallRequest,
            ServiceInfo,
            MethodInfo,
            ServerStatus,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "servers", description = "Server management operations"),
        (name = "services", description = "gRPC service discovery and information"),
        (name = "grpc", description = "gRPC method execution"),
        (name = "schema", description = "JSON schema generation for gRPC methods")
    ),
    servers(
        (url = "/", description = "Local development server"),
        (url = "http://localhost:8080", description = "Default local server")
    )
)]
pub struct ApiDoc;
