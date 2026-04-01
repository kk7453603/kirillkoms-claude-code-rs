use serde_json::Value;

/// Parse JSON leniently: trims whitespace and attempts standard parsing.
/// Also handles trailing commas in objects/arrays by stripping them first.
pub fn parse_json_lenient(s: &str) -> Result<Value, serde_json::Error> {
    let trimmed = s.trim();
    // First try standard parsing
    match serde_json::from_str(trimmed) {
        Ok(v) => Ok(v),
        Err(e) => {
            // Try removing trailing commas before } and ]
            let cleaned = remove_trailing_commas(trimmed);
            serde_json::from_str(&cleaned).or(Err(e))
        }
    }
}

fn remove_trailing_commas(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escape_next = false;
    let chars: Vec<char> = s.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];
        if escape_next {
            escape_next = false;
            result.push(c);
            continue;
        }
        if c == '\\' && in_string {
            escape_next = true;
            result.push(c);
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            result.push(c);
            continue;
        }
        if !in_string && c == ',' {
            // Look ahead for ] or } (skipping whitespace)
            let rest = &chars[i + 1..];
            let next_non_ws = rest.iter().find(|ch| !ch.is_whitespace());
            if next_non_ws == Some(&'}') || next_non_ws == Some(&']') {
                // Skip this trailing comma
                continue;
            }
        }
        result.push(c);
    }

    result
}

/// Pretty-print a JSON value with 2-space indentation.
pub fn pretty_print_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Truncate a JSON value at a maximum nesting depth.
/// Objects/arrays deeper than `max_depth` are replaced with a string placeholder.
pub fn truncate_json(value: &Value, max_depth: usize) -> Value {
    truncate_recursive(value, 0, max_depth)
}

fn truncate_recursive(value: &Value, current_depth: usize, max_depth: usize) -> Value {
    if current_depth >= max_depth {
        match value {
            Value::Object(map) => {
                Value::String(format!("{{...{} keys}}", map.len()))
            }
            Value::Array(arr) => {
                Value::String(format!("[...{} items]", arr.len()))
            }
            other => other.clone(),
        }
    } else {
        match value {
            Value::Object(map) => {
                let truncated: serde_json::Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), truncate_recursive(v, current_depth + 1, max_depth)))
                    .collect();
                Value::Object(truncated)
            }
            Value::Array(arr) => {
                let truncated: Vec<Value> = arr
                    .iter()
                    .map(|v| truncate_recursive(v, current_depth + 1, max_depth))
                    .collect();
                Value::Array(truncated)
            }
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_lenient_standard() {
        let v = parse_json_lenient(r#"{"key": "value"}"#).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn parse_json_lenient_trailing_comma() {
        let v = parse_json_lenient(r#"{"key": "value",}"#).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn parse_json_lenient_whitespace() {
        let v = parse_json_lenient("  \n  42  \n  ").unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn pretty_print_json_basic() {
        let v = serde_json::json!({"a": 1});
        let pretty = pretty_print_json(&v);
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("\"a\""));
    }

    #[test]
    fn truncate_json_depth() {
        let v = serde_json::json!({
            "a": {
                "b": {
                    "c": 1
                }
            }
        });
        let truncated = truncate_json(&v, 1);
        // At depth 1, the inner object should be replaced
        assert!(truncated["a"].is_string());
        assert!(truncated["a"].as_str().unwrap().contains("..."));
    }
}
