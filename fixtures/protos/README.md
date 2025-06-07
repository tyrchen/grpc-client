# Protobuf Examples for Data Firewall

This directory contains example Protocol Buffer schema definitions used to demonstrate and test the gRPC and Protobuf support in Data Firewall (DFW).

## Files

- `example.proto`: A sample Protocol Buffer schema representing a user service with various data types, including sensitive data fields that can be processed by Data Firewall rules.

## Using the Examples

The example protobuf files can be used in conjunction with the Data Firewall configuration to demonstrate:

1. Protobuf message parsing and inspection
2. Field-level rule application to protobuf data
3. Redaction and masking of sensitive data in protobuf messages
4. gRPC support with protobuf message inspection

## Sample Configuration

See `fixtures/example_protobuf.yml` for a complete example configuration that uses these protobuf schemas. The configuration includes:

- Protobuf schema file references
- Include paths for protobuf compilation
- Rules specific to protobuf message fields
- Route configurations with protobuf message type specifications

## Running the Example

You can run the protobuf example with:

```bash
# You'll need to add the required dependencies first
cargo add protox prost prost-reflect --features="prost-reflect/serde"

# Then run the example
cargo run --example protobuf_example
```

## Benchmark

You can benchmark the protobuf handling with:

```bash
cargo bench -p dfw-benchmarks --bench proto_benchmarks
```
