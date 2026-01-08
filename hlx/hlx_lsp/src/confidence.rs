// Confidence Scoring for AI-Generated Code
// Analyzes code and provides confidence scores to help AI models self-correct

use tower_lsp::lsp_types::*;
use crate::contracts::ContractCatalogue;

/// Confidence level for code correctness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    High,      // 90-100%: Very likely correct
    Medium,    // 60-89%: Probably correct, but check
    Low,       // 30-59%: Likely issues, review carefully
    VeryLow,   // 0-29%: Almost certainly wrong
}

impl ConfidenceLevel {
    pub fn score(&self) -> u8 {
        match self {
            Self::High => 95,
            Self::Medium => 75,
            Self::Low => 45,
            Self::VeryLow => 15,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::High => "✓",
            Self::Medium => "⚠",
            Self::Low => "⚠⚠",
            Self::VeryLow => "✗",
        }
    }
}

/// Code confidence analyzer
pub struct ConfidenceAnalyzer<'a> {
    catalogue: &'a ContractCatalogue,
}

impl<'a> ConfidenceAnalyzer<'a> {
    pub fn new(catalogue: &'a ContractCatalogue) -> Self {
        Self { catalogue }
    }

    /// Analyze a line of code and return confidence score
    pub fn analyze_line(&self, line: &str) -> (ConfidenceLevel, Vec<String>) {
        let mut confidence = 100;
        let mut issues = Vec::new();

        // Check for contract usage
        if line.contains('@') {
            let contract_confidence = self.check_contract_usage(line);
            confidence = std::cmp::min(confidence, contract_confidence.0);
            issues.extend(contract_confidence.1);
        }

        // Check variable naming patterns
        let naming_confidence = self.check_naming_patterns(line);
        confidence = std::cmp::min(confidence, naming_confidence.0);
        issues.extend(naming_confidence.1);

        // Check for common mistakes
        let mistake_confidence = self.check_common_mistakes(line);
        confidence = std::cmp::min(confidence, mistake_confidence.0);
        issues.extend(mistake_confidence.1);

        let level = match confidence {
            90..=100 => ConfidenceLevel::High,
            60..=89 => ConfidenceLevel::Medium,
            30..=59 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        };

        (level, issues)
    }

    /// Check contract usage patterns
    fn check_contract_usage(&self, line: &str) -> (i32, Vec<String>) {
        let mut confidence = 100;
        let mut issues = Vec::new();

        // Find @ID patterns
        for (i, _) in line.match_indices('@') {
            let remaining = &line[i+1..];
            let id_end = remaining.find(|c: char| !c.is_numeric()).unwrap_or(remaining.len());

            if id_end == 0 {
                continue; // Not a contract
            }

            let contract_id = &remaining[..id_end];

            // Check if contract exists
            if self.catalogue.get_contract(contract_id).is_none() {
                confidence -= 40;
                issues.push(format!("Contract @{} not found in catalogue", contract_id));
            }
        }

        (confidence, issues)
    }

    /// Check variable naming patterns
    fn check_naming_patterns(&self, line: &str) -> (i32, Vec<String>) {
        let mut confidence = 100;
        let mut issues = Vec::new();

        // Check for suspicious variable names
        if line.contains("let") {
            // Extract variable name
            if let Some(let_pos) = line.find("let ") {
                let after_let = &line[let_pos + 4..];
                if let Some(eq_pos) = after_let.find('=') {
                    let var_name = after_let[..eq_pos].trim();

                    // Check if name suggests type mismatch
                    if var_name.starts_with("tensor") && !line.contains("@22") && !line.contains("@906") {
                        confidence -= 15;
                        issues.push(format!(
                            "Variable '{}' suggests Tensor type, but no Tensor contract used",
                            var_name
                        ));
                    }

                    if var_name.starts_with("matrix") && !line.contains("@906") && !line.contains("@22") {
                        confidence -= 15;
                        issues.push(format!(
                            "Variable '{}' suggests Matrix, consider @906 (GEMM) or @22 (Tensor)",
                            var_name
                        ));
                    }

                    if var_name.starts_with("arr") && !line.contains("@18") {
                        confidence -= 10;
                        issues.push(format!(
                            "Variable '{}' suggests Array, but @18 not used",
                            var_name
                        ));
                    }
                }
            }
        }

        (confidence, issues)
    }

