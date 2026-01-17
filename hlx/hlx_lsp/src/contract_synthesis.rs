//! Contract Synthesis from Natural Language
//!
//! Generates HLX contracts from natural language descriptions.
//! This is a unique AI-native feature that allows developers to
//! describe what they want and get contract code automatically.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A synthesized contract with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedContract {
    /// The contract code
    pub code: String,
    /// Explanation of what it does
    pub explanation: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Suggested contract ID
    pub suggested_id: Option<String>,
    /// Parameters that need to be filled in
    pub parameters: Vec<ContractParameter>,
}

/// A parameter in a synthesized contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractParameter {
    pub name: String,
    pub description: String,
    pub type_hint: String,
    pub example: String,
}

/// Natural language contract synthesis engine
pub struct ContractSynthesizer {
    /// Common patterns for synthesis
    patterns: Vec<SynthesisPattern>,
    /// Contract templates
    templates: HashMap<String, String>,
}

/// A pattern for recognizing synthesis intents
#[derive(Debug, Clone)]
struct SynthesisPattern {
    keywords: Vec<String>,
    template_name: String,
    confidence: f32,
}

impl ContractSynthesizer {
    pub fn new() -> Self {
        let mut synthesizer = Self {
            patterns: Vec::new(),
            templates: HashMap::new(),
        };

        synthesizer.initialize_patterns();
        synthesizer.initialize_templates();
        synthesizer
    }

