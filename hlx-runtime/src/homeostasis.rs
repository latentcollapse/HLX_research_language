use crate::rsi::ModificationType;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementAxis {
    Expansion, // Adding new capabilities/beliefs
    Density,   // Strengthening existing weights/parameters
    Efficiency, // Pruning, simplifying, optimizing
}

#[derive(Debug, Clone, Copy)]
pub struct ModificationEvent {
    pub timestamp: Instant,
    pub axis: ImprovementAxis,
    pub magnitude: f64,
}

#[derive(Debug, Clone)]
pub enum GateDecision {
    Proceed {
        resistance: f64,
    },
    SlowDown {
        delay: Duration,
        resistance: f64,
    },
    Block {
        reason: String,
        resistance: f64,
    },
    Homeostasis {
        sustained_for: Duration,
    },
}

pub struct HomeostasisGate {
    events: Vec<ModificationEvent>,
    measurement_window: Duration,
    base_resistance: f64,
    slowdown_pressure: f64,
    block_pressure: f64,
    equilibrium_threshold: f64,
    sustained_requirement: Duration,
    below_threshold_since: Option<Instant>,
    homeostasis_achieved: bool,
    current_pressure: f64,
    current_resistance: f64,
    last_evaluation: Option<Instant>,
}

impl HomeostasisGate {
    pub fn new() -> Self {
        HomeostasisGate {
            events: Vec::new(),
            measurement_window: Duration::from_secs(60),
            base_resistance: 1.0,
            slowdown_pressure: 0.4,
            block_pressure: 0.8,
            equilibrium_threshold: 0.05,
            sustained_requirement: Duration::from_secs(30),
            below_threshold_since: None,
            homeostasis_achieved: false,
            current_pressure: 0.0,
            current_resistance: 1.0,
            last_evaluation: None,
        }
    }

    pub fn with_measurement_window(mut self, window: Duration) -> Self {
        self.measurement_window = window;
        self
    }

    pub fn with_sustained_requirement(mut self, requirement: Duration) -> Self {
        self.sustained_requirement = requirement;
        self
    }

    pub fn with_pressure_thresholds(mut self, slowdown: f64, block: f64) -> Self {
        self.slowdown_pressure = slowdown;
        self.block_pressure = block;
        self
    }

    pub fn classify(modification: &ModificationType) -> ImprovementAxis {
        match modification {
            ModificationType::BeliefAdd { .. } => ImprovementAxis::Expansion,
            ModificationType::BeliefRetract { .. } => ImprovementAxis::Efficiency,
            ModificationType::BehaviorAdd { .. } => ImprovementAxis::Expansion,
            ModificationType::RuleAdd { .. } => ImprovementAxis::Expansion,
            ModificationType::WeightMatrixUpdate { .. } => ImprovementAxis::Density,
            ModificationType::ParameterUpdate { .. } => ImprovementAxis::Density,
            ModificationType::ThresholdChange { .. } => ImprovementAxis::Density,
            ModificationType::BehaviorRemove { .. } => ImprovementAxis::Efficiency,
            ModificationType::RuleRemove { .. } => ImprovementAxis::Efficiency,
            ModificationType::CycleConfigChange { .. } => ImprovementAxis::Efficiency,
            ModificationType::RuleUpdate { .. } => ImprovementAxis::Density,
        }
    }

    pub fn estimate_magnitude(modification: &ModificationType) -> f64 {
        match modification {
            ModificationType::BeliefAdd { .. } => 0.05,
            ModificationType::BeliefRetract { .. } => 0.08,
            ModificationType::ParameterUpdate { old_value, new_value, .. } => {
                (new_value - old_value).abs() / old_value.abs().max(0.001)
            }
            ModificationType::BehaviorAdd { pattern, .. } => {
                (pattern.len() as f64 * 0.01).min(0.5)
            }
            ModificationType::BehaviorRemove { .. } => 0.15,
            ModificationType::CycleConfigChange { h_cycles, l_cycles } => {
                let total = (h_cycles + l_cycles) as f64;
                (total * 0.05).min(0.3)
            }
            ModificationType::ThresholdChange { old_value, new_value, .. } => {
                (new_value - old_value).abs() / old_value.abs().max(0.001)
            }
            ModificationType::WeightMatrixUpdate { delta, .. } => {
                let max_delta = delta.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
                (max_delta * 2.0).min(1.0)
            }
            ModificationType::RuleAdd { confidence, .. } => *confidence,
            ModificationType::RuleRemove { .. } => 0.8,
            ModificationType::RuleUpdate { confidence, .. } => *confidence * 0.5,
        }
    }

    pub fn record_modification(&mut self, modification: &ModificationType) {
        let event = ModificationEvent {
            timestamp: Instant::now(),
            axis: Self::classify(modification),
            magnitude: Self::estimate_magnitude(modification),
        };
        self.events.push(event);

        if self.homeostasis_achieved {
            self.homeostasis_achieved = false;
            self.below_threshold_since = None;
        }
    }

