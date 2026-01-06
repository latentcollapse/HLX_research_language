//! Value Store
//!
//! Content-Addressed Storage (CAS) for HLX values.
//! Handles are BLAKE3 hashes of LC-B encoded values.

use hlx_core::{Value, Result, HlxError, lcb};
use std::collections::HashMap;

/// Content-addressed storage for values
pub struct ValueStore {
    /// Handle -> Value mapping
    store: HashMap<String, Value>,
}

impl ValueStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
    
    /// Store a value and return its handle
    pub fn store(&mut self, value: Value) -> Result<String> {
        // Encode value to LC-B
        let encoded = lcb::encode(&value)?;
        
        // Compute BLAKE3 hash
        let hash = blake3::hash(&encoded);
        let handle = format!("&h_{}", hex::encode(&hash.as_bytes()[..16]));
        
        // Store if not already present (content-addressed = dedup)
        self.store.entry(handle.clone()).or_insert(value);
        
        Ok(handle)
    }
    
    /// Retrieve a value by handle
    pub fn retrieve(&self, handle: &str) -> Result<Value> {
        self.store.get(handle).cloned().ok_or_else(|| {
            HlxError::HandleNotFound { handle: handle.to_string() }
        })
    }
    
    /// Check if a handle exists
    pub fn exists(&self, handle: &str) -> bool {
        self.store.contains_key(handle)
    }
    
    /// Get the number of stored values
    pub fn len(&self) -> usize {
        self.store.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
    
    /// Clear all stored values
    pub fn clear(&mut self) {
        self.store.clear();
    }
    
    /// List all handles
    pub fn handles(&self) -> impl Iterator<Item = &String> {
        self.store.keys()
    }
}

impl Default for ValueStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_retrieve() {
        let mut store = ValueStore::new();
        
        let value = Value::String("hello world".to_string());
        let handle = store.store(value.clone()).unwrap();
        
        assert!(handle.starts_with("&h_"));
        assert!(store.exists(&handle));
        
        let retrieved = store.retrieve(&handle).unwrap();
        assert_eq!(retrieved, value);
    }

    #[test]
    fn test_content_addressing() {
        let mut store = ValueStore::new();
        
        // Same value should get same handle
        let v1 = Value::Integer(42);
        let v2 = Value::Integer(42);
        
        let h1 = store.store(v1).unwrap();
        let h2 = store.store(v2).unwrap();
        
        assert_eq!(h1, h2);
        assert_eq!(store.len(), 1); // Deduplication
    }

    #[test]
    fn test_different_values() {
        let mut store = ValueStore::new();
        
        let h1 = store.store(Value::Integer(1)).unwrap();
        let h2 = store.store(Value::Integer(2)).unwrap();
        
        assert_ne!(h1, h2);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_handle_not_found() {
        let store = ValueStore::new();
        let result = store.retrieve("&h_nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_value() {
        let mut store = ValueStore::new();
        
        let value = Value::Object(
            vec![
                ("name".to_string(), Value::String("Alice".to_string())),
                ("age".to_string(), Value::Integer(30)),
            ]
            .into_iter()
            .collect(),
        );
        
        let handle = store.store(value.clone()).unwrap();
        let retrieved = store.retrieve(&handle).unwrap();
        
        assert_eq!(retrieved, value);
    }
}
