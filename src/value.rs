//! Configuration value types and conversion utilities.

//! Configuration value types and conversion utilities.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a configuration value that can be of various types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<ConfigValue>),
    /// Object/map of key-value pairs
    Object(HashMap<String, ConfigValue>),
    /// Null value
    Null,
}

impl ConfigValue {
    /// Returns the value as a string reference if it's a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an i64 if it's an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the value as an f64 if it's a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Returns the value as a bool if it's a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as an array reference if it's an array.
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns the value as an object reference if it's an object.
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Checks if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }

    /// Coerces the value to a string representation.
    /// This method provides intelligent conversion from any ConfigValue type to String.
    pub fn coerce_to_string(&self) -> String {
        match self {
            ConfigValue::String(s) => s.clone(),
            ConfigValue::Integer(i) => i.to_string(),
            ConfigValue::Float(f) => f.to_string(),
            ConfigValue::Boolean(b) => b.to_string(),
            ConfigValue::Array(_) => "[array]".to_string(),
            ConfigValue::Object(_) => "[object]".to_string(),
            ConfigValue::Null => "".to_string(),
        }
    }

    /// Coerces the value to a boolean representation.
    /// This method provides intelligent conversion from various ConfigValue types to bool.
    /// Returns None if the value cannot be meaningfully converted to a boolean.
    pub fn coerce_to_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" | "t" | "y" => Some(true),
                "false" | "0" | "no" | "off" | "f" | "n" | "" => Some(false),
                _ => None,
            },
            ConfigValue::Integer(i) => Some(*i != 0),
            ConfigValue::Float(f) => Some(*f != 0.0),
            ConfigValue::Null => Some(false),
            ConfigValue::Array(arr) => Some(!arr.is_empty()),
            ConfigValue::Object(obj) => Some(!obj.is_empty()),
        }
    }

    /// Returns the type name of the ConfigValue variant.
    pub fn type_name(&self) -> &'static str {
        match self {
            ConfigValue::String(_) => "String",
            ConfigValue::Integer(_) => "Integer",
            ConfigValue::Float(_) => "Float",
            ConfigValue::Boolean(_) => "Boolean",
            ConfigValue::Array(_) => "Array",
            ConfigValue::Object(_) => "Object",
            ConfigValue::Null => "Null",
        }
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        ConfigValue::String(s)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue::String(s.to_string())
    }
}

impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self {
        ConfigValue::Integer(i)
    }
}

impl From<f64> for ConfigValue {
    fn from(f: f64) -> Self {
        ConfigValue::Float(f)
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        ConfigValue::Boolean(b)
    }
}

impl From<Vec<ConfigValue>> for ConfigValue {
    fn from(arr: Vec<ConfigValue>) -> Self {
        ConfigValue::Array(arr)
    }
}

impl From<HashMap<String, ConfigValue>> for ConfigValue {
    fn from(obj: HashMap<String, ConfigValue>) -> Self {
        ConfigValue::Object(obj)
    }
}

impl From<i32> for ConfigValue {
    fn from(i: i32) -> Self {
        ConfigValue::Integer(i as i64)
    }
}

impl From<u32> for ConfigValue {
    fn from(i: u32) -> Self {
        ConfigValue::Integer(i as i64)
    }
}

impl From<f32> for ConfigValue {
    fn from(f: f32) -> Self {
        ConfigValue::Float(f as f64)
    }
}

impl From<Option<ConfigValue>> for ConfigValue {
    fn from(opt: Option<ConfigValue>) -> Self {
        opt.unwrap_or(ConfigValue::Null)
    }
}

/// Error type for ConfigValue conversion failures
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionError {
    pub from_type: String,
    pub to_type: String,
    pub value: String,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cannot convert {} value '{}' to {}",
            self.from_type, self.value, self.to_type
        )
    }
}

impl std::error::Error for ConversionError {}

