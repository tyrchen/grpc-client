![](https://github.com/tyrchen/rust-lib-template/workflows/build/badge.svg)

# grpc-client

A modern gRPC command-line client written in Rust.

## Overview

`grpc-client` is a clean and efficient gRPC CLI tool that focuses on the most common use cases with a streamlined, maintainable architecture. It leverages Rust's type safety and performance while providing an intuitive command-line interface.

## Features

- **Service Discovery**: List available gRPC services using server reflection
- **Method Invocation**: Call gRPC methods with JSON request data
- **Multiple Formats**: Support for JSON and text output formats
- **Security**: TLS support with custom certificates
- **Streaming**: Support for all gRPC streaming types (planned)

## Installation

### Prerequisites

- Rust 1.70 or later

### From Source

```bash
git clone https://github.com/TODO/grpc-client.git
cd grpc-client
cargo build --release
```

The binary will be available at `target/release/grpc-client`.

## Usage

### List Services

```bash
# List all services
grpc-client list localhost:9090

# List methods for a specific service
grpc-client list localhost:9090 myservice.MyService
```

### Describe Services and Methods

```bash
# Describe a service
grpc-client describe localhost:9090 myservice.MyService

# Describe a specific method
grpc-client describe localhost:9090 myservice.MyService.GetUser
```

### Call Methods

```bash
# Call with JSON data
grpc-client call localhost:9090 myservice.MyService.GetUser -d '{"id": "123"}'

# Call with data from file
grpc-client call localhost:9090 myservice.MyService.GetUser -d @request.json

# Call with data from stdin
echo '{"id": "123"}' | grpc-client call localhost:9090 myservice.MyService.GetUser -d @-
```

### Options

- `--plaintext`: Use plain HTTP/2 instead of TLS
- `--format json|text`: Output format (default: json)
- `-H "name: value"`: Add custom headers
- `--verbose`: Verbose output
- `--emit-defaults`: Include default values in JSON output

## Examples

```bash
# Basic service listing
grpc-client list localhost:9090

# Call with authentication header
grpc-client call localhost:9090 auth.AuthService.Login \
  -H "Authorization: Bearer token123" \
  -d '{"username": "user", "password": "pass"}'

# Use plaintext connection
grpc-client --plaintext list localhost:8080
```

## Development Status

🚧 **This project is currently in active development.**

### Completed
- [x] Project setup and dependencies
- [x] CLI interface foundation
- [x] Core domain types
- [x] Basic connection management structure

### In Progress
- [ ] gRPC reflection client implementation
- [ ] List command functionality
- [ ] Describe command functionality
- [ ] Call command (unary RPC)

### Planned
- [ ] Streaming RPC support
- [ ] Advanced TLS configuration
- [ ] File-based schema sources
- [ ] Shell completion

## Architecture

The project follows a clean, modular architecture:

- **CLI Module**: Command-line argument parsing with clap
- **Domain Module**: Core types and business logic
- **Connection Module**: gRPC connection management
- **Reflection Module**: Server reflection client
- **Format Module**: Output formatting (JSON/text)

## Technology Stack

- **Rust 2021**: Modern, safe systems programming
- **tonic**: gRPC implementation for Rust
- **clap**: Command-line argument parsing
- **tokio**: Async runtime
- **anyhow**: Error handling
- **serde**: Serialization

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2025 Tyr Chen
