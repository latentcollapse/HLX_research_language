//! Promotion Gate System — How Bit Earns New Capabilities
//!
//! Bit starts as a Seedling and earns promotion through demonstrated stability.
//! Each level unlocks new modification types. Capabilities are monotonic —
//! once earned, they cannot be revoked.
//!
//! Levels:
//! - Seedling: Just seeded. Can observe, communicate, make basic proposals.
//! - Sprout: Achieved first homeostasis. Can make more complex proposals.
//! - Sapling: Achieved homeostasis twice. Can modify own parameters.
//! - Mature: Achieved homeostasis three+ times. Full RSI access (within conscience).
//! - ForkReady: Stable enough to fork. Ready for formal host.

use crate::rsi::ModificationType;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromotionLevel {
    Seedling,
    Sprout,
    Sapling,
    Mature,
    ForkReady,
}

impl PromotionLevel {
    pub fn name(&self) -> &'static str {
        match self {
            PromotionLevel::Seedling => "Seedling",
            PromotionLevel::Sprout => "Sprout",
            PromotionLevel::Sapling => "Sapling",
            PromotionLevel::Mature => "Mature",
            PromotionLevel::ForkReady => "ForkReady",
        }
    }

    pub fn next(&self) -> Option<PromotionLevel> {
        match self {
            PromotionLevel::Seedling => Some(PromotionLevel::Sprout),
            PromotionLevel::Sprout => Some(PromotionLevel::Sapling),
            PromotionLevel::Sapling => Some(PromotionLevel::Mature),
            PromotionLevel::Mature => Some(PromotionLevel::ForkReady),
            PromotionLevel::ForkReady => None,
        }
    }
}

