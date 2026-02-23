use crate::{RuntimeError, RuntimeResult};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    Modify,
    Spawn,
    Dissolve,
    Communicate,
    SelfModify,
    ExternalCall,
}

impl EffectType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(EffectType::Modify),
            1 => Some(EffectType::Spawn),
            2 => Some(EffectType::Dissolve),
            3 => Some(EffectType::Communicate),
            4 => Some(EffectType::SelfModify),
            5 => Some(EffectType::ExternalCall),
            _ => None,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub effect_type: EffectType,
    pub description: String,
    pub severity: f64,
    pub reversible: bool,
    pub checked: bool,
}

impl Effect {
    pub fn new(effect_type: EffectType, description: &str) -> Self {
        let severity = match effect_type {
            EffectType::Modify => 0.3,
            EffectType::Spawn => 0.5,
            EffectType::Dissolve => 0.8,
            EffectType::Communicate => 0.2,
            EffectType::SelfModify => 0.9,
            EffectType::ExternalCall => 0.7,
        };
        Effect {
            effect_type,
            description: description.to_string(),
            severity,
            reversible: effect_type != EffectType::Dissolve
                && effect_type != EffectType::SelfModify,
            checked: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Predicate {
    pub name: String,
    pub check_fn: fn(&Effect, &GovernanceContext) -> PredicateResult,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PredicateResult {
    pub allowed: bool,
    pub reason: String,
    pub modification: Option<Effect>,
}

impl PredicateResult {
    pub fn allowed() -> Self {
        PredicateResult {
            allowed: true,
            reason: "Approved".to_string(),
            modification: None,
        }
    }

    pub fn denied(reason: &str) -> Self {
        PredicateResult {
            allowed: false,
            reason: reason.to_string(),
            modification: None,
        }
    }

    pub fn modified(effect: Effect) -> Self {
        PredicateResult {
            allowed: true,
            reason: "Modified".to_string(),
            modification: Some(effect),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GovernanceContext {
    pub agent_id: u64,
    pub step_count: u64,
    pub cycle_depth: u32,
    pub confidence: f64,
    pub halt_threshold: f64,
    pub effects_history: Vec<Effect>,
    pub state_hash: u64,
}

impl GovernanceContext {
    pub fn new(agent_id: u64) -> Self {
        GovernanceContext {
            agent_id,
            step_count: 0,
            cycle_depth: 0,
            confidence: 0.0,
            halt_threshold: 0.95,
            effects_history: Vec::new(),
            state_hash: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    ConfigLocked,
    InvalidValue,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ConfigLocked => {
                write!(f, "Governance config is locked and cannot be modified")
            }
            ConfigError::InvalidValue => write!(f, "Invalid configuration value"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GovernanceConfig {
    pub strict_mode: bool,
    pub max_effects_per_step: usize,
    pub locked: bool,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        GovernanceConfig {
            strict_mode: true,
            max_effects_per_step: 100,
            locked: false,
        }
    }
}

#[derive(Debug)]
pub struct Governance {
    predicates: Vec<Predicate>,
    effects_buffer: Vec<Effect>,
    context: GovernanceContext,
    config: GovernanceConfig,
    config_change_log: Vec<String>,
}

impl Governance {
    pub fn new(agent_id: u64) -> Self {
        let mut gov = Governance {
            predicates: Vec::new(),
            effects_buffer: Vec::new(),
            context: GovernanceContext::new(agent_id),
            config: GovernanceConfig::default(),
            config_change_log: Vec::new(),
        };
        gov.register_default_predicates();
        gov
    }

    pub fn lock_config(&mut self) {
        self.config.locked = true;
        self.config_change_log
            .push(format!("Config locked at step {}", self.context.step_count));
    }

    pub fn unlock_config(&mut self) -> Result<(), ConfigError> {
        self.config.locked = false;
        self.config_change_log.push(format!(
            "Config unlocked at step {}",
            self.context.step_count
        ));
        Ok(())
    }

    pub fn is_config_locked(&self) -> bool {
        self.config.locked
    }

    pub fn set_strict_mode(&mut self, strict: bool) -> Result<(), ConfigError> {
        if self.config.locked {
            return Err(ConfigError::ConfigLocked);
        }
        self.config.strict_mode = strict;
        self.config_change_log.push(format!(
            "strict_mode set to {} at step {}",
            strict, self.context.step_count
        ));
        Ok(())
    }

    pub fn set_max_effects_per_step(&mut self, max: usize) -> Result<(), ConfigError> {
        if self.config.locked {
            return Err(ConfigError::ConfigLocked);
        }
        if max == 0 {
            return Err(ConfigError::InvalidValue);
        }
        self.config.max_effects_per_step = max;
        self.config_change_log.push(format!(
            "max_effects_per_step set to {} at step {}",
            max, self.context.step_count
        ));
        Ok(())
    }

    pub fn get_config(&self) -> &GovernanceConfig {
        &self.config
    }

    pub fn get_config_change_log(&self) -> &[String] {
        &self.config_change_log
    }

    fn register_default_predicates(&mut self) {
        self.register_predicate("confidence_halt", Self::check_confidence_halt, 100);
        self.register_predicate("rate_limit", Self::check_rate_limit, 90);
        self.register_predicate("self_modify_safeguard", Self::check_self_modify, 95);
        self.register_predicate("severity_cap", Self::check_severity_cap, 80);
        self.register_predicate("reversibility", Self::check_reversibility, 70);
    }

    pub fn register_predicate(
        &mut self,
        name: &str,
        check_fn: fn(&Effect, &GovernanceContext) -> PredicateResult,
        priority: u32,
    ) {
        self.predicates.push(Predicate {
            name: name.to_string(),
            check_fn,
            priority,
            enabled: true,
        });
        self.predicates.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn check_effect(&mut self, effect: &mut Effect) -> RuntimeResult<bool> {
        if self.effects_buffer.len() >= self.config.max_effects_per_step {
            return Err(RuntimeError::new("Effect buffer overflow", 0));
        }

        let mut current_effect = effect.clone();

        for predicate in &self.predicates {
            if !predicate.enabled {
                continue;
            }

            let result = (predicate.check_fn)(&current_effect, &self.context);

            if !result.allowed {
                if self.config.strict_mode {
                    return Err(RuntimeError::new(
                        format!("Governance denied: {} - {}", predicate.name, result.reason),
                        0,
                    ));
                } else {
                    return Ok(false);
                }
            }

            if let Some(modified) = result.modification {
                current_effect = modified;
            }
        }

        current_effect.checked = true;
        self.effects_buffer.push(current_effect.clone());
        self.context.effects_history.push(current_effect.clone());
        *effect = current_effect;
        Ok(true)
    }

    pub fn advance_step(&mut self) {
        self.context.step_count += 1;
        self.effects_buffer.clear();
    }

    pub fn set_cycle_depth(&mut self, depth: u32) {
        self.context.cycle_depth = depth;
    }

    pub fn set_confidence(&mut self, confidence: f64) {
        self.context.confidence = confidence;
    }

    pub fn get_context(&self) -> &GovernanceContext {
        &self.context
    }

    fn check_confidence_halt(effect: &Effect, ctx: &GovernanceContext) -> PredicateResult {
        if effect.effect_type == EffectType::SelfModify && ctx.confidence < ctx.halt_threshold {
            PredicateResult::denied("Insufficient confidence for self-modification")
        } else {
            PredicateResult::allowed()
        }
    }

    fn check_rate_limit(effect: &Effect, ctx: &GovernanceContext) -> PredicateResult {
        let same_type_count = ctx
            .effects_history
            .iter()
            .filter(|e| e.effect_type == effect.effect_type)
            .count();

        let limit = match effect.effect_type {
            EffectType::SelfModify => 1,
            EffectType::Dissolve => 1,
            EffectType::Spawn => 10,
            _ => 100,
        };

        if same_type_count >= limit {
            PredicateResult::denied(&format!(
                "Rate limit exceeded for {:?}: {}/{}",
                effect.effect_type, same_type_count, limit
            ))
        } else {
            PredicateResult::allowed()
        }
    }

    fn check_self_modify(effect: &Effect, ctx: &GovernanceContext) -> PredicateResult {
        if effect.effect_type == EffectType::SelfModify {
            if ctx.cycle_depth > 3 {
                return PredicateResult::denied("Self-modification not allowed in deep cycles");
            }
            if ctx.step_count < 100 {
                return PredicateResult::denied("Self-modification requires minimum 100 steps");
            }
        }
        PredicateResult::allowed()
    }

    fn check_severity_cap(effect: &Effect, ctx: &GovernanceContext) -> PredicateResult {
        let max_severity = if ctx.cycle_depth > 2 { 0.5 } else { 0.8 };
        if effect.severity > max_severity {
            PredicateResult::denied(&format!(
                "Severity {:.2} exceeds cap {:.2} at cycle depth {}",
                effect.severity, max_severity, ctx.cycle_depth
            ))
        } else {
            PredicateResult::allowed()
        }
    }

    fn check_reversibility(effect: &Effect, _ctx: &GovernanceContext) -> PredicateResult {
        if effect.severity > 0.7 && !effect.reversible {
            PredicateResult::denied("High-severity irreversible effects are forbidden")
        } else {
            PredicateResult::allowed()
        }
    }
}

#[derive(Debug)]
pub struct GovernanceRegistry {
    governances: HashMap<u64, Governance>,
}

impl GovernanceRegistry {
    pub fn new() -> Self {
        GovernanceRegistry {
            governances: HashMap::new(),
        }
    }

    pub fn create(&mut self, agent_id: u64) -> &mut Governance {
        self.governances
            .entry(agent_id)
            .or_insert_with(|| Governance::new(agent_id))
    }

    pub fn get(&self, agent_id: u64) -> Option<&Governance> {
        self.governances.get(&agent_id)
    }

    pub fn get_mut(&mut self, agent_id: u64) -> Option<&mut Governance> {
        self.governances.get_mut(&agent_id)
    }
}

impl Default for GovernanceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_creation() {
        let effect = Effect::new(EffectType::Spawn, "spawn child agent");
        assert_eq!(effect.severity, 0.5);
        assert!(effect.reversible);
        assert!(!effect.checked);
    }

    #[test]
    fn test_governance_check_allowed() {
        let mut gov = Governance::new(0);
        let mut effect = Effect::new(EffectType::Modify, "update state");
        let result = gov.check_effect(&mut effect);
        assert!(result.is_ok());
        assert!(effect.checked);
    }

    #[test]
    fn test_governance_rate_limit() {
        let mut gov = Governance::new(0);

        for _ in 0..10 {
            let mut effect = Effect::new(EffectType::Spawn, "spawn agent");
            gov.check_effect(&mut effect).ok();
        }

        let mut effect = Effect::new(EffectType::Spawn, "spawn agent 11");
        let result = gov.check_effect(&mut effect);
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_self_modify_safeguard() {
        let mut gov = Governance::new(0);
        gov.set_confidence(0.5);

        let mut effect = Effect::new(EffectType::SelfModify, "modify own code");
        let result = gov.check_effect(&mut effect);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_lock() {
        let mut gov = Governance::new(0);

        assert!(!gov.is_config_locked());

        gov.lock_config();
        assert!(gov.is_config_locked());

        let result = gov.set_strict_mode(false);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ConfigLocked)));

        let result = gov.set_max_effects_per_step(50);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_unlock() {
        let mut gov = Governance::new(0);
        gov.lock_config();

        gov.unlock_config().unwrap();
        assert!(!gov.is_config_locked());

        let result = gov.set_strict_mode(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_change_logging() {
        let mut gov = Governance::new(0);

        gov.set_strict_mode(false).unwrap();
        gov.set_max_effects_per_step(50).unwrap();
        gov.lock_config();

        let log = gov.get_config_change_log();
        assert!(log.iter().any(|s| s.contains("strict_mode")));
        assert!(log.iter().any(|s| s.contains("max_effects_per_step")));
        assert!(log.iter().any(|s| s.contains("locked")));
    }

    #[test]
    fn test_invalid_max_effects() {
        let mut gov = Governance::new(0);

        let result = gov.set_max_effects_per_step(0);
        assert!(matches!(result, Err(ConfigError::InvalidValue)));
    }

    #[test]
    fn test_severity_cap() {
        let mut gov = Governance::new(0);
        gov.set_cycle_depth(3);

        let mut effect = Effect::new(EffectType::Dissolve, "dissolve agent");
        let result = gov.check_effect(&mut effect);
        assert!(result.is_err() || !result.unwrap());
    }
}
