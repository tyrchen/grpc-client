#![allow(clippy::too_many_arguments)]

use crate::{
    cli::{Cli, Command, FormatType},
    codec::BytesCodec,
    connection::create_channel,
    domain::{Endpoint, OutputFormat, ServiceName},
    reflection::{
        MessageDescriptor, MethodDescriptor, ServiceDescriptor, StreamingType, Symbol,
        create_reflection_client,
    },
    server::config::GrpcServerConfig,
};
use anyhow::{Context, Ok, Result, anyhow, bail};
use bytes::Bytes;
use dashmap::DashMap;
use futures::stream::{self, TryStreamExt};
use http::uri::PathAndQuery;
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use prost_types::FileDescriptorSet;
use serde_json::Value;
use std::sync::Arc;
use tonic::{
    Code, Request, Response, Status, Streaming, client::Grpc, metadata::MetadataKey,
    transport::Channel,
};

#[derive(Debug, Clone)]
pub struct GrpcClient {
    pub endpoint: Endpoint,
    pub headers: Vec<(String, String)>,
    pub format: OutputFormat,
    pub verbose: bool,
    pub ca_cert_path: Option<String>,
    pub plaintext: bool,
    pub cache: PerformanceCache,
}

impl GrpcClient {
    /// Create a new GrpcClient from server configuration
    pub fn from_config(config: &GrpcServerConfig) -> Result<Self> {
        let endpoint = Endpoint::parse(&config.endpoint)?;
        let headers = config
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let ca_cert_path = config.ca_cert.clone();
        let plaintext = config.plaintext;
        let verbose = false;

        let format = OutputFormat::Json {
            pretty: true,
            emit_defaults: false,
        };

        Ok(Self {
            endpoint,
            headers,
            format,
            verbose,
            ca_cert_path,
            plaintext,
            cache: PerformanceCache::new(),
        })
    }

    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let endpoint = match &cli.command {
            Command::List { endpoint, .. } | Command::Describe { endpoint, .. } => {
                Endpoint::parse(endpoint)?
            }
            Command::Call { endpoint, .. } => Endpoint::parse(endpoint)?,
            Command::Server { .. } => todo!(),
        };

