//! Contract Suggestion Engine
//!
//! Maps natural language queries to HLX contract IDs.
//! Helps users discover contracts without memorizing IDs.

use crate::contracts::ContractCatalogue;
use std::collections::HashMap;

/// A suggested contract with relevance score
#[derive(Debug, Clone)]
pub struct ContractSuggestion {
    pub contract_id: String,
    pub contract_name: String,
    pub description: String,
    pub score: f32,
}

/// Natural language contract suggestion engine
pub struct ContractSuggestionEngine {
    /// Keyword → Contract ID mappings
    keyword_map: HashMap<String, Vec<String>>,
}

impl ContractSuggestionEngine {
    /// Create a new suggestion engine from a contract catalogue
    pub fn new(catalogue: &ContractCatalogue) -> Self {
        let mut keyword_map: HashMap<String, Vec<String>> = HashMap::new();

        // Build keyword index from all contracts
        for (id, spec) in &catalogue.contracts {
            // Skip compiler-internal contracts
            if spec.status == "compiler-internal" {
                continue;
            }

            // Extract keywords from name
            let name_keywords = Self::extract_keywords(&spec.name.to_lowercase());
            for keyword in &name_keywords {
                keyword_map.entry(keyword.clone())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }

            // Extract keywords from description
            let desc_keywords = Self::extract_keywords(&spec.description.to_lowercase());
            for keyword in &desc_keywords {
                keyword_map.entry(keyword.clone())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }

            // Extract keywords from usage
            let usage_keywords = Self::extract_keywords(&spec.usage.to_lowercase());
            for keyword in &usage_keywords {
                keyword_map.entry(keyword.clone())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }

            // Add tier-specific keywords
            if spec.tier.starts_with("T4") {
                keyword_map.entry("gpu".to_string())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }
        }

        Self { keyword_map }
    }

    /// Suggest contracts based on natural language query
    /// Returns top N suggestions sorted by relevance
    pub fn suggest(&self, query: &str, catalogue: &ContractCatalogue, max_results: usize) -> Vec<ContractSuggestion> {
        let query_lower = query.to_lowercase();
        let query_keywords = Self::extract_keywords(&query_lower);

        // Score each contract based on keyword matches
        let mut scores: HashMap<String, f32> = HashMap::new();

        for keyword in &query_keywords {
            if let Some(contract_ids) = self.keyword_map.get(keyword) {
                for contract_id in contract_ids {
                    *scores.entry(contract_id.clone()).or_insert(0.0) += 1.0;
                }
            }

            // Fuzzy matching for common misspellings
            for (map_keyword, contract_ids) in &self.keyword_map {
                if Self::is_similar(keyword, map_keyword) {
                    for contract_id in contract_ids {
                        *scores.entry(contract_id.clone()).or_insert(0.0) += 0.5;
                    }
                }
            }
        }

        // Add specific intent matching
        self.add_intent_scores(&query_lower, &mut scores);

        // Convert to suggestions and sort by score
        let mut suggestions: Vec<ContractSuggestion> = scores
            .into_iter()
            .filter_map(|(id, score)| {
                catalogue.get_contract(&id).map(|spec| ContractSuggestion {
                    contract_id: id.clone(),
                    contract_name: spec.name.clone(),
                    description: spec.description.clone(),
                    score,
                })
            })
            .collect();

        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        suggestions.truncate(max_results);

        suggestions
    }

    /// Extract meaningful keywords from text
    fn extract_keywords(text: &str) -> Vec<String> {
        // Common stop words to ignore
        let stop_words: Vec<&str> = vec![
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "can", "of", "at", "by", "for",
            "with", "about", "against", "between", "into", "through", "during",
            "before", "after", "above", "below", "to", "from", "up", "down",
            "in", "out", "on", "off", "over", "under", "again", "further",
            "then", "once", "here", "there", "when", "where", "why", "how",
            "all", "both", "each", "few", "more", "most", "other", "some",
            "such", "no", "nor", "not", "only", "own", "same", "so", "than",
            "too", "very", "this", "that", "these", "those",
        ];

        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| {
                !word.is_empty() &&
                word.len() > 2 &&
                !stop_words.contains(&word)
            })
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if two keywords are similar (simple edit distance)
    fn is_similar(a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }

        // Must be close in length
        let len_diff = (a.len() as i32 - b.len() as i32).abs();
        if len_diff > 2 {
            return false;
        }

        // Simple substring check
        if a.len() >= 4 && b.len() >= 4 {
            if a.contains(b) || b.contains(a) {
                return true;
            }
        }

