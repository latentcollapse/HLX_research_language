// AI-Optimized Diagnostic Messages
// Provides rich, teaching-focused error messages that help AI models learn patterns

use tower_lsp::lsp_types::*;
use crate::contracts::ContractCatalogue;
use std::collections::HashMap;

/// Enhanced diagnostic with AI-focused teaching context
pub struct AIDiagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: String,
    pub teaching_context: Option<String>,
    pub common_mistake: Option<String>,
    pub suggested_fix: Option<String>,
    pub related_contracts: Vec<String>,
}

impl AIDiagnostic {
    /// Convert to LSP Diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        let mut full_message = self.message.clone();

        if let Some(teaching) = &self.teaching_context {
            full_message.push_str(&format!("\n\n{}", teaching));
        }

        if let Some(mistake) = &self.common_mistake {
            full_message.push_str(&format!("\n\n🤖 AI models often: {}", mistake));
        }

        if let Some(fix) = &self.suggested_fix {
            full_message.push_str(&format!("\n\n✓ Fix:\n{}", fix));
        }

        if !self.related_contracts.is_empty() {
            full_message.push_str(&format!("\n\n🔗 Related: {}",
                self.related_contracts.join(", ")));
        }

        Diagnostic {
            range: self.range,
            severity: Some(self.severity),
            code: Some(NumberOrString::String(self.code.clone())),
            source: Some("hlx-ai".to_string()),
            message: full_message,
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        }
    }
}

/// AI diagnostic builder with teaching context
pub struct AIDiagnosticBuilder<'a> {
    catalogue: &'a ContractCatalogue,
    common_mistakes: HashMap<String, CommonMistake>,
}

struct CommonMistake {
    description: String,
    why_happens: String,
}

impl<'a> AIDiagnosticBuilder<'a> {
    pub fn new(catalogue: &'a ContractCatalogue) -> Self {
        let mut common_mistakes = HashMap::new();

        // Populate common AI mistakes database
        common_mistakes.insert(
            "unknown_field".to_string(),
            CommonMistake {
                description: "Use field name that doesn't exist in contract".to_string(),
                why_happens: "Confuse similar contracts or hallucinate field names from natural language".to_string(),
            }
        );

        common_mistakes.insert(
            "missing_required".to_string(),
            CommonMistake {
                description: "Omit required fields".to_string(),
                why_happens: "Treat all fields as optional or forget requirements from docs".to_string(),
            }
        );

        common_mistakes.insert(
            "wrong_contract".to_string(),
            CommonMistake {
                description: "Use wrong contract ID for intended operation".to_string(),
                why_happens: "Remember contract numbers incorrectly or confuse similar operations".to_string(),
            }
        );

        Self {
            catalogue,
            common_mistakes,
        }
    }

    /// Create diagnostic for unknown field error
    pub fn unknown_field(
        &self,
        range: Range,
        field_name: &str,
        contract_id: &str,
        valid_fields: &[String],
    ) -> AIDiagnostic {
        let spec = self.catalogue.get_contract(contract_id);
        let contract_name = spec.map(|s| s.name.as_str()).unwrap_or("Unknown");

        let teaching_context = format!(
            "Contract @{} ({}) expects specific field names.\n\
             Valid fields: {}",
            contract_id,
            contract_name,
            valid_fields.iter().map(|f| format!("'{}'", f)).collect::<Vec<_>>().join(", ")
        );

        // Check for typos
        let suggested_fix = self.suggest_field_correction(field_name, valid_fields, contract_id);

        // Find related contracts
        let related_contracts = self.find_contracts_with_field(field_name);

        let common_mistake = self.common_mistakes.get("unknown_field")
            .map(|m| m.why_happens.clone());

        AIDiagnostic {
            range,
            severity: DiagnosticSeverity::ERROR,
            message: format!(
                "Unknown field '{}' for contract @{} ({})",
                field_name, contract_id, contract_name
            ),
            code: "unknown-field".to_string(),
            teaching_context: Some(teaching_context),
            common_mistake,
            suggested_fix: Some(suggested_fix),
            related_contracts,
        }
    }