        let headers = cli
            .header
            .iter()
            .map(|h| {
                let parts: Vec<&str> = h.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
                } else {
                    anyhow::bail!("Invalid header format: {}. Expected 'name: value'", h)
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse headers")?;

        let format = match cli.format {
            FormatType::Json => OutputFormat::Json {
                pretty: true,
                emit_defaults: false,
            },
            FormatType::Text => OutputFormat::Text { compact: false },
        };

        let client = Self {
            endpoint,
            headers,
            format,
            verbose: cli.verbose,
            ca_cert_path: cli.ca.clone(),
            plaintext: cli.plaintext,
            cache: PerformanceCache::new(),
        };

        Ok(client)
    }

    pub async fn handle_service_list(&self) -> Result<Vec<ServiceName>> {
        let channel = self.get_or_create_channel().await?;
        let mut client = create_reflection_client(channel);
        let services = client
            .list_services()
            .await
            .context("Failed to list services")?;

        Ok(services)
    }

    pub async fn handle_method_list(&self, service: &str) -> Result<Vec<MethodDescriptor>> {
        let channel = self.get_or_create_channel().await?;
        let mut client = create_reflection_client(channel);
        let service = ServiceName::new(service.to_string());
        let methods = client
            .list_methods(&service)
            .await
            .with_context(|| format!("Failed to list methods for service: {}", service))?;
        Ok(methods)
    }

    pub async fn handle_describe(&self, symbol: &str) -> Result<Symbol> {
        let channel = self.get_or_create_channel().await?;

        let mut client = create_reflection_client(channel);

        let symbol = client
            .resolve_symbol(symbol)
            .await
            .with_context(|| format!("Failed to resolve symbol: {}", symbol))?;

        Ok(symbol)
    }

    pub async fn handle_call(&self, method: &str, data: Value) -> Result<Vec<Value>> {
        let (service_name, method_name) = parse_method(method)?;

        if self.verbose {
            println!("Calling method: {}.{}", service_name, method_name);
            println!("Endpoint: {}", self.endpoint);
        }

        let channel = self.get_or_create_channel().await?;

        let mut client = create_reflection_client(channel.clone());
        let service = ServiceName::new(service_name.to_string());
        let service = client
            .get_service(&service)
            .await
            .with_context(|| format!("Failed to get service: {}", service_name))?;

        // Find the method
        let method = service
            .methods
            .iter()
            .find(|m| m.name.as_str() == method_name)
            .ok_or_else(|| {
                anyhow!(
                    "Method {} not found in service {}",
                    method_name,
                    service_name
                )
            })?;

        if self.verbose {
            println!("Method info:");
            println!("  Input type: {}", method.input_type);
            println!("  Output type: {}", method.output_type);
            println!(
                "  Streaming type: {}",
                match method.streaming_type {
                    StreamingType::Unary => "Unary",
                    StreamingType::ServerStream => "ServerStream",
                    StreamingType::ClientStream => "ClientStream",
                    StreamingType::BiDirectional => "BiDirectional",
                }
            );
            println!("  Request data: {:?}", data);
        }

        // Route to appropriate handler based on streaming type
        match method.streaming_type {
            StreamingType::Unary => {
                let ret = self.handle_unary(channel, &service, method, data).await?;
                Ok(vec![ret])
            }
            StreamingType::ServerStream => {
                let ret = self
                    .handle_server_streaming(channel, &service, method, data)
                    .await?;
                Ok(ret)
            }
            StreamingType::ClientStream => {
                let ret = self
                    .handle_client_streaming(channel, &service, method, data)
                    .await?;
                Ok(ret)
            }
            StreamingType::BiDirectional => {
                let ret = self
                    .handle_bidi_streaming(channel, &service, method, data)
                    .await?;
                Ok(ret)
            }
        }
    }

    pub async fn format_service_description(&self, service: &ServiceDescriptor) -> Result<()> {
        match &self.format {
            OutputFormat::Json { pretty, .. } => {
                let methods_json: Vec<Value> = service
                    .methods
                    .iter()
                    .map(|method| {
                        serde_json::json!({
                            "name": method.name.as_str(),
                            "input_type": method.input_type,
                            "output_type": method.output_type,
                            "client_streaming": method.client_streaming,
                            "server_streaming": method.server_streaming,
                            "description": method.description
                        })
                    })
                    .collect();

                let service_json = serde_json::json!({
                    "name": service.name.as_str(),
                    "description": service.description,
                    "methods": methods_json
                });

                let output = if *pretty {
                    serde_json::to_string_pretty(&service_json)?
                } else {
                    serde_json::to_string(&service_json)?
                };
                println!("{}", output);
            }
            OutputFormat::Text { .. } => {
                println!("service {} {{", service.name.as_str());

                if let Some(desc) = &service.description {
                    println!("  // {}", desc);
                }

                for method in &service.methods {
                    let streaming_prefix = match (method.client_streaming, method.server_streaming)
                    {
                        (true, true) => "stream ",
                        (true, false) => "stream ",
                        (false, true) => "",
                        (false, false) => "",
                    };

                    let streaming_suffix = match (method.client_streaming, method.server_streaming)
                    {
                        (true, true) => " returns (stream response)",
                        (true, false) => " returns (response)",
                        (false, true) => " returns (stream response)",
                        (false, false) => " returns (response)",
                    };

                    println!(
                        "  rpc {}({}{}) {};",
                        method.name.as_str(),
                        streaming_prefix,
                        method.input_type,
                        streaming_suffix.replace("response", &method.output_type)
                    );

                    if let Some(desc) = &method.description {
                        println!("    // {}", desc);
                    }
                }

                println!("}}");
            }
        }

        Ok(())
    }

    pub async fn format_method_description(&self, method: &MethodDescriptor) -> Result<()> {
        match &self.format {
            OutputFormat::Json { pretty, .. } => {
                let streaming_type_str = match method.streaming_type {
                    StreamingType::Unary => "unary",
                    StreamingType::ServerStream => "server_streaming",
                    StreamingType::ClientStream => "client_streaming",
                    StreamingType::BiDirectional => "bidirectional",
                };

                let method_json = serde_json::json!({
                    "name": method.name.as_str(),
                    "service": method.service.as_str(),
                    "input_type": method.input_type,
                    "output_type": method.output_type,
                    "streaming_type": streaming_type_str,
                    "client_streaming": method.client_streaming,
                    "server_streaming": method.server_streaming,
                    "description": method.description,
                    "full_name": format!("{}.{}", method.service.as_str(), method.name.as_str())
                });

                let output = if *pretty {
                    serde_json::to_string_pretty(&method_json)?
                } else {
                    serde_json::to_string(&method_json)?
                };
                println!("{}", output);
            }
            OutputFormat::Text { .. } => {
                let streaming_info = match (method.client_streaming, method.server_streaming) {
                    (true, true) => " (bidirectional streaming)",
                    (true, false) => " (client streaming)",
                    (false, true) => " (server streaming)",
                    (false, false) => " (unary)",
                };

                println!(
                    "Method: {}.{}{}",
                    method.service.as_str(),
                    method.name.as_str(),
                    streaming_info
                );
                println!("  Service: {}", method.service.as_str());
                println!("  Input type: {}", method.input_type);
                println!("  Output type: {}", method.output_type);

                if let Some(desc) = &method.description {
                    println!("  Description: {}", desc);
                }

                // Show protobuf-style method signature
                let streaming_prefix = match (method.client_streaming, method.server_streaming) {
                    (true, true) => "stream ",
                    (true, false) => "stream ",
                    (false, true) => "",
                    (false, false) => "",
                };

                let streaming_suffix = match (method.client_streaming, method.server_streaming) {
                    (true, true) => "stream ",
                    (true, false) => "",
                    (false, true) => "stream ",
                    (false, false) => "",
                };

                println!(
                    "  Signature: rpc {}({}{}) returns ({}{});",
                    method.name.as_str(),
                    streaming_prefix,
                    method.input_type,
                    streaming_suffix,
                    method.output_type
                );
            }
        }

        Ok(())
    }

    pub async fn format_message_description(&self, message: &MessageDescriptor) -> Result<()> {
        match &self.format {
            OutputFormat::Json { pretty, .. } => {
                let fields_json: Vec<Value> = message
                    .fields
                    .iter()
                    .map(|field| {
                        serde_json::json!({
                            "name": field.name,
                            "type": field.field_type,
                            "number": field.number,
                            "optional": field.optional,
                            "repeated": field.repeated
                        })
                    })
                    .collect();

                let message_json = serde_json::json!({
                    "name": message.name,
                    "description": message.description,
                    "fields": fields_json
                });

                let output = if *pretty {
                    serde_json::to_string_pretty(&message_json)?
                } else {
                    serde_json::to_string(&message_json)?
                };
                println!("{}", output);
            }
            OutputFormat::Text { .. } => {
                println!("message {} {{", message.name);

                if let Some(desc) = &message.description {
                    println!("  // {}", desc);
                }

                for field in &message.fields {
                    let field_modifier = if field.repeated {
                        "repeated "
                    } else if field.optional {
                        "optional "
                    } else {
                        ""
                    };

                    println!(
                        "  {}{} {} = {};",
                        field_modifier, field.field_type, field.name, field.number
                    );
                }

                println!("}}");
            }
        }

        Ok(())
    }

    async fn handle_unary(
        &self,
        channel: Channel,
        service: &ServiceDescriptor,
        method: &MethodDescriptor,
        data: Value,
    ) -> Result<Value> {
        let input = &method.input_type;
        let output = &method.output_type;
        let service_name = service.name.as_str();
        let method_name = method.name.as_str();
        // Create descriptor pool using optimized caching
        let pool = self
            .get_or_create_descriptor_pool(channel.clone(), input, output)
            .await?;

        // Get message descriptors
        let input_descriptor = pool.get_message_by_name(input).with_context(|| {
            format!(
                "Failed to get message descriptor for: {}",
                method.input_type
            )
        })?;

        let output_descriptor = pool
            .get_message_by_name(output)
            .with_context(|| format!("Failed to get message descriptor for: {}", output))?;

        if self.verbose {
            println!("Making gRPC call to {}/{}", service.name, method.name);
            println!("Input type: {}", method.input_type);
            println!("Output type: {}", method.output_type);
            println!("Request: {}", data);
        }

        let request_message = parse_request_message(data, input_descriptor)?;

        // Prepare client and request
        let mut client = prepare_grpc_client(channel).await?;
        let path_and_query = create_method_path(service_name, method_name)?;
        let request =
            self.create_grpc_request_with_headers(Bytes::from(request_message.encode_to_vec()))?;

        // Make unary call
        let response = client
            .unary(request, path_and_query, BytesCodec)
            .await
            .context("gRPC call failed")?;

        let response_bytes: Bytes = response.into_inner();

        // Decode response using utility
        decode_response_message(response_bytes.as_ref(), output_descriptor)
    }

    async fn handle_server_streaming(
        &self,
        channel: Channel,
        service: &ServiceDescriptor,
        method: &MethodDescriptor,
        data: Value,
    ) -> Result<Vec<Value>> {
        let input = &method.input_type;
        let output = &method.output_type;
        let service_name = service.name.as_str();
        let method_name = method.name.as_str();
        // Create descriptor pool using utility
        let pool = self
            .create_descriptor_pool(channel.clone(), input, output)
            .await?;

        // Get message descriptors
        let input_desc = pool
            .get_message_by_name(input)
            .with_context(|| format!("Failed to get message descriptor for: {}", input))?;

        let output_desc = pool
            .get_message_by_name(output)
            .with_context(|| format!("Failed to get message descriptor for: {}", output))?;

        if self.verbose {
            println!(
                "Making server streaming gRPC call to {}/{}",
                service_name, method_name
            );
            println!("Input type: {}", input);
            println!("Output type: {}", output);
            println!("Request: {:?}", data);
        }

        let request_message = parse_request_message(data, input_desc)?;

        // Prepare client and request
        let mut client = prepare_grpc_client(channel).await?;
        let path_and_query = create_method_path(service_name, method_name)?;
        let request =
            self.create_grpc_request_with_headers(Bytes::from(request_message.encode_to_vec()))?;

        // Make the server streaming call
        let response = client
            .server_streaming(request, path_and_query, BytesCodec)
            .await
            .context("Server streaming gRPC call failed")?;

        let responses = process_response_stream(response, output_desc, self.verbose).await?;

        Ok(responses)
    }

    async fn handle_client_streaming(
        &self,
        channel: Channel,
        service: &ServiceDescriptor,
        method: &MethodDescriptor,
        data: Value,
    ) -> Result<Vec<Value>> {
        let input = &method.input_type;
        let output = &method.output_type;
        let service_name = service.name.as_str();
        let method_name = method.name.as_str();
        // Create descriptor pool using utility
        let pool = self
            .create_descriptor_pool(channel.clone(), input, output)
            .await?;

        // Get message descriptors
        let input_descriptor = pool
            .get_message_by_name(input)
            .with_context(|| format!("Failed to get message descriptor for: {}", input))?;

        let output_descriptor = pool
            .get_message_by_name(output)
            .with_context(|| format!("Failed to get message descriptor for: {}", output))?;

        let mut client = prepare_grpc_client(channel).await?;
        let path_and_query = create_method_path(service_name, method_name)?;

        let request_stream = create_request_stream(data, input_descriptor)?;
        let request = self.create_grpc_request_with_headers(request_stream)?;

        let response = client
            .client_streaming(request, path_and_query, BytesCodec)
            .await
            .map_err(|e| handle_stream_error(&e.into(), "Client streaming", self.verbose))?;

        let response_bytes = response.into_inner();

        let ret = decode_response_message(response_bytes.as_ref(), output_descriptor)
            .map_err(|e| handle_stream_error(&e, "Response decoding", self.verbose))?;
        Ok(vec![ret])
    }

    async fn handle_bidi_streaming(
        &self,
        channel: Channel,
        service: &ServiceDescriptor,
        method: &MethodDescriptor,
        data: Value,
    ) -> Result<Vec<Value>> {
        let input = &method.input_type;
        let output = &method.output_type;
        let service_name = service.name.as_str();
        let method_name = method.name.as_str();

        let pool = self
            .create_descriptor_pool(channel.clone(), input, output)
            .await?;

        // Get message descriptors
        let input_descriptor = pool
            .get_message_by_name(input)
            .with_context(|| format!("Failed to get message descriptor for: {}", input))?;

        let output_descriptor = pool
            .get_message_by_name(output)
            .with_context(|| format!("Failed to get message descriptor for: {}", output))?;

        // Prepare client and request
        let mut client = prepare_grpc_client(channel).await?;
        let path_and_query = create_method_path(service_name, method_name)?;

        let request_stream = create_request_stream(data, input_descriptor)?;
        let request = self.create_grpc_request_with_headers(request_stream)?;

        // Make bidirectional streaming call
        let response_stream = client
            .streaming(request, path_and_query, BytesCodec)
            .await
            .context("Bidirectional streaming call failed")?;

        let responses =
            process_response_stream(response_stream, output_descriptor, self.verbose).await?;

        Ok(responses)
    }

    // Utility methods to reduce code duplication

    async fn create_descriptor_pool(
        &self,
        channel: Channel,
        input_type: &str,
        output_type: &str,
    ) -> Result<DescriptorPool> {
        let mut reflection_client = create_reflection_client(channel);

        // Get file descriptor for the input type
        let input_file_desc = reflection_client
            .get_file_containing_symbol(input_type)
            .await
            .with_context(|| {
                format!(
                    "Failed to get file descriptor for input type: {}",
                    input_type
                )
            })?;

        // Get file descriptor for the output type
        let output_file_desc = reflection_client
            .get_file_containing_symbol(output_type)
            .await
            .with_context(|| {
                format!(
                    "Failed to get file descriptor for output type: {}",
                    output_type
                )
            })?;

        // Create descriptor pool from file descriptors
        let mut descriptor_set = FileDescriptorSet {
            file: vec![input_file_desc.clone()],
        };

        // Add output file descriptor if different
        if input_file_desc != output_file_desc {
            descriptor_set.file.push(output_file_desc);
        }

        DescriptorPool::from_file_descriptor_set(descriptor_set)
            .context("Failed to create descriptor pool")
    }

    fn create_grpc_request_with_headers<T>(&self, body: T) -> Result<Request<T>> {
        let mut request = Request::new(body);

        // Add headers to the request
        for (key, value) in &self.headers {
            let key = MetadataKey::from_bytes(key.as_bytes())
                .with_context(|| format!("Invalid header key: {}", key))?;
            request
                .metadata_mut()
                .insert(key, value.parse().context("Invalid header value")?);
        }

        Ok(request)
    }

    pub async fn get_or_create_channel(&self) -> Result<Channel> {
        let cache_key = format!("{}:{}", self.endpoint.host, self.endpoint.port);

        // Try to get cached connection first
        if let Some(channel) = self.cache.get_connection(&cache_key) {
            if self.verbose {
                println!("🚀 Using cached connection to {}", cache_key);
            }
            return Ok(channel);
        }

        // Create new connection if not cached
        if self.verbose {
            println!("🔗 Creating new connection to {}", cache_key);
        }

        let channel = create_channel(&self.endpoint, self.plaintext, &self.ca_cert_path)
            .await
            .context("Failed to create gRPC channel")?;

        // Cache the connection for reuse
        self.cache.store_connection(cache_key, channel.clone());

        Ok(channel)
    }

    pub async fn get_or_create_descriptor_pool(
        &self,
        channel: Channel,
        input_type: &str,
        output_type: &str,
    ) -> Result<DescriptorPool> {
        let cache_key = format!("{}:{}:{}", self.endpoint.host, input_type, output_type);

        if let Some(pool) = self.cache.get_descriptor_pool(&cache_key) {
            if self.verbose {
                println!("📋 Using cached descriptor pool for {}", cache_key);
            }
            return Ok(pool);
        }

        if self.verbose {
            println!("📝 Creating new descriptor pool for {}", cache_key);
        }

        let pool = self
            .create_descriptor_pool(channel, input_type, output_type)
            .await?;

        self.cache.store_descriptor_pool(cache_key, pool.clone());

        Ok(pool)
    }
}

fn create_request_stream(
    data: Value,
    input_descriptor: prost_reflect::MessageDescriptor,
) -> Result<stream::Iter<impl Iterator<Item = Bytes>>> {
    let items = match data {
        Value::Array(arr) => arr,
        Value::Object(map) => vec![Value::Object(map)],
        _ => bail!("Invalid request data: {}", data),
    };

    let messages = items
        .into_iter()
        .map(|item| parse_request_message(item, input_descriptor.clone()))
        .collect::<Result<Vec<DynamicMessage>>>()?;
    let messages = messages
        .into_iter()
        .map(|msg| Bytes::from(msg.encode_to_vec()));

    Ok(stream::iter(messages))
}

async fn process_response_stream(
    response: Response<Streaming<Bytes>>,
    output_descriptor: prost_reflect::MessageDescriptor,
    verbose: bool,
) -> Result<Vec<Value>> {
    let mut stream = response.into_inner();

    // Process each response in the stream with enhanced error handling
    let mut total_bytes_processed = 0;
    let mut responses = Vec::new();

    while let Some(response_bytes) = stream
        .try_next()
        .await
        .map_err(|e| handle_stream_error(&e.into(), "Server streaming", verbose))?
    {
        // Monitor memory usage
        total_bytes_processed += response_bytes.len();
        monitor_stream_memory_usage(total_bytes_processed, verbose)?;

        // Decode response using utility
        let ret = decode_response_message(response_bytes.as_ref(), output_descriptor.clone())
            .map_err(|e| handle_stream_error(&e, "Response decoding", verbose))?;

        responses.push(ret);
    }

    Ok(responses)
}

fn parse_method(method: &str) -> Result<(&str, &str)> {
    // Check for slash first (service/method format)
    if let Some(pos) = method.rfind('/') {
        Ok((&method[..pos], &method[pos + 1..]))
    } else if let Some(pos) = method.rfind('.') {
        Ok((&method[..pos], &method[pos + 1..]))
    } else {
        anyhow::bail!(
            "Invalid method format: {}. Expected 'service.method' or 'service/method'",
            method
        )
    }
}

fn decode_response_message(buf: &[u8], desc: prost_reflect::MessageDescriptor) -> Result<Value> {
    let msg = DynamicMessage::decode(desc, buf).context("Failed to decode response message")?;
    let data = serde_json::to_value(&msg).context("Failed to convert response to JSON")?;
    Ok(data)
}

fn create_method_path(service: &str, method: &str) -> Result<PathAndQuery> {
    let method_path = format!("/{}/{}", service, method);
    PathAndQuery::try_from(method_path).context("Invalid method path")
}

async fn prepare_grpc_client(channel: Channel) -> Result<Grpc<Channel>> {
    let mut client = Grpc::new(channel);
    client
        .ready()
        .await
        .context("Failed to prepare gRPC client")?;
    Ok(client)
}

fn parse_request_message(
    value: Value,
    input_desc: prost_reflect::MessageDescriptor,
) -> Result<DynamicMessage> {
    DynamicMessage::deserialize(input_desc, value).context("Failed to deserialize request message")
}

// Performance optimization: Connection and descriptor pool caching
#[derive(Debug, Clone, Default)]
pub struct PerformanceCache {
    connections: Arc<DashMap<String, Channel>>,
    descriptor_pools: Arc<DashMap<String, DescriptorPool>>,
}

impl PerformanceCache {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            descriptor_pools: Arc::new(DashMap::new()),
        }
    }

    fn get_connection(&self, key: &str) -> Option<Channel> {
        self.connections.get(key).map(|r| r.value().clone())
    }

    fn store_connection(&self, key: String, channel: Channel) {
        self.connections.insert(key, channel);
    }

    fn get_descriptor_pool(&self, key: &str) -> Option<DescriptorPool> {
        self.descriptor_pools.get(key).map(|r| r.value().clone())
    }

    fn store_descriptor_pool(&self, key: String, pool: DescriptorPool) {
        self.descriptor_pools.insert(key, pool);
    }
}

