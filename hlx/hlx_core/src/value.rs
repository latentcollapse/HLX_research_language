//! HLX Value System
//!
//! The 7 fundamental types:
//! - Null
//! - Boolean
//! - Integer (i64)
//! - Float (f64, no NaN/Inf)
//! - String (UTF-8, NFC normalized)
//! - Array (heterogeneous, ordered)
//! - Object (sorted keys)
//!
//! Plus Contract (type-tagged value with schema)

use serde::{Deserialize, Serialize};
use std::fmt;
use im::{Vector, OrdMap};

use crate::error::{HlxError, Result};

/// Field index in a contract (0-255)
pub type FieldIndex = u8;

/// A contract ID (0-999 typical, up to 65535 supported)
pub type ContractId = u16;

/// The fundamental HLX value type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Absence of value
    Null,

    /// Logical true/false
    Boolean(bool),

    /// 64-bit signed integer
    Integer(i64),

    /// IEEE 754 double (NaN/Inf not allowed)
    Float(f64),

    /// UTF-8 string (NFC normalized)
    String(String),

    /// Ordered, heterogeneous collection
    Array(Vector<Value>),

    /// Key-value mapping (keys always sorted)
    Object(OrdMap<String, Value>),

    /// Type-tagged value with schema
    Contract(Contract),

    /// Content-addressed handle reference
    Handle(String),
}

/// A contract instance - type-tagged structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    /// Contract ID (schema identifier)
    pub id: ContractId,

    /// Fields indexed by position (must be in order)
    pub fields: Vec<(FieldIndex, Value)>,
}

impl Value {
    /// Create a validated float (rejects NaN/Infinity)
    pub fn float(f: f64) -> Result<Self> {
        if f.is_nan() || f.is_infinite() {
            return Err(HlxError::FloatSpecial);
        }
        // Normalize -0.0 to +0.0
        let normalized = if f == 0.0 { 0.0 } else { f };
        Ok(Value::Float(normalized))
    }

    /// Create a float, panicking on invalid values (for tests)
    pub fn float_unchecked(f: f64) -> Self {
        Self::float(f).expect("Invalid float")
    }

    /// Create an object from key-value pairs (auto-sorts keys)
    pub fn object(pairs: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        let map: OrdMap<String, Value> = pairs
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();
        Value::Object(map)
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Contract(_) => "contract",
            Value::Handle(_) => "handle",
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if value is truthy (for boolean context)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Integer(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Contract(_) => true,
            Value::Handle(_) => true,
        }
    }

