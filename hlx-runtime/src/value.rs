use crate::tensor::Tensor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Tensor(Tensor),
    /// First-class function reference (used for lambdas / closures)
    Function(String),
    Void,
    Nil,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::I64(_) => "i64",
            Value::F64(_) => "f64",
            Value::Bool(_) => "bool",
            Value::String(_) => "String",
            Value::Bytes(_) => "Bytes",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Tensor(_) => "Tensor",
            Value::Function(_) => "Function",
            Value::Void => "void",
            Value::Nil => "nil",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::I64(n) => *n != 0,
            Value::Nil | Value::Void => false,
            _ => true,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value::Bytes(b) => b.clone(),
            Value::String(s) => s.as_bytes().to_vec(),
            Value::I64(n) => n.to_le_bytes().to_vec(),
            Value::F64(n) => n.to_le_bytes().to_vec(),
            Value::Bool(b) => vec![if *b { 1 } else { 0 }],
            _ => vec![],
        }
    }

    /// Compute the nesting depth of this value (for stack overflow prevention).
    /// Returns 0 for scalars, max(child_depths)+1 for containers.
    /// Capped at `limit` to avoid stack overflow during the depth check itself.
    pub fn depth(&self, limit: usize) -> usize {
        if limit == 0 {
            return 0;
        }
        match self {
            Value::Array(elems) => elems
                .iter()
                .map(|e| e.depth(limit - 1))
                .max()
                .map_or(1, |d| d + 1),
            Value::Map(entries) => entries
                .values()
                .map(|v| v.depth(limit - 1))
                .max()
                .map_or(1, |d| d + 1),
            _ => 0,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I64(n) => write!(f, "{}", n),
            Value::F64(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "<bytes:{}>", b.len()),
            Value::Array(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Tensor(t) => write!(f, "{}", t),
            Value::Function(name) => write!(f, "<fn:{}>", name),
            Value::Void => write!(f, "void"),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::I64(n)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::F64(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(|x| x.into()).collect())
    }
}
