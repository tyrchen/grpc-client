use crate::domain::{MethodName, ServiceName};
use anyhow::{Context, Result};
use async_trait::async_trait;
use prost::Message;
use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    ListServiceResponse, ServerReflectionRequest, server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamingType {
    Unary,         // 1 request → 1 response
    ServerStream,  // 1 request → multiple responses
    ClientStream,  // multiple requests → 1 response
    BiDirectional, // multiple requests → multiple responses
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Symbol {
    Service(ServiceDescriptor),
    Method(MethodDescriptor),
    Message(MessageDescriptor),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceDescriptor {
    pub name: ServiceName,
    pub methods: Vec<MethodDescriptor>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MethodDescriptor {
    pub name: MethodName,
    pub service: ServiceName,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub streaming_type: StreamingType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageDescriptor {
    pub name: String,
    pub fields: Vec<FieldDescriptor>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDescriptor {
    pub name: String,
    pub field_type: String,
    pub number: i32,
    pub optional: bool,
    pub repeated: bool,
}

#[async_trait]
pub trait SchemaSource: Send + Sync {
    async fn list_services(&mut self) -> Result<Vec<ServiceName>>;
    async fn get_service(&mut self, service: &ServiceName) -> Result<ServiceDescriptor>;
    async fn resolve_symbol(&mut self, symbol: &str) -> Result<Symbol>;
    async fn list_methods(&mut self, service: &ServiceName) -> Result<Vec<MethodDescriptor>>;
    async fn get_file_containing_symbol(&mut self, symbol: &str) -> Result<FileDescriptorProto>;
    async fn get_file_by_filename(&mut self, name: &str) -> Result<FileDescriptorProto>;
}

pub struct ReflectionClient {
    client: ServerReflectionClient<Channel>,
    service_cache: HashMap<String, ServiceDescriptor>,
    file_cache: HashMap<String, FileDescriptorProto>,
}

impl ReflectionClient {
    pub fn new(channel: Channel) -> Self {
        Self {
            client: ServerReflectionClient::new(channel),
            service_cache: HashMap::new(),
            file_cache: HashMap::new(),
        }
    }

    pub fn clear_cache(&mut self) {
        self.service_cache.clear();
        self.file_cache.clear();
    }

    async fn make_request(&mut self, request: MessageRequest) -> Result<MessageResponse> {
        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(request),
        };

        let response = self
            .client
            .server_reflection_info(tokio_stream::once(request))
            .await
            .context("Failed to connect to reflection service")?;

        let mut stream = response.into_inner();

        if let Some(response) = stream.message().await? {
            if let Some(message_response) = response.message_response {
                return Ok(message_response);
            }
        }

        anyhow::bail!("No response received from reflection service")
    }

    fn parse_service_names(&self, response: &ListServiceResponse) -> Vec<ServiceName> {
        response
            .service
            .iter()
            .map(|service| ServiceName::new(service.name.clone()))
            .collect()
    }

    fn parse_service_from_file(
        &self,
        file_desc: &FileDescriptorProto,
        service_name: &str,
    ) -> Result<ServiceDescriptor> {
        let package = file_desc.package.as_deref().unwrap_or("");

        for service in &file_desc.service {
            let full_name = if package.is_empty() {
                service.name.clone().unwrap_or_default()
            } else {
                format!(
                    "{}.{}",
                    package,
                    service.name.as_ref().unwrap_or(&String::new())
                )
            };

            if full_name == service_name || service.name.as_deref() == Some(service_name) {
                return Ok(self.build_service_descriptor(service, &full_name, package));
            }
        }

        anyhow::bail!("Service {} not found in file descriptor", service_name)
    }

    fn build_service_descriptor(
        &self,
        service: &ServiceDescriptorProto,
        full_name: &str,
        package: &str,
    ) -> ServiceDescriptor {
        let methods = service
            .method
            .iter()
            .map(|method| self.build_method_descriptor(method, full_name, package))
            .collect();

        ServiceDescriptor {
            name: ServiceName::new(full_name.to_string()),
            methods,
            description: service.options.as_ref().and(None),
        }
    }

    fn build_method_descriptor(
        &self,
        method: &MethodDescriptorProto,
        service_name: &str,
        package: &str,
    ) -> MethodDescriptor {
        let input_type =
            self.resolve_type_name(method.input_type.as_deref().unwrap_or(""), package);
        let output_type =
            self.resolve_type_name(method.output_type.as_deref().unwrap_or(""), package);

        let client_streaming = method.client_streaming.unwrap_or(false);
        let server_streaming = method.server_streaming.unwrap_or(false);

        let streaming_type = match (client_streaming, server_streaming) {
            (false, false) => StreamingType::Unary,
            (false, true) => StreamingType::ServerStream,
            (true, false) => StreamingType::ClientStream,
            (true, true) => StreamingType::BiDirectional,
        };

        MethodDescriptor {
            name: MethodName::new(method.name.clone().unwrap_or_default()),
            service: ServiceName::new(service_name.to_string()),
            input_type,
            output_type,
            client_streaming,
            server_streaming,
            streaming_type,
            description: None,
        }
    }

    /// Resolve type name, removing leading dots and handling package names
    fn resolve_type_name(&self, type_name: &str, _package: &str) -> String {
        // Remove leading dot if present
        if let Some(stripped) = type_name.strip_prefix('.') {
            stripped.to_string()
        } else {
            type_name.to_string()
        }
    }
}

#[async_trait]
impl SchemaSource for ReflectionClient {
    async fn list_services(&mut self) -> Result<Vec<ServiceName>> {
        let request = MessageRequest::ListServices(String::new());
        let response = self.make_request(request).await?;

        match response {
            MessageResponse::ListServicesResponse(list_response) => {
                Ok(self.parse_service_names(&list_response))
            }
            MessageResponse::ErrorResponse(error) => {
                anyhow::bail!(
                    "Reflection error: {} - {}",
                    error.error_code,
                    error.error_message
                )
            }
            _ => anyhow::bail!("Unexpected response type for list services"),
        }
    }

    async fn get_service(&mut self, service: &ServiceName) -> Result<ServiceDescriptor> {
        // Check cache first
        if let Some(cached) = self.service_cache.get(service.as_str()) {
            return Ok(cached.clone());
        }

        // Get file descriptor containing the service
        let file_desc = self.get_file_containing_symbol(service.as_str()).await?;
        let service_desc = self.parse_service_from_file(&file_desc, service.as_str())?;

        // Cache the service descriptor
        self.service_cache
            .insert(service.as_str().to_string(), service_desc.clone());

        Ok(service_desc)
    }

    async fn resolve_symbol(&mut self, symbol: &str) -> Result<Symbol> {
        // Try to resolve as service first
        let service_name = ServiceName::new(symbol.to_string());
        if let Ok(service) = self.get_service(&service_name).await {
            return Ok(Symbol::Service(service));
        }

        // Try to resolve as service.method format
        if let Some(dot_pos) = symbol.rfind('.') {
            let service_part = &symbol[..dot_pos];
            let method_part = &symbol[dot_pos + 1..];

            let service_name = ServiceName::new(service_part.to_string());
            if let Ok(service) = self.get_service(&service_name).await {
                // Look for the method in the service
                for method in &service.methods {
                    if method.name.as_str() == method_part {
                        return Ok(Symbol::Method(method.clone()));
                    }
                }
            }
        }

        // Try to resolve as service/method format
        if let Some(slash_pos) = symbol.rfind('/') {
            let service_part = &symbol[..slash_pos];
            let method_part = &symbol[slash_pos + 1..];

            let service_name = ServiceName::new(service_part.to_string());
            if let Ok(service) = self.get_service(&service_name).await {
                // Look for the method in the service
                for method in &service.methods {
                    if method.name.as_str() == method_part {
                        return Ok(Symbol::Method(method.clone()));
                    }
                }
            }
        }

        // For now, we don't support message type resolution without more complex descriptor parsing
        // This would require parsing all file descriptors and building a complete type registry

        anyhow::bail!(
            "Symbol not found: {}. Supported formats: 'ServiceName' or 'ServiceName.MethodName' or 'ServiceName/MethodName'",
            symbol
        )
    }

    async fn list_methods(&mut self, service: &ServiceName) -> Result<Vec<MethodDescriptor>> {
        let service_desc = self.get_service(service).await?;
        Ok(service_desc.methods)
    }

    async fn get_file_containing_symbol(&mut self, symbol: &str) -> Result<FileDescriptorProto> {
        // Check cache first
        if let Some(cached) = self.file_cache.get(symbol) {
            return Ok(cached.clone());
        }

        let request = MessageRequest::FileContainingSymbol(symbol.to_string());
        let response = self.make_request(request).await?;

        match response {
            MessageResponse::FileDescriptorResponse(file_response) => {
                if let Some(file_data) = file_response.file_descriptor_proto.first() {
                    let file_desc = FileDescriptorProto::decode(&file_data[..])
                        .context("Failed to decode file descriptor")?;

                    // Cache the file descriptor by its name for future lookups
                    self.file_cache
                        .insert(file_desc.name().to_string(), file_desc.clone());
                    Ok(file_desc)
                } else {
                    anyhow::bail!("No file descriptor found for symbol: {}", symbol)
                }
            }
            MessageResponse::ErrorResponse(error) => {
                anyhow::bail!(
                    "Reflection error: {} - {}",
                    error.error_code,
                    error.error_message
                )
            }
            _ => anyhow::bail!("Unexpected response type for file containing symbol"),
        }
    }

    async fn get_file_by_filename(&mut self, name: &str) -> Result<FileDescriptorProto> {
        if let Some(file_desc) = self.file_cache.get(name) {
            return Ok(file_desc.clone());
        }

        let request = MessageRequest::FileByFilename(name.to_string());
        let response = self.make_request(request).await?;

        match response {
            MessageResponse::FileDescriptorResponse(file_response) => {
                if let Some(file_data) = file_response.file_descriptor_proto.first() {
                    let file_desc = FileDescriptorProto::decode(&file_data[..])
                        .context("Failed to decode file descriptor")?;
                    self.file_cache.insert(name.to_string(), file_desc.clone());
                    Ok(file_desc)
                } else {
                    anyhow::bail!("No file descriptor found for filename: {}", name)
                }
            }
            MessageResponse::ErrorResponse(error) => {
                anyhow::bail!(
                    "Reflection error for filename {}: {} - {}",
                    name,
                    error.error_code,
                    error.error_message
                )
            }
            _ => anyhow::bail!(
                "Unexpected response type for get_file_by_filename for {}",
                name
            ),
        }
    }
}

/// Create a reflection client from a gRPC channel
pub fn create_reflection_client(channel: Channel) -> Box<dyn SchemaSource> {
    Box::new(ReflectionClient::new(channel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_types() {
        let service = ServiceDescriptor {
            name: ServiceName::new("TestService".to_string()),
            methods: Vec::new(),
            description: Some("Test service".to_string()),
        };

        assert_eq!(service.name.as_str(), "TestService");
        assert_eq!(service.description, Some("Test service".to_string()));
    }

    #[test]
    fn test_method_descriptor() {
        let method = MethodDescriptor {
            name: MethodName::new("GetUser".to_string()),
            service: ServiceName::new("UserService".to_string()),
            input_type: "GetUserRequest".to_string(),
            output_type: "GetUserResponse".to_string(),
            client_streaming: false,
            server_streaming: false,
            streaming_type: StreamingType::Unary,
            description: None,
        };

        assert_eq!(method.name.as_str(), "GetUser");
        assert_eq!(method.service.as_str(), "UserService");
        assert!(!method.client_streaming);
        assert!(!method.server_streaming);
        assert_eq!(method.streaming_type, StreamingType::Unary);
    }

    #[test]
    fn test_streaming_type_detection() {
        // Test unary
        let unary_method = MethodDescriptor {
            name: MethodName::new("GetUser".to_string()),
            service: ServiceName::new("UserService".to_string()),
            input_type: "GetUserRequest".to_string(),
            output_type: "GetUserResponse".to_string(),
            client_streaming: false,
            server_streaming: false,
            streaming_type: StreamingType::Unary,
            description: None,
        };
        assert_eq!(unary_method.streaming_type, StreamingType::Unary);

        // Test server streaming
        let server_stream_method = MethodDescriptor {
            name: MethodName::new("ListUsers".to_string()),
            service: ServiceName::new("UserService".to_string()),
            input_type: "ListUsersRequest".to_string(),
            output_type: "ListUsersResponse".to_string(),
            client_streaming: false,
            server_streaming: true,
            streaming_type: StreamingType::ServerStream,
            description: None,
        };
        assert_eq!(
            server_stream_method.streaming_type,
            StreamingType::ServerStream
        );

        // Test client streaming
        let client_stream_method = MethodDescriptor {
            name: MethodName::new("CreateUsers".to_string()),
            service: ServiceName::new("UserService".to_string()),
            input_type: "CreateUserRequest".to_string(),
            output_type: "CreateUsersResponse".to_string(),
            client_streaming: true,
            server_streaming: false,
            streaming_type: StreamingType::ClientStream,
            description: None,
        };
        assert_eq!(
            client_stream_method.streaming_type,
            StreamingType::ClientStream
        );

        // Test bidirectional streaming
        let bidirectional_method = MethodDescriptor {
            name: MethodName::new("ChatUsers".to_string()),
            service: ServiceName::new("UserService".to_string()),
            input_type: "ChatMessage".to_string(),
            output_type: "ChatMessage".to_string(),
            client_streaming: true,
            server_streaming: true,
            streaming_type: StreamingType::BiDirectional,
            description: None,
        };
        assert_eq!(
            bidirectional_method.streaming_type,
            StreamingType::BiDirectional
        );
    }

    #[test]
    fn test_field_descriptor() {
        let field = FieldDescriptor {
            name: "user_id".to_string(),
            field_type: "string".to_string(),
            number: 1,
            optional: false,
            repeated: false,
        };

        assert_eq!(field.name, "user_id");
        assert_eq!(field.field_type, "string");
        assert_eq!(field.number, 1);
        assert!(!field.optional);
        assert!(!field.repeated);
    }

    #[test]
    fn test_symbol_enum() {
        let service = ServiceDescriptor {
            name: ServiceName::new("TestService".to_string()),
            methods: Vec::new(),
            description: None,
        };

        let symbol = Symbol::Service(service);

        match symbol {
            Symbol::Service(s) => assert_eq!(s.name.as_str(), "TestService"),
            _ => panic!("Expected Service symbol"),
        }
    }

    #[tokio::test]
    async fn test_type_name_resolution() {
        // Create a dummy channel for testing
        let channel = Channel::from_static("http://localhost:9090").connect_lazy();
        let client = ReflectionClient::new(channel);

        assert_eq!(
            client.resolve_type_name(".package.Message", "package"),
            "package.Message"
        );
        assert_eq!(client.resolve_type_name("Message", "package"), "Message");
        assert_eq!(
            client.resolve_type_name(".google.protobuf.Empty", ""),
            "google.protobuf.Empty"
        );
    }

    #[test]
    fn test_symbol_resolution_parsing() {
        // Test service.method parsing logic
        let service_method = "UserService.GetUser";
        if let Some(dot_pos) = service_method.rfind('.') {
            let service_part = &service_method[..dot_pos];
            let method_part = &service_method[dot_pos + 1..];
            assert_eq!(service_part, "UserService");
            assert_eq!(method_part, "GetUser");
        } else {
            panic!("Expected to find dot in service.method");
        }

        // Test service/method parsing logic
        let service_method = "UserService/GetUser";
        if let Some(slash_pos) = service_method.rfind('/') {
            let service_part = &service_method[..slash_pos];
            let method_part = &service_method[slash_pos + 1..];
            assert_eq!(service_part, "UserService");
            assert_eq!(method_part, "GetUser");
        } else {
            panic!("Expected to find slash in service/method");
        }

        // Test fully qualified service.method parsing
        let fully_qualified = "com.example.UserService.GetUser";
        if let Some(dot_pos) = fully_qualified.rfind('.') {
            let service_part = &fully_qualified[..dot_pos];
            let method_part = &fully_qualified[dot_pos + 1..];
            assert_eq!(service_part, "com.example.UserService");
            assert_eq!(method_part, "GetUser");
        } else {
            panic!("Expected to find dot in fully qualified name");
        }
    }
}