        false
    }

    /// Add intent-based scoring for common patterns
    fn add_intent_scores(&self, query: &str, scores: &mut HashMap<String, f32>) {
        // Math operations
        if query.contains("add") || query.contains("plus") || query.contains("sum") {
            *scores.entry("200".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("subtract") || query.contains("minus") || query.contains("sub") {
            *scores.entry("201".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("multiply") || query.contains("times") || query.contains("mul") {
            *scores.entry("202".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("divide") || query.contains("div") {
            *scores.entry("203".to_string()).or_insert(0.0) += 2.0;
        }

        // Matrix/tensor operations
        if (query.contains("matrix") || query.contains("matrices")) &&
           (query.contains("multiply") || query.contains("mul") || query.contains("product")) {
            *scores.entry("906".to_string()).or_insert(0.0) += 3.0;
        }

        // Neural network operations
        if query.contains("layer") && query.contains("norm") {
            *scores.entry("907".to_string()).or_insert(0.0) += 3.0;
        }
        if query.contains("gelu") || query.contains("activation") {
            *scores.entry("908".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("softmax") {
            *scores.entry("909".to_string()).or_insert(0.0) += 3.0;
        }

        // String operations
        if query.contains("concat") || (query.contains("join") && query.contains("string")) {
            *scores.entry("300".to_string()).or_insert(0.0) += 2.0;
        }

        // Array operations
        if query.contains("array") {
            if query.contains("length") || query.contains("size") {
                *scores.entry("400".to_string()).or_insert(0.0) += 2.0;
            }
            if query.contains("get") || query.contains("access") || query.contains("index") {
                *scores.entry("401".to_string()).or_insert(0.0) += 2.0;
            }
            if query.contains("push") || query.contains("append") || query.contains("add") {
                *scores.entry("403".to_string()).or_insert(0.0) += 2.0;
            }
        }

        // I/O operations
        if query.contains("print") || query.contains("output") || query.contains("display") {
            *scores.entry("600".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("read") && query.contains("file") {
            *scores.entry("602".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("write") && query.contains("file") {
            *scores.entry("603".to_string()).or_insert(0.0) += 2.0;
        }
        if query.contains("http") || query.contains("request") || query.contains("api") {
            *scores.entry("603".to_string()).or_insert(0.0) += 2.0; // HTTP request
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ContractSpec, ContractField};
    use std::collections::HashMap;

    fn create_test_catalogue() -> ContractCatalogue {
        let mut contracts = HashMap::new();

        // Add test contracts
        contracts.insert("200".to_string(), ContractSpec {
            name: "Add".to_string(),
            tier: "T2-Reserved".to_string(),
            signature: "@200 { lhs, rhs }".to_string(),
            description: "Add two numbers".to_string(),
            fields: HashMap::new(),
            example: "@200 { lhs: 5, rhs: 3 }".to_string(),
            usage: "Addition operation for integers and floats".to_string(),
            performance: None,
            related: vec![],
            status: "stable".to_string(),
            implementation: None,
        });

        contracts.insert("906".to_string(), ContractSpec {
            name: "GEMM".to_string(),
            tier: "T4-GPU".to_string(),
            signature: "@906 { A, B }".to_string(),
            description: "Matrix multiplication".to_string(),
            fields: HashMap::new(),
            example: "@906 { A: matrix_a, B: matrix_b }".to_string(),
            usage: "General matrix multiply for neural networks".to_string(),
            performance: Some("O(M×N×K), GPU-accelerated".to_string()),
            related: vec![],
            status: "stable".to_string(),
            implementation: None,
        });

        ContractCatalogue {
            version: "1.0.0".to_string(),
            last_updated: "2026-01-08".to_string(),
            total_contracts: 2,
            contract_id_space: "0-∞".to_string(),
            tier_system: HashMap::new(),
            contracts,
            open_slots: None,
            notes: None,
        }
    }

    #[test]
    fn test_suggest_addition() {
        let catalogue = create_test_catalogue();
        let engine = ContractSuggestionEngine::new(&catalogue);

        let suggestions = engine.suggest("add two numbers", &catalogue, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].contract_id, "200");
    }

    #[test]
    fn test_suggest_matrix_multiply() {
        let catalogue = create_test_catalogue();
        let engine = ContractSuggestionEngine::new(&catalogue);

        let suggestions = engine.suggest("multiply matrices", &catalogue, 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].contract_id, "906");
    }

    #[test]
    fn test_keyword_extraction() {
        let keywords = ContractSuggestionEngine::extract_keywords("add two numbers together");
        assert!(keywords.contains(&"add".to_string()));
        assert!(keywords.contains(&"numbers".to_string()));
        assert!(!keywords.contains(&"two".to_string())); // Stop word
    }
}
