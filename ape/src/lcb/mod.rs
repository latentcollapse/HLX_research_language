//! LC-B Wire Format — Axiom's canonical binary serialization (Part XI)
//!
//! Every value has exactly one binary representation. This is A3 made concrete.
//! BLAKE3(LC-B(value)) is the identity of any value.

use crate::trust::TrustLevel;
use std::collections::BTreeMap;

/// APE's canonical value type for wire-format serialization.
/// Every value has exactly one LC-B encoding. BLAKE3(LC-B(value)) = identity.
#[derive(Debug, Clone, PartialEq)]
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
    Sealed(Box<Value>),
    Provenance(TrustLevel),
    Void,
}

/// A contract (struct) value with named fields
#[derive(Debug, Clone, PartialEq)]
pub struct ContractValue {
    pub name: String,
    pub fields: BTreeMap<String, Value>,
}

/// Tag bytes for LC-B encoding (Section 11.1)
pub mod tags {
    // Primitives 0x00-0x0F
    pub const I64: u8 = 0x01;
    pub const F64: u8 = 0x02;
    pub const BOOL_TRUE: u8 = 0x03;
    pub const BOOL_FALSE: u8 = 0x04;
    pub const STRING: u8 = 0x05;
    pub const BYTES: u8 = 0x06;
    pub const VOID: u8 = 0x0F;

    // Containers 0x10-0x1F
    pub const ARRAY: u8 = 0x10;
    pub const CONTRACT: u8 = 0x11;
    pub const TENSOR: u8 = 0x12;
    pub const MAP: u8 = 0x13;

    // References 0x20-0x2F
    pub const HANDLE: u8 = 0x20;
    pub const PROVENANCE: u8 = 0x21;
    pub const SEED: u8 = 0x22;
    pub const AGENT_ID: u8 = 0x23;

    // Intent 0x30-0x3F
    pub const INTENT_DECL: u8 = 0x30;
    pub const DO_RESULT: u8 = 0x31;

    // System 0x40-0x4F
    pub const HALT: u8 = 0x40;
    pub const BARRIER: u8 = 0x41;

    // SCALE 0x50-0x5F
    pub const SHARED_STATE: u8 = 0x50;
    pub const WORK_ITEM: u8 = 0x51;
    pub const EPOCH: u8 = 0x52;

    // Enum 0x60-0x6F
    pub const ENUM: u8 = 0x60;

    // RT-01: Sealed 0x70-0x7F
    pub const SEALED: u8 = 0x70;

    // RT-06: Provenance value 0x80-0x8F
    pub const PROVENANCE_VALUE: u8 = 0x80;
}

/// Encode a Value to LC-B canonical binary format
pub fn encode(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_value(value, &mut buf);
    buf
}

/// Compute the BLAKE3 content-address of a value
/// This is the identity of the value per A3
pub fn content_address(value: &Value) -> String {
    let bytes = encode(value);
    let hash = blake3::hash(&bytes);
    hash.to_hex().to_string()
}

/// Compute content address with domain separation by contract ID (Section 4.8)
pub fn content_address_with_domain(contract_id: &str, value: &Value) -> String {
    let mut hasher = blake3::Hasher::new();
    // Domain separation: hash contract_id first
    hasher.update(contract_id.as_bytes());
    hasher.update(&[0x00]); // separator
    hasher.update(&encode(value));
    hasher.finalize().to_hex().to_string()
}

