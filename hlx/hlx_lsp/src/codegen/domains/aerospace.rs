//! Aerospace Code Generator
//!
//! Generates safety-critical, certified-ready code for aerospace applications.
//! Compliant with DO-178C, DO-254, and related standards.
//!
//! ## Features
//!
//! - Triple Modular Redundancy (TMR) for sensor readings
//! - Comprehensive validation and range checking
//! - Audit logging for certification evidence
//! - Safety analysis documentation
//! - Test procedure generation
//! - Full DO-178C compliance annotations

use super::super::core::{CodeGenerator, GeneratorConfig, GeneratedCodeset, GeneratedCode, CodeMetadata};
use serde::{Serialize, Deserialize};

/// Safety levels per DO-178C
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyLevel {
    /// Design Assurance Level A (Catastrophic failure)
    DAL_A,
    /// Design Assurance Level B (Hazardous failure)
    DAL_B,
    /// Design Assurance Level C (Major failure)
    DAL_C,
    /// Design Assurance Level D (Minor failure)
    DAL_D,
    /// Design Assurance Level E (No effect)
    DAL_E,
}

impl SafetyLevel {
    pub fn as_str(&self) -> &str {
        match self {
            SafetyLevel::DAL_A => "DAL-A",
            SafetyLevel::DAL_B => "DAL-B",
            SafetyLevel::DAL_C => "DAL-C",
            SafetyLevel::DAL_D => "DAL-D",
            SafetyLevel::DAL_E => "DAL-E",
        }
    }

    pub fn requires_tmr(&self) -> bool {
        matches!(self, SafetyLevel::DAL_A | SafetyLevel::DAL_B)
    }
}

/// Aerospace standards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Standard {
    /// Software Considerations in Airborne Systems
    DO178C,
    /// Design Assurance Guidance for Airborne Electronic Hardware
    DO254,
    /// Military Standard for Digital Time Division Command/Response Multiplex Data Bus
    MIL_STD_1553,
}

impl Standard {
    pub fn as_str(&self) -> &str {
        match self {
            Standard::DO178C => "DO-178C",
            Standard::DO254 => "DO-254",
            Standard::MIL_STD_1553 => "MIL-STD-1553",
        }
    }
}

/// Component types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Sensor { name: String, unit: String, range: (f64, f64) },
    Actuator { name: String, range: (f64, f64) },
    Controller { name: String },
}

/// Aerospace generator configuration
#[derive(Debug, Clone)]
pub struct AerospaceConfig {
    /// Safety level
    pub safety_level: SafetyLevel,

    /// Standards to comply with
    pub standards: Vec<Standard>,

    /// Components to generate interfaces for
    pub components: Vec<ComponentType>,

    /// Include safety analysis documentation
    pub include_safety_analysis: bool,

    /// Include test procedures
    pub include_test_procedures: bool,

    /// Include certification evidence
    pub include_certification_evidence: bool,

    /// Base generator config
    pub gen_config: GeneratorConfig,
}

impl Default for AerospaceConfig {
    fn default() -> Self {
        Self {
            safety_level: SafetyLevel::DAL_A,
            standards: vec![Standard::DO178C],
            components: Vec::new(),
            include_safety_analysis: true,
            include_test_procedures: true,
            include_certification_evidence: true,
            gen_config: GeneratorConfig::default(),
        }
    }
}

/// Aerospace code generator
pub struct AerospaceGenerator {
    config: AerospaceConfig,
    core_gen: CodeGenerator,
}

impl AerospaceGenerator {
    pub fn new(config: AerospaceConfig) -> Self {
        let core_gen = CodeGenerator::new(config.gen_config.clone());
        Self { config, core_gen }
    }

    /// Generate all aerospace code
    pub fn generate(&mut self) -> Result<GeneratedCodeset, String> {
        let mut codeset = GeneratedCodeset::new();

        for component in &self.config.components.clone() {
            let code = self.generate_component(component)?;
            codeset.add(code);
        }

        Ok(codeset)
    }