    /// Compute nesting depth (for MAX_DEPTH check)
    pub fn depth(&self) -> usize {
        match self {
            Value::Array(arr) => {
                1 + arr.iter().map(|v| v.depth()).max().unwrap_or(0)
            }
            Value::Object(obj) => {
                1 + obj.values().map(|v| v.depth()).max().unwrap_or(0)
            }
            Value::Contract(c) => {
                1 + c.fields.iter().map(|(_, v)| v.depth()).max().unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Validate depth constraint
    pub fn validate_depth(&self, max_depth: usize) -> Result<()> {
        let d = self.depth();
        if d > max_depth {
            return Err(HlxError::DepthExceeded { depth: d, max: max_depth });
        }
        Ok(())
    }

    // === Arithmetic ===

    pub fn add(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x + y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x + y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 + y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x + *y as f64),
            (Value::String(x), Value::String(y)) => Ok(Value::String(format!("{}{}", x, y))),
            _ => Err(HlxError::TypeError {
                expected: "numeric or string".to_string(),
                got: format!("{} + {}", self.type_name(), other.type_name()),
            }),
        }
    }

    pub fn sub(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x - y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x - y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 - y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x - *y as f64),
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} - {}", self.type_name(), other.type_name()),
            }),
        }
    }

    pub fn mul(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x * y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x * y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 * y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x * *y as f64),
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} * {}", self.type_name(), other.type_name()),
            }),
        }
    }

    pub fn div(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y == 0 { return Err(HlxError::ValidationFail { message: "Division by zero".to_string() }); }
                Value::float(*x as f64 / *y as f64)
            }
            (Value::Float(x), Value::Float(y)) => {
                if *y == 0.0 { return Err(HlxError::ValidationFail { message: "Division by zero".to_string() }); }
                Value::float(x / y)
            }
            (Value::Integer(x), Value::Float(y)) => {
                if *y == 0.0 { return Err(HlxError::ValidationFail { message: "Division by zero".to_string() }); }
                Value::float(*x as f64 / y)
            }
            (Value::Float(x), Value::Integer(y)) => {
                if *y == 0 { return Err(HlxError::ValidationFail { message: "Division by zero".to_string() }); }
                Value::float(x / *y as f64)
            }
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} / {}", self.type_name(), other.type_name()),
            }),
        }
    }

    pub fn rem(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y == 0 { return Err(HlxError::ValidationFail { message: "Modulo by zero".to_string() }); }
                Ok(Value::Integer(x % y))
            }
            (Value::Float(x), Value::Float(y)) => {
                if *y == 0.0 { return Err(HlxError::ValidationFail { message: "Modulo by zero".to_string() }); }
                Value::float(x % y)
            }
            (Value::Integer(x), Value::Float(y)) => {
                if *y == 0.0 { return Err(HlxError::ValidationFail { message: "Modulo by zero".to_string() }); }
                Value::float((*x as f64) % y)
            }
            (Value::Float(x), Value::Integer(y)) => {
                if *y == 0 { return Err(HlxError::ValidationFail { message: "Modulo by zero".to_string() }); }
                Value::float(x % (*y as f64))
            }
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} % {}", self.type_name(), other.type_name()),
            }),
        }
    }

    // === Comparison ===

    pub fn lt(&self, other: &Value) -> Result<bool> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Ok(x < y),
            (Value::Float(x), Value::Float(y)) => Ok(x < y),
            (Value::Integer(x), Value::Float(y)) => Ok((*x as f64) < *y),
            (Value::Float(x), Value::Integer(y)) => Ok(*x < (*y as f64)),
            _ => Err(HlxError::TypeError {
                expected: "comparable".to_string(),
                got: format!("{} < {}", self.type_name(), other.type_name()),
            }),
        }
    }

    pub fn le(&self, other: &Value) -> Result<bool> {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Ok(x <= y),
            (Value::Float(x), Value::Float(y)) => Ok(x <= y),
            (Value::Integer(x), Value::Float(y)) => Ok((*x as f64) <= *y),
            (Value::Float(x), Value::Integer(y)) => Ok(*x <= (*y as f64)),
            _ => Err(HlxError::TypeError {
                expected: "comparable".to_string(),
                got: format!("{} <= {}", self.type_name(), other.type_name()),
            }),
        }
    }

    // === JSON Conversion ===

    pub fn from_json(sjv: serde_json::Value) -> Result<Self> {
        match sjv {
            serde_json::Value::Null => Ok(Value::Null),
            serde_json::Value::Bool(b) => Ok(Value::Boolean(b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Value::float(f)
                } else {
                    Err(HlxError::ValidationFail { message: "Invalid JSON number".to_string() })
                }
            }
            serde_json::Value::String(s) => Ok(Value::String(s)),
            serde_json::Value::Array(arr) => {
                let mut vec = Vector::new();
                for v in arr {
                    vec.push_back(Value::from_json(v)?);
                }
                Ok(Value::Array(vec))
            }
            serde_json::Value::Object(obj) => {
                let mut map = OrdMap::new();
                for (k, v) in obj {
                    map.insert(k, Value::from_json(v)?);
                }
                Ok(Value::Object(map))
            }
        }
    }

    pub fn to_json(&self) -> Result<serde_json::Value> {
        match self {
            Value::Null => Ok(serde_json::Value::Null),
            Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
            Value::Float(f) => {
                let n = serde_json::Number::from_f64(*f).ok_or_else(|| HlxError::ValidationFail { 
                    message: "Invalid float for JSON".to_string() 
                })?;
                Ok(serde_json::Value::Number(n))
            }
            Value::String(s) => Ok(serde_json::Value::String(s.clone())),
            Value::Array(arr) => {
                let mut vec = Vec::new();
                for val in arr {
                    vec.push(val.to_json()?);
                }
                Ok(serde_json::Value::Array(vec))
            }
            Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, val) in obj {
                    map.insert(k.clone(), val.to_json()?);
                }
                Ok(serde_json::Value::Object(map))
            }
            Value::Contract(c) => {
                let mut map = serde_json::Map::new();
                map.insert("__contract_id".to_string(), serde_json::Value::Number((c.id as i64).into()));
                for (idx, val) in &c.fields {
                    map.insert(format!("field_{}", idx), val.to_json()?);
                }
                Ok(serde_json::Value::Object(map))
            }
            Value::Handle(h) => Ok(serde_json::Value::String(format!("hlx://handle/{}", h))),
        }
    }
}