fn encode_value(value: &Value, buf: &mut Vec<u8>) {
    match value {
        Value::I64(n) => {
            buf.push(tags::I64);
            buf.extend_from_slice(&n.to_le_bytes());
        }
        Value::F64(n) => {
            buf.push(tags::F64);
            buf.extend_from_slice(&n.to_le_bytes());
        }
        Value::Bool(true) => {
            buf.push(tags::BOOL_TRUE);
        }
        Value::Bool(false) => {
            buf.push(tags::BOOL_FALSE);
        }
        Value::String(s) => {
            buf.push(tags::STRING);
            encode_leb128(s.len() as u64, buf);
            buf.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            buf.push(tags::BYTES);
            encode_leb128(b.len() as u64, buf);
            buf.extend_from_slice(b);
        }
        Value::Handle(h) => {
            buf.push(tags::HANDLE);
            // Handles are 32 bytes (BLAKE3 digest) but we store as string for now
            let h_bytes = h.as_bytes();
            encode_leb128(h_bytes.len() as u64, buf);
            buf.extend_from_slice(h_bytes);
        }
        Value::Array(elems) => {
            buf.push(tags::ARRAY);
            encode_leb128(elems.len() as u64, buf);
            for elem in elems {
                encode_value(elem, buf);
            }
        }
        Value::Map(entries) => {
            buf.push(tags::MAP);
            encode_leb128(entries.len() as u64, buf);
            // Keys sorted by BLAKE3 for determinism (Section 4.4)
            let mut sorted: Vec<_> = entries.iter().collect();
            sorted.sort_by(|(k1, _), (k2, _)| {
                let h1 = blake3::hash(k1.as_bytes());
                let h2 = blake3::hash(k2.as_bytes());
                h1.as_bytes().cmp(h2.as_bytes())
            });
            for (key, val) in sorted {
                encode_leb128(key.len() as u64, buf);
                buf.extend_from_slice(key.as_bytes());
                encode_value(val, buf);
            }
        }
        Value::Contract(c) => {
            buf.push(tags::CONTRACT);
            // Contract name
            encode_leb128(c.name.len() as u64, buf);
            buf.extend_from_slice(c.name.as_bytes());
            // Fields sorted by ascending @N order (Section 11.2 rule 8)
            // BTreeMap is already sorted by key name; for true @N order
            // we'd need index info, but for now name order is deterministic
            encode_leb128(c.fields.len() as u64, buf);
            for (fname, fval) in &c.fields {
                encode_leb128(fname.len() as u64, buf);
                buf.extend_from_slice(fname.as_bytes());
                encode_value(fval, buf);
            }
        }
        Value::Enum(name, variant) => {
            buf.push(tags::ENUM);
            encode_leb128(name.len() as u64, buf);
            buf.extend_from_slice(name.as_bytes());
            encode_leb128(variant.len() as u64, buf);
            buf.extend_from_slice(variant.as_bytes());
        }
        // RT-01: Sealed values CANNOT be serialized — this is a runtime error
        // In a real system this would HALT. Here we encode a marker for debugging
        // but this path should never be reached if the interpreter enforces correctly.
        Value::Sealed(_) => {
            panic!(
                "RT-01: Attempted to serialize Sealed value — this violates Axiom safety invariant"
            );
        }
        // RT-06: Provenance as first-class value
        Value::Provenance(level) => {
            buf.push(tags::PROVENANCE_VALUE);
            buf.push(*level as u8);
        }
        Value::Void => {
            buf.push(tags::VOID);
        }
    }
}

/// Decode a Value from LC-B binary format
pub fn decode(data: &[u8]) -> Option<Value> {
    let mut pos = 0;
    decode_value(data, &mut pos)
}

