// Contract Catalogue Integration for LSP

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Contract field specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractField {
    #[serde(rename = "type")]
    pub field_type: String,
    pub description: String,
    pub required: bool,
}

/// Full contract specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSpec {
    pub name: String,
    pub tier: String,
    pub signature: String,
    pub description: String,
    pub fields: HashMap<String, ContractField>,
    pub example: String,
    pub usage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<String>,
    #[serde(default)]
    pub related: Vec<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
}

/// Contract catalogue root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCatalogue {
    pub version: String,
    pub last_updated: String,
    pub total_contracts: u32,
    pub contract_id_space: String,
    pub tier_system: HashMap<String, String>,
    pub contracts: HashMap<String, ContractSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_slots: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

impl ContractCatalogue {
    /// Load contract catalogue from JSON file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let catalogue: ContractCatalogue = serde_json::from_str(&contents)?;
        Ok(catalogue)
    }

    /// Get contract by ID
    pub fn get_contract(&self, id: &str) -> Option<&ContractSpec> {
        self.contracts.get(id)
    }

    /// Get all contract IDs (sorted numerically)
    pub fn get_all_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.contracts.keys().cloned().collect();
        ids.sort_by_key(|id| id.parse::<u32>().unwrap_or(0));
        ids
    }

    /// Search contracts by name (case-insensitive)
    pub fn search_by_name(&self, query: &str) -> Vec<(&String, &ContractSpec)> {
        let query_lower = query.to_lowercase();
        self.contracts
            .iter()
            .filter(|(_, spec)| spec.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get contracts by tier
    pub fn get_by_tier(&self, tier: &str) -> Vec<(&String, &ContractSpec)> {
        self.contracts
            .iter()
            .filter(|(_, spec)| spec.tier.starts_with(tier))
            .collect()
    }

    /// Get contracts by status
    pub fn get_by_status(&self, status: &str) -> Vec<(&String, &ContractSpec)> {
        self.contracts
            .iter()
            .filter(|(_, spec)| spec.status == status)
            .collect()
    }

    /// Filter contracts by relevance for a given context
    pub fn filter_by_relevance(&self, context: &str) -> Vec<String> {
        let mut relevant_ids = Vec::new();

        for (id, spec) in &self.contracts {
            // Always exclude compiler-internal contracts
            if spec.status == "compiler-internal" {
                continue;
            }

            let id_num = id.parse::<u32>().unwrap_or(0);
            let is_relevant = match context {
                "math" => {
                    // Math operations (200-299) or numeric types (14, 15)
                    (id_num >= 200 && id_num < 300) || id_num == 14 || id_num == 15
                },
                "value" => {
                    // Value-producing contracts: types, math, strings, arrays, GPU ops
                    id_num < 100 || // All types
                    (id_num >= 200 && id_num < 700) || // Operations
                    (id_num >= 900 && id_num < 1000) // GPU ops
                },
                "control" => {
                    // Control flow (500-599) and boolean type
                    (id_num >= 500 && id_num < 600) || id_num == 17
                },
                "io" => {
                    // I/O operations (600-699) and string type
                    (id_num >= 600 && id_num < 700) || id_num == 16
                },
                "field" => {
                    // Inside braces - show types and value operations
                    id_num < 100 || // Types
                    (id_num >= 200 && id_num < 500) // Math/String/Array ops
                },
                _ => true, // "general" - show everything except compiler-internal
            };

            if is_relevant {
                relevant_ids.push(id.clone());
            }
        }

        // Sort by numeric ID
        relevant_ids.sort_by_key(|id| id.parse::<u32>().unwrap_or(0));
        relevant_ids
    }

    /// Format contract for completion item detail
    pub fn format_completion_detail(&self, id: &str) -> Option<String> {
        self.get_contract(id)
            .map(|spec| format!("{} - {}", spec.name, spec.description))
    }

    /// Generate LSP snippet for contract with field placeholders
    pub fn generate_snippet(&self, id: &str) -> Option<String> {
        self.get_contract(id).map(|spec| {
            if spec.fields.is_empty() {
                // No fields, just empty braces
                format!("@{} {{ $0 }}", id)
            } else {
                // Generate field placeholders
                let mut placeholder_idx = 1;
                let mut field_snippets = Vec::new();

                // Sort fields by required first, then alphabetically
                let mut sorted_fields: Vec<_> = spec.fields.iter().collect();
                sorted_fields.sort_by(|a, b| {
                    match (b.1.required, a.1.required) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => a.0.cmp(b.0)
                    }
                });

                for (field_name, _field_spec) in sorted_fields {
                    let placeholder = format!("{}: ${}", field_name, placeholder_idx);
                    field_snippets.push(placeholder);
                    placeholder_idx += 1;
                }

                format!("@{} {{ {} }}$0", id, field_snippets.join(", "))
            }
        })
    }

    /// Format contract for hover documentation (Markdown)
    pub fn format_hover_doc(&self, id: &str) -> Option<String> {
        self.get_contract(id).map(|spec| {
            let mut doc = String::new();

            // Header
            doc.push_str(&format!("# @{}: {}\n\n", id, spec.name));

            // Tier and status
            doc.push_str(&format!("**Tier:** {} | **Status:** {}\n\n", spec.tier, spec.status));

            // Description
            doc.push_str(&format!("{}\n\n", spec.description));

            // Signature
            doc.push_str(&format!("## Signature\n```hlx\n{}\n```\n\n", spec.signature));

            // Fields
            if !spec.fields.is_empty() {
                doc.push_str("## Fields\n\n");
                for (name, field) in &spec.fields {
                    let required = if field.required { "**required**" } else { "optional" };
                    doc.push_str(&format!("- `{}` ({}, {}): {}\n",
                        name, field.field_type, required, field.description));
                }
                doc.push_str("\n");
            }

            // Example
            doc.push_str(&format!("## Example\n```hlx\n{}\n```\n\n", spec.example));

            // Usage
            doc.push_str(&format!("## Usage\n{}\n\n", spec.usage));

            // Performance (optional)
            if let Some(perf) = &spec.performance {
                doc.push_str(&format!("## Performance\n{}\n\n", perf));
            }

            // Implementation (optional)
            if let Some(impl_path) = &spec.implementation {
                doc.push_str(&format!("**Implementation:** `{}`\n\n", impl_path));
            }

            // Related contracts
            if !spec.related.is_empty() {
                doc.push_str("## Related Contracts\n");
                for related_id in &spec.related {
                    if let Some(related_spec) = self.get_contract(related_id) {
                        doc.push_str(&format!("- @{}: {}\n", related_id, related_spec.name));
                    } else {
                        doc.push_str(&format!("- @{}\n", related_id));
                    }
                }
            }

            doc
        })
    }
}

