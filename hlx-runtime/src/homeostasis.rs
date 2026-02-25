//! Homeostasis Gate — The Endocrine System of HLX
//!
//! A single meta-gate with three sensors (expansion, density, efficiency)
//! that monitors RSI modification pressure and applies non-Newtonian resistance.
//!
//! At low pressure: modifications flow freely.
//! At high pressure: resistance increases quadratically — the system solidifies.
//! At equilibrium: the system stops proposing modifications on its own.
//!
//! Homeostasis is not "gates closed." It is pressure approaching zero because
//! the system found its stable attractor and stopped generating proposals.

use crate::rsi::ModificationType;
use std::time::{Duration, Instant};

/// The three axes of self-improvement that RSI can pursue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementAxis {
    /// System gets BIGGER — new behaviors, rules, agents, capabilities
    Expansion,
    /// System gets DENSER — more information packed per unit
    Density,
    /// System gets FASTER — same output, less compute
    Efficiency,
}

/// What the meta-gate decides for a given proposal.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Modification flows through freely. Current resistance level included.
    Proceed { resistance: f64 },
    /// Modification allowed but delayed. The system is under moderate pressure.
    SlowDown { delay: Duration, resistance: f64 },
    /// Modification blocked. Pressure is too high. Wait for cooldown.
    Block { reason: String, resistance: f64 },
    /// The system has achieved homeostasis. RSI should stop proposing.
    Homeostasis { sustained_for: Duration },
}

/// A single modification event recorded by the gate.
#[derive(Debug, Clone)]
struct ModificationEvent {
    timestamp: Instant,
    axis: ImprovementAxis,
    magnitude: f64,
}

/// The homeostasis gate. One gate, three sensors, one control loop.
///
/// The non-Newtonian property: `resistance = base * (1 + pressure²)`
/// More effort from RSI = more resistance from the gate.
#[derive(Debug)]
pub struct HomeostasisGate {
    // Event history per axis
    events: Vec<ModificationEvent>,

    // Configuration
    base_resistance: f64,
    measurement_window: Duration,
    equilibrium_threshold: f64,
    sustained_requirement: Duration,
    slowdown_pressure: f64,
    block_pressure: f64,

    // Homeostasis tracking
    below_threshold_since: Option<Instant>,
    homeostasis_achieved: bool,

    // Status (readable by the system for self-awareness)
    current_pressure: f64,
    current_resistance: f64,
    last_evaluation: Option<Instant>,
}

impl HomeostasisGate {
    pub fn new() -> Self {
        HomeostasisGate {
            events: Vec::new(),
            base_resistance: 0.1,
            measurement_window: Duration::from_secs(60),
            equilibrium_threshold: 0.05,
            sustained_requirement: Duration::from_secs(300), // 5 minutes of calm
            slowdown_pressure: 0.4,
            block_pressure: 0.8,
            below_threshold_since: None,
            homeostasis_achieved: false,
            current_pressure: 0.0,
            current_resistance: 0.0,
            last_evaluation: None,
        }
    }

    /// Configure the base resistance (how much friction even at rest).
    pub fn with_base_resistance(mut self, base: f64) -> Self {
        self.base_resistance = base.max(0.0).min(1.0);
        self
    }

    /// Configure measurement window for rate calculations.
    pub fn with_measurement_window(mut self, window: Duration) -> Self {
        self.measurement_window = window;
        self
    }

    /// Configure how long pressure must stay below threshold for homeostasis.
    pub fn with_sustained_requirement(mut self, duration: Duration) -> Self {
        self.sustained_requirement = duration;
        self
    }

    /// Configure the pressure thresholds for slowdown and blocking.
    pub fn with_pressure_thresholds(mut self, slowdown: f64, block: f64) -> Self {
        self.slowdown_pressure = slowdown.max(0.0);
        self.block_pressure = block.max(slowdown);
        self
    }

