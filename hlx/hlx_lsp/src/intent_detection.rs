//! Intent Detection System
//!
//! Understands what the developer is trying to accomplish
//! and provides proactive suggestions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detected developer intent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeveloperIntent {
    /// Building a new feature
    BuildingFeature {
        feature_type: FeatureType,
        confidence: f32,
    },
    /// Debugging code
    Debugging {
        problem_area: String,
        confidence: f32,
    },
    /// Refactoring existing code
    Refactoring {
        refactor_type: RefactorType,
        confidence: f32,
    },
    /// Writing tests
    WritingTests {
        test_type: String,
        confidence: f32,
    },
    /// Exploring/learning
    Exploring {
        topic: String,
        confidence: f32,
    },
    /// Optimizing performance
    Optimizing {
        target: String,
        confidence: f32,
    },
    /// Creating contracts
    CreatingContracts {
        purpose: String,
        confidence: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeatureType {
    RestAPI,
    DataProcessing,
    UserInterface,
    ContractOrchestration,
    LatentSpaceOperation,
    Validation,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefactorType {
    ExtractFunction,
    RenameSymbol,
    SimplifyLogic,
    ConvertToContracts,
    ImproveStructure,
}

/// Contextual hint based on detected intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentHint {
    pub message: String,
    pub action: Option<String>, // Suggested code action
    pub documentation: Option<String>, // Link to relevant docs
    pub priority: u32, // Higher = more important
}

/// Intent detection engine
pub struct IntentDetector {
    /// Current context
    recent_activity: Vec<Activity>,
    /// Intent history
    intent_history: Vec<DeveloperIntent>,
}

#[derive(Debug, Clone)]
struct Activity {
    activity_type: ActivityType,
    timestamp: std::time::Instant,
    context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ActivityType {
    Edit,
    CompletionRequest,
    DefinitionLookup,
    ReferenceLookup,
    HoverRequest,
    CodeAction,
    Diagnostic,
}

impl IntentDetector {
    pub fn new() -> Self {
        Self {
            recent_activity: Vec::new(),
            intent_history: Vec::new(),
        }
    }

    /// Record an activity
    pub fn record_activity(&mut self, activity_type: ActivityType, context: String) {
        self.recent_activity.push(Activity {
            activity_type,
            timestamp: std::time::Instant::now(),
            context,
        });

        // Keep only recent activity (last 50)
        if self.recent_activity.len() > 50 {
            self.recent_activity.drain(0..10);
        }
    }

    /// Detect current intent from recent activity
    pub fn detect_intent(&mut self, current_code: &str, cursor_context: &str) -> Vec<DeveloperIntent> {
        let mut intents = Vec::new();

        // Analyze code patterns
        intents.extend(self.detect_from_code_patterns(current_code));

        // Analyze recent activity
        intents.extend(self.detect_from_activity());

        // Analyze cursor context
        intents.extend(self.detect_from_context(cursor_context));

        // Store detected intents
        self.intent_history.extend(intents.clone());

        // Keep history limited
        if self.intent_history.len() > 20 {
            self.intent_history.drain(0..5);
        }

        intents
    }

    /// Detect intent from code patterns
    fn detect_from_code_patterns(&self, code: &str) -> Vec<DeveloperIntent> {
        let mut intents = Vec::new();

        // Pattern: Many contract calls → Building contract orchestration
        let contract_count = code.matches('@').count();
        if contract_count > 5 {
            intents.push(DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::ContractOrchestration,
                confidence: (contract_count as f32 / 20.0).min(0.9),
            });
        }

        // Pattern: @lstx → Working with latent space
        if code.contains("@lstx") {
            intents.push(DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::LatentSpaceOperation,
                confidence: 0.95,
            });
        }

        // Pattern: Many validation checks → Building validation
        if code.matches("validate").count() > 3 || code.matches("check").count() > 3 {
            intents.push(DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::Validation,
                confidence: 0.8,
            });
        }

        // Pattern: Function definitions with "test" → Writing tests
        if code.matches("fn test_").count() > 0 || code.contains("assert") {
            intents.push(DeveloperIntent::WritingTests {
                test_type: "unit".to_string(),
                confidence: 0.9,
            });
        }

        // Pattern: print/debug statements → Debugging
        if code.matches("print(").count() > 2 || code.matches("debug(").count() > 0 {
            intents.push(DeveloperIntent::Debugging {
                problem_area: "unknown".to_string(),
                confidence: 0.7,
            });
        }

        // Pattern: TODO/FIXME comments → Exploring/Planning
        if code.contains("TODO") || code.contains("FIXME") {
            intents.push(DeveloperIntent::Exploring {
                topic: "implementation planning".to_string(),
                confidence: 0.6,
            });
        }

        intents
    }

    /// Detect intent from recent activity
    fn detect_from_activity(&self) -> Vec<DeveloperIntent> {
        let mut intents = Vec::new();

        // Count recent activity types
        let mut activity_counts: HashMap<ActivityType, usize> = HashMap::new();
        for activity in self.recent_activity.iter().rev().take(20) {
            *activity_counts.entry(activity.activity_type.clone()).or_insert(0) += 1;
        }

        // Many definition lookups → Exploring code
        if let Some(&count) = activity_counts.get(&ActivityType::DefinitionLookup) {
            if count > 5 {
                intents.push(DeveloperIntent::Exploring {
                    topic: "codebase navigation".to_string(),
                    confidence: 0.75,
                });
            }
        }

        // Many reference lookups → Refactoring (preparing to change)
        if let Some(&count) = activity_counts.get(&ActivityType::ReferenceLookup) {
            if count > 3 {
                intents.push(DeveloperIntent::Refactoring {
                    refactor_type: RefactorType::RenameSymbol,
                    confidence: 0.7,
                });
            }
        }

        // Many code actions → Debugging/fixing
        if let Some(&count) = activity_counts.get(&ActivityType::CodeAction) {
            if count > 4 {
                intents.push(DeveloperIntent::Debugging {
                    problem_area: "error fixing".to_string(),
                    confidence: 0.8,
                });
            }
        }

        intents
    }

    /// Detect intent from cursor context
    fn detect_from_context(&self, context: &str) -> Vec<DeveloperIntent> {
        let mut intents = Vec::new();
        let context_lower = context.to_lowercase();

        // Context: Inside a new function
        if context_lower.contains("fn ") && context_lower.matches('{').count() == context_lower.matches('}').count() + 1 {
            intents.push(DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::Unknown,
                confidence: 0.6,
            });
        }

        // Context: After "let" keyword
        if context_lower.ends_with("let ") || context_lower.ends_with("let") {
            // Likely declaring a variable, building something
            intents.push(DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::Unknown,
                confidence: 0.5,
            });
        }

        // Context: Typing @contract or @lstx
        if context_lower.contains("@contract") {
            intents.push(DeveloperIntent::CreatingContracts {
                purpose: "custom contract".to_string(),
                confidence: 0.85,
            });
        }

        intents
    }

    /// Generate hints based on detected intent
    pub fn generate_hints(&self, intents: &[DeveloperIntent]) -> Vec<IntentHint> {
        let mut hints = Vec::new();

        for intent in intents {
            match intent {
                DeveloperIntent::BuildingFeature { feature_type, confidence } => {
                    if *confidence > 0.7 {
                        hints.push(self.hint_for_feature(feature_type));
                    }
                }
                DeveloperIntent::Debugging { problem_area, confidence } => {
                    if *confidence > 0.6 {
                        hints.push(IntentHint {
                            message: format!("Looks like you're debugging {}. Consider adding contract assertions to catch issues early.", problem_area),
                            action: Some("Add @assert contract".to_string()),
                            documentation: Some("docs/debugging.md".to_string()),
                            priority: 2,
                        });
                    }
                }
                DeveloperIntent::Refactoring { refactor_type, confidence } => {
                    if *confidence > 0.65 {
                        hints.push(IntentHint {
                            message: format!("Refactoring detected: {:?}. Use Ctrl+. for refactoring actions.", refactor_type),
                            action: Some("Show refactorings".to_string()),
                            documentation: None,
                            priority: 3,
                        });
                    }
                }
                DeveloperIntent::WritingTests { test_type, confidence } => {
                    if *confidence > 0.8 {
                        hints.push(IntentHint {
                            message: format!("Writing {} tests. Consider using contract validation for comprehensive coverage.", test_type),
                            action: Some("Generate test contract".to_string()),
                            documentation: Some("docs/testing.md".to_string()),
                            priority: 2,
                        });
                    }
                }
                DeveloperIntent::CreatingContracts { purpose, confidence } => {
                    if *confidence > 0.75 {
                        hints.push(IntentHint {
                            message: format!("Creating contract for: {}. Check the contract catalog for existing solutions.", purpose),
                            action: Some("Search contract catalog".to_string()),
                            documentation: Some("docs/contracts.md".to_string()),
                            priority: 4,
                        });
                    }
                }
                DeveloperIntent::Optimizing { target, confidence } => {
                    if *confidence > 0.7 {
                        hints.push(IntentHint {
                            message: format!("Optimizing {}. Contracts can be backend-accelerated for better performance.", target),
                            action: Some("Convert to contract".to_string()),
                            documentation: Some("docs/performance.md".to_string()),
                            priority: 3,
                        });
                    }
                }
                _ => {}
            }
        }

        hints
    }

    /// Generate hint for specific feature type
    fn hint_for_feature(&self, feature_type: &FeatureType) -> IntentHint {
        match feature_type {
            FeatureType::ContractOrchestration => IntentHint {
                message: "Building with contracts! Consider using composition patterns for complex workflows.".to_string(),
                action: Some("Show contract patterns".to_string()),
                documentation: Some("docs/contract-composition.md".to_string()),
                priority: 3,
            },
            FeatureType::LatentSpaceOperation => IntentHint {
                message: "Working with latent space! @lstx operations can be optimized with caching.".to_string(),
                action: Some("Add LSTX caching".to_string()),
                documentation: Some("docs/lstx.md".to_string()),
                priority: 4,
            },
            FeatureType::Validation => IntentHint {
                message: "Building validation logic. HLX validation contracts provide composable, reusable checks.".to_string(),
                action: Some("Use validation contract".to_string()),
                documentation: Some("docs/validation.md".to_string()),
                priority: 3,
            },
            _ => IntentHint {
                message: "Building new functionality. Contracts provide type-safe, composable building blocks.".to_string(),
                action: Some("Browse contract catalog".to_string()),
                documentation: Some("docs/getting-started.md".to_string()),
                priority: 2,
            },
        }
    }

    /// Get recent intent history
    pub fn get_history(&self) -> &[DeveloperIntent] {
        &self.intent_history
    }
}

