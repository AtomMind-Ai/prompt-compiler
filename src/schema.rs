use crate::types::{ValidationError, ValidationResult, RepairAction, ErrorType, RepairType};
use anyhow::Result;
use jsonschema::JSONSchema;
use serde_json::Value;

pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate JSON against a schema
    pub fn validate(json: &Value, schema: &Value) -> ValidationResult {
        let mut errors = Vec::new();
        let mut repairs = Vec::new();
        
        // Compile schema
        let compiled = match JSONSchema::compile(schema) {
            Ok(s) => s,
            Err(e) => {
                return ValidationResult {
                    is_valid: false,
                    errors: vec![ValidationError {
                        path: "schema".to_string(),
                        message: format!("Invalid schema: {}", e),
                        error_type: ErrorType::SchemaViolation,
                    }],
                    repairs: vec![],
                };
            }
        };
        
        // Validate
        let validation_result = compiled.validate(json);
        
        if let Err(validation_errors) = validation_result {
            for error in validation_errors {
                errors.push(ValidationError {
                    path: error.instance_path.to_string(),
                    message: error.to_string(),
                    error_type: Self::classify_error(&error),
                });
            }
        }
        
        // Attempt repairs
        let (repaired_json, repair_actions) = Self::attempt_repairs(json, &errors);
        repairs.extend(repair_actions);
        
        // Re-validate repaired JSON
        let is_valid = if !repairs.is_empty() {
            let revalidation = compiled.validate(&repaired_json);
            revalidation.is_ok()
        } else {
            errors.is_empty()
        };
        
        ValidationResult {
            is_valid,
            errors,
            repairs,
        }
    }

    fn classify_error(error: &jsonschema::ValidationError) -> ErrorType {
        let msg = error.to_string().to_lowercase();
        
        if msg.contains("required") || msg.contains("missing") {
            ErrorType::MissingKey
        } else if msg.contains("type") {
            ErrorType::WrongType
        } else if msg.contains("format") {
            ErrorType::InvalidFormat
        } else if msg.contains("array") || msg.contains("items") {
            ErrorType::TruncatedArray
        } else if msg.contains("string") {
            ErrorType::MalformedString
        } else {
            ErrorType::SchemaViolation
        }
    }

    fn attempt_repairs(json: &Value, errors: &[ValidationError]) -> (Value, Vec<RepairAction>) {
        let mut repaired = json.clone();
        let mut repairs = Vec::new();
        
        for error in errors {
            match error.error_type {
                ErrorType::MissingKey => {
                    if let Some(key) = Self::extract_key_from_path(&error.path) {
                        if let Some(obj) = repaired.as_object_mut() {
                            if !obj.contains_key(&key) {
                                obj.insert(key.clone(), Value::Null);
                                repairs.push(RepairAction {
                                    path: error.path.clone(),
                                    action: RepairType::InsertedDefault,
                                    description: format!("Inserted null for missing key: {}", key),
                                });
                            }
                        }
                    }
                }
                ErrorType::WrongType => {
                    // Try type conversion
                    if let Some(converted) = Self::attempt_type_conversion(repaired.pointer(&error.path)) {
                        if let Some(parent) = repaired.pointer_mut(&error.path) {
                            *parent = converted;
                            repairs.push(RepairAction {
                                path: error.path.clone(),
                                action: RepairType::ConvertedType,
                                description: "Attempted type conversion".to_string(),
                            });
                        }
                    }
                }
                ErrorType::MalformedString => {
                    if let Some(s) = repaired.pointer_mut(&error.path) {
                        if let Some(str_val) = s.as_str() {
                            let cleaned = Self::clean_string(str_val);
                            *s = Value::String(cleaned);
                            repairs.push(RepairAction {
                                path: error.path.clone(),
                                action: RepairType::FixedArray,
                                description: "Cleaned malformed string".to_string(),
                            });
                        }
                    }
                }
                ErrorType::TruncatedArray => {
                    if let Some(arr) = repaired.pointer_mut(&error.path) {
                        if let Some(arr_val) = arr.as_array_mut() {
                            // Remove null or invalid entries
                            arr_val.retain(|v| !v.is_null());
                            repairs.push(RepairAction {
                                path: error.path.clone(),
                                action: RepairType::RemovedInvalid,
                                description: "Removed invalid array entries".to_string(),
                            });
                        }
                    }
                }
                _ => {
                    // Other errors are not automatically repairable
                }
            }
        }
        
        (repaired, repairs)
    }

    fn extract_key_from_path(path: &str) -> Option<String> {
        // Extract the last component from JSON pointer path
        let parts: Vec<&str> =path.split('/').collect();
        if !parts.is_empty() {
            Some(parts.last()?.to_string())
        } else {
            None
        }
    }

    fn attempt_type_conversion(value: Option<&Value>) -> Option<Value> {
        match value? {
            Value::String(s) => {
                // Try to parse as number
                if let Ok(n) = s.parse::<i64>() {
                    return Some(Value::Number(n.into()));
                }
                if let Ok(n) = s.parse::<f64>() {
                    return Some(Value::Number(serde_json::Number::from_f64(n)?));
                }
                if let Ok(b) = s.parse::<bool>() {
                    return Some(Value::Bool(b));
                }
                None
            }
            Value::Number(n) => {
                Some(Value::String(n.to_string()))
            }
            Value::Bool(b) => Some(Value::String(b.to_string())),
            Value::Array(arr) => {
                if arr.len() == 1 {
                    Self::attempt_type_conversion(arr.first())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn clean_string(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii() || *c as u32 <= 0x10FFFF)
            .collect()
    }

    /// Validate JSON structure without schema (basic well-formedness)
    pub fn validate_structure(json: &str) -> Result<Value> {
        let parsed: Value = serde_json::from_str(json)?;
        Ok(parsed)
    }

    /// Check if JSON is well-formed
    pub fn is_well_formed(json: &str) -> bool {
        serde_json::from_str::<Value>(json).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_structure() {
        let valid_json = r#"{"key": "value"}"#;
        let result = SchemaValidator::validate_structure(valid_json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_well_formed() {
        assert!(SchemaValidator::is_well_formed(r#"{"key": "value"}"#));
        assert!(!SchemaValidator::is_well_formed(r#"{"key": invalid}"#));
    }

    #[test]
    fn test_validate_with_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        
        let valid = serde_json::json!({"name": "test"});
        let invalid = serde_json::json!({"other": "test"});
        
        let result_valid = SchemaValidator::validate(&valid, &schema);
        let result_invalid = SchemaValidator::validate(&invalid, &schema);
        
        assert!(result_valid.is_valid);
        assert!(!result_invalid.is_valid);
    }

    #[test]
    fn test_clean_string() {
        let input = "hello\x00world";
        let cleaned = SchemaValidator::clean_string(input);
        assert!(!cleaned.contains('\x00'));
    }
}
