use std::{fs, io::Read as _};

use anyhow::{Context as _, Result};
use clap::Parser;
use grpc_client::{
    OutputFormat,
    cli::{Cli, Command},
    client::GrpcClient,
    reflection::{StreamingType, Symbol},
    server::start_server,
};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::List { service, .. } => {
            let client = GrpcClient::from_cli(&cli)?;

            match service {
                Some(name) => {
                    let methods = client.handle_method_list(name).await?;
                    if client.verbose {
                        println!("Methods for service '{}':", name);
                    }

                    for method in methods {
                        match &client.format {
                            OutputFormat::Json { pretty, .. } => {
                                let streaming_type_str = match method.streaming_type {
                                    StreamingType::Unary => "unary",
                                    StreamingType::ServerStream => "server_streaming",
                                    StreamingType::ClientStream => "client_streaming",
                                    StreamingType::BiDirectional => "bidirectional",
                                };

                                let json = if *pretty {
                                    serde_json::to_string_pretty(&serde_json::json!({
                                        "name": method.name.as_str(),
                                        "service": method.service.as_str(),
                                        "input_type": method.input_type,
                                        "output_type": method.output_type,
                                        "streaming_type": streaming_type_str,
                                        "client_streaming": method.client_streaming,
                                        "server_streaming": method.server_streaming
                                    }))?
                                } else {
                                    serde_json::to_string(&serde_json::json!({
                                        "name": method.name.as_str(),
                                        "service": method.service.as_str(),
                                        "input_type": method.input_type,
                                        "output_type": method.output_type,
                                        "streaming_type": streaming_type_str,
                                        "client_streaming": method.client_streaming,
                                        "server_streaming": method.server_streaming
                                    }))?
                                };
                                println!("{}", json);
                            }
                            OutputFormat::Text { .. } => {
                                let streaming_indicator = match method.streaming_type {
                                    StreamingType::Unary => "",
                                    StreamingType::ServerStream => " (server streaming)",
                                    StreamingType::ClientStream => " (client streaming)",
                                    StreamingType::BiDirectional => " (bidirectional)",
                                };
                                println!(
                                    "{}.{}{}",
                                    method.service.as_str(),
                                    method.name.as_str(),
                                    streaming_indicator
                                );
                            }
                        }
                    }
                }
                None => {
                    let services = client.handle_service_list().await?;
                    if client.verbose {
                        println!("Available services:");
                    }

                    for service in services {
                        match &client.format {
                            OutputFormat::Json { pretty, .. } => {
                                let json = if *pretty {
                                    serde_json::to_string_pretty(&serde_json::json!({
                                        "name": service.as_str()
                                    }))?
                                } else {
                                    serde_json::to_string(&serde_json::json!({
                                        "name": service.as_str()
                                    }))?
                                };
                                println!("{}", json);
                            }
                            OutputFormat::Text { .. } => {
                                println!("{}", service.as_str());
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        Command::Describe { symbol, .. } => {
            if cli.verbose {
                println!("Describing symbol: {}", symbol);
            }

            let client = GrpcClient::from_cli(&cli)?;

            let symbol = client.handle_describe(symbol).await?;

            match symbol {
                Symbol::Service(service_desc) => {
                    client.format_service_description(&service_desc).await?;
                }
                Symbol::Method(method_desc) => {
                    client.format_method_description(&method_desc).await?;
                }
                Symbol::Message(message_desc) => {
                    client.format_message_description(&message_desc).await?;
                }
            }
            Ok(())
        }
        Command::Call {
            method,
            data,
            emit_defaults,
            ..
        } => {
            let client = GrpcClient::from_cli(&cli)?;
            if cli.verbose {
                // println!("Calling method: {}.{}", service_name, method_name);
                println!("Endpoint: {}", client.endpoint);
            }
            let format = if let OutputFormat::Json { pretty, .. } = &client.format {
                OutputFormat::Json {
                    pretty: *pretty,
                    emit_defaults: *emit_defaults,
                }
            } else {
                client.format.clone()
            };
            let data = parse_request_data(data.as_deref())?;
            let ret = client.handle_call(method, data).await?;
            for response in ret {
                format_call_response(&response, &format)?;
            }

            Ok(())
        }
        Command::Server {
            port,
            config,
            ui_path,
        } => start_server(*port, config, ui_path).await,
    }
}

fn parse_request_data(data: Option<&str>) -> Result<Value> {
    let request_json = match data {
        Some("@-") => {
            // Read from stdin
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .context("Failed to read from stdin")?;
            input.trim().to_string()
        }
        Some(data) if data.starts_with('@') => {
            // Read from file
            let filename = &data[1..];
            fs::read_to_string(filename)
                .with_context(|| format!("Failed to read file: {}", filename))?
                .trim()
                .to_string()
        }
        Some(data) => {
            // Use provided data
            data.to_string()
        }
        None => {
            // Empty request
            "{}".to_string()
        }
    };

    let data = serde_json::from_str::<serde_json::Value>(&request_json)
        .with_context(|| format!("Invalid JSON in request: {}", request_json))?;

    Ok(data)
}

fn format_call_response(v: &Value, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json { pretty, .. } => {
            if *pretty {
                // Response is already pretty-printed
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("{}", serde_json::to_string(&v)?);
            }
        }
        OutputFormat::Text { .. } => {
            // Parse and display in text format
            println!("Response:");
            print_json_as_text(v, 0);
        }
    }

    Ok(())
}
fn print_json_as_text(value: &Value, indent: usize) {
    let indent_str = "  ".repeat(indent);

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match val {
                    Value::Object(_) | Value::Array(_) => {
                        println!("{}{}:", indent_str, key);
                        print_json_as_text(val, indent + 1);
                    }
                    _ => {
                        println!("{}{}: {}", indent_str, key, format_json_value(val));
                    }
                }
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                println!("{}[{}]:", indent_str, i);
                print_json_as_text(val, indent + 1);
            }
        }
        _ => {
            println!("{}{}", indent_str, format_json_value(value));
        }
    }
}

fn format_json_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_parse_request_data_json_string() {
        let result = parse_request_data(Some(r#"{"name": "test"}"#)).unwrap();
        assert_eq!(result, json!({"name": "test"}));
    }

    #[tokio::test]
    async fn test_parse_request_data_empty() {
        let result = parse_request_data(None).unwrap();
        assert_eq!(result, json!({}));
    }

    #[tokio::test]
    async fn test_parse_request_data_invalid_json() {
        let result = parse_request_data(Some(r#"{"invalid": json"#));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
    }

    #[test]
    fn test_format_json_value() {
        assert_eq!(
            format_json_value(&Value::String("test".to_string())),
            "test"
        );
        assert_eq!(
            format_json_value(&Value::Number(serde_json::Number::from(42))),
            "42"
        );
        assert_eq!(format_json_value(&Value::Bool(true)), "true");
        assert_eq!(format_json_value(&Value::Null), "null");
    }

    #[tokio::test]
    async fn test_format_call_response_json_pretty() {
        let format = OutputFormat::Json {
            pretty: true,
            emit_defaults: false,
        };

        // Capture output would require more complex testing setup,
        // so we'll just ensure it doesn't panic
        let result = format_call_response(&json!({"name": "test"}), &format);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_format_call_response_json_compact() {
        let format = OutputFormat::Json {
            pretty: false,
            emit_defaults: false,
        };
        let result = format_call_response(&json!({"name": "test"}), &format);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_format_call_response_text() {
        let format = OutputFormat::Text { compact: false };
        let result = format_call_response(&json!({"name": "test"}), &format);
        assert!(result.is_ok());
    }
}
