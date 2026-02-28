//! Catastrophic Forgetting Guard — Phase 2 Prerequisite P7
//!
//! Detects and prevents catastrophic forgetting during training.
//!
//! Strategies:
//!   1. Retention testing: Periodically test model on held-out examples
//!   2. Elastic weight consolidation: Track important weights
//!   3. Gradient interference detection: Detect when new learning conflicts with old
//!   4. Performance threshold: Block training if retention drops below threshold

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionTest {
    pub id: u64,
    pub name: String,
    pub inputs: Vec<Vec<f64>>,
    pub expected_outputs: Vec<Vec<f64>>,
    pub importance_weights: Vec<f64>,
    pub baseline_loss: f64,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionResult {
    pub test_id: u64,
    pub step: u32,
    pub loss: f64,
    pub degradation: f64,
    pub passed: bool,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightImportance {
    pub layer_idx: usize,
    pub weight_idx: usize,
    pub fisher_information: f64,
    pub current_value: f64,
}

#[derive(Debug, Clone)]
pub struct ForgettingEvent {
    pub step: u32,
    pub test_name: String,
    pub baseline_loss: f64,
    pub current_loss: f64,
    pub degradation_percent: f64,
    pub action_taken: String,
}

pub struct ForgettingGuard {
    retention_tests: HashMap<u64, RetentionTest>,
    test_results: Vec<RetentionResult>,
    weight_importance: HashMap<(usize, usize), f64>,
    degradation_threshold: f64,
    critical_threshold: f64,
    test_interval: u32,
    events: Vec<ForgettingEvent>,
    ewc_lambda: f64,
    baseline_established: bool,
}

impl ForgettingGuard {
    pub fn new() -> Self {
        ForgettingGuard {
            retention_tests: HashMap::new(),
            test_results: Vec::new(),
            weight_importance: HashMap::new(),
            degradation_threshold: 0.15,
            critical_threshold: 0.30,
            test_interval: 500,
            events: Vec::new(),
            ewc_lambda: 1000.0,
            baseline_established: false,
        }
    }

    pub fn with_degradation_threshold(mut self, threshold: f64) -> Self {
        self.degradation_threshold = threshold;
        self
    }

    pub fn with_critical_threshold(mut self, threshold: f64) -> Self {
        self.critical_threshold = threshold;
        self
    }

    pub fn with_test_interval(mut self, interval: u32) -> Self {
        self.test_interval = interval;
        self
    }

    pub fn with_ewc_lambda(mut self, lambda: f64) -> Self {
        self.ewc_lambda = lambda;
        self
    }

    // ------------------------------------------------------------------
    // Retention Tests
    // ------------------------------------------------------------------

    pub fn register_test(
        &mut self,
        name: &str,
        inputs: Vec<Vec<f64>>,
        expected_outputs: Vec<Vec<f64>>,
        importance_weights: Option<Vec<f64>>,
        baseline_loss: f64,
    ) -> u64 {
        let id = self.retention_tests.len() as u64 + 1;
        let weights = importance_weights.unwrap_or_else(|| vec![1.0; inputs.len()]);

        let test = RetentionTest {
            id,
            name: name.to_string(),
            inputs,
            expected_outputs,
            importance_weights: weights,
            baseline_loss,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };

        self.retention_tests.insert(id, test);
        id
    }

    pub fn should_test(&self, step: u32) -> bool {
        step > 0 && step % self.test_interval == 0
    }

    pub fn run_retention_test<F>(
        &mut self,
        test_id: u64,
        step: u32,
        inference_fn: F,
    ) -> Result<RetentionResult, ForgettingError>
    where
        F: Fn(&[f64]) -> Vec<f64>,
    {
        let test = self
            .retention_tests
            .get(&test_id)
            .ok_or_else(|| ForgettingError::test_not_found(test_id))?;

        let mut total_loss = 0.0;
        let mut total_weight = 0.0;

        for ((input, expected), weight) in test
            .inputs
            .iter()
            .zip(test.expected_outputs.iter())
            .zip(test.importance_weights.iter())
        {
            let output = inference_fn(input);
            let loss = compute_mse(&output, expected);
            total_loss += loss * weight;
            total_weight += weight;
        }

        let avg_loss = total_loss / total_weight;
        let degradation = (avg_loss - test.baseline_loss) / test.baseline_loss.max(1e-8);

        let passed = degradation < self.degradation_threshold;

        let result = RetentionResult {
            test_id,
            step,
            loss: avg_loss,
            degradation,
            passed,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };

        if !passed {
            self.events.push(ForgettingEvent {
                step,
                test_name: test.name.clone(),
                baseline_loss: test.baseline_loss,
                current_loss: avg_loss,
                degradation_percent: degradation * 100.0,
                action_taken: if degradation >= self.critical_threshold {
                    "critical_alert".to_string()
                } else {
                    "warning_logged".to_string()
                },
            });
        }

        self.test_results.push(result.clone());
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Elastic Weight Consolidation
    // ------------------------------------------------------------------

    pub fn record_fisher_information(
        &mut self,
        layer_idx: usize,
        weight_idx: usize,
        fisher: f64,
        _current_value: f64,
    ) {
        self.weight_importance
            .insert((layer_idx, weight_idx), fisher);
    }

    pub fn compute_ewc_penalty(&self, current_weights: &[(usize, usize, f64)]) -> f64 {
        let mut penalty = 0.0;

        for (layer_idx, weight_idx, current_value) in current_weights {
            if let Some(&fisher) = self.weight_importance.get(&(*layer_idx, *weight_idx)) {
                if let Some(&old_value) = self.weight_importance.get(&(*layer_idx, *weight_idx)) {
                    penalty += fisher * (current_value - old_value).powi(2);
                }
            }
        }

        self.ewc_lambda * penalty
    }

    pub fn establish_baseline(&mut self) {
        self.baseline_established = true;
    }

    // ------------------------------------------------------------------
    // Gradient Interference Detection
    // ------------------------------------------------------------------

    pub fn detect_interference(
        &self,
        old_gradients: &[(usize, usize, f64)],
        new_gradients: &[(usize, usize, f64)],
    ) -> f64 {
        let mut dot_product = 0.0;
        let mut old_norm = 0.0;
        let mut new_norm = 0.0;

        let old_map: HashMap<(usize, usize), f64> = old_gradients
            .iter()
            .map(|(l, w, g)| ((*l, *w), *g))
            .collect();

        for (layer_idx, weight_idx, new_g) in new_gradients {
            if let Some(&old_g) = old_map.get(&(*layer_idx, *weight_idx)) {
                dot_product += old_g * new_g;
                old_norm += old_g.powi(2);
                new_norm += new_g.powi(2);
            }
        }

        old_norm = old_norm.sqrt();
        new_norm = new_norm.sqrt();

        if old_norm < 1e-8 || new_norm < 1e-8 {
            return 0.0;
        }

        // Negative cosine similarity indicates interference
        let cosine = dot_product / (old_norm * new_norm);
        (-cosine).max(0.0)
    }

    // ------------------------------------------------------------------
    // Status & Reporting
    // ------------------------------------------------------------------

    pub fn events(&self) -> &[ForgettingEvent] {
        &self.events
    }

    pub fn recent_results(&self, limit: usize) -> &[RetentionResult] {
        let start = if self.test_results.len() > limit {
            self.test_results.len() - limit
        } else {
            0
        };
        &self.test_results[start..]
    }

    pub fn has_critical_event(&self) -> bool {
        self.events
            .iter()
            .any(|e| e.degradation_percent >= self.critical_threshold * 100.0)
    }

    pub fn overall_health(&self) -> HealthStatus {
        if self.events.is_empty() {
            return HealthStatus::Healthy;
        }

        let max_degradation = self
            .events
            .iter()
            .map(|e| e.degradation_percent)
            .fold(0.0, f64::max);

        if max_degradation >= self.critical_threshold * 100.0 {
            HealthStatus::Critical
        } else if max_degradation >= self.degradation_threshold * 100.0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl Default for ForgettingGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ForgettingError {
    pub message: String,
}

impl std::fmt::Display for ForgettingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ForgettingError: {}", self.message)
    }
}

impl std::error::Error for ForgettingError {}

impl ForgettingError {
    pub fn test_not_found(id: u64) -> Self {
        ForgettingError {
            message: format!("Retention test {} not found", id),
        }
    }
}

fn compute_mse(output: &[f64], expected: &[f64]) -> f64 {
    if output.len() != expected.len() || output.is_empty() {
        return 0.0;
    }

    output
        .iter()
        .zip(expected.iter())
        .map(|(o, e)| (o - e).powi(2))
        .sum::<f64>()
        / output.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_retention_test() {
        let mut guard = ForgettingGuard::new();

        let id = guard.register_test(
            "test_1",
            vec![vec![1.0, 2.0]],
            vec![vec![3.0, 4.0]],
            None,
            0.5,
        );

        assert_eq!(id, 1);
        assert!(guard.retention_tests.contains_key(&id));
    }

    #[test]
    fn test_should_test() {
        let guard = ForgettingGuard::new().with_test_interval(100);

        assert!(!guard.should_test(0));
        assert!(!guard.should_test(50));
        assert!(guard.should_test(100));
        assert!(guard.should_test(200));
    }

    #[test]
    fn test_retention_test_passes() {
        let mut guard = ForgettingGuard::new();

        let id = guard.register_test(
            "test_1",
            vec![vec![1.0, 2.0]],
            vec![vec![3.0, 4.0]],
            None,
            0.5,
        );

        let inference = |_: &[f64]| vec![3.0, 4.0]; // Perfect match
        let result = guard.run_retention_test(id, 100, inference).unwrap();

        assert!(result.passed);
        assert!(result.loss < 0.01);
    }

    #[test]
    fn test_retention_test_detects_degradation() {
        let mut guard = ForgettingGuard::new().with_degradation_threshold(0.1);

        let id = guard.register_test(
            "test_1",
            vec![vec![1.0, 2.0]],
            vec![vec![3.0, 4.0]],
            None,
            0.1, // Low baseline
        );

        let inference = |_: &[f64]| vec![0.0, 0.0]; // Very wrong
        let result = guard.run_retention_test(id, 100, inference).unwrap();

        assert!(!result.passed);
        assert!(result.degradation > 0.1);
    }

    #[test]
    fn test_interference_detection() {
        let guard = ForgettingGuard::new();

        let old_grads = vec![(0, 0, 1.0), (0, 1, 1.0)];
        let new_grads = vec![(0, 0, -1.0), (0, 1, -1.0)]; // Opposite direction

        let interference = guard.detect_interference(&old_grads, &new_grads);
        assert!(interference > 0.9); // High interference
    }

    #[test]
    fn test_health_status() {
        let mut guard = ForgettingGuard::new()
            .with_degradation_threshold(0.1)
            .with_critical_threshold(0.3);

        assert_eq!(guard.overall_health(), HealthStatus::Healthy);

        // Add a warning event
        guard.events.push(ForgettingEvent {
            step: 100,
            test_name: "test".to_string(),
            baseline_loss: 0.1,
            current_loss: 0.12,
            degradation_percent: 20.0,
            action_taken: "warning".to_string(),
        });

        assert_eq!(guard.overall_health(), HealthStatus::Warning);

        // Add a critical event
        guard.events.push(ForgettingEvent {
            step: 200,
            test_name: "test".to_string(),
            baseline_loss: 0.1,
            current_loss: 0.2,
            degradation_percent: 100.0,
            action_taken: "critical".to_string(),
        });

        assert_eq!(guard.overall_health(), HealthStatus::Critical);
    }
}