    /// Initialize synthesis patterns
    fn initialize_patterns(&mut self) {
        // Pattern: validation/checking
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "validate".to_string(),
                "check".to_string(),
                "verify".to_string(),
                "ensure".to_string(),
            ],
            template_name: "validation".to_string(),
            confidence: 0.9,
        });

        // Pattern: transformation/conversion
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "convert".to_string(),
                "transform".to_string(),
                "map".to_string(),
                "change".to_string(),
            ],
            template_name: "transformation".to_string(),
            confidence: 0.85,
        });

        // Pattern: computation/calculation
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "calculate".to_string(),
                "compute".to_string(),
                "sum".to_string(),
                "average".to_string(),
            ],
            template_name: "computation".to_string(),
            confidence: 0.9,
        });

        // Pattern: filtering/selection
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "filter".to_string(),
                "select".to_string(),
                "find".to_string(),
                "search".to_string(),
            ],
            template_name: "filtering".to_string(),
            confidence: 0.85,
        });

        // Pattern: aggregation
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "aggregate".to_string(),
                "combine".to_string(),
                "merge".to_string(),
                "collect".to_string(),
            ],
            template_name: "aggregation".to_string(),
            confidence: 0.8,
        });

        // Pattern: LSTX operations
        self.patterns.push(SynthesisPattern {
            keywords: vec![
                "latent".to_string(),
                "lstx".to_string(),
                "semantic".to_string(),
                "embedding".to_string(),
            ],
            template_name: "lstx".to_string(),
            confidence: 0.95,
        });
    }

    /// Initialize contract templates
    fn initialize_templates(&mut self) {
        // Validation template
        self.templates.insert(
            "validation".to_string(),
            r#"@contract validation {
    input: ${1:value},
    rules: [
        ${2:rule1},
        ${3:rule2}
    ],
    on_error: "${4:error_message}"
}"#.to_string(),
        );

        // Transformation template
        self.templates.insert(
            "transformation".to_string(),
            r#"@contract transform {
    input: ${1:source},
    mapping: ${2:transformation_fn},
    output_type: "${3:target_type}"
}"#.to_string(),
        );

        // Computation template
        self.templates.insert(
            "computation".to_string(),
            r#"@contract compute {
    inputs: [${1:input1}, ${2:input2}],
    operation: "${3:operation}",
    precision: ${4:decimal_places}
}"#.to_string(),
        );

        // Filtering template
        self.templates.insert(
            "filtering".to_string(),
            r#"@contract filter {
    collection: ${1:data},
    predicate: ${2:condition},
    max_results: ${3:limit}
}"#.to_string(),
        );

        // Aggregation template
        self.templates.insert(
            "aggregation".to_string(),
            r#"@contract aggregate {
    sources: [${1:source1}, ${2:source2}],
    strategy: "${3:merge_strategy}",
    output_format: "${4:format}"
}"#.to_string(),
        );

        // LSTX template
        self.templates.insert(
            "lstx".to_string(),
            r#"@lstx {
    table: "${1:table_name}",
    namespace: "${2:namespace}",
    value: ${3:value},
    operations: [
        { op: "${4:operation}", params: ${5:params} }
    ]
}"#.to_string(),
        );
    }

    /// Synthesize a contract from natural language description
    pub fn synthesize(&self, description: &str) -> Option<SynthesizedContract> {
        // Find best matching pattern
        let (pattern, confidence) = self.find_best_pattern(description)?;

        // Get template
        let template = self.templates.get(&pattern.template_name)?;

        // Extract parameters from description
        let parameters = self.extract_parameters(description, &pattern.template_name);

        // Generate explanation
        let explanation = self.generate_explanation(description, &pattern.template_name);

        Some(SynthesizedContract {
            code: template.clone(),
            explanation,
            confidence,
            suggested_id: None, // User can assign ID
            parameters,
        })
    }

    /// Find best matching pattern for description
    fn find_best_pattern(&self, description: &str) -> Option<(&SynthesisPattern, f32)> {
        let desc_lower = description.to_lowercase();
        let mut best_match: Option<(&SynthesisPattern, f32)> = None;

        for pattern in &self.patterns {
            let mut score = 0.0;
            let mut matches = 0;

            for keyword in &pattern.keywords {
                if desc_lower.contains(keyword) {
                    matches += 1;
                    score += pattern.confidence;
                }
            }

            if matches > 0 {
                let avg_score = score / matches as f32;
                if best_match.is_none() || avg_score > best_match.unwrap().1 {
                    best_match = Some((pattern, avg_score));
                }
            }
        }

        best_match
    }

    /// Extract parameters from description
    fn extract_parameters(&self, description: &str, template: &str) -> Vec<ContractParameter> {
        let mut params = Vec::new();

        match template {
            "validation" => {
                params.push(ContractParameter {
                    name: "value".to_string(),
                    description: "The value to validate".to_string(),
                    type_hint: "Any".to_string(),
                    example: "user_input".to_string(),
                });
                params.push(ContractParameter {
                    name: "rules".to_string(),
                    description: "Validation rules to apply".to_string(),
                    type_hint: "Array<Rule>".to_string(),
                    example: r#"["not_empty", "valid_email"]"#.to_string(),
                });
            }
            "transformation" => {
                params.push(ContractParameter {
                    name: "source".to_string(),
                    description: "The input to transform".to_string(),
                    type_hint: "Any".to_string(),
                    example: "raw_data".to_string(),
                });
                params.push(ContractParameter {
                    name: "transformation_fn".to_string(),
                    description: "The transformation function".to_string(),
                    type_hint: "Function".to_string(),
                    example: "x => x * 2".to_string(),
                });
            }
            "computation" => {
                params.push(ContractParameter {
                    name: "inputs".to_string(),
                    description: "Values to compute with".to_string(),
                    type_hint: "Array<Number>".to_string(),
                    example: "[a, b, c]".to_string(),
                });
                params.push(ContractParameter {
                    name: "operation".to_string(),
                    description: "Operation to perform".to_string(),
                    type_hint: "String".to_string(),
                    example: r#""sum""#.to_string(),
                });
            }
            "filtering" => {
                params.push(ContractParameter {
                    name: "collection".to_string(),
                    description: "The collection to filter".to_string(),
                    type_hint: "Array".to_string(),
                    example: "items".to_string(),
                });
                params.push(ContractParameter {
                    name: "predicate".to_string(),
                    description: "Filter condition".to_string(),
                    type_hint: "Function".to_string(),
                    example: "x => x > 0".to_string(),
                });
            }
            "lstx" => {
                params.push(ContractParameter {
                    name: "table_name".to_string(),
                    description: "Latent space table".to_string(),
                    type_hint: "String".to_string(),
                    example: r#""embeddings""#.to_string(),
                });
                params.push(ContractParameter {
                    name: "operation".to_string(),
                    description: "Latent space operation".to_string(),
                    type_hint: "String".to_string(),
                    example: r#""query""#.to_string(),
                });
            }
            _ => {}
        }

        params
    }

    /// Generate explanation for synthesized contract
    fn generate_explanation(&self, description: &str, template: &str) -> String {
        match template {
            "validation" => format!(
                "This contract validates input based on your requirements: '{}'. \
                 Fill in the validation rules and error message.",
                description
            ),
            "transformation" => format!(
                "This contract transforms data according to: '{}'. \
                 Specify the source and transformation function.",
                description
            ),
            "computation" => format!(
                "This contract performs computation: '{}'. \
                 Provide the input values and operation type.",
                description
            ),
            "filtering" => format!(
                "This contract filters data based on: '{}'. \
                 Define the collection and filter condition.",
                description
            ),
            "aggregation" => format!(
                "This contract aggregates data: '{}'. \
                 Specify data sources and merge strategy.",
                description
            ),
            "lstx" => format!(
                "This contract performs latent space operations: '{}'. \
                 Configure the table, namespace, and operations.",
                description
            ),
            _ => format!("Contract based on: '{}'", description),
        }
    }

    /// Suggest improvements to existing contract
    pub fn suggest_improvements(&self, contract_code: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check for missing error handling
        if !contract_code.contains("on_error") && !contract_code.contains("catch") {
            suggestions.push("Consider adding error handling with 'on_error' field".to_string());
        }

        // Check for hardcoded values
        if contract_code.contains('"') && contract_code.matches('"').count() > 4 {
            suggestions.push("Consider parameterizing hardcoded strings".to_string());
        }

        // Check for complex nesting
        let brace_depth = contract_code.matches('{').count();
        if brace_depth > 3 {
            suggestions.push("Contract has deep nesting - consider breaking into smaller contracts".to_string());
        }

        suggestions
    }

    /// Convert user intent to contract suggestion
    pub fn intent_to_contract(&self, intent: &str) -> Vec<SynthesizedContract> {
        let mut results = Vec::new();

        // Try direct synthesis
        if let Some(contract) = self.synthesize(intent) {
            results.push(contract);
        }

        // Try variations if confidence is low
        if results.is_empty() || results[0].confidence < 0.7 {
            // Try adding context words
            for context in &["validate", "transform", "compute"] {
                let enhanced = format!("{} {}", context, intent);
                if let Some(contract) = self.synthesize(&enhanced) {
                    results.push(contract);
                }
            }
        }

        results
    }
}