fn handle_stream_error(
    error: &anyhow::Error,
    operation_name: &str,
    verbose: bool,
) -> anyhow::Error {
    if verbose {
        println!("❌ {} error: {}", operation_name, error);
    }

    if let Some(status) = error.downcast_ref::<Status>() {
        match status.code() {
            Code::Unavailable => {
                anyhow!(
                    "{} failed: Server unavailable. Please check the server is running and accessible.",
                    operation_name
                )
            }
            Code::DeadlineExceeded => {
                anyhow!(
                    "{} failed: Request timed out. The server may be overloaded.",
                    operation_name
                )
            }
            Code::ResourceExhausted => {
                anyhow!(
                    "{} failed: Server resource exhausted. Try reducing request size or frequency.",
                    operation_name
                )
            }
            Code::PermissionDenied => {
                anyhow!(
                    "{} failed: Permission denied. Check authentication credentials.",
                    operation_name
                )
            }
            Code::Unauthenticated => {
                anyhow!(
                    "{} failed: Authentication required. Provide valid credentials.",
                    operation_name
                )
            }
            Code::NotFound => {
                anyhow!(
                    "{} failed: Service or method not found. Verify the service and method names.",
                    operation_name
                )
            }
            _ => {
                anyhow!(
                    "{} failed: gRPC error ({}): {}",
                    operation_name,
                    status.code(),
                    status.message()
                )
            }
        }
    } else {
        anyhow!("{} failed: {}", operation_name, error)
    }
}