impl TryFrom<ConfigValue> for String {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::String(s) => Ok(s),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "String".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

impl TryFrom<ConfigValue> for i64 {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::Integer(i) => Ok(i),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "i64".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

impl TryFrom<ConfigValue> for f64 {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::Float(f) => Ok(f),
            ConfigValue::Integer(i) => Ok(i as f64),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "f64".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

impl TryFrom<ConfigValue> for bool {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::Boolean(b) => Ok(b),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "bool".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

impl TryFrom<ConfigValue> for Vec<ConfigValue> {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::Array(arr) => Ok(arr),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "Vec<ConfigValue>".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

impl TryFrom<ConfigValue> for HashMap<String, ConfigValue> {
    type Error = ConversionError;

    fn try_from(value: ConfigValue) -> Result<Self, Self::Error> {
        match value {
            ConfigValue::Object(obj) => Ok(obj),
            _ => Err(ConversionError {
                from_type: value.type_name().to_string(),
                to_type: "HashMap<String, ConfigValue>".to_string(),
                value: value.coerce_to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_creation() {
        let string_val = ConfigValue::from("test");
        assert_eq!(string_val.as_str(), Some("test"));

        let int_val = ConfigValue::from(42i64);
        assert_eq!(int_val.as_i64(), Some(42));

        let bool_val = ConfigValue::from(true);
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_type_checking() {
        let null_val = ConfigValue::Null;
        assert!(null_val.is_null());
        assert_eq!(null_val.as_str(), None);
    }

    #[test]
    fn test_coerce_to_string() {
        // Test string values
        let string_val = ConfigValue::String("hello".to_string());
        assert_eq!(string_val.coerce_to_string(), "hello");

        // Test integer values
        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.coerce_to_string(), "42");

        // Test float values
        let float_val = ConfigValue::Float(3.14);
        assert_eq!(float_val.coerce_to_string(), "3.14");

        // Test boolean values
        let bool_true = ConfigValue::Boolean(true);
        assert_eq!(bool_true.coerce_to_string(), "true");
        let bool_false = ConfigValue::Boolean(false);
        assert_eq!(bool_false.coerce_to_string(), "false");

        // Test null values
        let null_val = ConfigValue::Null;
        assert_eq!(null_val.coerce_to_string(), "");

        // Test array values
        let array_val = ConfigValue::Array(vec![ConfigValue::Integer(1)]);
        assert_eq!(array_val.coerce_to_string(), "[array]");

        // Test object values
        let mut obj = HashMap::new();
        obj.insert("key".to_string(), ConfigValue::String("value".to_string()));
        let obj_val = ConfigValue::Object(obj);
        assert_eq!(obj_val.coerce_to_string(), "[object]");
    }

    #[test]
    fn test_coerce_to_bool() {
        // Test boolean values
        let bool_true = ConfigValue::Boolean(true);
        assert_eq!(bool_true.coerce_to_bool(), Some(true));
        let bool_false = ConfigValue::Boolean(false);
        assert_eq!(bool_false.coerce_to_bool(), Some(false));

        // Test string values - truthy
        let true_strings = vec![
            "true", "TRUE", "True", "1", "yes", "YES", "on", "ON", "t", "T", "y", "Y",
        ];
        for s in true_strings {
            let string_val = ConfigValue::String(s.to_string());
            assert_eq!(
                string_val.coerce_to_bool(),
                Some(true),
                "Failed for string: {}",
                s
            );
        }

        // Test string values - falsy
        let false_strings = vec![
            "false", "FALSE", "False", "0", "no", "NO", "off", "OFF", "f", "F", "n", "N", "",
        ];
        for s in false_strings {
            let string_val = ConfigValue::String(s.to_string());
            assert_eq!(
                string_val.coerce_to_bool(),
                Some(false),
                "Failed for string: {}",
                s
            );
        }

        // Test string values - invalid
        let invalid_strings = vec!["maybe", "invalid", "2", "hello"];
        for s in invalid_strings {
            let string_val = ConfigValue::String(s.to_string());
            assert_eq!(
                string_val.coerce_to_bool(),
                None,
                "Should be None for string: {}",
                s
            );
        }

        // Test integer values
        let int_zero = ConfigValue::Integer(0);
        assert_eq!(int_zero.coerce_to_bool(), Some(false));
        let int_positive = ConfigValue::Integer(42);
        assert_eq!(int_positive.coerce_to_bool(), Some(true));
        let int_negative = ConfigValue::Integer(-1);
        assert_eq!(int_negative.coerce_to_bool(), Some(true));

        // Test float values
        let float_zero = ConfigValue::Float(0.0);
        assert_eq!(float_zero.coerce_to_bool(), Some(false));
        let float_positive = ConfigValue::Float(3.14);
        assert_eq!(float_positive.coerce_to_bool(), Some(true));
        let float_negative = ConfigValue::Float(-2.5);
        assert_eq!(float_negative.coerce_to_bool(), Some(true));

        // Test null values
        let null_val = ConfigValue::Null;
        assert_eq!(null_val.coerce_to_bool(), Some(false));

        // Test array values
        let empty_array = ConfigValue::Array(vec![]);
        assert_eq!(empty_array.coerce_to_bool(), Some(false));
        let non_empty_array = ConfigValue::Array(vec![ConfigValue::Integer(1)]);
        assert_eq!(non_empty_array.coerce_to_bool(), Some(true));

        // Test object values
        let empty_obj = ConfigValue::Object(HashMap::new());
        assert_eq!(empty_obj.coerce_to_bool(), Some(false));
        let mut non_empty_obj = HashMap::new();
        non_empty_obj.insert("key".to_string(), ConfigValue::String("value".to_string()));
        let non_empty_obj_val = ConfigValue::Object(non_empty_obj);
        assert_eq!(non_empty_obj_val.coerce_to_bool(), Some(true));
    }

    #[test]
    fn test_from_conversions() {
        // Test From implementations
        let string_val: ConfigValue = "hello".into();
        assert_eq!(string_val, ConfigValue::String("hello".to_string()));

        let int_val: ConfigValue = 42i64.into();
        assert_eq!(int_val, ConfigValue::Integer(42));

        let int32_val: ConfigValue = 42i32.into();
        assert_eq!(int32_val, ConfigValue::Integer(42));

        let uint32_val: ConfigValue = 42u32.into();
        assert_eq!(uint32_val, ConfigValue::Integer(42));

        let float_val: ConfigValue = 3.14f64.into();
        assert_eq!(float_val, ConfigValue::Float(3.14));

        let float32_val: ConfigValue = 3.14f32.into();
        assert_eq!(float32_val, ConfigValue::Float(3.14f32 as f64));

        let bool_val: ConfigValue = true.into();
        assert_eq!(bool_val, ConfigValue::Boolean(true));

        let array_val: ConfigValue = vec![
            ConfigValue::Integer(1),
            ConfigValue::String("test".to_string()),
        ]
        .into();
        assert!(matches!(array_val, ConfigValue::Array(_)));

        let mut obj = HashMap::new();
        obj.insert("key".to_string(), ConfigValue::String("value".to_string()));
        let obj_val: ConfigValue = obj.into();
        assert!(matches!(obj_val, ConfigValue::Object(_)));

        let none_val: ConfigValue = None::<ConfigValue>.into();
        assert_eq!(none_val, ConfigValue::Null);

        let some_val: ConfigValue = Some(ConfigValue::String("test".to_string())).into();
        assert_eq!(some_val, ConfigValue::String("test".to_string()));
    }

    #[test]
    fn test_try_from_conversions_success() {
        // Test successful TryFrom conversions
        let string_val = ConfigValue::String("hello".to_string());
        let converted_string: String = string_val.try_into().unwrap();
        assert_eq!(converted_string, "hello");

        let int_val = ConfigValue::Integer(42);
        let converted_int: i64 = int_val.try_into().unwrap();
        assert_eq!(converted_int, 42);

        let float_val = ConfigValue::Float(3.14);
        let converted_float: f64 = float_val.try_into().unwrap();
        assert_eq!(converted_float, 3.14);

        // Test integer to float conversion
        let int_to_float = ConfigValue::Integer(42);
        let converted_float: f64 = int_to_float.try_into().unwrap();
        assert_eq!(converted_float, 42.0);

        let bool_val = ConfigValue::Boolean(true);
        let converted_bool: bool = bool_val.try_into().unwrap();
        assert_eq!(converted_bool, true);

        let array_val = ConfigValue::Array(vec![ConfigValue::Integer(1)]);
        let converted_array: Vec<ConfigValue> = array_val.try_into().unwrap();
        assert_eq!(converted_array, vec![ConfigValue::Integer(1)]);

        let mut obj = HashMap::new();
        obj.insert("key".to_string(), ConfigValue::String("value".to_string()));
        let obj_val = ConfigValue::Object(obj.clone());
        let converted_obj: HashMap<String, ConfigValue> = obj_val.try_into().unwrap();
        assert_eq!(converted_obj, obj);
    }

    #[test]
    fn test_try_from_conversions_failure() {
        // Test failed TryFrom conversions
        let int_val = ConfigValue::Integer(42);
        let string_result: Result<String, ConversionError> = int_val.try_into();
        assert!(string_result.is_err());
        let err = string_result.unwrap_err();
        assert_eq!(err.from_type, "Integer");
        assert_eq!(err.to_type, "String");
        assert_eq!(err.value, "42");

        let string_val = ConfigValue::String("hello".to_string());
        let int_result: Result<i64, ConversionError> = string_val.try_into();
        assert!(int_result.is_err());

        let bool_val = ConfigValue::Boolean(true);
        let float_result: Result<f64, ConversionError> = bool_val.try_into();
        assert!(float_result.is_err());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(
            ConfigValue::String("test".to_string()).type_name(),
            "String"
        );
        assert_eq!(ConfigValue::Integer(42).type_name(), "Integer");
        assert_eq!(ConfigValue::Float(3.14).type_name(), "Float");
        assert_eq!(ConfigValue::Boolean(true).type_name(), "Boolean");
        assert_eq!(ConfigValue::Array(vec![]).type_name(), "Array");
        assert_eq!(ConfigValue::Object(HashMap::new()).type_name(), "Object");
        assert_eq!(ConfigValue::Null.type_name(), "Null");
    }

    #[test]
    fn test_serde_serialization() {
        // Test that serde serialization works correctly
        let value = ConfigValue::String("test".to_string());
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "\"test\"");

        let value = ConfigValue::Integer(42);
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "42");

        let value = ConfigValue::Boolean(true);
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "true");

        let value = ConfigValue::Null;
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "null");
    }

    #[test]
    fn test_serde_deserialization() {
        // Test that serde deserialization works correctly
        let deserialized: ConfigValue = serde_json::from_str("\"test\"").unwrap();
        assert_eq!(deserialized, ConfigValue::String("test".to_string()));

        let deserialized: ConfigValue = serde_json::from_str("42").unwrap();
        assert_eq!(deserialized, ConfigValue::Integer(42));

        let deserialized: ConfigValue = serde_json::from_str("3.14").unwrap();
        assert_eq!(deserialized, ConfigValue::Float(3.14));

        let deserialized: ConfigValue = serde_json::from_str("true").unwrap();
        assert_eq!(deserialized, ConfigValue::Boolean(true));

        let deserialized: ConfigValue = serde_json::from_str("null").unwrap();
        assert_eq!(deserialized, ConfigValue::Null);

        let deserialized: ConfigValue = serde_json::from_str("[1, 2, 3]").unwrap();
        assert!(matches!(deserialized, ConfigValue::Array(_)));

        let deserialized: ConfigValue = serde_json::from_str("{\"key\": \"value\"}").unwrap();
        assert!(matches!(deserialized, ConfigValue::Object(_)));
    }
}