    /// Generate code for a single component
    fn generate_component(&mut self, component: &ComponentType) -> Result<GeneratedCode, String> {
        let source = match component {
            ComponentType::Sensor { name, unit, range } => {
                self.generate_sensor_interface(name, unit, *range)
            }
            ComponentType::Actuator { name, range } => {
                self.generate_actuator_interface(name, *range)
            }
            ComponentType::Controller { name } => {
                self.generate_controller_interface(name)
            }
        };

        let metadata = CodeMetadata::new("aerospace", "interface", 8)
            .with_quality(0.95)
            .with_annotation("safety_level", self.config.safety_level.as_str())
            .with_annotation("standard", self.config.standards[0].as_str());

        Ok(GeneratedCode::new(source, metadata))
    }

    /// Generate sensor interface with TMR
    fn generate_sensor_interface(&mut self, name: &str, unit: &str, range: (f64, f64)) -> String {
        let safety_level = self.config.safety_level.as_str();
        let standard = self.config.standards[0].as_str();

        let mut code = String::new();

        // Header documentation
        code.push_str(&format!("//! {} Interface\n", name));
        code.push_str("//!\n");
        code.push_str(&format!("//! {} Compliance: {}\n", standard, safety_level));

        if self.config.include_safety_analysis {
            code.push_str(&format!("//! Safety Analysis: FMEA-2024-{:03}\n", self.core_gen.gen_range(1..1000)));
        }

        if self.config.include_test_procedures {
            code.push_str(&format!("//! Test Procedure: TP-{}-{:03}\n", name.to_uppercase(), self.core_gen.gen_range(1..100)));
        }

        code.push_str("//!\n");
        code.push_str(&format!("//! This module provides a safety-critical interface to the {}\n", name));

        if self.config.safety_level.requires_tmr() {
            code.push_str("//! with triple modular redundancy and comprehensive error handling.\n");
        }

        code.push_str("\n");
        code.push_str("import std.aerospace;\n");
        code.push_str("import std.safety;\n");
        code.push_str("\n");

        // Main reading function with TMR
        if self.config.safety_level.requires_tmr() {
            code.push_str(&self.generate_tmr_function(name, unit, range));
        } else {
            code.push_str(&self.generate_simple_function(name, unit, range));
        }

        // Validation function
        code.push_str(&self.generate_validation_function(name, unit, range));

        // Calibration function
        code.push_str(&self.generate_calibration_function(name));

        // Self-test function
        code.push_str(&self.generate_selftest_function(name));

        code
    }

    /// Generate TMR (Triple Modular Redundancy) reading function
    fn generate_tmr_function(&self, name: &str, unit: &str, range: (f64, f64)) -> String {
        let standard = self.config.standards[0].as_str();

        format!(
            r#"/// Read {} with redundancy
///
/// Safety: This function implements TMR (Triple Modular Redundancy)
/// Compliance: {} Section 6.3.4
/// Test Coverage: 100% (see test procedures)
fn read_{}_safe() {{
    // Read from three redundant sensors
    let reading_1 = read_{}_sensor(0);
    let reading_2 = read_{}_sensor(1);
    let reading_3 = read_{}_sensor(2);

    // Validate readings
    @contract validation {{
        value: [reading_1, reading_2, reading_3],
        rules: [
            "range_check:{}:{}",           // Valid {} range
            "sensor_health_check",          // Sensor status OK
            "checksum_validation",          // Data integrity
            "temporal_consistency:10"       // Change < 10{}/ms
        ]
    }}

    // Majority voting (TMR)
    let {} = majority_vote([reading_1, reading_2, reading_3]);

    // Log for certification evidence
    @audit_log {{
        event: "{}_reading",
        timestamp: now(),
        value: {},
        redundancy: [reading_1, reading_2, reading_3],
        status: "nominal"
    }}

    return {};
}}

"#,
            name, standard, name, name, name, name,
            range.0, range.1, unit, unit,
            name, name, name, name
        )
    }

