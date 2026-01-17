//! Core Code Generator
//!
//! Orchestrates code generation across domains with quality control and diversity.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Configuration for code generation
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Random seed for deterministic generation
    pub seed: u64,

    /// Minimum quality score (0.0-1.0)
    pub quality_threshold: f32,

    /// Minimum diversity score (0.0-1.0)
    pub min_diversity: f32,

    /// Target complexity range (1-10)
    pub complexity_range: (u32, u32),

    /// Maximum generation attempts per example
    pub max_attempts: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            quality_threshold: 0.8,
            min_diversity: 0.75,
            complexity_range: (2, 10),
            max_attempts: 10,
        }
    }
}

/// Core code generator
pub struct CodeGenerator {
    config: GeneratorConfig,
    rng: StdRng,
    stats: GenerationStats,
}

impl CodeGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            rng: StdRng::seed_from_u64(config.seed),
            config,
            stats: GenerationStats::default(),
        }
    }

    /// Generate random number in range
    pub fn gen_range(&mut self, range: std::ops::Range<usize>) -> usize {
        self.rng.gen_range(range)
    }

    /// Choose random element from slice
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.gen_range(0..items.len())]
    }

    /// Generate random bool with probability
    pub fn gen_bool(&mut self, probability: f32) -> bool {
        self.rng.gen::<f32>() < probability
    }

    /// Record successful generation
    pub fn record_success(&mut self) {
        self.stats.successful += 1;
    }

    /// Record failed generation
    pub fn record_failure(&mut self) {
        self.stats.failed += 1;
    }

    /// Get generation statistics
    pub fn stats(&self) -> &GenerationStats {
        &self.stats
    }
}

/// Generation statistics
#[derive(Debug, Default, Clone)]
pub struct GenerationStats {
    pub successful: usize,
    pub failed: usize,
    pub total_attempts: usize,
}

impl GenerationStats {
    pub fn success_rate(&self) -> f32 {
        if self.successful + self.failed == 0 {
            return 0.0;
        }
        self.successful as f32 / (self.successful + self.failed) as f32
    }
}

/// A codeset is a collection of generated code examples
#[derive(Debug, Clone)]
pub struct GeneratedCodeset {
    examples: Vec<GeneratedCode>,
    metadata: CodesetMetadata,
}

impl GeneratedCodeset {
    pub fn new() -> Self {
        Self {
            examples: Vec::new(),
            metadata: CodesetMetadata::default(),
        }
    }

    pub fn add(&mut self, code: GeneratedCode) {
        self.examples.push(code);
    }

    pub fn len(&self) -> usize {
        self.examples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.examples.is_empty()
    }

    pub fn examples(&self) -> &[GeneratedCode] {
        &self.examples
    }

    pub fn total_lines(&self) -> usize {
        self.examples.iter().map(|e| e.line_count()).sum()
    }
}

impl Default for GeneratedCodeset {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about the codeset
#[derive(Debug, Clone, Default)]
pub struct CodesetMetadata {
    pub domain: String,
    pub generation_time_ms: u64,
    pub quality_score: f32,
    pub diversity_score: f32,
}

/// A single generated code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    /// The source code
    pub source: String,

    /// Metadata about this code
    pub metadata: CodeMetadata,
}

impl GeneratedCode {
    pub fn new(source: String, metadata: CodeMetadata) -> Self {
        Self { source, metadata }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn line_count(&self) -> usize {
        self.source.lines().count()
    }
}

/// Metadata about generated code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetadata {
    /// Domain (aerospace, fintech, ml, etc.)
    pub domain: String,

    /// Intent (validation, transformation, etc.)
    pub intent: String,

    /// Complexity score (1-10)
    pub complexity: u32,

    /// Quality score (0.0-1.0)
    pub quality: f32,

    /// Additional annotations
    pub annotations: HashMap<String, String>,
}

impl CodeMetadata {
    pub fn new(domain: &str, intent: &str, complexity: u32) -> Self {
        Self {
            domain: domain.to_string(),
            intent: intent.to_string(),
            complexity,
            quality: 0.0,
            annotations: HashMap::new(),
        }
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality;
        self
    }

    pub fn with_annotation(mut self, key: &str, value: &str) -> Self {
        self.annotations.insert(key.to_string(), value.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_determinism() {
        let config = GeneratorConfig {
            seed: 42,
            ..Default::default()
        };

        let mut gen1 = CodeGenerator::new(config.clone());
        let mut gen2 = CodeGenerator::new(config);

        // Same seed = same random sequence
        assert_eq!(gen1.gen_range(0..100), gen2.gen_range(0..100));
        assert_eq!(gen1.gen_range(0..100), gen2.gen_range(0..100));
    }

    #[test]
    fn test_codeset_operations() {
        let mut codeset = GeneratedCodeset::new();

        let code = GeneratedCode::new(
            "fn test() { }".to_string(),
            CodeMetadata::new("test", "testing", 1),
        );

        codeset.add(code);

        assert_eq!(codeset.len(), 1);
        assert!(!codeset.is_empty());
    }
}