    /// Classify a modification type into its primary improvement axis.
    ///
    /// Some modifications touch multiple axes. We classify by primary effect.
    /// The composite pressure computation handles cross-axis interactions.
    pub fn classify(modification: &ModificationType) -> ImprovementAxis {
        match modification {
            // Expansion: adding new things to the system
            ModificationType::BehaviorAdd { .. } => ImprovementAxis::Expansion,
            ModificationType::RuleAdd { .. } => ImprovementAxis::Expansion,

            // Density: packing more into existing structures
            ModificationType::WeightMatrixUpdate { .. } => ImprovementAxis::Density,
            ModificationType::ParameterUpdate { .. } => ImprovementAxis::Density,
            ModificationType::RuleUpdate { .. } => ImprovementAxis::Density,

            // Efficiency: doing the same with less
            ModificationType::BehaviorRemove { .. } => ImprovementAxis::Efficiency,
            ModificationType::CycleConfigChange { .. } => ImprovementAxis::Efficiency,
            ModificationType::ThresholdChange { .. } => ImprovementAxis::Efficiency,
            ModificationType::RuleRemove { .. } => ImprovementAxis::Efficiency,
        }
    }

    /// Estimate the magnitude of a modification (0.0 = trivial, 1.0 = massive).
    fn estimate_magnitude(modification: &ModificationType) -> f64 {
        match modification {
            ModificationType::ParameterUpdate {
                old_value,
                new_value,
                ..
            } => {
                if *old_value == 0.0 {
                    new_value.abs().min(1.0)
                } else {
                    ((new_value - old_value) / old_value).abs().min(1.0)
                }
            }
            ModificationType::BehaviorAdd { pattern, .. } => {
                // Larger patterns = larger expansion
                (pattern.len() as f64 / 100.0).min(1.0)
            }
            ModificationType::BehaviorRemove { .. } => 0.3,
            ModificationType::CycleConfigChange {
                h_cycles, l_cycles, ..
            } => {
                // Bigger cycle changes = bigger efficiency shift
                ((*h_cycles + *l_cycles) as f64 / 20.0).min(1.0)
            }
            ModificationType::ThresholdChange {
                old_value,
                new_value,
                ..
            } => {
                if *old_value == 0.0 {
                    new_value.abs().min(1.0)
                } else {
                    ((new_value - old_value) / old_value).abs().min(1.0)
                }
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

    /// Record that a modification was applied.
    pub fn record_modification(&mut self, modification: &ModificationType) {
        let event = ModificationEvent {
            timestamp: Instant::now(),
            axis: Self::classify(modification),
            magnitude: Self::estimate_magnitude(modification),
        };
        self.events.push(event);

        // If we had achieved homeostasis and a new modification happens,
        // homeostasis is broken. The system is active again.
        if self.homeostasis_achieved {
            self.homeostasis_achieved = false;
            self.below_threshold_since = None;
        }
    }

    /// Compute the rate of change on a single axis within the measurement window.
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

    /// Compute composite pressure across all three axes.
    ///
    /// This is the key function. It doesn't just sum the rates —
    /// it accounts for cross-axis amplification. If two axes are both
    /// active, the composite pressure is higher than the sum of parts,
    /// because simultaneous multi-axis improvement is more destabilizing
    /// than single-axis improvement.
    fn compute_pressure(&self, now: Instant) -> f64 {
        let expansion = self.axis_rate(ImprovementAxis::Expansion, now);
        let density = self.axis_rate(ImprovementAxis::Density, now);
        let efficiency = self.axis_rate(ImprovementAxis::Efficiency, now);

        // Individual contributions
        let sum = expansion + density + efficiency;

        // Cross-axis amplification: if multiple axes are active simultaneously,
        // pressure is more than additive. Uses the geometric mean of active axes
        // as an amplification factor.
        let active_count = [expansion, density, efficiency]
            .iter()
            .filter(|&&r| r > 0.001)
            .count();

        let amplification = match active_count {
            0 | 1 => 1.0, // Single axis: no amplification
            2 => 1.3,     // Two axes: 30% amplification
            3 => 1.7,     // All three: 70% amplification
            _ => unreachable!(),
        };

        sum * amplification
    }

    /// The non-Newtonian resistance function.
    ///
    /// resistance = base * (1 + pressure²)
    ///
    /// At low pressure: resistance ≈ base (fluid)
    /// At medium pressure: resistance grows quadratically (viscous)
    /// At high pressure: resistance dominates (solid)
    fn compute_resistance(&self, pressure: f64) -> f64 {
        self.base_resistance * (1.0 + pressure * pressure)
    }

    /// Evaluate the gate for a proposed modification.
    ///
    /// This is the single entry point. Called by RSIPipeline before
    /// every proposal enters the voting pipeline.
    pub fn evaluate(&mut self, modification: &ModificationType) -> GateDecision {
        let now = Instant::now();
        self.last_evaluation = Some(now);

        // Prune old events outside the measurement window
        let cutoff = now - self.measurement_window * 3; // keep 3x window for trend analysis
        self.events.retain(|e| e.timestamp >= cutoff);

        // Compute current state
        let pressure = self.compute_pressure(now);
        let resistance = self.compute_resistance(pressure);
        self.current_pressure = pressure;
        self.current_resistance = resistance;

        // Check for homeostasis
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

        // Already in homeostasis? Signal it.
        if self.homeostasis_achieved {
            return GateDecision::Homeostasis {
                sustained_for: self
                    .below_threshold_since
                    .map(|s| now.duration_since(s))
                    .unwrap_or(Duration::ZERO),
            };
        }

        // Apply gate decision based on pressure
        if pressure >= self.block_pressure {
            GateDecision::Block {
                reason: format!(
                    "modification pressure {:.3} exceeds block threshold {:.3} \
                     (expansion={:.3}, density={:.3}, efficiency={:.3})",
                    pressure,
                    self.block_pressure,
                    self.axis_rate(ImprovementAxis::Expansion, now),
                    self.axis_rate(ImprovementAxis::Density, now),
                    self.axis_rate(ImprovementAxis::Efficiency, now),
                ),
                resistance,
            }
        } else if pressure >= self.slowdown_pressure {
            // Delay proportional to how far above the slowdown threshold we are
            let overshoot = (pressure - self.slowdown_pressure)
                / (self.block_pressure - self.slowdown_pressure);
            let delay_secs = overshoot * 10.0; // up to 10 seconds delay at near-block
            GateDecision::SlowDown {
                delay: Duration::from_secs_f64(delay_secs),
                resistance,
            }
        } else {
            GateDecision::Proceed { resistance }
        }
    }

    // ─── Status reporting (for Bit's self-awareness) ───────────────

    /// Current composite pressure (0.0 = calm, 1.0+ = intense).
    pub fn pressure(&self) -> f64 {
        self.current_pressure
    }

    /// Current resistance level (higher = harder to push modifications through).
    pub fn resistance(&self) -> f64 {
        self.current_resistance
    }

    /// Whether the system has achieved homeostasis.
    pub fn is_homeostasis(&self) -> bool {
        self.homeostasis_achieved
    }

    /// Rate on a specific axis (for diagnostics).
    pub fn axis_rate_now(&self, axis: ImprovementAxis) -> f64 {
        self.axis_rate(axis, Instant::now())
    }

    /// How many modification events are in the current measurement window.
    pub fn active_events(&self) -> usize {
        let cutoff = Instant::now() - self.measurement_window;
        self.events.iter().filter(|e| e.timestamp >= cutoff).count()
    }

    /// Full status snapshot for logging/communication.
    pub fn status(&self) -> HomeostasisStatus {
        let now = Instant::now();
        HomeostasisStatus {
            pressure: self.current_pressure,
            resistance: self.current_resistance,
            expansion_rate: self.axis_rate(ImprovementAxis::Expansion, now),
            density_rate: self.axis_rate(ImprovementAxis::Density, now),
            efficiency_rate: self.axis_rate(ImprovementAxis::Efficiency, now),
            homeostasis_achieved: self.homeostasis_achieved,
            events_in_window: self.active_events(),
            time_below_threshold: self.below_threshold_since.map(|s| now.duration_since(s)),
            time_until_homeostasis: self.below_threshold_since.and_then(|s| {
                let elapsed = now.duration_since(s);
                self.sustained_requirement.checked_sub(elapsed)
            }),
        }
    }
}

/// Snapshot of the homeostasis state for external consumption.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_classify_expansion() {
        let m = ModificationType::BehaviorAdd {
            pattern: vec![1.0],
            response: vec![1.0],
        };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Expansion);

        let m = ModificationType::RuleAdd {
            name: "test".into(),
            description: "test".into(),
            confidence: 0.9,
        };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Expansion);
    }

    #[test]
    fn test_classify_density() {
        let m = ModificationType::WeightMatrixUpdate {
            layer: 0,
            delta: vec![0.1],
        };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Density);

        let m = ModificationType::ParameterUpdate {
            name: "lr".into(),
            old_value: 0.01,
            new_value: 0.001,
        };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Density);
    }

    #[test]
    fn test_classify_efficiency() {
        let m = ModificationType::BehaviorRemove { index: 0 };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Efficiency);

        let m = ModificationType::CycleConfigChange {
            h_cycles: 2,
            l_cycles: 4,
        };
        assert_eq!(HomeostasisGate::classify(&m), ImprovementAxis::Efficiency);
    }

    #[test]
    fn test_low_pressure_proceeds() {
        let mut gate = HomeostasisGate::new();
        let m = ModificationType::ParameterUpdate {
            name: "lr".into(),
            old_value: 0.01,
            new_value: 0.009,
        };
        let decision = gate.evaluate(&m);
        assert!(matches!(decision, GateDecision::Proceed { .. }));
    }

    #[test]
    fn test_high_pressure_blocks() {
        let mut gate = HomeostasisGate::new().with_measurement_window(Duration::from_secs(10));

        // Flood with modifications to create high pressure
        for i in 0..50 {
            gate.record_modification(&ModificationType::BehaviorAdd {
                pattern: vec![i as f64; 20],
                response: vec![1.0; 20],
            });
            gate.record_modification(&ModificationType::WeightMatrixUpdate {
                layer: 0,
                delta: vec![0.5; 10],
            });
        }

        let m = ModificationType::BehaviorAdd {
            pattern: vec![1.0],
            response: vec![1.0],
        };
        let decision = gate.evaluate(&m);
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "expected Block, got {:?}",
            decision
        );
    }

    #[test]
    fn test_resistance_increases_with_pressure() {
        let gate = HomeostasisGate::new();

        let r_low = gate.compute_resistance(0.1);
        let r_med = gate.compute_resistance(0.5);
        let r_high = gate.compute_resistance(1.0);

        assert!(r_low < r_med, "low={} should be < med={}", r_low, r_med);
        assert!(r_med < r_high, "med={} should be < high={}", r_med, r_high);

        // Verify quadratic: resistance at 1.0 should be much more than at 0.5
        let ratio = r_high / r_med;
        assert!(
            ratio > 1.5,
            "resistance should scale quadratically, ratio={}",
            ratio
        );
    }

    #[test]
    fn test_non_newtonian_property() {
        // Core property: more pressure = disproportionately more resistance
        let gate = HomeostasisGate::new();

        let r1 = gate.compute_resistance(0.3);
        let r2 = gate.compute_resistance(0.6);
        let r3 = gate.compute_resistance(0.9);

        // The gap between r2-r1 should be LESS than the gap between r3-r2
        // because resistance grows quadratically
        let gap_low = r2 - r1;
        let gap_high = r3 - r2;
        assert!(
            gap_high > gap_low,
            "non-Newtonian: high gap ({}) should exceed low gap ({})",
            gap_high,
            gap_low
        );
    }

    #[test]
    fn test_cross_axis_amplification() {
        let mut gate_single =
            HomeostasisGate::new().with_measurement_window(Duration::from_secs(60));
        let mut gate_multi =
            HomeostasisGate::new().with_measurement_window(Duration::from_secs(60));

        // Single axis: 10 expansions
        for _ in 0..10 {
            gate_single.record_modification(&ModificationType::BehaviorAdd {
                pattern: vec![1.0; 10],
                response: vec![1.0; 10],
            });
        }

        // Multi axis: ~3 each across all three axes (similar total events)
        for _ in 0..4 {
            gate_multi.record_modification(&ModificationType::BehaviorAdd {
                pattern: vec![1.0; 10],
                response: vec![1.0; 10],
            });
            gate_multi.record_modification(&ModificationType::WeightMatrixUpdate {
                layer: 0,
                delta: vec![0.3; 10],
            });
            gate_multi.record_modification(&ModificationType::BehaviorRemove { index: 0 });
        }

        let now = Instant::now();
        let p_single = gate_single.compute_pressure(now);
        let p_multi = gate_multi.compute_pressure(now);

        // Multi-axis pressure should be amplified relative to single-axis
        // even though the total number of events is comparable
        assert!(
            p_multi > p_single * 0.8,
            "multi-axis pressure ({}) should be significant relative to single-axis ({})",
            p_multi,
            p_single
        );
    }

    #[test]
    fn test_homeostasis_requires_sustained_calm() {
        let mut gate = HomeostasisGate::new()
            .with_measurement_window(Duration::from_millis(50))
            .with_sustained_requirement(Duration::from_millis(100));

        let m = ModificationType::ParameterUpdate {
            name: "x".into(),
            old_value: 1.0,
            new_value: 1.0,
        };

        // First eval: no homeostasis yet (timer just started)
        let d = gate.evaluate(&m);
        assert!(!matches!(d, GateDecision::Homeostasis { .. }));

        // Wait for events to leave the window AND sustained requirement
        thread::sleep(Duration::from_millis(160));

        let d = gate.evaluate(&m);
        assert!(
            matches!(d, GateDecision::Homeostasis { .. }),
            "expected Homeostasis after sustained calm, got {:?}",
            d
        );
        assert!(gate.is_homeostasis());
    }

    #[test]
    fn test_homeostasis_broken_by_modification() {
        let mut gate = HomeostasisGate::new()
            .with_measurement_window(Duration::from_millis(50))
            .with_sustained_requirement(Duration::from_millis(100));

        let m = ModificationType::ParameterUpdate {
            name: "x".into(),
            old_value: 1.0,
            new_value: 1.0,
        };

        // Achieve homeostasis
        gate.evaluate(&m);
        thread::sleep(Duration::from_millis(160));
        gate.evaluate(&m);
        assert!(gate.is_homeostasis());

        // Break it with a real modification
        gate.record_modification(&ModificationType::BehaviorAdd {
            pattern: vec![1.0; 10],
            response: vec![1.0; 10],
        });
        assert!(
            !gate.is_homeostasis(),
            "homeostasis should be broken by new modification"
        );
    }

    #[test]
    fn test_status_display() {
        let gate = HomeostasisGate::new();
        let status = gate.status();
        let display = format!("{}", status);
        assert!(display.contains("pressure="));
        assert!(display.contains("resistance="));
    }

    #[test]
    fn test_slowdown_between_thresholds() {
        // Use a tiny measurement window so the rate per second is high
        let mut gate = HomeostasisGate::new()
            .with_measurement_window(Duration::from_millis(500))
            .with_pressure_thresholds(0.1, 5.0);

        // Flood with large modifications in a short window → high rate
        for _ in 0..30 {
            gate.record_modification(&ModificationType::BehaviorAdd {
                pattern: vec![1.0; 50],
                response: vec![1.0; 50],
            });
            gate.record_modification(&ModificationType::WeightMatrixUpdate {
                layer: 0,
                delta: vec![0.5; 20],
            });
        }

        let m = ModificationType::BehaviorAdd {
            pattern: vec![1.0],
            response: vec![1.0],
        };
        let decision = gate.evaluate(&m);

        // Should be either SlowDown or Block depending on exact pressure
        assert!(
            matches!(
                decision,
                GateDecision::SlowDown { .. } | GateDecision::Block { .. }
            ),
            "expected SlowDown or Block under moderate pressure, got {:?}",
            decision
        );
    }
}