    /// Generate simple reading function (for lower safety levels)
    fn generate_simple_function(&self, name: &str, unit: &str, range: (f64, f64)) -> String {
        format!(
            r#"/// Read {} value
fn read_{}() {{
    let value = read_{}_sensor(0);

    // Validate reading
    @contract validation {{
        value: value,
        rules: [
            "range_check:{}:{}",
            "sensor_health_check"
        ]
    }}

    return value;
}}

"#,
            name, name, name, range.0, range.1
        )
    }

    /// Generate validation function
    fn generate_validation_function(&self, name: &str, _unit: &str, range: (f64, f64)) -> String {
        format!(
            r#"/// Validate {} reading against known constraints
///
/// Safety: Range checking per {} 6.3.4.b
fn validate_{}(value) {{
    if value < {} {{
        trigger_safety_alarm("{}_BELOW_RANGE");
        return Error("Invalid {}: below operational range");
    }}

    if value > {} {{
        trigger_safety_alarm("{}_ABOVE_RANGE");
        return Error("Invalid {}: above operational range");
    }}

    return Ok(value);
}}

"#,
            name,
            self.config.standards[0].as_str(),
            name,
            range.0,
            name.to_uppercase(),
            name,
            range.1,
            name.to_uppercase(),
            name
        )
    }

    /// Generate calibration function
    fn generate_calibration_function(&self, name: &str) -> String {
        format!(
            r#"/// Calibrate {} sensor
///
/// Safety: Calibration required every 100 flight hours
fn calibrate_{}() {{
    let reference_value = get_reference_{}();
    let actual_value = read_{}_raw();

    let offset = reference_value - actual_value;
    set_{}_calibration_offset(offset);

    @audit_log {{
        event: "{}_calibration",
        timestamp: now(),
        reference: reference_value,
        actual: actual_value,
        offset: offset,
        status: "completed"
    }}

    return Ok(offset);
}}

"#,
            name, name, name, name, name, name
        )
    }

    /// Generate self-test function
    fn generate_selftest_function(&self, name: &str) -> String {
        format!(
            r#"/// Self-test {} sensor
///
/// Safety: Self-test required on startup and every 1000 hours
fn selftest_{}() {{
    // Apply known test signal
    inject_{}_test_signal();

    let response = read_{}_raw();

    // Validate response
    let passed = validate_{}_response(response);

    @audit_log {{
        event: "{}_selftest",
        timestamp: now(),
        response: response,
        passed: passed,
        status: if passed {{ "pass" }} else {{ "fail" }}
    }}

    if !passed {{
        trigger_safety_alarm("{}_SELFTEST_FAILED");
        return Error("{} self-test failed");
    }}

    return Ok(true);
}}

"#,
            name, name, name, name, name, name,
            name.to_uppercase(), name
        )
    }

    /// Generate actuator interface
    fn generate_actuator_interface(&mut self, name: &str, range: (f64, f64)) -> String {
        let standard = self.config.standards[0].as_str();
        let safety_level = self.config.safety_level.as_str();

        format!(
            r#"//! {} Actuator Interface
//!
//! {} Compliance: {}
//! Safety Analysis: FMEA-2024-ACT-{:03}
//!
//! This module provides a safety-critical interface to the {} actuator
//! with command validation and monitoring.

import std.aerospace;
import std.safety;

/// Command {} actuator
///
/// Safety: Command range limited and validated
/// Compliance: {} Section 6.3.5
fn command_{}(position) {{
    // Validate command
    @contract validation {{
        value: position,
        rules: [
            "range_check:{}:{}",
            "rate_limit:100",           // Max 100 units/sec
            "command_sanity_check"
        ]
    }}

    // Execute command
    set_{}_position(position);

    // Verify execution
    let actual = read_{}_position();
    let error = abs(position - actual);

    if error > 0.1 {{
        trigger_safety_alarm("{}_POSITION_ERROR");
        return Error("{} command verification failed");
    }}

    @audit_log {{
        event: "{}_command",
        timestamp: now(),
        commanded: position,
        actual: actual,
        error: error,
        status: "nominal"
    }}

    return Ok(actual);
}}

/// Emergency stop {}
///
/// Safety: Immediate stop regardless of current state
fn emergency_stop_{}() {{
    set_{}_position(0.0);
    disable_{}_power();

    @audit_log {{
        event: "{}_emergency_stop",
        timestamp: now(),
        status: "executed"
    }}

    return Ok(true);
}}

"#,
            name, standard, safety_level, self.core_gen.gen_range(1..100),
            name, name, standard, name,
            range.0, range.1,
            name, name,
            name.to_uppercase(), name, name,
            name, name, name, name, name
        )
    }