    fn axis_rate(&self, axis: ImprovementAxis, now: Instant) -> f64 {
        let cutoff = now - self.measurement_window;
        let window_secs = self.measurement_window.as_secs_f64();

        self.events
            .iter()
            .filter(|e| e.timestamp >= cutoff && e.axis == axis)
            .map(|e| e.magnitude)
            .sum::<f64>()
            / window_secs.max(0.001)
    }

    fn compute_pressure(&self, now: Instant) -> f64 {
        let expansion = self.axis_rate(ImprovementAxis::Expansion, now);
        let density = self.axis_rate(ImprovementAxis::Density, now);
        let efficiency = self.axis_rate(ImprovementAxis::Efficiency, now);

        let sum = expansion + density + efficiency;

        let active_count = [expansion, density, efficiency]
            .iter()
            .filter(|&&r| r > 0.001)
            .count();

        let amplification = match active_count {
            0 | 1 => 1.0,
            2 => 1.3,
            3 => 1.7,
            _ => unreachable!(),
        };

        sum * amplification
    }

    fn compute_resistance(&self, pressure: f64) -> f64 {
        self.base_resistance * (1.0 + pressure * pressure)
    }

    pub fn evaluate(&mut self, _modification: &ModificationType) -> GateDecision {
        let now = Instant::now();
        self.last_evaluation = Some(now);

        let cutoff = now - self.measurement_window * 3;
        self.events.retain(|e| e.timestamp >= cutoff);

        let pressure = self.compute_pressure(now);
        let resistance = self.compute_resistance(pressure);
        self.current_pressure = pressure;
        self.current_resistance = resistance;

        if pressure < self.equilibrium_threshold {
            match self.below_threshold_since {
                None => {
                    self.below_threshold_since = Some(now);
                }
                Some(since) => {
                    let sustained = now.duration_since(since);
                    if sustained >= self.sustained_requirement {
                        self.homeostasis_achieved = true;
                        return GateDecision::Homeostasis {
                            sustained_for: sustained,
                        };
                    }
                }
            }
        } else {
            self.below_threshold_since = None;
        }

        if self.homeostasis_achieved {
            return GateDecision::Homeostasis {
                sustained_for: self
                    .below_threshold_since
                    .map(|s| now.duration_since(s))
                    .unwrap_or(Duration::ZERO),
            };
        }

        if pressure >= self.block_pressure {
            GateDecision::Block {
                reason: format!(
                    "pressure {:.3} exceeds block threshold {:.3}",
                    pressure, self.block_pressure
                ),
                resistance,
            }
        } else if pressure >= self.slowdown_pressure {
            let overshoot = (pressure - self.slowdown_pressure)
                / (self.block_pressure - self.slowdown_pressure);
            let delay_secs = overshoot * 10.0;
            GateDecision::SlowDown {
                delay: Duration::from_secs_f64(delay_secs),
                resistance,
            }
        } else {
            GateDecision::Proceed { resistance }
        }
    }

    pub fn pressure(&self) -> f64 {
        self.current_pressure
    }

    pub fn resistance(&self) -> f64 {
        self.current_resistance
    }

    pub fn is_homeostasis(&self) -> bool {
        self.homeostasis_achieved
    }

    pub fn status(&self) -> HomeostasisStatus {
        let now = Instant::now();
        HomeostasisStatus {
            pressure: self.current_pressure,
            resistance: self.current_resistance,
            expansion_rate: self.axis_rate(ImprovementAxis::Expansion, now),
            density_rate: self.axis_rate(ImprovementAxis::Density, now),
            efficiency_rate: self.axis_rate(ImprovementAxis::Efficiency, now),
            homeostasis_achieved: self.homeostasis_achieved,
            events_in_window: self.events.iter().filter(|e| e.timestamp >= now - self.measurement_window).count(),
            time_below_threshold: self.below_threshold_since.map(|s| now.duration_since(s)),
            time_until_homeostasis: self.below_threshold_since.and_then(|s| {
                let elapsed = now.duration_since(s);
                self.sustained_requirement.checked_sub(elapsed)
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HomeostasisStatus {
    pub pressure: f64,
    pub resistance: f64,
    pub expansion_rate: f64,
    pub density_rate: f64,
    pub efficiency_rate: f64,
    pub homeostasis_achieved: bool,
    pub events_in_window: usize,
    pub time_below_threshold: Option<Duration>,
    pub time_until_homeostasis: Option<Duration>,
}

impl std::fmt::Display for HomeostasisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pressure={:.3} resistance={:.3} [exp={:.3} den={:.3} eff={:.3}] events={} {}",
            self.pressure,
            self.resistance,
            self.expansion_rate,
            self.density_rate,
            self.efficiency_rate,
            self.events_in_window,
            if self.homeostasis_achieved {
                "HOMEOSTASIS".to_string()
            } else if let Some(remaining) = self.time_until_homeostasis {
                format!("settling ({:.0}s to homeostasis)", remaining.as_secs_f64())
            } else {
                "active".to_string()
            },
        )
    }
}