impl Contract {
    /// Create a new contract with validation
    pub fn new(id: ContractId, fields: Vec<(FieldIndex, Value)>) -> Result<Self> {
        // Validate field ordering
        for window in fields.windows(2) {
            if window[0].0 >= window[1].0 {
                return Err(HlxError::FieldOrder);
            }
        }
        Ok(Self { id, fields })
    }

    /// Create contract without validation (for internal use)
    pub fn new_unchecked(id: ContractId, fields: Vec<(FieldIndex, Value)>) -> Self {
        Self { id, fields }
    }

    /// Get a field by index
    pub fn get(&self, idx: FieldIndex) -> Option<&Value> {
        self.fields.iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, v)| v)
    }

    /// Number of fields
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

// === Display implementations ===

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => {
                if fl.fract() == 0.0 {
                    write!(f, "{:.1}", fl)
                } else {
                    write!(f, "{}", fl)
                }
            }
            Value::String(s) => write!(f, "\"{}\"", s.escape_default()),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Contract(c) => write!(f, "{}", c),
            Value::Handle(h) => write!(f, "{}", h),
        }
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{} {{", self.id)?;
        for (i, (idx, val)) in self.fields.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "@{}: {}", idx, val)?;
        }
        write!(f, "}}")
    }
}

// === From implementations for ergonomic construction ===

impl From<bool> for Value {
    fn from(b: bool) -> Self { Value::Boolean(b) }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self { Value::Integer(i) }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self { Value::Integer(i as i64) }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self { Value::String(s.to_string()) }
}

impl From<String> for Value {
    fn from(s: String) -> Self { Value::String(s) }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<std::collections::BTreeMap<String, Value>> for Value {
    fn from(v: std::collections::BTreeMap<String, Value>) -> Self {
        Value::Object(v.into_iter().collect())
    }
}

impl From<OrdMap<String, Value>> for Value {
    fn from(v: OrdMap<String, Value>) -> Self {
        Value::Object(v)
    }
}

impl From<Contract> for Value {
    fn from(c: Contract) -> Self { Value::Contract(c) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_validation() {
        assert!(Value::float(3.14).is_ok());
        assert!(Value::float(0.0).is_ok());
        assert!(Value::float(-0.0).is_ok());
        assert!(Value::float(f64::NAN).is_err());
        assert!(Value::float(f64::INFINITY).is_err());
        assert!(Value::float(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_negative_zero_normalization() {
        let v = Value::float(-0.0).unwrap();
        if let Value::Float(f) = v {
            assert!(f.is_sign_positive());
        }
    }

    #[test]
    fn test_object_key_sorting() {
        let obj = Value::object([
            ("z", Value::Integer(1)),
            ("a", Value::Integer(2)),
            ("m", Value::Integer(3)),
        ]);
        
        if let Value::Object(map) = obj {
            let keys: Vec<_> = map.keys().cloned().collect();
            assert_eq!(keys, vec!["a", "m", "z"]);
        }
    }

    #[test]
    fn test_contract_field_order() {
        // Valid: ascending order
        assert!(Contract::new(14, vec![
            (0, Value::from("alice")),
            (1, Value::Integer(30)),
            (2, Value::Boolean(true)),
        ]).is_ok());

        // Invalid: wrong order
        assert!(Contract::new(14, vec![
            (2, Value::Boolean(true)),
            (0, Value::from("alice")),
        ]).is_err());
    }

    #[test]
    fn test_depth_calculation() {
        let shallow = Value::Integer(42);
        assert_eq!(shallow.depth(), 0);

        let nested = Value::Array(Vector::from(vec![
            Value::Array(Vector::from(vec![
                Value::Integer(1),
            ])),
        ]));
        assert_eq!(nested.depth(), 2);
    }
}
