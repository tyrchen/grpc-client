use crate::server::handlers;
use crate::server::openapi::ApiDoc;
use crate::server::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Create the main application router with all routes and middleware
pub fn create_router(state: AppState, ui_path: &str) -> Router {
    create_router_with_swagger(state, ui_path, true)
}

/// Create the main application router with optional Swagger UI
pub fn create_router_with_swagger(state: AppState, ui_path: &str, enable_swagger: bool) -> Router {
    // API routes
    let api_routes = Router::new()
        .route("/servers", get(handlers::list_servers))
        .route(
            "/servers/{server_id}/services",
            get(handlers::list_services),
        )
        .route(
            "/servers/{server_id}/services/{service_name}",
            get(handlers::describe_service),
        )
        .route(
            "/servers/{server_id}/services/{service_name}/methods/{method_name}",
            get(handlers::describe_method),
        )
        .route("/servers/{server_id}/call", post(handlers::call_method))
        .route("/health", get(handlers::health_check))
        .with_state(state);

    // CORS middleware for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Main router with API routes and static file serving as fallback
    let mut router = Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new(ui_path))
        .layer(ServiceBuilder::new().layer(cors));

    // Add Swagger UI if enabled
    if enable_swagger {
        let swagger_routes =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
        router = router.merge(swagger_routes);
    }

    router
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::ServerConfig;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_endpoint() {
        let config = ServerConfig::default();
        let state = AppState::new(config).await.unwrap();
        let app = create_router_with_swagger(state, "ui/dist", false);

        let server = TestServer::new(app).unwrap();
        let response = server.get("/api/health").await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_servers_endpoint() {
        let config = ServerConfig::default();
        let state = AppState::new(config).await.unwrap();
        let app = create_router_with_swagger(state, "ui/dist", false);

        let server = TestServer::new(app).unwrap();
        let response = server.get("/api/servers").await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_openapi_doc_generation() {
        // Test that OpenAPI doc can be generated without stack overflow
        let openapi_doc = ApiDoc::openapi();

        // Verify basic structure
        assert_eq!(openapi_doc.info.title, "gRPC Client Web API");
        assert_eq!(openapi_doc.info.version, "0.1.0");
        assert!(!openapi_doc.paths.paths.is_empty());
        assert!(!openapi_doc.components.as_ref().unwrap().schemas.is_empty());
    }
}
