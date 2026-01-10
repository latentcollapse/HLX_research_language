//! Performance Lens
//!
//! Shows execution cost estimates inline as you code.
//! Helps identify bottlenecks and optimize before running.

use tower_lsp::lsp_types::*;
use crate::contracts::ContractCatalogue;

/// Performance cost estimate
#[derive(Debug, Clone)]
pub struct PerformanceCost {
    pub range: Range,
    pub cost_ms: f64,
    pub tier: String,
    pub description: String,
    pub severity: CostSeverity,
}

/// Cost severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum CostSeverity {
    Fast,      // < 1ms
    Normal,    // 1-10ms
    Slow,      // 10-100ms
    VerySlow,  // > 100ms
}

/// Performance analyzer
pub struct PerformanceLens {
    /// Contract execution costs (contract_id -> ms)
    costs: std::collections::HashMap<String, f64>,
}

impl PerformanceLens {
    pub fn new() -> Self {
        let mut costs = std::collections::HashMap::new();

        // Math operations (T2) - Very fast
        costs.insert("200".to_string(), 0.001); // Add
        costs.insert("201".to_string(), 0.001); // Subtract
        costs.insert("202".to_string(), 0.001); // Multiply
        costs.insert("203".to_string(), 0.001); // Divide

        // String operations (T3) - Fast
        costs.insert("300".to_string(), 0.01);  // Concat
        costs.insert("301".to_string(), 0.02);  // Split
        costs.insert("302".to_string(), 0.015); // Substring

        // Array operations (T4) - Fast to Normal
        costs.insert("400".to_string(), 0.001); // Length
        costs.insert("401".to_string(), 0.001); // Index
        costs.insert("402".to_string(), 0.01);  // Slice
        costs.insert("403".to_string(), 0.02);  // Push

        // I/O operations (T6) - Slow
        costs.insert("600".to_string(), 1.0);   // Print
        costs.insert("601".to_string(), 5.0);   // Read input
        costs.insert("602".to_string(), 10.0);  // Read file
        costs.insert("603".to_string(), 10.0);  // Write file
        costs.insert("604".to_string(), 50.0);  // HTTP request

        // GPU operations (T4-GPU) - Fast but setup cost
        costs.insert("906".to_string(), 2.3);   // GEMM
        costs.insert("907".to_string(), 1.5);   // LayerNorm
        costs.insert("908".to_string(), 0.8);   // GELU
        costs.insert("909".to_string(), 1.2);   // Softmax
        costs.insert("910".to_string(), 0.5);   // CrossEntropy

        Self { costs }
    }

    /// Analyze performance of a document
    pub fn analyze(&self, text: &str, catalogue: &ContractCatalogue) -> Vec<PerformanceCost> {
        let mut results = Vec::new();

        for (line_idx, line) in text.lines().enumerate() {
            // Find contract invocations
            for cost in self.analyze_line(line, line_idx, catalogue) {
                results.push(cost);
            }

            // Find loops (multiply cost by iteration count)
            if let Some(loop_cost) = self.analyze_loop(line, line_idx) {
                results.push(loop_cost);
            }
        }

        results
    }

    /// Analyze a single line
    fn analyze_line(
        &self,
        line: &str,
        line_idx: usize,
        catalogue: &ContractCatalogue,
    ) -> Vec<PerformanceCost> {
        let mut costs = Vec::new();

        // Find all @ symbols
        for (i, ch) in line.chars().enumerate() {
            if ch == '@' {
                // Extract contract ID
                let rest = &line[i + 1..];
                let id_len = rest.chars().take_while(|c| c.is_numeric()).count();

                if id_len > 0 {
                    let contract_id = &rest[..id_len];

                    // Get cost estimate
                    let cost_ms = self.costs.get(contract_id)
                        .copied()
                        .unwrap_or(0.1); // Default estimate

                    // Get contract info
                    let (tier, description) = if let Some(spec) = catalogue.get_contract(contract_id) {
                        (spec.tier.clone(), spec.name.clone())
                    } else {
                        ("Unknown".to_string(), format!("Contract @{}", contract_id))
                    };

                    let severity = self.classify_cost(cost_ms);

                    costs.push(PerformanceCost {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: i as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (i + 1 + id_len) as u32,
                            },
                        },
                        cost_ms,
                        tier,
                        description,
                        severity,
                    });
                }
            }
        }

        costs
    }

    /// Analyze loop performance
    fn analyze_loop(&self, line: &str, line_idx: usize) -> Option<PerformanceCost> {
        if !line.contains("loop ") {
            return None;
        }

        // Estimate: Default loops are potentially expensive
        let cost_ms = 100.0; // Assume worst case

        Some(PerformanceCost {
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
            cost_ms,
            tier: "Control".to_string(),
            description: "Loop iteration".to_string(),
            severity: CostSeverity::Slow,
        })
    }

    /// Classify cost into severity
    fn classify_cost(&self, cost_ms: f64) -> CostSeverity {
        if cost_ms < 1.0 {
            CostSeverity::Fast
        } else if cost_ms < 10.0 {
            CostSeverity::Normal
        } else if cost_ms < 100.0 {
            CostSeverity::Slow
        } else {
            CostSeverity::VerySlow
        }
    }

    /// Create inlay hint for performance cost
    pub fn create_inlay_hint(&self, cost: &PerformanceCost) -> InlayHint {
        let (icon, _color) = match cost.severity {
            CostSeverity::Fast => ("⚡", "green"),
            CostSeverity::Normal => ("⏱", "blue"),
            CostSeverity::Slow => ("🐢", "orange"),
            CostSeverity::VerySlow => ("🔴", "red"),
        };

        let label = if cost.cost_ms < 1.0 {
            format!(" {} ~{:.2}µs", icon, cost.cost_ms * 1000.0)
        } else if cost.cost_ms < 1000.0 {
            format!(" {} ~{:.1}ms", icon, cost.cost_ms)
        } else {
            format!(" {} ~{:.2}s", icon, cost.cost_ms / 1000.0)
        };

        InlayHint {
            position: cost.range.end,
            label: InlayHintLabel::String(label),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "{} ({})\nEstimated cost: {:.2}ms",
                cost.description,
                cost.tier,
                cost.cost_ms
            ))),
            padding_left: Some(true),
            padding_right: None,
            data: None,
        }
    }

    /// Create diagnostic for expensive operations
    pub fn create_diagnostic(&self, cost: &PerformanceCost) -> Option<Diagnostic> {
        // Only warn about very slow operations
        if cost.severity != CostSeverity::VerySlow {
            return None;
        }

        Some(Diagnostic {
            range: cost.range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("performance".to_string())),
            source: Some("hlx-perf".to_string()),
            message: format!(
                "Expensive operation: {} (~{:.0}ms)\n💡 Consider optimizing or caching this operation",
                cost.description,
                cost.cost_ms
            ),
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        })
    }
}

impl Default for PerformanceLens {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_classification() {
        let lens = PerformanceLens::new();

        assert_eq!(lens.classify_cost(0.5), CostSeverity::Fast);
        assert_eq!(lens.classify_cost(5.0), CostSeverity::Normal);
        assert_eq!(lens.classify_cost(50.0), CostSeverity::Slow);
        assert_eq!(lens.classify_cost(500.0), CostSeverity::VerySlow);
    }

    #[test]
    fn test_contract_cost_lookup() {
        let lens = PerformanceLens::new();

        assert_eq!(lens.costs.get("200"), Some(&0.001)); // Math is fast
        assert_eq!(lens.costs.get("602"), Some(&10.0));  // File I/O is slow
    }
}