fn decode_value(data: &[u8], pos: &mut usize) -> Option<Value> {
    if *pos >= data.len() {
        return None;
    }
    let tag = data[*pos];
    *pos += 1;

    match tag {
        tags::I64 => {
            if *pos + 8 > data.len() {
                return None;
            }
            let bytes: [u8; 8] = data[*pos..*pos + 8].try_into().ok()?;
            *pos += 8;
            Some(Value::I64(i64::from_le_bytes(bytes)))
        }
        tags::F64 => {
            if *pos + 8 > data.len() {
                return None;
            }
            let bytes: [u8; 8] = data[*pos..*pos + 8].try_into().ok()?;
            *pos += 8;
            Some(Value::F64(f64::from_le_bytes(bytes)))
        }
        tags::BOOL_TRUE => Some(Value::Bool(true)),
        tags::BOOL_FALSE => Some(Value::Bool(false)),
        tags::STRING => {
            let len = decode_leb128(data, pos)? as usize;
            if *pos + len > data.len() {
                return None;
            }
            let s = String::from_utf8(data[*pos..*pos + len].to_vec()).ok()?;
            *pos += len;
            Some(Value::String(s))
        }
        tags::BYTES => {
            let len = decode_leb128(data, pos)? as usize;
            if *pos + len > data.len() {
                return None;
            }
            let b = data[*pos..*pos + len].to_vec();
            *pos += len;
            Some(Value::Bytes(b))
        }
        tags::HANDLE => {
            let len = decode_leb128(data, pos)? as usize;
            if *pos + len > data.len() {
                return None;
            }
            let h = String::from_utf8(data[*pos..*pos + len].to_vec()).ok()?;
            *pos += len;
            Some(Value::Handle(h))
        }
        tags::ARRAY => {
            let count = decode_leb128(data, pos)? as usize;
            let mut elems = Vec::with_capacity(count);
            for _ in 0..count {
                elems.push(decode_value(data, pos)?);
            }
            Some(Value::Array(elems))
        }
        tags::MAP => {
            let count = decode_leb128(data, pos)? as usize;
            let mut map = BTreeMap::new();
            for _ in 0..count {
                let klen = decode_leb128(data, pos)? as usize;
                if *pos + klen > data.len() {
                    return None;
                }
                let key = String::from_utf8(data[*pos..*pos + klen].to_vec()).ok()?;
                *pos += klen;
                let val = decode_value(data, pos)?;
                map.insert(key, val);
            }
            Some(Value::Map(map))
        }
        tags::CONTRACT => {
            let nlen = decode_leb128(data, pos)? as usize;
            if *pos + nlen > data.len() {
                return None;
            }
            let name = String::from_utf8(data[*pos..*pos + nlen].to_vec()).ok()?;
            *pos += nlen;
            let fcount = decode_leb128(data, pos)? as usize;
            let mut fields = BTreeMap::new();
            for _ in 0..fcount {
                let flen = decode_leb128(data, pos)? as usize;
                if *pos + flen > data.len() {
                    return None;
                }
                let fname = String::from_utf8(data[*pos..*pos + flen].to_vec()).ok()?;
                *pos += flen;
                let fval = decode_value(data, pos)?;
                fields.insert(fname, fval);
            }
            Some(Value::Contract(ContractValue { name, fields }))
        }
        tags::ENUM => {
            let nlen = decode_leb128(data, pos)? as usize;
            if *pos + nlen > data.len() {
                return None;
            }
            let name = String::from_utf8(data[*pos..*pos + nlen].to_vec()).ok()?;
            *pos += nlen;
            let vlen = decode_leb128(data, pos)? as usize;
            if *pos + vlen > data.len() {
                return None;
            }
            let variant = String::from_utf8(data[*pos..*pos + vlen].to_vec()).ok()?;
            *pos += vlen;
            Some(Value::Enum(name, variant))
        }
        // RT-01: Sealed cannot appear in wire format — reject
        tags::SEALED => None,
        // RT-06: Provenance decoding
        tags::PROVENANCE_VALUE => {
            if *pos >= data.len() {
                return None;
            }
            let level_byte = data[*pos];
            *pos += 1;
            let level = match level_byte {
                0 => crate::trust::TrustLevel::TrustedInternal,
                1 => crate::trust::TrustLevel::TrustedVerified,
                2 => crate::trust::TrustLevel::UntrustedExternal,
                3 => crate::trust::TrustLevel::UntrustedTainted,
                _ => return None,
            };
            Some(Value::Provenance(level))
        }
        tags::VOID => Some(Value::Void),
        _ => None,
    }
}

/// Encode a u64 as LEB128
fn encode_leb128(mut value: u64, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decode a LEB128 u64
fn decode_leb128(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        if *pos >= data.len() {
            return None;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_primitives() {
        let values = vec![
            Value::I64(42),
            Value::I64(-1),
            Value::I64(0),
            Value::F64(3.14),
            Value::Bool(true),
            Value::Bool(false),
            Value::String("hello axiom".to_string()),
            Value::Void,
        ];
        for val in &values {
            let encoded = encode(val);
            let decoded = decode(&encoded).expect("decode failed");
            assert_eq!(val, &decoded, "roundtrip failed for {:?}", val);
        }
    }

    #[test]
    fn test_roundtrip_array() {
        let val = Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
        let encoded = encode(&val);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_roundtrip_contract() {
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::F64(3.0));
        fields.insert("y".to_string(), Value::F64(4.0));
        let val = Value::Contract(ContractValue {
            name: "Point".to_string(),
            fields,
        });
        let encoded = encode(&val);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_content_address_deterministic() {
        let val = Value::I64(42);
        let h1 = content_address(&val);
        let h2 = content_address(&val);
        assert_eq!(h1, h2, "A1: identical inputs must produce identical hashes");
    }

    #[test]
    fn test_domain_separation() {
        let val = Value::I64(42);
        let h1 = content_address_with_domain("Point", &val);
        let h2 = content_address_with_domain("Vector", &val);
        assert_ne!(h1, h2, "Domain separation must produce different hashes");
    }

    #[test]
    fn test_canonical_encoding() {
        // A3: one value → one encoding
        let val = Value::String("deterministic".to_string());
        let enc1 = encode(&val);
        let enc2 = encode(&val);
        assert_eq!(enc1, enc2, "A3: canonical encoding must be identical");
    }
}
