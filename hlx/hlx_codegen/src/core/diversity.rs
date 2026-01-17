//! Diversity Engine
//!
//! Ensures generated code has sufficient variety to be useful for training.

use std::collections::HashSet;

/// Diversity engine tracks patterns to avoid repetition
pub struct DiversityEngine {
    seen_patterns: HashSet<String>,
}

impl DiversityEngine {
    pub fn new() -> Self {
        Self {
            seen_patterns: HashSet::new(),
        }
    }

    /// Check if code pattern is diverse enough
    pub fn is_diverse(&mut self, code: &str) -> bool {
        let pattern = self.extract_pattern(code);

        if self.seen_patterns.contains(&pattern) {
            return false;
        }

        self.seen_patterns.insert(pattern);
        true
    }

    /// Extract structural pattern (abstract away specifics)
    fn extract_pattern(&self, code: &str) -> String {
        // Simple pattern: count function definitions and contracts
        let fn_count = code.matches("fn ").count();
        let contract_count = code.matches("@contract").count();

        format!("fn:{},contract:{}", fn_count, contract_count)
    }
}

impl Default for DiversityEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Diversity score
#[derive(Debug, Clone)]
pub struct DiversityScore {
    pub score: f32,
}