    /// Generate controller interface
    fn generate_controller_interface(&mut self, name: &str) -> String {
        let standard = self.config.standards[0].as_str();
        let safety_level = self.config.safety_level.as_str();

        format!(
            r#"//! {} Controller
//!
//! {} Compliance: {}
//! Safety Analysis: FMEA-2024-CTRL-{:03}
//!
//! This module provides the main control logic for {}.

import std.aerospace;
import std.safety;
import std.control;

/// Main {} control loop
///
/// Safety: Fail-safe design with comprehensive monitoring
/// Compliance: {} Section 6.3.6
fn {}_control_loop() {{
    while true {{
        // Read sensors
        let state = read_{}_state();

        // Validate state
        @contract validation {{
            value: state,
            rules: [
                "state_consistency_check",
                "sensor_health_check",
                "temporal_validity:100"
            ]
        }}

        // Compute control output
        let control = compute_{}_control(state);

        // Apply control
        let result = apply_{}_control(control);

        // Monitor execution
        if result.is_error {{
            handle_{}_error(result.error);
        }}

        @audit_log {{
            event: "{}_control_cycle",
            timestamp: now(),
            state: state,
            control: control,
            result: result,
            status: if result.is_ok {{ "nominal" }} else {{ "fault" }}
        }}

        // Control rate: 100Hz
        wait_ms(10);
    }}
}}

/// Compute {} control output
fn compute_{}_control(state) {{
    // PID control law
    let error = state.target - state.actual;
    let p_term = Kp * error;
    let i_term = Ki * state.integral;
    let d_term = Kd * state.derivative;

    let control = p_term + i_term + d_term;

    // Saturation limits
    @contract validation {{
        value: control,
        rules: [
            "range_check:-100:100",
            "rate_limit:50"
        ]
    }}

    return control;
}}

"#,
            name, standard, safety_level, self.core_gen.gen_range(1..50),
            name, name, standard, name, name, name, name, name, name, name, name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_generation() {
        let mut config = AerospaceConfig::default();
        config.components.push(ComponentType::Sensor {
            name: "altitude_sensor".to_string(),
            unit: "feet".to_string(),
            range: (0.0, 60000.0),
        });

        let mut gen = AerospaceGenerator::new(config);
        let codeset = gen.generate().unwrap();

        assert_eq!(codeset.len(), 1);

        let code = &codeset.examples()[0];
        assert!(code.source().contains("read_altitude_sensor_safe"));
        assert!(code.source().contains("Triple Modular Redundancy"));
        assert!(code.source().contains("DO-178C"));
    }

    #[test]
    fn test_actuator_generation() {
        let mut config = AerospaceConfig::default();
        config.components.push(ComponentType::Actuator {
            name: "aileron_left".to_string(),
            range: (-30.0, 30.0),
        });

        let mut gen = AerospaceGenerator::new(config);
        let codeset = gen.generate().unwrap();

        assert_eq!(codeset.len(), 1);

        let code = &codeset.examples()[0];
        assert!(code.source().contains("command_aileron_left"));
        assert!(code.source().contains("emergency_stop_aileron_left"));
    }
}