    /// Create diagnostic for missing required field
    pub fn missing_required_field(
        &self,
        range: Range,
        field_name: &str,
        contract_id: &str,
    ) -> AIDiagnostic {
        let spec = self.catalogue.get_contract(contract_id);
        let contract_name = spec.map(|s| s.name.as_str()).unwrap_or("Unknown");

        let field_type = spec
            .and_then(|s| s.fields.get(field_name))
            .map(|f| f.field_type.as_str())
            .unwrap_or("Unknown");

        let teaching_context = format!(
            "Contract @{} ({}) requires field '{}' of type {}.\n\
             This field is mandatory and must be provided.",
            contract_id, contract_name, field_name, field_type
        );

        let suggested_fix = if let Some(s) = spec {
            format!(
                "@{} {{ {}: <value> }}\n\nExample:\n{}",
                contract_id,
                field_name,
                s.example
            )
        } else {
            format!("@{} {{ {}: <value> }}", contract_id, field_name)
        };

        let common_mistake = self.common_mistakes.get("missing_required")
            .map(|m| m.why_happens.clone());

        AIDiagnostic {
            range,
            severity: DiagnosticSeverity::WARNING,
            message: format!(
                "Missing required field '{}' for contract @{} ({})",
                field_name, contract_id, contract_name
            ),
            code: "missing-required-field".to_string(),
            teaching_context: Some(teaching_context),
            common_mistake,
            suggested_fix: Some(suggested_fix),
            related_contracts: vec![],
        }
    }

    /// Suggest correction for typo'd field name
    fn suggest_field_correction(
        &self,
        wrong_field: &str,
        valid_fields: &[String],
        contract_id: &str,
    ) -> String {
        let closest = valid_fields
            .iter()
            .min_by_key(|f| self.levenshtein_distance(wrong_field, f));

        if let Some(closest_field) = closest {
            let distance = self.levenshtein_distance(wrong_field, closest_field);

            if distance <= 2 {
                return format!(
                    "Did you mean '{}'?\n@{} {{ {}: <value> }}",
                    closest_field, contract_id, closest_field
                );
            }
        }

        if let Some(spec) = self.catalogue.get_contract(contract_id) {
            format!("Correct usage:\n{}", spec.example)
        } else {
            "Use autocomplete (Ctrl+Space) to see valid fields".to_string()
        }
    }

    /// Find contracts that have this field name
    fn find_contracts_with_field(&self, field_name: &str) -> Vec<String> {
        let mut contracts = Vec::new();

        for (id, spec) in &self.catalogue.contracts {
            if spec.fields.contains_key(field_name) {
                contracts.push(format!("@{} ({})", id, spec.name));
                if contracts.len() >= 3 {
                    break;
                }
            }
        }

        contracts
    }

    /// Simple Levenshtein distance for typo detection
    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(
                        matrix[i - 1][j] + 1,
                        matrix[i][j - 1] + 1
                    ),
                    matrix[i - 1][j - 1] + cost
                );
            }
        }

        matrix[a_len][b_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        // Create a minimal valid catalogue for testing
        let catalogue = ContractCatalogue {
            version: String::from("test"),
            last_updated: String::from("2026-01-12"),
            total_contracts: 0,
            contract_id_space: String::from("test"),
            tier_system: HashMap::new(),
            contracts: HashMap::new(),
            open_slots: None,
            notes: None,
        };

        let builder = AIDiagnosticBuilder {
            catalogue: &catalogue,
            common_mistakes: HashMap::new(),
        };

        assert_eq!(builder.levenshtein_distance("lhs", "rhs"), 1);
        assert_eq!(builder.levenshtein_distance("matrix", "matrixx"), 1);
        assert_eq!(builder.levenshtein_distance("A", "B"), 1);
    }
}
