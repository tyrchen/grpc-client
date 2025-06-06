use anyhow::{Context, Result};
use prost_reflect::{FieldDescriptor, Kind, MessageDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// JSON Schema representation for frontend form generation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchema {
    /// JSON Schema version
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Type of the schema (typically "object")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Human-readable title
    pub title: Option<String>,
    /// Schema description
    pub description: Option<String>,
    /// Object properties definition
    pub properties: Option<HashMap<String, JsonSchemaProperty>>,
    /// Required field names
    pub required: Option<Vec<String>>,
}

/// Property definition in JSON Schema
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaProperty {
    /// Property type (string, number, boolean, object, array)
    #[serde(rename = "type")]
    pub property_type: String,
    /// Human-readable title
    pub title: Option<String>,
    /// Property description
    pub description: Option<String>,
    /// Format specification (e.g., "date", "email")
    pub format: Option<String>,
    /// Array item type definition
    pub items: Option<Box<JsonSchemaProperty>>,
    /// Object property definitions
    pub properties: Option<HashMap<String, JsonSchemaProperty>>,
    /// Required property names
    pub required: Option<Vec<String>>,
    /// Enumeration values
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Minimum numeric value
    pub minimum: Option<f64>,
    /// Maximum numeric value
    pub maximum: Option<f64>,
    /// Minimum string length
    pub min_length: Option<usize>,
    /// Maximum string length
    pub max_length: Option<usize>,
    /// Regex pattern
    pub pattern: Option<String>,
    /// OneOf schema definitions
    pub one_of: Option<Vec<JsonSchemaProperty>>,
}

/// Schema processor for converting protobuf descriptors to JSON Schema
#[derive(Debug, Clone, Default)]
pub struct SchemaProcessor {
    /// Cache for processed message schemas
    schema_cache: HashMap<String, JsonSchema>,
}

impl SchemaProcessor {
    /// Create a new schema processor
    pub fn new() -> Self {
        Self {
            schema_cache: HashMap::new(),
        }
    }

    /// Generate JSON Schema from a protobuf message descriptor
    pub fn generate_schema(&mut self, descriptor: &MessageDescriptor) -> Result<JsonSchema> {
        let type_name = descriptor.full_name();

        // Check cache first
        if let Some(cached) = self.schema_cache.get(type_name) {
            return Ok(cached.clone());
        }

        let schema = self.message_to_schema(descriptor)?;

        // Cache the result
        self.schema_cache
            .insert(type_name.to_string(), schema.clone());

        Ok(schema)
    }

    /// Convert a protobuf message descriptor to JSON Schema
    fn message_to_schema(&self, descriptor: &MessageDescriptor) -> Result<JsonSchema> {
        let mut properties = HashMap::new();
        let mut required_fields = Vec::new();

        // Process each field in the message
        for field in descriptor.fields() {
            let field_name = field.name();
            let property = self.field_to_property(&field)?;

            properties.insert(field_name.to_string(), property);

            // Add to required if field is required (not optional or repeated)
            // Note: prost_reflect doesn't have is_optional, so we check if it's not a list or map
            if !field.is_list() && !field.is_map() {
                // For proto3, all fields are optional by default, so we'll be conservative
                // and only mark fields as required if they're clearly not optional
                required_fields.push(field_name.to_string());
            }
        }

        Ok(JsonSchema {
            schema: "https://json-schema.org/draft/2020-12/schema".to_string(),
            schema_type: "object".to_string(),
            title: Some(descriptor.name().to_string()),
            description: Some(format!("Schema for {} message", descriptor.full_name())),
            properties: Some(properties),
            required: if required_fields.is_empty() {
                None
            } else {
                Some(required_fields)
            },
        })
    }