impl Default for IntentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_orchestration_detection() {
        let detector = IntentDetector::new();
        let code = "@200 { } @201 { } @202 { } @203 { } @204 { } @205 { }";

        let intents = detector.detect_from_code_patterns(code);

        assert!(!intents.is_empty());
        assert!(intents.iter().any(|i| matches!(
            i,
            DeveloperIntent::BuildingFeature { feature_type: FeatureType::ContractOrchestration, .. }
        )));
    }

    #[test]
    fn test_lstx_detection() {
        let detector = IntentDetector::new();
        let code = "@lstx { table: \"embeddings\" }";

        let intents = detector.detect_from_code_patterns(code);

        assert!(intents.iter().any(|i| matches!(
            i,
            DeveloperIntent::BuildingFeature { feature_type: FeatureType::LatentSpaceOperation, .. }
        )));
    }

    #[test]
    fn test_test_writing_detection() {
        let detector = IntentDetector::new();
        let code = "fn test_addition() { assert_eq!(1 + 1, 2); }";

        let intents = detector.detect_from_code_patterns(code);

        assert!(intents.iter().any(|i| matches!(i, DeveloperIntent::WritingTests { .. })));
    }

    #[test]
    fn test_debugging_detection() {
        let detector = IntentDetector::new();
        let code = "print(x); print(y); print(z);";

        let intents = detector.detect_from_code_patterns(code);

        assert!(intents.iter().any(|i| matches!(i, DeveloperIntent::Debugging { .. })));
    }

    #[test]
    fn test_hint_generation() {
        let detector = IntentDetector::new();
        let intents = vec![
            DeveloperIntent::BuildingFeature {
                feature_type: FeatureType::LatentSpaceOperation,
                confidence: 0.9,
            },
        ];

        let hints = detector.generate_hints(&intents);

        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("latent space"));
    }
}