    /// Check for common AI mistakes
    fn check_common_mistakes(&self, line: &str) -> (i32, Vec<String>) {
        let mut confidence = 100;
        let mut issues = Vec::new();

        // Check for undefined DEFAULT_MAX_ITER usage
        if line.contains("loop") && !line.contains("DEFAULT_MAX_ITER") {
            confidence -= 20;
            issues.push("Loop without DEFAULT_MAX_ITER bound (safety requirement)".to_string());
        }

        // Check for common typos
        if line.contains("@906 {") {
            if line.contains("C:") && !line.contains("A:") && !line.contains("B:") {
                confidence -= 30;
                issues.push("@906 (GEMM) uses fields 'A' and 'B', not 'C'".to_string());
            }
        }

        // Check for missing semicolons
        if (line.contains("let ") || line.contains("return ")) && line.trim().ends_with('}') {
            confidence -= 10;
            issues.push("Statement might be missing semicolon".to_string());
        }

        (confidence, issues)
    }

    /// Create diagnostic with confidence annotation
    pub fn create_confidence_diagnostic(&self, line_idx: usize, line: &str) -> Option<Diagnostic> {
        let (level, issues) = self.analyze_line(line);

        // Only create diagnostic for low/very low confidence
        if level == ConfidenceLevel::High || level == ConfidenceLevel::Medium {
            return None;
        }

        let message = format!(
            "{} Confidence: {}%\n\nPotential issues:\n{}",
            level.icon(),
            level.score(),
            issues.iter().map(|i| format!("  • {}", i)).collect::<Vec<_>>().join("\n")
        );

        Some(Diagnostic {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: 0,
                },
                end: Position {
                    line: line_idx as u32,
                    character: line.len() as u32,
                },
            },
            severity: Some(if level == ConfidenceLevel::VeryLow {
                DiagnosticSeverity::WARNING
            } else {
                DiagnosticSeverity::INFORMATION
            }),
            code: Some(NumberOrString::String("low-confidence".to_string())),
            source: Some("hlx-confidence".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::contracts::{ContractSpec, ContractField};

    fn create_test_catalogue() -> ContractCatalogue {
        let mut contracts = HashMap::new();

        // Add @906 (GEMM)
        contracts.insert("906".to_string(), ContractSpec {
            name: "GEMM".to_string(),
            tier: "T4-GPU".to_string(),
            signature: "@906 { A, B }".to_string(),
            description: "Matrix multiply".to_string(),
            fields: {
                let mut f = HashMap::new();
                f.insert("A".to_string(), ContractField {
                    field_type: "Tensor".to_string(),
                    description: "Left matrix".to_string(),
                    required: true,
                });
                f.insert("B".to_string(), ContractField {
                    field_type: "Tensor".to_string(),
                    description: "Right matrix".to_string(),
                    required: true,
                });
                f
            },
            example: "let C = @906 { A: a, B: b };".to_string(),
            usage: "Matrix multiplication".to_string(),
            performance: None,
            related: vec![],
            status: "stable".to_string(),
            implementation: None,
        });

        ContractCatalogue {
            version: "1.0.0".to_string(),
            last_updated: "2026-01-08".to_string(),
            total_contracts: 1,
            contract_id_space: "0-∞".to_string(),
            tier_system: HashMap::new(),
            contracts,
            open_slots: None,
            notes: None,
        }
    }

    #[test]
    fn test_confidence_high() {
        let catalogue = create_test_catalogue();
        let analyzer = ConfidenceAnalyzer::new(&catalogue);

        let (level, _) = analyzer.analyze_line("let result = @906 { A: matrix_a, B: matrix_b };");
        assert_eq!(level, ConfidenceLevel::High);
    }

    #[test]
    fn test_confidence_low_typo() {
        let catalogue = create_test_catalogue();
        let analyzer = ConfidenceAnalyzer::new(&catalogue);

        let (level, issues) = analyzer.analyze_line("let result = @906 { C: wrong };");
        assert!(level == ConfidenceLevel::Low || level == ConfidenceLevel::VeryLow);
        assert!(issues.iter().any(|i| i.contains("C")));
    }

    #[test]
    fn test_confidence_naming_mismatch() {
        let catalogue = create_test_catalogue();
        let analyzer = ConfidenceAnalyzer::new(&catalogue);

        let (level, issues) = analyzer.analyze_line("let tensor_a = @14 { @0: 42 };");
        assert_eq!(level, ConfidenceLevel::Medium);
        assert!(issues.iter().any(|i| i.contains("tensor")));
    }
}
