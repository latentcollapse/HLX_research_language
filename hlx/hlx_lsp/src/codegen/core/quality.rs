//! Quality Validation System

use crate::codegen::core::GeneratedCode;

/// Quality validator ensures generated code meets standards
pub struct QualityValidator;

impl QualityValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate code quality
    pub fn validate(&self, code: &GeneratedCode) -> QualityScore {
        let mut score = QualityScore::default();

        // Basic syntax check (must not be empty)
        if !code.source().is_empty() {
            score.syntax = 1.0;
        }

        // Check for required patterns based on domain
        if code.metadata.domain == "aerospace" {
            score.compliance = self.check_aerospace_compliance(code);
        }

        score.overall = (score.syntax + score.compliance + score.documentation) / 3.0;
        score
    }

    fn check_aerospace_compliance(&self, code: &GeneratedCode) -> f32 {
        let source = code.source();
        let mut compliance = 0.0;

        // Check for safety patterns
        if source.contains("@contract validation") {
            compliance += 0.3;
        }

        if source.contains("// DO-178C") || source.contains("// Safety") {
            compliance += 0.3;
        }

        if source.contains("@audit_log") {
            compliance += 0.2;
        }

        if source.contains("triple modular redundancy") || source.contains("TMR") {
            compliance += 0.2;
        }

        compliance
    }
}

impl Default for QualityValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Quality score breakdown
#[derive(Debug, Clone, Default)]
pub struct QualityScore {
    pub syntax: f32,
    pub compliance: f32,
    pub documentation: f32,
    pub overall: f32,
}