    /// Convert a protobuf field descriptor to JSON Schema property
    fn field_to_property(&self, field: &FieldDescriptor) -> Result<JsonSchemaProperty> {
        let mut property = match field.kind() {
            Kind::Double | Kind::Float => JsonSchemaProperty {
                property_type: "number".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                format: Some("float".to_string()),
                ..Default::default()
            },
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => JsonSchemaProperty {
                property_type: "integer".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                format: Some("int32".to_string()),
                minimum: Some(i32::MIN as f64),
                maximum: Some(i32::MAX as f64),
                ..Default::default()
            },
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => JsonSchemaProperty {
                property_type: "integer".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                format: Some("int64".to_string()),
                ..Default::default()
            },
            Kind::Uint32 | Kind::Fixed32 => JsonSchemaProperty {
                property_type: "integer".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                format: Some("uint32".to_string()),
                minimum: Some(0.0),
                maximum: Some(u32::MAX as f64),
                ..Default::default()
            },
            Kind::Uint64 | Kind::Fixed64 => JsonSchemaProperty {
                property_type: "integer".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                format: Some("uint64".to_string()),
                minimum: Some(0.0),
                ..Default::default()
            },
            Kind::Bool => JsonSchemaProperty {
                property_type: "boolean".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                ..Default::default()
            },
            Kind::String => JsonSchemaProperty {
                property_type: "string".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {}", field.name())),
                ..Default::default()
            },
            Kind::Bytes => JsonSchemaProperty {
                property_type: "string".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Field: {} (base64 encoded)", field.name())),
                format: Some("byte".to_string()),
                ..Default::default()
            },
            Kind::Message(msg_desc) => {
                // Recursive message type
                JsonSchemaProperty {
                    property_type: "object".to_string(),
                    title: Some(field.name().to_string()),
                    description: Some(format!("Nested message: {}", msg_desc.full_name())),
                    properties: Some(self.message_to_properties(&msg_desc)?),
                    ..Default::default()
                }
            }
            Kind::Enum(enum_desc) => {
                // Enum type with possible values
                let enum_values: Vec<serde_json::Value> = enum_desc
                    .values()
                    .map(|v| serde_json::Value::String(v.name().to_string()))
                    .collect();

                JsonSchemaProperty {
                    property_type: "string".to_string(),
                    title: Some(field.name().to_string()),
                    description: Some(format!("Enum: {}", enum_desc.full_name())),
                    enum_values: Some(enum_values),
                    ..Default::default()
                }
            }
        };

        // Handle repeated fields (arrays)
        if field.is_list() {
            property = JsonSchemaProperty {
                property_type: "array".to_string(),
                title: Some(field.name().to_string()),
                description: Some(format!("Repeated field: {}", field.name())),
                items: Some(Box::new(property)),
                ..Default::default()
            };
        }

        // Handle map fields
        if field.is_map() {
            let map_entry = field.kind();
            if let Kind::Message(entry_desc) = map_entry {
                let value_field = entry_desc
                    .fields()
                    .find(|f| f.name() == "value")
                    .context("Map entry missing value field")?;

                let _value_property = self.field_to_property(&value_field)?;

                property = JsonSchemaProperty {
                    property_type: "object".to_string(),
                    title: Some(field.name().to_string()),
                    description: Some(format!("Map field: {}", field.name())),
                    properties: None, // Maps are open-ended
                    ..Default::default()
                };
            }
        }

        Ok(property)
    }

    /// Convert message descriptor to properties map (helper for nested messages)
    fn message_to_properties(
        &self,
        descriptor: &MessageDescriptor,
    ) -> Result<HashMap<String, JsonSchemaProperty>> {
        let mut properties = HashMap::new();

        for field in descriptor.fields() {
            let field_name = field.name();
            let property = self.field_to_property(&field)?;
            properties.insert(field_name.to_string(), property);
        }

        Ok(properties)
    }

    /// Generate validation rules from protobuf constraints (future extension)
    pub fn generate_validation_rules(
        &self,
        descriptor: &MessageDescriptor,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut rules = HashMap::new();

        // Basic validation rules based on protobuf field types
        for field in descriptor.fields() {
            let field_name = field.name();
            let mut field_rules = serde_json::Map::new();

            // Required field validation
            // Note: prost_reflect doesn't have is_optional, so we check if it's not a list
            if !field.is_list() {
                field_rules.insert("required".to_string(), serde_json::Value::Bool(true));
            }

            // Type-specific validation
            match field.kind() {
                Kind::String => {
                    // Add string length constraints if needed
                    field_rules.insert(
                        "type".to_string(),
                        serde_json::Value::String("string".to_string()),
                    );
                }
                Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                    field_rules.insert(
                        "type".to_string(),
                        serde_json::Value::String("integer".to_string()),
                    );
                    field_rules.insert(
                        "minimum".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(i32::MIN)),
                    );
                    field_rules.insert(
                        "maximum".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(i32::MAX)),
                    );
                }
                Kind::Uint32 | Kind::Fixed32 => {
                    field_rules.insert(
                        "type".to_string(),
                        serde_json::Value::String("integer".to_string()),
                    );
                    field_rules.insert(
                        "minimum".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(0)),
                    );
                    field_rules.insert(
                        "maximum".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(u32::MAX)),
                    );
                }
                _ => {}
            }

            if !field_rules.is_empty() {
                rules.insert(
                    field_name.to_string(),
                    serde_json::Value::Object(field_rules),
                );
            }
        }

        Ok(rules)
    }

    /// Clear the schema cache
    pub fn clear_cache(&mut self) {
        self.schema_cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "cached_schemas": self.schema_cache.len(),
            "schema_names": self.schema_cache.keys().collect::<Vec<_>>()
        })
    }
}

impl Default for JsonSchemaProperty {
    fn default() -> Self {
        Self {
            property_type: "string".to_string(),
            title: None,
            description: None,
            format: None,
            items: None,
            properties: None,
            required: None,
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            one_of: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_processor_creation() {
        let processor = SchemaProcessor::new();
        assert_eq!(processor.schema_cache.len(), 0);
    }

    #[test]
    fn test_json_schema_property_default() {
        let property = JsonSchemaProperty::default();
        assert_eq!(property.property_type, "string");
        assert!(property.title.is_none());
        assert!(property.description.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let processor = SchemaProcessor::new();
        let stats = processor.get_cache_stats();
        assert_eq!(stats["cached_schemas"], 0);
    }
}
