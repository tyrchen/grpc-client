# Swagger API Documentation Integration

This document describes the OpenAPI/Swagger integration added to the gRPC Client Web API.

## Overview

The gRPC Client now includes comprehensive API documentation using OpenAPI 3.0 specification with Swagger UI for interactive exploration.

## Features

- **Complete API Documentation**: All endpoints are documented with request/response schemas
- **Interactive Testing**: Swagger UI allows testing API endpoints directly from the browser
- **Schema Validation**: Request and response types are properly defined with validation rules
- **Organized by Tags**: Endpoints are grouped by functionality (servers, services, grpc, schema, health)

## Accessing the Documentation

### Swagger UI

Visit the interactive Swagger UI at: <http://localhost:8080/swagger-ui/>

### OpenAPI JSON Specification

The raw OpenAPI specification is available at: <http://localhost:8080/api-docs/openapi.json>

## API Endpoints Documented

### Health Check

- `GET /api/health` - Service health status

### Server Management

- `GET /api/servers` - List all configured servers

### Service Discovery

- `GET /api/servers/{server_id}/services` - List services for a server
- `GET /api/servers/{server_id}/services/{service_name}` - Describe a specific service

### gRPC Method Execution

- `POST /api/servers/{server_id}/call` - Execute a gRPC method call

### Schema Generation

- `GET /api/servers/{server_id}/services/{service_name}/methods/{method_name}/schema` - Get JSON schema for method input

## Schema Types

The following data structures are documented in the OpenAPI specification:

- **ErrorResponse**: Standard error response format
- **CallRequest**: gRPC method call request structure
- **CallResponse**: gRPC method call response structure
- **ServiceInfo**: Service information with methods
- **MethodInfo**: Individual method details
- **ServerStatus**: Server connection status

## Configuration

Swagger UI can be enabled/disabled using the `create_router_with_swagger()` function:

```rust
// Enable Swagger UI
let app = create_router_with_swagger(state, "ui/dist", true);

// Disable Swagger UI (for production)
let app = create_router_with_swagger(state, "ui/dist", false);
```

## Implementation Details

### OpenAPI Generation

- Uses `utoipa` crate for OpenAPI specification generation
- Automatic schema generation from Rust types using `#[derive(ToSchema)]`
- Path documentation using `#[utoipa::path]` annotations

### Swagger UI Integration

- Uses `utoipa-swagger-ui` for embedded Swagger UI
- Serves static assets for the interactive interface
- Configurable endpoint paths

### Error Handling

- Consistent error response format across all endpoints
- Proper HTTP status codes for different error conditions
- Detailed error messages with optional additional details

## Development Notes

### Adding New Endpoints

When adding new API endpoints:

1. Add `#[utoipa::path]` annotation to the handler function
2. Include the function in the `paths()` section of `ApiDoc`
3. Add any new request/response types to the `schemas()` section
4. Use appropriate tags to group related endpoints

### Schema Limitations

- Recursive schema types (like `JsonSchemaProperty`) are excluded from OpenAPI to prevent stack overflow
- Complex nested structures may need manual schema definitions

### Testing

- OpenAPI generation is tested to ensure no circular dependencies
- Swagger UI endpoints are conditionally enabled for testing
- All existing functionality remains unaffected

## Benefits

1. **Developer Experience**: Easy API exploration and testing
2. **Documentation**: Always up-to-date API documentation
3. **Client Generation**: OpenAPI spec can be used to generate client libraries
4. **Validation**: Request/response validation using schema definitions
5. **Standards Compliance**: Follows OpenAPI 3.0 specification

## Future Enhancements

- Add authentication documentation for secured endpoints
- Include example requests/responses for complex operations
- Add rate limiting documentation
- Expand schema definitions for more complex types
