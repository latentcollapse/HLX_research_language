//! Promotion Gate System — How Bit Earns New Capabilities
//!
//! Bit starts as a Seedling and earns promotion through demonstrated stability.
//! Each level unlocks new modification types. Capabilities are monotonic —
//! once earned, they cannot be revoked.

use crate::rsi::ModificationType;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromotionLevel {
    Seedling, // Level 0
    Sprout,   // Level 1
    Sapling,  // Level 2
    Tree,     // Level 3
    Grove,    // Level 4
}

impl PromotionLevel {
    pub fn name(&self) -> &'static str {
        match self {
            PromotionLevel::Seedling => "Seedling",
            PromotionLevel::Sprout => "Sprout",
            PromotionLevel::Sapling => "Sapling",
            PromotionLevel::Tree => "Tree",
            PromotionLevel::Grove => "Grove",
        }
    }

    pub fn next(&self) -> Option<PromotionLevel> {
        match self {
            PromotionLevel::Seedling => Some(PromotionLevel::Sprout),
            PromotionLevel::Sprout => Some(PromotionLevel::Sapling),
            PromotionLevel::Sapling => Some(PromotionLevel::Tree),
            PromotionLevel::Tree => Some(PromotionLevel::Grove),
            PromotionLevel::Grove => None,
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
            
            // Mapping belief RSI to rule modification class as it's the closest fit
            ModificationType::BeliefAdd { .. }
            | ModificationType::BeliefRetract { .. } => ModificationTypeClass::RuleModification,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromotionCriteria {
    pub min_bcas_score: f64,
    pub min_belief_count: usize,
    pub min_successful_interactions: u32,
    pub required_predicates: Vec<String>,
    pub human_approval_required: bool,
    pub rsi_capability_unlocked: bool,
}

impl PromotionCriteria {
    pub fn for_level(level: PromotionLevel) -> Option<Self> {
        match level {
            PromotionLevel::Seedling => None,
            PromotionLevel::Sprout => Some(PromotionCriteria {
                min_bcas_score: 0.40,
                min_belief_count: 1000,
                min_successful_interactions: 100,
                required_predicates: vec!["name".into(), "am".into(), "can".into(), "knows".into()],
                human_approval_required: false,
                rsi_capability_unlocked: false,
            }),
            PromotionLevel::Sapling => Some(PromotionCriteria {
                min_bcas_score: 0.60,
                min_belief_count: 10_000,
                min_successful_interactions: 500,
                required_predicates: vec!["causes".into(), "implies".into(), "prevents".into(), "enables".into()],
                human_approval_required: false,
                rsi_capability_unlocked: false,
            }),
            PromotionLevel::Tree => Some(PromotionCriteria {
                min_bcas_score: 0.75,
                min_belief_count: 50_000,
                min_successful_interactions: 2000,
                required_predicates: vec![],
                human_approval_required: true,
                rsi_capability_unlocked: false,
            }),
            PromotionLevel::Grove => Some(PromotionCriteria {
                min_bcas_score: 0.90,
                min_belief_count: 50_000,
                min_successful_interactions: 2000,
                required_predicates: vec![],
                human_approval_required: true,
                rsi_capability_unlocked: true,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromotionGate {
    current_level: PromotionLevel,
    bcas_score: f64,
    belief_count: usize,
    successful_interactions: u32,
    active_predicates: Vec<String>,
    human_approved: bool,
    rsi_unlocked: bool,
    force_open: bool,
    force_open_since: Option<Instant>,
    promotion_history: Vec<(PromotionLevel, Instant)>,
}

impl PromotionGate {
    pub fn new() -> Self {
        PromotionGate {
            current_level: PromotionLevel::Seedling,
            bcas_score: 0.0,
            belief_count: 0,
            successful_interactions: 0,
            active_predicates: Vec::new(),
            human_approved: false,
            rsi_unlocked: false,
            force_open: false,
            force_open_since: None,
            promotion_history: vec![(PromotionLevel::Seedling, Instant::now())],
        }
    }

    pub fn current_level(&self) -> PromotionLevel {
        self.current_level
    }

    pub fn set_bcas_score(&mut self, score: f64) {
        self.bcas_score = score.clamp(0.0, 1.0);
        self.evaluate_promotion();
    }

    pub fn set_belief_count(&mut self, count: usize) {
        self.belief_count = count;
        self.evaluate_promotion();
    }

    pub fn on_successful_interaction(&mut self) {
        self.successful_interactions += 1;
        self.evaluate_promotion();
    }

    pub fn add_predicate(&mut self, predicate: &str) {
        if !self.active_predicates.contains(&predicate.to_string()) {
            self.active_predicates.push(predicate.to_string());
            self.evaluate_promotion();
        }
    }

    pub fn approve_by_human(&mut self) {
        self.human_approved = true;
        self.evaluate_promotion();
    }

    pub fn unlock_rsi(&mut self) {
        self.rsi_unlocked = true;
        self.evaluate_promotion();
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
            PromotionLevel::Tree | PromotionLevel::Grove => vec![
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
            // Auto-evaluate next level as well
            self.evaluate_promotion();
            return Some(self.current_level);
        }

        None
    }

    fn meets_criteria(&self, criteria: &PromotionCriteria) -> bool {
        if self.bcas_score < criteria.min_bcas_score { return false; }
        if self.belief_count < criteria.min_belief_count { return false; }
        if self.successful_interactions < criteria.min_successful_interactions { return false; }
        
        for req in &criteria.required_predicates {
            if !self.active_predicates.contains(req) {
                return false;
            }
        }

        if criteria.human_approval_required && !self.human_approved {
            return false;
        }

        if criteria.rsi_capability_unlocked && !self.rsi_unlocked {
            return false;
        }

        true
    }

    pub fn criteria_progress(&self) -> CriteriaProgress {
        let next_level = self.current_level.next();
        let criteria = next_level.and_then(PromotionCriteria::for_level);

        match (next_level, criteria) {
            (Some(next), Some(c)) => CriteriaProgress {
                next_level: Some(next),
                bcas_progress: (self.bcas_score, c.min_bcas_score),
                belief_progress: (self.belief_count, c.min_belief_count),
                interaction_progress: (self.successful_interactions, c.min_successful_interactions),
                eligible: self.meets_criteria(&c),
            },
            _ => CriteriaProgress {
                next_level: None,
                bcas_progress: (self.bcas_score, 0.0),
                belief_progress: (self.belief_count, 0),
                interaction_progress: (self.successful_interactions, 0),
                eligible: true,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CriteriaProgress {
    pub next_level: Option<PromotionLevel>,
    pub bcas_progress: (f64, f64),
    pub belief_progress: (usize, usize),
    pub interaction_progress: (u32, u32),
    pub eligible: bool,
}

impl std::fmt::Display for CriteriaProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(next) = &self.next_level {
            write!(
                f,
                "Progress toward {}: bcas={:.2}/{:.2} beliefs={}/{} interactions={}/{} {}",
                next.name(),
                self.bcas_progress.0,
                self.bcas_progress.1,
                self.belief_progress.0,
                self.belief_progress.1,
                self.interaction_progress.0,
                self.interaction_progress.1,
                if self.eligible { "ELIGIBLE" } else { "progressing" }
            )
        } else {
            write!(f, "Maximum level reached (Grove)")
        }
    }
}