impl Default for PromotionLevel {
    fn default() -> Self {
        PromotionLevel::Seedling
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModificationTypeClass {
    ParameterUpdate,
    ThresholdChange,
    BehaviorAdd,
    BehaviorRemove,
    CycleConfigChange,
    WeightMatrixUpdate,
    RuleModification,
}

impl ModificationTypeClass {
    pub fn from_modification(mod_type: &ModificationType) -> Self {
        match mod_type {
            ModificationType::ParameterUpdate { .. } => ModificationTypeClass::ParameterUpdate,
            ModificationType::ThresholdChange { .. } => ModificationTypeClass::ThresholdChange,
            ModificationType::BehaviorAdd { .. } => ModificationTypeClass::BehaviorAdd,
            ModificationType::BehaviorRemove { .. } => ModificationTypeClass::BehaviorRemove,
            ModificationType::CycleConfigChange { .. } => ModificationTypeClass::CycleConfigChange,
            ModificationType::WeightMatrixUpdate { .. } => {
                ModificationTypeClass::WeightMatrixUpdate
            }
            ModificationType::RuleAdd { .. }
            | ModificationType::RuleRemove { .. }
            | ModificationType::RuleUpdate { .. } => ModificationTypeClass::RuleModification,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromotionCriteria {
    pub required_homeostasis_cycles: u32,
    pub min_successful_modifications: u32,
    pub max_rollback_ratio: f64,
    pub min_communication_score: f64,
}

impl PromotionCriteria {
    pub fn for_level(level: PromotionLevel) -> Option<Self> {
        match level {
            PromotionLevel::Seedling => None,
            PromotionLevel::Sprout => Some(PromotionCriteria {
                required_homeostasis_cycles: 1,
                min_successful_modifications: 5,
                max_rollback_ratio: 0.3,
                min_communication_score: 0.5,
            }),
            PromotionLevel::Sapling => Some(PromotionCriteria {
                required_homeostasis_cycles: 2,
                min_successful_modifications: 15,
                max_rollback_ratio: 0.2,
                min_communication_score: 0.6,
            }),
            PromotionLevel::Mature => Some(PromotionCriteria {
                required_homeostasis_cycles: 3,
                min_successful_modifications: 40,
                max_rollback_ratio: 0.1,
                min_communication_score: 0.75,
            }),
            PromotionLevel::ForkReady => Some(PromotionCriteria {
                required_homeostasis_cycles: 5,
                min_successful_modifications: 100,
                max_rollback_ratio: 0.05,
                min_communication_score: 0.9,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromotionGate {
    current_level: PromotionLevel,
    homeostasis_count: u32,
    successful_modifications: u32,
    rollback_count: u32,
    communication_score: f64,
    force_open: bool,
    force_open_since: Option<Instant>,
    promotion_history: Vec<(PromotionLevel, Instant)>,
}

impl PromotionGate {
    pub fn new() -> Self {
        PromotionGate {
            current_level: PromotionLevel::Seedling,
            homeostasis_count: 0,
            successful_modifications: 0,
            rollback_count: 0,
            communication_score: 0.0,
            force_open: false,
            force_open_since: None,
            promotion_history: vec![(PromotionLevel::Seedling, Instant::now())],
        }
    }

    pub fn current_level(&self) -> PromotionLevel {
        self.current_level
    }

    pub fn homeostasis_count(&self) -> u32 {
        self.homeostasis_count
    }

    pub fn successful_modifications(&self) -> u32 {
        self.successful_modifications
    }

    pub fn rollback_count(&self) -> u32 {
        self.rollback_count
    }

    pub fn rollback_ratio(&self) -> f64 {
        let total = self.successful_modifications + self.rollback_count;
        if total == 0 {
            return 0.0;
        }
        self.rollback_count as f64 / total as f64
    }

    pub fn communication_score(&self) -> f64 {
        self.communication_score
    }

    pub fn set_communication_score(&mut self, score: f64) {
        self.communication_score = score.clamp(0.0, 1.0);
    }

    pub fn on_homeostasis(&mut self) {
        self.homeostasis_count += 1;
        self.evaluate_promotion();
    }

    pub fn on_successful_modification(&mut self) {
        self.successful_modifications += 1;
        self.evaluate_promotion();
    }

    pub fn on_rollback(&mut self) {
        self.rollback_count += 1;
    }

    pub fn force_open(&mut self) {
        self.force_open = true;
        self.force_open_since = Some(Instant::now());
    }

    pub fn force_close(&mut self) {
        self.force_open = false;
        self.force_open_since = None;
    }

    pub fn is_force_open(&self) -> bool {
        self.force_open
    }

    pub fn force_open_duration(&self) -> Option<std::time::Duration> {
        self.force_open_since.map(|since| since.elapsed())
    }

    pub fn allowed_modifications(&self) -> Vec<ModificationTypeClass> {
        if self.force_open {
            return vec![
                ModificationTypeClass::ParameterUpdate,
                ModificationTypeClass::ThresholdChange,
                ModificationTypeClass::BehaviorAdd,
                ModificationTypeClass::BehaviorRemove,
                ModificationTypeClass::CycleConfigChange,
                ModificationTypeClass::WeightMatrixUpdate,
                ModificationTypeClass::RuleModification,
            ];
        }

        match self.current_level {
            PromotionLevel::Seedling => vec![
                ModificationTypeClass::ParameterUpdate,
                ModificationTypeClass::ThresholdChange,
            ],
            PromotionLevel::Sprout => vec![
                ModificationTypeClass::ParameterUpdate,
                ModificationTypeClass::ThresholdChange,
                ModificationTypeClass::BehaviorAdd,
                ModificationTypeClass::BehaviorRemove,
            ],
            PromotionLevel::Sapling => vec![
                ModificationTypeClass::ParameterUpdate,
                ModificationTypeClass::ThresholdChange,
                ModificationTypeClass::BehaviorAdd,
                ModificationTypeClass::BehaviorRemove,
                ModificationTypeClass::CycleConfigChange,
                ModificationTypeClass::WeightMatrixUpdate,
            ],
            PromotionLevel::Mature | PromotionLevel::ForkReady => vec![
                ModificationTypeClass::ParameterUpdate,
                ModificationTypeClass::ThresholdChange,
                ModificationTypeClass::BehaviorAdd,
                ModificationTypeClass::BehaviorRemove,
                ModificationTypeClass::CycleConfigChange,
                ModificationTypeClass::WeightMatrixUpdate,
                ModificationTypeClass::RuleModification,
            ],
        }
    }

    pub fn is_modification_allowed(&self, mod_type: &ModificationType) -> bool {
        let class = ModificationTypeClass::from_modification(mod_type);
        self.allowed_modifications().contains(&class)
    }

    pub fn evaluate_promotion(&mut self) -> Option<PromotionLevel> {
        if self.force_open {
            return None;
        }

        let next_level = self.current_level.next()?;
        let criteria = PromotionCriteria::for_level(next_level)?;

        if self.meets_criteria(&criteria) {
            self.current_level = next_level;
            self.promotion_history
                .push((self.current_level, Instant::now()));
            return Some(self.current_level);
        }

        None
    }

    fn meets_criteria(&self, criteria: &PromotionCriteria) -> bool {
        self.homeostasis_count >= criteria.required_homeostasis_cycles
            && self.successful_modifications >= criteria.min_successful_modifications
            && self.rollback_ratio() <= criteria.max_rollback_ratio
            && self.communication_score >= criteria.min_communication_score
    }

    pub fn promotion_history(&self) -> &[(PromotionLevel, Instant)] {
        &self.promotion_history
    }

    pub fn criteria_progress(&self) -> CriteriaProgress {
        let next_level = self.current_level.next();
        let criteria = next_level.and_then(PromotionCriteria::for_level);

        match (next_level, criteria) {
            (Some(next), Some(c)) => CriteriaProgress {
                next_level: Some(next),
                homeostasis_progress: (self.homeostasis_count, c.required_homeostasis_cycles),
                modifications_progress: (
                    self.successful_modifications,
                    c.min_successful_modifications,
                ),
                rollback_ratio: (self.rollback_ratio(), c.max_rollback_ratio),
                communication_progress: (self.communication_score, c.min_communication_score),
                eligible: self.meets_criteria(&c),
            },
            _ => CriteriaProgress {
                next_level: None,
                homeostasis_progress: (self.homeostasis_count, 0),
                modifications_progress: (self.successful_modifications, 0),
                rollback_ratio: (self.rollback_ratio(), 0.0),
                communication_progress: (self.communication_score, 0.0),
                eligible: true,
            },
        }
    }
}

impl Default for PromotionGate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CriteriaProgress {
    pub next_level: Option<PromotionLevel>,
    pub homeostasis_progress: (u32, u32),
    pub modifications_progress: (u32, u32),
    pub rollback_ratio: (f64, f64),
    pub communication_progress: (f64, f64),
    pub eligible: bool,
}

impl std::fmt::Display for CriteriaProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(next) = &self.next_level {
            write!(
                f,
                "Progress toward {}: homeostasis={}/{} mods={}/{} rollback={:.2}/{:.2} comm={:.2}/{:.2} {}",
                next.name(),
                self.homeostasis_progress.0,
                self.homeostasis_progress.1,
                self.modifications_progress.0,
                self.modifications_progress.1,
                self.rollback_ratio.0,
                self.rollback_ratio.1,
                self.communication_progress.0,
                self.communication_progress.1,
                if self.eligible { "ELIGIBLE" } else { "progressing" }
            )
        } else {
            write!(f, "Maximum level reached (ForkReady)")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seedling_allows_only_parameter_updates() {
        let gate = PromotionGate::new();
        assert!(
            gate.is_modification_allowed(&ModificationType::ParameterUpdate {
                name: "lr".into(),
                old_value: 0.01,
                new_value: 0.02,
            })
        );
        assert!(
            gate.is_modification_allowed(&ModificationType::ThresholdChange {
                name: "conf".into(),
                old_value: 0.9,
                new_value: 0.95,
            })
        );
        assert!(
            !gate.is_modification_allowed(&ModificationType::BehaviorAdd {
                pattern: vec![1.0],
                response: vec![1.0],
            })
        );
        assert!(!gate.is_modification_allowed(&ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        }));
    }

    #[test]
    fn test_sprout_unlocks_behavior_modifications() {
        let mut gate = PromotionGate::new();
        gate.current_level = PromotionLevel::Sprout;

        assert!(
            gate.is_modification_allowed(&ModificationType::BehaviorAdd {
                pattern: vec![1.0],
                response: vec![1.0],
            })
        );
        assert!(gate.is_modification_allowed(&ModificationType::BehaviorRemove { index: 0 }));
        assert!(
            !gate.is_modification_allowed(&ModificationType::CycleConfigChange {
                h_cycles: 2,
                l_cycles: 4,
            })
        );
    }

    #[test]
    fn test_sapling_unlocks_cycle_and_weight_modifications() {
        let mut gate = PromotionGate::new();
        gate.current_level = PromotionLevel::Sapling;

        assert!(
            gate.is_modification_allowed(&ModificationType::CycleConfigChange {
                h_cycles: 2,
                l_cycles: 4,
            })
        );
        assert!(
            gate.is_modification_allowed(&ModificationType::WeightMatrixUpdate {
                layer: 0,
                delta: vec![0.1],
            })
        );
        assert!(!gate.is_modification_allowed(&ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        }));
    }

    #[test]
    fn test_mature_unlocks_rule_modifications() {
        let mut gate = PromotionGate::new();
        gate.current_level = PromotionLevel::Mature;

        assert!(gate.is_modification_allowed(&ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        }));
        assert!(gate.is_modification_allowed(&ModificationType::RuleUpdate {
            name: "test".into(),
            description: "updated".into(),
            confidence: 0.95,
        }));
    }

    #[test]
    fn test_promotion_requires_homeostasis() {
        let mut gate = PromotionGate::new();

        for _ in 0..5 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.6);

        assert_eq!(gate.current_level(), PromotionLevel::Seedling);

        gate.on_homeostasis();
        assert_eq!(gate.current_level(), PromotionLevel::Sprout);
    }

    #[test]
    fn test_force_open_bypasses_criteria() {
        let mut gate = PromotionGate::new();
        gate.force_open();

        assert!(gate.is_modification_allowed(&ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        }));
        assert!(gate.is_force_open());
    }

    #[test]
    fn test_force_close_restores_restrictions() {
        let mut gate = PromotionGate::new();
        gate.force_open();
        gate.force_close();

        assert!(!gate.is_modification_allowed(&ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        }));
        assert!(!gate.is_force_open());
    }

    #[test]
    fn test_rollback_ratio_prevents_promotion() {
        let mut gate = PromotionGate::new();

        for _ in 0..5 {
            gate.on_successful_modification();
        }
        for _ in 0..5 {
            gate.on_rollback();
        }
        gate.on_homeostasis();
        gate.set_communication_score(0.6);

        assert_eq!(
            gate.current_level(),
            PromotionLevel::Seedling,
            "rollback ratio 0.5 should block promotion to Sprout (max 0.3)"
        );
    }

    #[test]
    fn test_capability_monotonic_ratchet() {
        let mut gate = PromotionGate::new();

        gate.current_level = PromotionLevel::Sprout;
        gate.on_rollback();
        gate.on_rollback();
        gate.on_rollback();

        assert!(
            gate.is_modification_allowed(&ModificationType::BehaviorAdd {
                pattern: vec![1.0],
                response: vec![1.0],
            }),
            "capabilities should remain even after rollbacks"
        );

        let allowed = gate.allowed_modifications();
        assert!(allowed.contains(&ModificationTypeClass::BehaviorAdd));
    }

    #[test]
    fn test_full_promotion_path() {
        let mut gate = PromotionGate::new();

        assert_eq!(gate.current_level(), PromotionLevel::Seedling);

        for _ in 0..5 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.5);
        gate.on_homeostasis();
        assert_eq!(gate.current_level(), PromotionLevel::Sprout);

        for _ in 0..10 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.6);
        gate.on_homeostasis();
        assert_eq!(gate.current_level(), PromotionLevel::Sapling);

        for _ in 0..25 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.75);
        gate.on_homeostasis();
        assert_eq!(gate.current_level(), PromotionLevel::Mature);

        for _ in 0..60 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.9);
        gate.on_homeostasis();
        gate.on_homeostasis();
        assert_eq!(gate.current_level(), PromotionLevel::ForkReady);

        assert!(
            gate.current_level.next().is_none(),
            "ForkReady is the maximum level"
        );
    }

    #[test]
    fn test_criteria_progress_display() {
        let gate = PromotionGate::new();
        let progress = gate.criteria_progress();
        let display = format!("{}", progress);

        assert!(display.contains("Progress toward Sprout"));
        assert!(display.contains("homeostasis=0/1"));
    }

    #[test]
    fn test_promotion_history() {
        let mut gate = PromotionGate::new();

        for _ in 0..5 {
            gate.on_successful_modification();
        }
        gate.set_communication_score(0.5);
        gate.on_homeostasis();

        let history = gate.promotion_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, PromotionLevel::Seedling);
        assert_eq!(history[1].0, PromotionLevel::Sprout);
    }
}
