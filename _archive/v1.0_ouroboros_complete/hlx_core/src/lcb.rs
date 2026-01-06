//! LC-B Wire Format
//!
//! Binary encoding for HLX values and capsules.
//!
//! ## Type Tags (from WIRE_FORMATS.md)
//! - 0x00: Null
//! - 0x01: False  
//! - 0x02: True
//! - 0x10: Integer (LEB128)
//! - 0x20: Float (IEEE754 big-endian)
//! - 0x30: String (LEB128 len + UTF-8)
//! - 0x40: Array (LEB128 count + elements)
//! - 0x50: Object (LEB128 count + sorted k-v pairs)
//! - 0x60: Contract (LEB128 ID + LEB128 field count + fields)
//! - 0x70: Handle (LEB128 len + ASCII)
//!
//! ## Determinism
//! - Object keys always sorted lexicographically
//! - Contract fields always in ascending index order
//! - No padding or alignment bytes

use crate::value::{Value, Contract, FieldIndex, ContractId};
use crate::error::{HlxError, Result};
use crate::{LCB_MAGIC, MAX_DEPTH};

// === Type Tags ===

const TAG_NULL: u8 = 0x00;
const TAG_FALSE: u8 = 0x01;
const TAG_TRUE: u8 = 0x02;
const TAG_INTEGER: u8 = 0x10;
const TAG_FLOAT: u8 = 0x20;
const TAG_STRING: u8 = 0x30;
const TAG_ARRAY: u8 = 0x40;
const TAG_OBJECT: u8 = 0x50;
const TAG_CONTRACT: u8 = 0x60;
const TAG_HANDLE: u8 = 0x70;

/// Encode a Value to LC-B binary format
pub fn encode(value: &Value) -> Result<Vec<u8>> {
    value.validate_depth(MAX_DEPTH)?;
    let mut buf = Vec::new();
    buf.push(LCB_MAGIC);
    encode_value(value, &mut buf)?;
    Ok(buf)
}

/// Encode a Value without the magic prefix (for embedding)
pub fn encode_value(value: &Value, buf: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => {
            buf.push(TAG_NULL);
        }
        Value::Boolean(false) => {
            buf.push(TAG_FALSE);
        }
        Value::Boolean(true) => {
            buf.push(TAG_TRUE);
        }
        Value::Integer(i) => {
            buf.push(TAG_INTEGER);
            encode_leb128_signed(*i, buf);
        }
        Value::Float(f) => {
            buf.push(TAG_FLOAT);
            buf.extend_from_slice(&f.to_be_bytes());
        }
        Value::String(s) => {
            buf.push(TAG_STRING);
            encode_leb128_unsigned(s.len() as u64, buf);
            buf.extend_from_slice(s.as_bytes());
        }
        Value::Array(arr) => {
            buf.push(TAG_ARRAY);
            encode_leb128_unsigned(arr.len() as u64, buf);
            for elem in arr.iter() {
                encode_value(elem, buf)?;
            }
        }
        Value::Object(obj) => {
            buf.push(TAG_OBJECT);
            encode_leb128_unsigned(obj.len() as u64, buf);
            // BTreeMap is already sorted
            for (key, val) in obj.iter() {
                // Encode key as string (without tag)
                encode_leb128_unsigned(key.len() as u64, buf);
                buf.extend_from_slice(key.as_bytes());
                // Encode value with tag
                encode_value(val, buf)?;
            }
        }
        Value::Contract(contract) => {
            buf.push(TAG_CONTRACT);
            encode_leb128_unsigned(contract.id as u64, buf);
            encode_leb128_unsigned(contract.fields.len() as u64, buf);
            for (idx, val) in &contract.fields {
                buf.push(*idx);
                encode_value(val, buf)?;
            }
        }
        Value::Handle(h) => {
            buf.push(TAG_HANDLE);
            encode_leb128_unsigned(h.len() as u64, buf);
            buf.extend_from_slice(h.as_bytes());
        }
    }
    Ok(())
}

/// Decode LC-B binary to Value
pub fn decode(bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() {
        return Err(HlxError::LcBinaryDecode { 
            reason: "Empty input".to_string() 
        });
    }
    
    let mut pos = 0;
    
    // Check magic byte
    if bytes[pos] == LCB_MAGIC {
        pos += 1;
    }
    
    let (value, consumed) = decode_value(&bytes[pos..])?;
    
    // Check for trailing bytes
    if pos + consumed != bytes.len() {
        return Err(HlxError::LcBinaryDecode {
            reason: format!(
                "Trailing bytes: expected {}, got {}",
                pos + consumed,
                bytes.len()
            ),
        });
    }
    
    Ok(value)
}