/// Thread-safe contract catalogue cache
pub struct ContractCache {
    catalogue: Arc<ContractCatalogue>,
}

impl ContractCache {
    /// Load and cache the contract catalogue
    pub fn new(catalogue_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let catalogue = ContractCatalogue::load_from_file(catalogue_path)?;
        Ok(Self {
            catalogue: Arc::new(catalogue),
        })
    }

    /// Get reference to catalogue
    pub fn catalogue(&self) -> &ContractCatalogue {
        &self.catalogue
    }

    /// Clone the Arc (cheap)
    pub fn clone_arc(&self) -> Arc<ContractCatalogue> {
        Arc::clone(&self.catalogue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_catalogue() {
        // This will fail in tests unless CONTRACT_CATALOGUE.json is in the right place
        // But it's a good smoke test for CI
        let result = ContractCatalogue::load_from_file("../../CONTRACT_CATALOGUE.json");
        if let Ok(catalogue) = result {
            assert!(catalogue.contracts.len() > 0);
            println!("Loaded {} contracts", catalogue.total_contracts);
        }
    }

    #[test]
    fn test_contract_methods() {
        let mut catalogue = ContractCatalogue {
            version: "1.0.0".to_string(),
            last_updated: "2026-01-08".to_string(),
            total_contracts: 2,
            contract_id_space: "0-∞".to_string(),
            tier_system: HashMap::new(),
            contracts: HashMap::new(),
            open_slots: None,
            notes: None,
        };

        let spec = ContractSpec {
            name: "TestContract".to_string(),
            tier: "T0-Core".to_string(),
            signature: "@999 { }".to_string(),
            description: "Test".to_string(),
            fields: HashMap::new(),
            example: "@999 { }".to_string(),
            usage: "Testing".to_string(),
            performance: None,
            related: vec![],
            status: "stable".to_string(),
            implementation: None,
        };

        catalogue.contracts.insert("999".to_string(), spec);

        assert!(catalogue.get_contract("999").is_some());
        assert_eq!(catalogue.get_all_ids().len(), 1);
    }
}
