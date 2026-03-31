use serde_json::{json, Value};

/// Validate tool input against JSON schema.
///
/// Returns Ok(()) if valid, Err with a list of error messages otherwise.
pub fn validate_input(input: &Value, schema: &Value) -> Result<(), Vec<String>> {
    // Check required fields
    let mut errors = Vec::new();

    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for req in required {
            if let Some(field_name) = req.as_str() {
                if input.get(field_name).is_none() {
                    errors.push(format!("Missing required field: '{}'", field_name));
                }
            }
        }
    }

    // Validate property types
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_schema) in properties {
            if let Some(value) = input.get(prop_name) {
                if let Some(prop_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                    let type_ok = match prop_type {
                        "string" => value.is_string(),
                        "number" | "integer" => value.is_number(),
                        "boolean" => value.is_boolean(),
                        "array" => value.is_array(),
                        "object" => value.is_object(),
                        "null" => value.is_null(),
                        _ => true,
                    };
                    if !type_ok {
                        errors.push(format!(
                            "Field '{}' expected type '{}', got '{}'",
                            prop_name,
                            prop_type,
                            value_type_name(value)
                        ));
                    }
                }

                // Validate enum values
                if let Some(enum_values) = prop_schema.get("enum").and_then(|e| e.as_array()) {
                    if !enum_values.contains(value) {
                        errors.push(format!(
                            "Field '{}' must be one of: {:?}",
                            prop_name,
                            enum_values
                                .iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                        ));
                    }
                }
            }
        }
    }

    // Check additionalProperties if set to false
    if schema
        .get("additionalProperties")
        .and_then(|v| v.as_bool())
        == Some(false)
    {
        if let Some(input_obj) = input.as_object() {
            if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                for key in input_obj.keys() {
                    if !properties.contains_key(key) {
                        errors.push(format!("Unknown property: '{}'", key));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Build a string parameter schema.
pub fn string_param(name: &str, description: &str, required: bool) -> Value {
    let _ = required; // required is handled at the object level
    json!({
        "name": name,
        "type": "string",
        "description": description,
    })
}

/// Build a number parameter schema.
pub fn number_param(name: &str, description: &str, required: bool) -> Value {
    let _ = required;
    json!({
        "name": name,
        "type": "number",
        "description": description,
    })
}

/// Build a boolean parameter schema.
pub fn bool_param(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "type": "boolean",
        "description": description,
    })
}

/// Build an array parameter schema.
pub fn array_param(name: &str, description: &str, item_schema: Value) -> Value {
    json!({
        "name": name,
        "type": "array",
        "description": description,
        "items": item_schema,
    })
}

/// Build a JSON Schema object with the given properties and required fields.
pub fn object_schema(properties: Vec<(&str, Value)>, required: Vec<&str>) -> Value {
    let mut props = serde_json::Map::new();
    for (name, schema) in properties {
        props.insert(name.to_string(), schema);
    }
    json!({
        "type": "object",
        "properties": props,
        "required": required,
    })
}

/// Build an enum (string with allowed values) parameter schema.
pub fn enum_param(name: &str, description: &str, values: Vec<&str>) -> Value {
    json!({
        "name": name,
        "type": "string",
        "description": description,
        "enum": values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_param() {
        let param = string_param("command", "The command to run", true);
        assert_eq!(param["type"], "string");
        assert_eq!(param["name"], "command");
        assert_eq!(param["description"], "The command to run");
    }

    #[test]
    fn test_number_param() {
        let param = number_param("timeout", "Timeout in ms", false);
        assert_eq!(param["type"], "number");
    }

    #[test]
    fn test_bool_param() {
        let param = bool_param("verbose", "Enable verbose output");
        assert_eq!(param["type"], "boolean");
    }

    #[test]
    fn test_array_param() {
        let item = json!({"type": "string"});
        let param = array_param("items", "List of items", item);
        assert_eq!(param["type"], "array");
        assert_eq!(param["items"]["type"], "string");
    }

    #[test]
    fn test_enum_param() {
        let param = enum_param("mode", "Operating mode", vec!["fast", "slow"]);
        assert_eq!(param["type"], "string");
        let enum_vals = param["enum"].as_array().unwrap();
        assert_eq!(enum_vals.len(), 2);
    }

    #[test]
    fn test_object_schema() {
        let schema = object_schema(
            vec![
                ("name", json!({"type": "string"})),
                ("age", json!({"type": "number"})),
            ],
            vec!["name"],
        );
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert!(schema["properties"]["age"].is_object());
        let req = schema["required"].as_array().unwrap();
        assert_eq!(req.len(), 1);
        assert_eq!(req[0], "name");
    }

    #[test]
    fn test_validate_input_valid() {
        let schema = object_schema(
            vec![
                ("name", json!({"type": "string"})),
                ("age", json!({"type": "number"})),
            ],
            vec!["name"],
        );
        let input = json!({"name": "Alice", "age": 30});
        assert!(validate_input(&input, &schema).is_ok());
    }

    #[test]
    fn test_validate_input_missing_required() {
        let schema = object_schema(
            vec![("name", json!({"type": "string"}))],
            vec!["name"],
        );
        let input = json!({});
        let err = validate_input(&input, &schema).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("Missing required field"));
    }

    #[test]
    fn test_validate_input_wrong_type() {
        let schema = object_schema(
            vec![("name", json!({"type": "string"}))],
            vec!["name"],
        );
        let input = json!({"name": 42});
        let err = validate_input(&input, &schema).unwrap_err();
        assert!(err[0].contains("expected type 'string'"));
    }

    #[test]
    fn test_validate_input_enum() {
        let schema = object_schema(
            vec![(
                "mode",
                json!({"type": "string", "enum": ["fast", "slow"]}),
            )],
            vec!["mode"],
        );
        let valid = json!({"mode": "fast"});
        assert!(validate_input(&valid, &schema).is_ok());

        let invalid = json!({"mode": "medium"});
        let err = validate_input(&invalid, &schema).unwrap_err();
        assert!(err[0].contains("must be one of"));
    }

    #[test]
    fn test_validate_additional_properties_false() {
        let mut schema = object_schema(
            vec![("name", json!({"type": "string"}))],
            vec![],
        );
        schema["additionalProperties"] = json!(false);

        let input = json!({"name": "Alice", "extra": true});
        let err = validate_input(&input, &schema).unwrap_err();
        assert!(err[0].contains("Unknown property"));
    }

    #[test]
    fn test_validate_optional_field_absent() {
        let schema = object_schema(
            vec![
                ("name", json!({"type": "string"})),
                ("age", json!({"type": "number"})),
            ],
            vec!["name"],
        );
        let input = json!({"name": "Alice"});
        assert!(validate_input(&input, &schema).is_ok());
    }
}
