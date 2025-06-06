use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::domain::OutputFormat;

#[async_trait]
pub trait ResponseFormatter: Send + Sync {
    async fn format_response(&self, response: &Value) -> Result<String>;
    async fn format_error(&self, status: &tonic::Status) -> Result<String>;
}

/// Trait for formatting streaming responses in real-time
#[async_trait]
pub trait StreamingFormatter: Send + Sync {
    /// Format and display a single streaming response
    async fn format_stream_response(&self, response: &Value, sequence: usize) -> Result<String>;

    /// Display stream started message
    async fn format_stream_start(&self) -> Result<String>;

    /// Display stream completion with summary
    async fn format_stream_complete(&self, total_responses: usize) -> Result<String>;

    /// Display progress indication for long streams
    async fn format_stream_progress(
        &self,
        processed: usize,
        total: Option<usize>,
    ) -> Result<String>;

    /// Format streaming error
    async fn format_stream_error(&self, status: &tonic::Status, sequence: usize) -> Result<String>;
}

/// Progress information for streaming operations
#[derive(Debug, Clone)]
pub struct StreamProgress {
    pub processed: usize,
    pub total: Option<usize>,
    pub current_rate: Option<f64>, // responses per second
    pub elapsed_time: std::time::Duration,
}

pub struct JsonFormatter {
    pretty: bool,
}

impl JsonFormatter {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn from_output_format(format: &OutputFormat) -> Self {
        match format {
            OutputFormat::Json { pretty, .. } => Self::new(*pretty),
            OutputFormat::Text { .. } => Self::new(true),
        }
    }
}

#[async_trait]
impl ResponseFormatter for JsonFormatter {
    async fn format_response(&self, response: &Value) -> Result<String> {
        if self.pretty {
            Ok(serde_json::to_string_pretty(response)?)
        } else {
            Ok(serde_json::to_string(response)?)
        }
    }

    async fn format_error(&self, status: &tonic::Status) -> Result<String> {
        let error_info = serde_json::json!({
            "error": {
                "code": status.code() as i32,
                "message": status.message(),
                "details": format!("{:?}", status.details()),
            }
        });

        if self.pretty {
            Ok(serde_json::to_string_pretty(&error_info)?)
        } else {
            Ok(serde_json::to_string(&error_info)?)
        }
    }
}

#[async_trait]
impl StreamingFormatter for JsonFormatter {
    async fn format_stream_response(&self, response: &Value, _sequence: usize) -> Result<String> {
        // For JSON, each response is a separate JSON line
        if self.pretty {
            Ok(serde_json::to_string_pretty(response)?)
        } else {
            Ok(serde_json::to_string(response)?)
        }
    }

    async fn format_stream_start(&self) -> Result<String> {
        Ok(String::new()) // JSON streams don't need a start message
    }

    async fn format_stream_complete(&self, total_responses: usize) -> Result<String> {
        let completion_info = serde_json::json!({
            "stream_complete": true,
            "total_responses": total_responses
        });

        if self.pretty {
            Ok(format!(
                "\n{}",
                serde_json::to_string_pretty(&completion_info)?
            ))
        } else {
            Ok(serde_json::to_string(&completion_info)?)
        }
    }

    async fn format_stream_progress(
        &self,
        processed: usize,
        total: Option<usize>,
    ) -> Result<String> {
        let progress_info = serde_json::json!({
            "stream_progress": {
                "processed": processed,
                "total": total
            }
        });

        if self.pretty {
            Ok(format!(
                "\n{}",
                serde_json::to_string_pretty(&progress_info)?
            ))
        } else {
            Ok(serde_json::to_string(&progress_info)?)
        }
    }

    async fn format_stream_error(&self, status: &tonic::Status, sequence: usize) -> Result<String> {
        let error_info = serde_json::json!({
            "stream_error": {
                "sequence": sequence,
                "code": status.code() as i32,
                "message": status.message(),
                "details": format!("{:?}", status.details()),
            }
        });

        if self.pretty {
            Ok(serde_json::to_string_pretty(&error_info)?)
        } else {
            Ok(serde_json::to_string(&error_info)?)
        }
    }
}

pub struct TextFormatter {
    compact: bool,
}

impl TextFormatter {
    pub fn new(compact: bool) -> Self {
        Self { compact }
    }

    pub fn from_output_format(format: &OutputFormat) -> Self {
        match format {
            OutputFormat::Text { compact } => Self::new(*compact),
            OutputFormat::Json { .. } => Self::new(false),
        }
    }
}

#[async_trait]
impl ResponseFormatter for TextFormatter {
    async fn format_response(&self, response: &Value) -> Result<String> {
        if self.compact {
            Ok(serde_json::to_string(response)?)
        } else {
            Ok(serde_json::to_string_pretty(response)?)
        }
    }

    async fn format_error(&self, status: &tonic::Status) -> Result<String> {
        Ok(format!(
            "Error {}: {}",
            status.code() as i32,
            status.message()
        ))
    }
}

#[async_trait]
impl StreamingFormatter for TextFormatter {
    async fn format_stream_response(&self, response: &Value, sequence: usize) -> Result<String> {
        let response_str = if self.compact {
            serde_json::to_string(response)?
        } else {
            serde_json::to_string_pretty(response)?
        };

        Ok(format!("[{}] {}", sequence, response_str))
    }

    async fn format_stream_start(&self) -> Result<String> {
        Ok("Starting stream...".to_string())
    }

    async fn format_stream_complete(&self, total_responses: usize) -> Result<String> {
        Ok(format!(
            "Stream completed. Total responses: {}",
            total_responses
        ))
    }

    async fn format_stream_progress(
        &self,
        processed: usize,
        total: Option<usize>,
    ) -> Result<String> {
        match total {
            Some(total) => Ok(format!("Progress: {}/{} responses", processed, total)),
            None => Ok(format!("Progress: {} responses", processed)),
        }
    }

    async fn format_stream_error(&self, status: &tonic::Status, sequence: usize) -> Result<String> {
        Ok(format!(
            "Stream error at sequence {}: {} - {}",
            sequence,
            status.code() as i32,
            status.message()
        ))
    }
}

pub fn create_formatter(format: &OutputFormat) -> Box<dyn ResponseFormatter> {
    match format {
        OutputFormat::Json { .. } => Box::new(JsonFormatter::from_output_format(format)),
        OutputFormat::Text { .. } => Box::new(TextFormatter::from_output_format(format)),
    }
}

pub fn create_streaming_formatter(format: &OutputFormat) -> Box<dyn StreamingFormatter> {
    match format {
        OutputFormat::Json { .. } => Box::new(JsonFormatter::from_output_format(format)),
        OutputFormat::Text { .. } => Box::new(TextFormatter::from_output_format(format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_json_formatter() {
        let formatter = JsonFormatter::new(true);
        let response = json!({"name": "test", "value": 42});

        let formatted = formatter.format_response(&response).await.unwrap();
        assert!(formatted.contains("test"));
        assert!(formatted.contains("42"));
    }

    #[tokio::test]
    async fn test_text_formatter() {
        let formatter = TextFormatter::new(false);
        let response = json!({"name": "test", "value": 42});

        let formatted = formatter.format_response(&response).await.unwrap();
        assert!(formatted.contains("test"));
    }

    #[test]
    fn test_create_formatter() {
        let json_format = OutputFormat::Json {
            pretty: true,
            emit_defaults: false,
        };
        let _formatter = create_formatter(&json_format);

        let text_format = OutputFormat::Text { compact: false };
        let _formatter = create_formatter(&text_format);
    }
}