/// Decode a single Value from bytes, returning (value, bytes_consumed)
pub fn decode_value(bytes: &[u8]) -> Result<(Value, usize)> {
    if bytes.is_empty() {
        return Err(HlxError::LcBinaryDecode {
            reason: "Unexpected end of input".to_string(),
        });
    }
    
    let tag = bytes[0];
    let mut pos = 1;
    
    match tag {
        TAG_NULL => Ok((Value::Null, pos)),
        
        TAG_FALSE => Ok((Value::Boolean(false), pos)),
        
        TAG_TRUE => Ok((Value::Boolean(true), pos)),
        
        TAG_INTEGER => {
            let (val, consumed) = decode_leb128_signed(&bytes[pos..])?;
            pos += consumed;
            Ok((Value::Integer(val), pos))
        }
        
        TAG_FLOAT => {
            if bytes.len() < pos + 8 {
                return Err(HlxError::LcBinaryDecode {
                    reason: "Float truncated".to_string(),
                });
            }
            let val = f64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
            
            // Validate no NaN/Inf
            if val.is_nan() || val.is_infinite() {
                return Err(HlxError::FloatSpecial);
            }
            
            pos += 8;
            Ok((Value::Float(val), pos))
        }
        
        TAG_STRING => {
            let (len, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            
            let len = len as usize;
            if bytes.len() < pos + len {
                return Err(HlxError::LcBinaryDecode {
                    reason: "String truncated".to_string(),
                });
            }
            
            let s = std::str::from_utf8(&bytes[pos..pos + len])
                .map_err(|e| HlxError::LcBinaryDecode {
                    reason: format!("Invalid UTF-8: {}", e),
                })?;
            
            pos += len;
            Ok((Value::String(s.to_string()), pos))
        }
        
        TAG_ARRAY => {
            let (count, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            
            let mut arr = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let (elem, consumed) = decode_value(&bytes[pos..])?;
                pos += consumed;
                arr.push(elem);
            }
            
            Ok((Value::from(arr), pos))
        }
        
        TAG_OBJECT => {
            let (count, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            
            let mut obj = std::collections::BTreeMap::new();
            for _ in 0..count {
                // Decode key (string without tag)
                let (key_len, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
                pos += consumed;
                
                let key_len = key_len as usize;
                if bytes.len() < pos + key_len {
                    return Err(HlxError::LcBinaryDecode {
                        reason: "Object key truncated".to_string(),
                    });
                }
                
                let key = std::str::from_utf8(&bytes[pos..pos + key_len])
                    .map_err(|e| HlxError::LcBinaryDecode {
                        reason: format!("Invalid UTF-8 in key: {}", e),
                    })?
                    .to_string();
                pos += key_len;
                
                // Decode value
                let (val, consumed) = decode_value(&bytes[pos..])?;
                pos += consumed;
                
                obj.insert(key, val);
            }
            
            Ok((Value::from(obj), pos))
        }
        
        TAG_CONTRACT => {
            let (id, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            let id = id as ContractId;
            
            let (field_count, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            
            let mut fields: Vec<(FieldIndex, Value)> = Vec::with_capacity(field_count as usize);
            let mut prev_idx: Option<FieldIndex> = None;
            
            for _ in 0..field_count {
                if bytes.len() <= pos {
                    return Err(HlxError::LcBinaryDecode {
                        reason: "Contract field index truncated".to_string(),
                    });
                }
                
                let idx = bytes[pos];
                pos += 1;
                
                // Validate field order
                if let Some(prev) = prev_idx {
                    if idx <= prev {
                        return Err(HlxError::FieldOrder);
                    }
                }
                prev_idx = Some(idx);
                
                let (val, consumed) = decode_value(&bytes[pos..])?;
                pos += consumed;
                
                fields.push((idx, val));
            }
            
            Ok((Value::Contract(Contract::new_unchecked(id, fields)), pos))
        }
        
        TAG_HANDLE => {
            let (len, consumed) = decode_leb128_unsigned(&bytes[pos..])?;
            pos += consumed;
            
            let len = len as usize;
            if bytes.len() < pos + len {
                return Err(HlxError::LcBinaryDecode {
                    reason: "Handle truncated".to_string(),
                });
            }
            
            let h = std::str::from_utf8(&bytes[pos..pos + len])
                .map_err(|e| HlxError::LcBinaryDecode {
                    reason: format!("Invalid handle encoding: {}", e),
                })?
                .to_string();
            
            pos += len;
            Ok((Value::Handle(h), pos))
        }
        
        _ => Err(HlxError::LcBinaryDecode {
            reason: format!("Unknown tag: 0x{:02x}", tag),
        }),
    }
}

// === LEB128 Encoding ===

/// Encode unsigned integer as LEB128
fn encode_leb128_unsigned(mut val: u64, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if val == 0 {
            break;
        }
    }
}

/// Encode signed integer as LEB128
fn encode_leb128_signed(mut val: i64, buf: &mut Vec<u8>) {
    let mut more = true;
    while more {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        
        // Check if we need more bytes
        let sign_bit = (byte & 0x40) != 0;
        if (val == 0 && !sign_bit) || (val == -1 && sign_bit) {
            more = false;
        } else {
            byte |= 0x80;
        }
        
        buf.push(byte);
    }
}

/// Decode unsigned LEB128, returning (value, bytes_consumed)
fn decode_leb128_unsigned(bytes: &[u8]) -> Result<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;
    
    loop {
        if pos >= bytes.len() {
            return Err(HlxError::LcBinaryDecode {
                reason: "LEB128 truncated".to_string(),
            });
        }
        
        let byte = bytes[pos];
        pos += 1;
        
        // Check for overlong encoding
        if shift >= 64 {
            return Err(HlxError::LcBinaryDecode {
                reason: "LEB128 overflow".to_string(),
            });
        }
        
        result |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        
        if (byte & 0x80) == 0 {
            break;
        }
    }
    
    Ok((result, pos))
}

/// Decode signed LEB128, returning (value, bytes_consumed)
fn decode_leb128_signed(bytes: &[u8]) -> Result<(i64, usize)> {
    let mut result: i64 = 0;
    let mut shift = 0;
    let mut pos = 0;
    let mut byte;
    
    loop {
        if pos >= bytes.len() {
            return Err(HlxError::LcBinaryDecode {
                reason: "LEB128 truncated".to_string(),
            });
        }
        
        byte = bytes[pos];
        pos += 1;
        
        if shift >= 64 {
            return Err(HlxError::LcBinaryDecode {
                reason: "LEB128 overflow".to_string(),
            });
        }
        
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;
        
        if (byte & 0x80) == 0 {
            break;
        }
    }
    
    // Sign extend if necessary
    if shift < 64 && (byte & 0x40) != 0 {
        result |= !0i64 << shift;
    }
    
    Ok((result, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_encode_decode_null() {
        let val = Value::Null;
        let encoded = encode(&val).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_encode_decode_boolean() {
        for b in [true, false] {
            let val = Value::Boolean(b);
            let encoded = encode(&val).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(val, decoded);
        }
    }

    #[test]
    fn test_encode_decode_integer() {
        for i in [0i64, 1, -1, 42, -42, 300, -300, i64::MAX, i64::MIN] {
            let val = Value::Integer(i);
            let encoded = encode(&val).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(val, decoded, "Failed for {}", i);
        }
    }

    #[test]
    fn test_encode_decode_float() {
        for f in [0.0, 1.0, -1.0, 3.14159, -2.718, 1e-10, 1e10] {
            let val = Value::float(f).unwrap();
            let encoded = encode(&val).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(val, decoded, "Failed for {}", f);
        }
    }

    #[test]
    fn test_encode_decode_string() {
        for s in ["", "hello", "cafÃ©", "æ—¥æœ¬èªž", "ðŸš€ðŸŽ‰"] {
            let val = Value::String(s.to_string());
            let encoded = encode(&val).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(val, decoded);
        }
    }

    #[test]
    fn test_encode_decode_array() {
        let val = Value::from(vec![
            Value::Integer(1),
            Value::String("two".to_string()),
            Value::Float(3.0),
        ]);
        let encoded = encode(&val).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_encode_decode_object() {
        let mut obj = BTreeMap::new();
        obj.insert("name".to_string(), Value::String("Alice".to_string()));
        obj.insert("age".to_string(), Value::Integer(30));
        let val = Value::from(obj);
        
        let encoded = encode(&val).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_encode_decode_contract() {
        let contract = Contract::new(14, vec![
            (0, Value::String("alice".to_string())),
            (1, Value::Integer(30)),
            (2, Value::Boolean(true)),
        ]).unwrap();
        let val = Value::Contract(contract);
        
        let encoded = encode(&val).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_determinism() {
        let val = Value::object([
            ("z", Value::Integer(1)),
            ("a", Value::Integer(2)),
            ("m", Value::Integer(3)),
        ]);
        
        // Encode multiple times
        let enc1 = encode(&val).unwrap();
        let enc2 = encode(&val).unwrap();
        
        // Must be byte-identical
        assert_eq!(enc1, enc2);
    }

    #[test]
    fn test_field_order_validation() {
        // Manually create invalid LC-B with wrong field order
        let mut buf = vec![LCB_MAGIC, TAG_CONTRACT];
        encode_leb128_unsigned(14, &mut buf); // contract ID
        encode_leb128_unsigned(2, &mut buf);   // 2 fields
        buf.push(5);                            // field @5 first (wrong!)
        buf.push(TAG_INTEGER);
        encode_leb128_signed(1, &mut buf);
        buf.push(0);                            // field @0 second (wrong!)
        buf.push(TAG_INTEGER);
        encode_leb128_signed(2, &mut buf);
        
        // Should fail validation
        assert!(decode(&buf).is_err());
    }

    #[test]
    fn test_leb128_roundtrip() {
        let mut buf = Vec::new();
        
        // Unsigned
        for val in [0u64, 1, 127, 128, 16384, u64::MAX] {
            buf.clear();
            encode_leb128_unsigned(val, &mut buf);
            let (decoded, _) = decode_leb128_unsigned(&buf).unwrap();
            assert_eq!(val, decoded);
        }
        
        // Signed
        for val in [0i64, 1, -1, 127, -127, 128, -128, i64::MAX, i64::MIN] {
            buf.clear();
            encode_leb128_signed(val, &mut buf);
            let (decoded, _) = decode_leb128_signed(&buf).unwrap();
            assert_eq!(val, decoded);
        }
    }
}
