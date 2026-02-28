use std::collections::BTreeMap;
use std::fmt;
use crate::trust::TrustLevel;

/// Runtime values in the Axiom interpreter
///
/// RT-02: Every value carries a trust tag (provenance algebra).
/// RT-01: Sealed values prevent serialization/display/export.
/// RT-06: Provenance is a first-class value type.
#[derive(Debug, Clone)]
pub enum Value {
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Handle(String),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Contract(ContractValue),
    Enum(String, String),
    /// RT-01: Sealed<T> — opaque wrapper preventing display/serialize/export
    Sealed(Box<Value>),
    /// RT-06: Provenance as a first-class value
    Provenance(TrustLevel),
    Void,
}

/// A runtime contract instance
#[derive(Debug, Clone)]
pub struct ContractValue {
    pub name: String,
    pub fields: BTreeMap<String, Value>,
}

/// RT-02: A value tagged with its trust provenance.
/// Trust travels WITH the value, not tracked by variable name.
#[derive(Debug, Clone)]
pub struct TrustedValue {
    pub value: Value,
    pub trust: TrustLevel,
}

impl TrustedValue {
    pub fn internal(value: Value) -> Self {
        TrustedValue { value, trust: TrustLevel::TrustedInternal }
    }

    pub fn external(value: Value) -> Self {
        TrustedValue { value, trust: TrustLevel::UntrustedExternal }
    }

    pub fn verified(value: Value) -> Self {
        TrustedValue { value, trust: TrustLevel::TrustedVerified }
    }

    pub fn with_trust(value: Value, trust: TrustLevel) -> Self {
        TrustedValue { value, trust }
    }

    /// RT-02: Trust algebra — combine produces least-trusted result
    pub fn combine_trust(levels: &[TrustLevel]) -> TrustLevel {
        TrustLevel::combine(levels)
    }
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::I64(_) => "i64",
            Value::F64(_) => "f64",
            Value::Bool(_) => "bool",
            Value::String(_) => "String",
            Value::Bytes(_) => "Bytes",
            Value::Handle(_) => "Handle",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Contract(c) => &c.name,
            Value::Enum(name, _) => name,
            Value::Sealed(_) => "Sealed",
            Value::Provenance(_) => "Provenance",
            Value::Void => "void",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::I64(n) => *n != 0,
            Value::Void => false,
            // RT-01: Cannot inspect Sealed values for truthiness
            Value::Sealed(_) => false,
            _ => true,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(n) => Some(*n),
            // RT-05: No implicit f64->i64 conversion removed
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(n) => Some(*n),
            // RT-05: No implicit i64->f64 conversion removed
            _ => None,
        }
    }

    /// RT-01: Check if this value is Sealed
    pub fn is_sealed(&self) -> bool {
        matches!(self, Value::Sealed(_))
    }

    /// RT-03: Checked addition — returns None on overflow
    pub fn checked_add(a: i64, b: i64) -> Option<i64> {
        a.checked_add(b)
    }

    /// RT-03: Checked subtraction
    pub fn checked_sub(a: i64, b: i64) -> Option<i64> {
        a.checked_sub(b)
    }

    /// RT-03: Checked multiplication
    pub fn checked_mul(a: i64, b: i64) -> Option<i64> {
        a.checked_mul(b)
    }

    /// RT-04: Total ordering for f64 (NaN sorts after all, -0.0 < +0.0)
    pub fn f64_total_cmp(a: f64, b: f64) -> std::cmp::Ordering {
        a.total_cmp(&b)
    }

    /// RT-04: Bit-level f64 equality (NaN==NaN, -0.0 != +0.0)
    pub fn f64_total_eq(a: f64, b: f64) -> bool {
        a.to_bits() == b.to_bits()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I64(n) => write!(f, "{}", n),
            Value::F64(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "<bytes len={}>", b.len()),
            Value::Handle(h) => write!(f, "&h_{}", &h[..8.min(h.len())]),
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
            Value::Contract(c) => {
                write!(f, "{} {{", c.name)?;
                for (i, (k, v)) in c.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Enum(name, variant) => write!(f, "{}.{}", name, variant),
            // RT-01: Sealed values CANNOT be displayed
            Value::Sealed(_) => write!(f, "<sealed>"),
            Value::Provenance(level) => write!(f, "{}", level),
            Value::Void => write!(f, "void"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::I64(a), Value::I64(b)) => a == b,
            // RT-04: Bit-level f64 equality (NaN==NaN, -0.0 != +0.0)
            (Value::F64(a), Value::F64(b)) => Value::f64_total_eq(*a, *b),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Handle(a), Value::Handle(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Contract(a), Value::Contract(b)) => a.name == b.name && a.fields == b.fields,
            (Value::Enum(a1, a2), Value::Enum(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Provenance(a), Value::Provenance(b)) => a == b,
            (Value::Void, Value::Void) => true,
            // RT-01: Sealed values CANNOT be compared for equality
            (Value::Sealed(_), _) | (_, Value::Sealed(_)) => false,
            _ => false,
        }
    }
}