impl Default for ContractSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_synthesis() {
        let synthesizer = ContractSynthesizer::new();
        let result = synthesizer.synthesize("validate email address");

        assert!(result.is_some());
        let contract = result.unwrap();
        assert!(contract.code.contains("@contract"));
        assert!(contract.confidence > 0.5);
    }

    #[test]
    fn test_transformation_synthesis() {
        let synthesizer = ContractSynthesizer::new();
        let result = synthesizer.synthesize("transform data to uppercase");

        assert!(result.is_some());
        let contract = result.unwrap();
        assert!(contract.code.contains("transform"));
    }

    #[test]
    fn test_lstx_synthesis() {
        let synthesizer = ContractSynthesizer::new();
        let result = synthesizer.synthesize("perform latent space query");

        assert!(result.is_some());
        let contract = result.unwrap();
        assert!(contract.code.contains("@lstx"));
        assert!(contract.confidence > 0.8);
    }

    #[test]
    fn test_parameter_extraction() {
        let synthesizer = ContractSynthesizer::new();
        let params = synthesizer.extract_parameters("validate email", "validation");

        assert!(!params.is_empty());
        assert!(params.iter().any(|p| p.name == "value"));
    }

    #[test]
    fn test_improvement_suggestions() {
        let synthesizer = ContractSynthesizer::new();
        let suggestions = synthesizer.suggest_improvements("@contract { value: 123 }");

        assert!(!suggestions.is_empty());
    }
}