fn monitor_stream_memory_usage(bytes_processed: usize, verbose: bool) -> Result<()> {
    const MEMORY_WARNING_THRESHOLD: usize = 100 * 1024 * 1024; // 100MB
    const MEMORY_ERROR_THRESHOLD: usize = 500 * 1024 * 1024; // 500MB

    if bytes_processed > MEMORY_ERROR_THRESHOLD {
        return Err(anyhow!(
            "Stream processing exceeded memory limit ({} MB). Stream terminated to prevent OOM.",
            bytes_processed / (1024 * 1024)
        ));
    }

    if bytes_processed > MEMORY_WARNING_THRESHOLD && verbose {
        println!(
            "⚠️  Stream has processed {} MB of data. Consider using --format text for large streams.",
            bytes_processed / (1024 * 1024)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    #[test]
    fn test_rfind_behavior() {
        let test_string = "test.Service.GetUser";
        if let Some(pos) = test_string.rfind('.') {
            let service = &test_string[..pos];
            let method = &test_string[pos + 1..];
            println!("rfind test: service='{}', method='{}'", service, method);
            assert_eq!(service, "test.Service");
            assert_eq!(method, "GetUser");
        } else {
            panic!("rfind should find a dot");
        }
    }

    #[test]
    fn test_parse_method() {
        // Test fully qualified service.method format
        let (service, method) = parse_method("test.Service.GetUser").unwrap();
        assert_eq!(service, "test.Service");
        assert_eq!(method, "GetUser");

        let (service, method) = parse_method("test.Service/GetUser").unwrap();
        assert_eq!(service, "test.Service");
        assert_eq!(method, "GetUser");

        // Test simple service.method format
        let (service, method) = parse_method("UserService.GetUser").unwrap();
        assert_eq!(service, "UserService");
        assert_eq!(method, "GetUser");

        assert!(parse_method("InvalidMethod").is_err());
    }

    #[test]
    fn test_parse_method_edge_cases() {
        // Test multiple dots - should use the last one
        let (service, method) = parse_method("com.example.Service.GetUser").unwrap();
        assert_eq!(service, "com.example.Service");
        assert_eq!(method, "GetUser");

        // Test multiple slashes - should use the last one
        let (service, method) = parse_method("com/example/Service/GetUser").unwrap();
        assert_eq!(service, "com/example/Service");
        assert_eq!(method, "GetUser");

        // Test edge cases with empty parts - these actually succeed due to rfind behavior
        let (service, method) = parse_method(".GetUser").unwrap();
        assert_eq!(service, "");
        assert_eq!(method, "GetUser");

        let (service, method) = parse_method("Service.").unwrap();
        assert_eq!(service, "Service");
        assert_eq!(method, "");

        let (service, method) = parse_method("/GetUser").unwrap();
        assert_eq!(service, "");
        assert_eq!(method, "GetUser");

        let (service, method) = parse_method("Service/").unwrap();
        assert_eq!(service, "Service");
        assert_eq!(method, "");
    }

    #[test]
    fn test_header_parsing() {
        let cli = Cli::parse_from([
            "grpc-client",
            "-H",
            "Authorization: Bearer token123",
            "-H",
            "X-Custom: value",
            "list",
            "localhost:9090",
        ]);

        let client = GrpcClient::from_cli(&cli).unwrap();
        assert_eq!(client.headers.len(), 2);
        assert_eq!(
            client.headers[0],
            ("Authorization".to_string(), "Bearer token123".to_string())
        );
        assert_eq!(
            client.headers[1],
            ("X-Custom".to_string(), "value".to_string())
        );
    }

    #[test]
    fn test_header_parsing_edge_cases() {
        // Test header with multiple colons
        let cli = Cli::parse_from([
            "grpc-client",
            "-H",
            "X-Data: key:value:more",
            "list",
            "localhost:9090",
        ]);

        let client = GrpcClient::from_cli(&cli).unwrap();
        assert_eq!(client.headers.len(), 1);
        assert_eq!(
            client.headers[0],
            ("X-Data".to_string(), "key:value:more".to_string())
        );

        // Test invalid header format
        let cli = Cli::parse_from([
            "grpc-client",
            "-H",
            "InvalidHeader",
            "list",
            "localhost:9090",
        ]);

        assert!(GrpcClient::from_cli(&cli).is_err());
    }

    #[test]
    fn test_create_method_path() {
        let path = create_method_path("UserService", "GetUser").unwrap();
        assert_eq!(path.as_str(), "/UserService/GetUser");

        let path = create_method_path("com.example.Service", "ListUsers").unwrap();
        assert_eq!(path.as_str(), "/com.example.Service/ListUsers");
    }

    #[test]
    fn test_create_grpc_request_with_headers() {
        let client = create_test_client_with_headers();

        let request = client
            .create_grpc_request_with_headers("test_body".to_string())
            .unwrap();
        assert_eq!(*request.get_ref(), "test_body");

        // Check that headers are added
        let metadata = request.metadata();
        assert!(metadata.len() >= 2); // At least our test headers
    }

    #[test]
    fn test_create_grpc_request_with_invalid_header() {
        let mut client = create_test_client();
        client
            .headers
            .push(("invalid\nheader".to_string(), "value".to_string()));

        let result = client.create_grpc_request_with_headers("test_body".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid header key")
        );
    }

    fn create_test_client() -> GrpcClient {
        let cli = Cli::parse_from(["grpc-client", "list", "localhost:9090"]);
        GrpcClient::from_cli(&cli).unwrap()
    }

    fn create_test_client_with_headers() -> GrpcClient {
        let cli = Cli::parse_from([
            "grpc-client",
            "-H",
            "Authorization: Bearer token",
            "-H",
            "X-Custom: value",
            "list",
            "localhost:9090",
        ]);
        GrpcClient::from_cli(&cli).unwrap()
    }

    #[tokio::test]
    async fn test_memory_monitoring() {
        // Test normal memory usage (should pass)
        let result = monitor_stream_memory_usage(1024, true);
        assert!(result.is_ok());

        // Test warning threshold (should pass but might print warning in verbose mode)
        let result = monitor_stream_memory_usage(150 * 1024 * 1024, true);
        assert!(result.is_ok());

        // Test error threshold (should fail)
        let result = monitor_stream_memory_usage(600 * 1024 * 1024, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("memory limit"));
    }

    #[test]
    fn test_stream_error_handling() {
        // Test gRPC status code error handling
        let status_error = anyhow::Error::from(Status::unavailable("service down"));
        let handled_error = handle_stream_error(&status_error, "Test operation", false);
        assert!(handled_error.to_string().contains("Server unavailable"));

        let timeout_error = anyhow::Error::from(Status::deadline_exceeded("timeout"));
        let handled_error = handle_stream_error(&timeout_error, "Test operation", false);
        assert!(handled_error.to_string().contains("Request timed out"));

        let auth_error = anyhow::Error::from(Status::unauthenticated("no token"));
        let handled_error = handle_stream_error(&auth_error, "Test operation", false);
        assert!(
            handled_error
                .to_string()
                .contains("Authentication required")
        );

        let not_found_error = anyhow::Error::from(Status::not_found("missing"));
        let handled_error = handle_stream_error(&not_found_error, "Test operation", false);
        assert!(
            handled_error
                .to_string()
                .contains("Service or method not found")
        );

        // Test generic error handling
        let generic_error = anyhow!("Something went wrong");
        let handled_error = handle_stream_error(&generic_error, "Test operation", false);
        assert!(handled_error.to_string().contains("Test operation failed"));
    }

    #[tokio::test]
    async fn test_performance_cache_connection() {
        let cache = PerformanceCache::new();
        let key = "localhost:9090".to_string();

        // Initially no connection cached
        assert!(cache.get_connection(&key).is_none());

        // Create a mock channel (we can't create a real one without a server)
        // For testing, we'll just verify the cache operations work
        // In a real scenario, this would be a proper Channel

        // Test that cache operations don't panic
        let result = cache.get_connection(&key);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_performance_cache_descriptor_pool() {
        let cache = PerformanceCache::new();
        let key = "localhost:9090:input:output".to_string();

        // Initially no descriptor pool cached
        assert!(cache.get_descriptor_pool(&key).is_none());

        // Test that cache operations don't panic
        let result = cache.get_descriptor_pool(&key);
        assert!(result.is_none());
    }
}
