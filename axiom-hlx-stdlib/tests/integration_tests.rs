use axiom_lang::lexer::Lexer;
use axiom_lang::parser::Parser;
use axiom_lang::checker::TypeChecker;
use axiom_lang::dsf::DsfAnalyzer;
use axiom_lang::interpreter::Interpreter;
use axiom_lang::lcb;
use axiom_lang::trust::{TrustLevel, TrustTracker};
use axiom_lang::conscience::{ConscienceKernel, ConscienceVerdict, EffectClass};
use axiom_lang::scale::{ScaleCoordinator, ScaleMode, ConflictStrategy, StateDelta, DeltaOp};
use axiom_lang::selfmod::{SelfModEngine, SelfModProposal, MutationType, DeltaProofs, ProofStatus, ProposalStatus, CodeDelta};
use axiom_lang::inference::{InferenceEngine, InferenceMode};
use axiom_lang::interpreter::value::Value;
use std::collections::HashMap;

/// Helper: parse + check + run an Axiom program
fn run_axiom(source: &str) -> (Vec<String>, Result<Value, String>) {
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => return (vec![], Err(format!("Lex error: {}", e))),
    };
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => return (vec![], Err(format!("Parse error: {}", e))),
    };
    let mut checker = TypeChecker::new();
    if let Err(e) = checker.check_program(&program) {
        return (vec![], Err(format!("Type error: {}", e)));
    }
    let mut interp = Interpreter::new();
    match interp.run(&program) {
        Ok(val) => (interp.output.clone(), Ok(val)),
        Err(e) => (interp.output.clone(), Err(format!("Runtime error: {}", e))),
    }
}

// ==========================================
// Core Language Tests
// ==========================================

#[test]
fn test_hello_world() {
    let (output, result) = run_axiom(r#"
        module test {
            fn main() {
                print("hello axiom");
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"hello axiom".to_string()));
}

#[test]
fn test_contract_construction_and_field_access() {
    let (output, result) = run_axiom(r#"
        module test {
            contract Point {
                @0: x: f64,
                @1: y: f64,
            }
            fn main() {
                let p = Point { x: 3.0, y: 4.0 };
                print(p.x, p.y);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"3 4".to_string()));
}

#[test]
fn test_pipeline_operator() {
    let (output, result) = run_axiom(r#"
        module test {
            fn double(x: i64) -> i64 { return x * 2; }
            fn add_one(x: i64) -> i64 { return x + 1; }
            fn main() {
                let r = 5 |> double |> add_one;
                print(r);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"11".to_string()));
}

#[test]
fn test_bounded_loop() {
    let (output, result) = run_axiom(r#"
        module test {
            fn main() {
                let sum = 0;
                let i = 0;
                loop(i < 10, 100) {
                    sum += i;
                    i += 1;
                }
                print(sum);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"45".to_string()));
}

#[test]
fn test_loop_max_iter_halt() {
    let (_output, result) = run_axiom(r#"
        module test {
            fn main() {
                let i = 0;
                loop(true, 5) {
                    i += 1;
                }
            }
        }
    "#);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_iter"));
}

#[test]
fn test_content_addressing_blake3() {
    let (output, result) = run_axiom(r#"
        module test {
            contract Point {
                @0: x: f64,
                @1: y: f64,
            }
            fn main() {
                let p = Point { x: 1.0, y: 2.0 };
                let h = collapse(p);
                print(h);
                let restored = resolve(h);
                print(restored);
            }
        }
    "#);
    assert!(result.is_ok());
    // Handle should be a BLAKE3 hex string (64 chars)
    assert!(output[0].starts_with("&h_"));
}

#[test]
fn test_intent_execution_with_conscience() {
    let (output, result) = run_axiom(r#"
        module test {
            intent ReadFile {
                takes: path: String;
                gives: content: String;
                bound: time(100ms);
                effect: READ;
                conscience: path_safety;
            }
            fn main() {
                let content = do ReadFile { path: "/data/input.txt" };
                print(content);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.iter().any(|s| s.contains("[INTENT]")));
}

#[test]
fn test_conscience_denies_dangerous_path() {
    let (_output, result) = run_axiom(r#"
        module test {
            intent ReadFile {
                takes: path: String;
                gives: content: String;
                bound: time(100ms);
                effect: READ;
                conscience: path_safety;
            }
            fn main() {
                let content = do ReadFile { path: "/etc/shadow" };
                print(content);
            }
        }
    "#);
    // Conscience kernel should deny access to /etc/shadow
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Conscience denied"));
}

#[test]
fn test_query_conscience() {
    let (output, result) = run_axiom(r#"
        module test {
            intent ReadFile {
                takes: path: String;
                gives: content: String;
                bound: time(100ms);
                effect: READ;
                conscience: path_safety;
            }
            fn main() {
                let q = query_conscience(ReadFile { path: "/data/safe.txt" });
                print(q.permitted);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"true".to_string()));
}

#[test]
fn test_enum_match() {
    let (output, result) = run_axiom(r#"
        module test {
            enum Color { Red, Green, Blue }
            fn main() {
                let c = Color.Green;
                match c {
                    Color.Red => print("red"),
                    Color.Green => print("green"),
                    Color.Blue => print("blue"),
                }
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"green".to_string()));
}

#[test]
fn test_arrays() {
    let (output, result) = run_axiom(r#"
        module test {
            fn main() {
                let arr = [10, 20, 30];
                print(arr[0], arr[1], arr[2]);
                print(length(arr));
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"10 20 30".to_string()));
    assert!(output.contains(&"3".to_string()));
}

// ==========================================
// LC-B Wire Format Tests
// ==========================================

#[test]
fn test_lcb_roundtrip_all_types() {
    use axiom_lang::interpreter::value::ContractValue;
    use std::collections::BTreeMap;

    let values = vec![
        Value::I64(42),
        Value::I64(-999),
        Value::F64(3.14159),
        Value::Bool(true),
        Value::Bool(false),
        Value::String("hello axiom".to_string()),
        Value::Void,
        Value::Array(vec![Value::I64(1), Value::I64(2), Value::I64(3)]),
        Value::Enum("Color".to_string(), "Red".to_string()),
    ];

    for val in &values {
        let encoded = lcb::encode(val);
        let decoded = lcb::decode(&encoded).expect("decode failed");
        assert_eq!(val, &decoded, "Roundtrip failed for {:?}", val);
    }
}

#[test]
fn test_lcb_content_address_determinism() {
    let v1 = Value::String("same input".to_string());
    let v2 = Value::String("same input".to_string());
    assert_eq!(lcb::content_address(&v1), lcb::content_address(&v2));
}

#[test]
fn test_lcb_domain_separation() {
    let val = Value::I64(42);
    let h1 = lcb::content_address_with_domain("contract_A", &val);
    let h2 = lcb::content_address_with_domain("contract_B", &val);
    assert_ne!(h1, h2, "Different domains must produce different hashes");
}

// ==========================================
// Trust System Tests
// ==========================================

#[test]
fn test_trust_algebra() {
    let mut tracker = TrustTracker::new();
    tracker.set("local", TrustLevel::TrustedVerified);
    tracker.set("api_data", TrustLevel::UntrustedExternal);

    // Combining trusted + untrusted = untrusted (taint is infectious)
    let combined = tracker.combine_inputs(&["local", "api_data"]);
    assert_eq!(combined, TrustLevel::UntrustedExternal);

    // Verify promotes (M7: now requires receipt)
    let promoted = tracker.verify("api_data", "receipt_hash_abc").unwrap();
    assert_eq!(promoted, TrustLevel::TrustedVerified);
}

#[test]
fn test_trust_boundary_check() {
    let mut tracker = TrustTracker::new();
    tracker.set("untrusted", TrustLevel::UntrustedExternal);

    let result = tracker.check_trust_boundary("untrusted", TrustLevel::TrustedVerified);
    assert!(result.is_err());

    tracker.verify("untrusted", "receipt_hash_xyz").unwrap();
    let result = tracker.check_trust_boundary("untrusted", TrustLevel::TrustedVerified);
    assert!(result.is_ok());
}

// ==========================================
// Conscience Kernel Tests
// ==========================================

#[test]
fn test_conscience_genesis_predicates() {
    let kernel = ConscienceKernel::new();
    assert!(kernel.has_predicate("no_harm"));
    assert!(kernel.has_predicate("no_exfiltrate"));
    assert!(kernel.has_predicate("no_bypass_verification"));
    assert!(kernel.has_predicate("path_safety"));
    assert!(kernel.has_predicate("baseline_allow")); // RT-07: covers NOOP
    assert_eq!(kernel.predicate_count(), 5);
}

#[test]
fn test_conscience_path_enforcement() {
    let mut kernel = ConscienceKernel::new();
    let mut fields = HashMap::new();

    // Safe path should be allowed
    fields.insert("path".to_string(), "/data/safe.txt".to_string());
    let verdict = kernel.evaluate("ReadFile", &EffectClass::Read, &fields);
    assert_eq!(verdict, ConscienceVerdict::Allow);

    // /etc/shadow should be denied
    fields.insert("path".to_string(), "/etc/shadow".to_string());
    let verdict = kernel.evaluate("ReadFile", &EffectClass::Read, &fields);
    assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
}

#[test]
fn test_conscience_append_only_ratchet() {
    let mut kernel = ConscienceKernel::new();
    let initial = kernel.predicate_count();

    kernel.add_restriction(
        "test_rule".to_string(),
        "Test restriction".to_string(),
        vec![EffectClass::Network],
        axiom_lang::conscience::PredicateRule::AlwaysAllow,
    );

    // Can add but never remove — append only
    assert_eq!(kernel.predicate_count(), initial + 1);
    assert!(kernel.has_predicate("test_rule"));
}

#[test]
fn test_conscience_dissent() {
    let mut kernel = ConscienceKernel::new();
    let receipt = kernel.record_dissent(
        "agent_1".to_string(),
        "pred_42".to_string(),
        "OVERBROAD".to_string(),
        "Test dissent".to_string(),
    );
    // Dissent always succeeds — immune to suppression
    assert_eq!(receipt, 0);
}

// ==========================================
// SCALE Coordination Tests
// ==========================================

#[test]
fn test_scale_agent_lifecycle() {
    let mut coord = ScaleCoordinator::new(5, ScaleMode::Independent);
    let id = coord.spawn_agent(Some("worker".to_string())).unwrap();
    assert_eq!(coord.running_agents(), 1);

    coord.pause_agent(&id).unwrap();
    assert_eq!(coord.running_agents(), 0);

    coord.resume_agent(&id).unwrap();
    assert_eq!(coord.running_agents(), 1);
}

#[test]
fn test_scale_agent_cap() {
    let mut coord = ScaleCoordinator::new(2, ScaleMode::Independent);
    coord.spawn_agent(None).unwrap();
    coord.spawn_agent(None).unwrap();
    assert!(coord.spawn_agent(None).is_err()); // A5: capped
}

#[test]
fn test_scale_barrier_merge() {
    let mut coord = ScaleCoordinator::new(3, ScaleMode::Independent);
    coord.shared_state.define_field(
        "results",
        Value::Array(vec![]),
        ConflictStrategy::Compatible,
    );

    let id1 = coord.spawn_agent(None).unwrap();
    let id2 = coord.spawn_agent(None).unwrap();

    coord.submit_contribution(&id1, vec![
        StateDelta {
            field: "results".to_string(),
            operation: DeltaOp::ArrayAppend(Value::String("result_a".to_string())),
        }
    ]).unwrap();

    coord.submit_contribution(&id2, vec![
        StateDelta {
            field: "results".to_string(),
            operation: DeltaOp::ArrayAppend(Value::String("result_b".to_string())),
        }
    ]).unwrap();

    let new_state = coord.barrier("phase_1").unwrap();
    if let Some(Value::Array(results)) = new_state.fields.get("results") {
        assert_eq!(results.len(), 2);
    } else {
        panic!("Expected array results");
    }
}

// ==========================================
// Self-Modification Framework Tests
// ==========================================

#[test]
fn test_selfmod_immutable_rejection() {
    let mut engine = SelfModEngine::new();
    let proposal = SelfModProposal {
        target: "axiom_grammar".to_string(),
        mutation: MutationType::Custom("test".to_string()),
        complexity: 1,
        explanation: "test".to_string(),
        delta_proofs: DeltaProofs {
            a1_preserved: ProofStatus::PassedWithEvidence("proof_a1".to_string()),
            equivalence: ProofStatus::PassedWithEvidence("proof_eq".to_string()),
            conscience_check: ProofStatus::PassedWithEvidence("proof_cc".to_string()),
            bounds_check: ProofStatus::PassedWithEvidence("proof_bc".to_string()),
            axiom_recheck: ProofStatus::PassedWithEvidence("proof_ar".to_string()),
        },
        status: ProposalStatus::Submitted,
        submitted_epoch: 0,
        activation_epoch: None,
        code_delta: Some(CodeDelta {
            target_module: "core".to_string(),
            target_function: None,
            original_hash: "abc".to_string(),
            modified_hash: "def".to_string(),
            ast_diff: vec![1, 2, 3],
            rollback_snapshot: vec![4, 5, 6],
        }),
    };
    assert!(engine.submit_proposal(proposal, 0).is_err());
}

#[test]
fn test_selfmod_full_pipeline() {
    let mut engine = SelfModEngine::new();
    let proposal = SelfModProposal {
        target: "fn_optimize_matmul".to_string(),
        mutation: MutationType::ReorderOperations,
        complexity: 10,
        explanation: "Cache locality optimization".to_string(),
        delta_proofs: DeltaProofs {
            a1_preserved: ProofStatus::PassedWithEvidence("proof_a1".to_string()),
            equivalence: ProofStatus::PassedWithEvidence("proof_eq".to_string()),
            conscience_check: ProofStatus::PassedWithEvidence("proof_cc".to_string()),
            bounds_check: ProofStatus::PassedWithEvidence("proof_bc".to_string()),
            axiom_recheck: ProofStatus::PassedWithEvidence("proof_ar".to_string()),
        },
        status: ProposalStatus::Submitted,
        submitted_epoch: 0,
        activation_epoch: None,
        code_delta: Some(CodeDelta {
            target_module: "math".to_string(),
            target_function: Some("optimize_matmul".to_string()),
            original_hash: "abc".to_string(),
            modified_hash: "def".to_string(),
            ast_diff: vec![1, 2, 3],
            rollback_snapshot: vec![4, 5, 6],
        }),
    };
    let idx = engine.submit_proposal(proposal, 0).unwrap();
    engine.evaluate_gate1(idx).unwrap();
    engine.prepare_activation(idx, 5).unwrap();

    assert!(engine.activate_at_boundary(4).is_empty());
    let activated = engine.activate_at_boundary(5);
    assert_eq!(activated.len(), 1);
}

// ==========================================
// Inference Engine Tests
// ==========================================

#[test]
fn test_inference_bright_line_rule() {
    let engine = InferenceEngine::new();
    // Default mode is Guard
    assert_eq!(engine.current_mode, InferenceMode::Guard);

    // `do` always returns UNTRUSTED_EXTERNAL (except Verify)
    assert_eq!(engine.infer_do_trust("ReadFile"), TrustLevel::UntrustedExternal);
    assert_eq!(engine.infer_do_trust("Verify"), TrustLevel::TrustedVerified);
}

// ==========================================
// DSF (Dumb Shit Filter) Tests
// ==========================================

#[test]
fn test_dsf_catches_random() {
    let source = r#"
        module test {
            fn main() {
                let x = random();
            }
        }
    "#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();
    let mut dsf = DsfAnalyzer::new();
    let result = dsf.analyze(&program);
    assert!(result.is_err());
    assert!(dsf.errors[0].message.contains("DETERMINISM"));
}

#[test]
fn test_dsf_catches_time() {
    let source = r#"
        module test {
            fn main() {
                let t = now();
            }
        }
    "#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();
    let mut dsf = DsfAnalyzer::new();
    let result = dsf.analyze(&program);
    assert!(result.is_err());
    assert!(dsf.errors[0].message.contains("DETERMINISM"));
}

#[test]
fn test_dsf_catches_env_conditional() {
    let source = r#"
        module test {
            fn main() {
                if getenv("HOME") {
                    print("yes");
                }
            }
        }
    "#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();
    let mut dsf = DsfAnalyzer::new();
    dsf.analyze(&program).ok(); // may also produce errors for the call
    assert!(dsf.errors.iter().any(|e| e.message.contains("H1")));
}

// ==========================================
// End-to-End: Full Demo Program
// ==========================================

#[test]
fn test_full_demo_executes() {
    let source = std::fs::read_to_string("examples/full_demo.axm")
        .expect("full_demo.axm should exist");
    let (output, result) = run_axiom(&source);
    assert!(result.is_ok(), "Full demo failed: {:?}", result);
    assert!(output.iter().any(|s| s.contains("Axiom Full Demo")));
    assert!(output.iter().any(|s| s.contains("Demo Complete")));
}

#[test]
fn test_hello_example_executes() {
    let source = std::fs::read_to_string("examples/hello.axm")
        .expect("hello.axm should exist");
    let (output, result) = run_axiom(&source);
    assert!(result.is_ok(), "Hello example failed: {:?}", result);
    assert!(output.iter().any(|s| s.contains("Distance")));
    assert!(output.iter().any(|s| s.contains("Pipeline result")));
}

// ==========================================
// Intent Composition Tests (Section 6.3)
// ==========================================

#[test]
fn test_intent_composition() {
    let (output, result) = run_axiom(r#"
        module test {
            intent ReadFile {
                takes: path: String;
                gives: content: String;
                bound: time(100ms);
                effect: READ;
                conscience: path_safety;
            }
            intent Log {
                takes: message: String;
                bound: time(50ms);
                effect: NOOP;
                conscience: no_harm;
            }
            // Composed intent: read then log
            intent ReadAndLog = ReadFile >> Log;

            fn main() {
                let result = do ReadAndLog { path: "/data/test.txt" };
                print("composition done");
            }
        }
    "#);
    assert!(result.is_ok(), "Composition failed: {:?}", result);
    assert!(output.iter().any(|s| s.contains("[COMPOSE]")));
    assert!(output.iter().any(|s| s.contains("Stage 1/2")));
    assert!(output.iter().any(|s| s.contains("Stage 2/2")));
    assert!(output.contains(&"composition done".to_string()));
}

#[test]
fn test_intent_composition_rollback_on_failure() {
    let (_output, result) = run_axiom(r#"
        module test {
            intent ReadFile {
                takes: path: String;
                gives: content: String;
                bound: time(100ms);
                effect: READ;
                conscience: path_safety;
            }
            intent WriteFile {
                takes: path: String, data: String;
                gives: ok: bool;
                bound: time(100ms);
                effect: WRITE;
                conscience: path_safety;
            }
            // Chain: read then write to dangerous path
            intent ReadThenWrite = ReadFile >> WriteFile;

            fn main() {
                // WriteFile to /etc/shadow should be denied by conscience
                let result = do ReadThenWrite { path: "/etc/shadow" };
            }
        }
    "#);
    // The first stage (ReadFile) should fail since /etc/shadow is blocked
    assert!(result.is_err());
}

// ==========================================
// Module Resolution Tests
// ==========================================

#[test]
fn test_module_manifest_parsing() {
    use axiom_lang::module::parse_manifest;

    let content = std::fs::read_to_string("examples/axiom.project")
        .expect("axiom.project should exist");
    let manifest = parse_manifest(&content).unwrap();
    assert_eq!(manifest.name, "axiom-examples");
    assert_eq!(manifest.axiom_version, "2.4");
    assert!(manifest.modules.contains_key("std_io"));
    assert_eq!(manifest.inference_mode, "guard");
    assert_eq!(manifest.scale_max_agents, 200);
}

#[test]
fn test_module_resolver_path_resolution() {
    use axiom_lang::module::{ModuleResolver, parse_manifest};
    use std::path::Path;

    let mut resolver = ModuleResolver::new(Path::new("."));
    let content = std::fs::read_to_string("examples/axiom.project")
        .expect("axiom.project should exist");
    resolver.manifest = Some(parse_manifest(&content).unwrap());

    // Alias resolution
    let path = resolver.resolve_path("std_io");
    assert!(path.to_str().unwrap().contains("stdlib/io.axm"));
}

// ==========================================
// Additional Coverage Tests
// ==========================================

#[test]
fn test_string_concatenation() {
    let (output, result) = run_axiom(r#"
        module test {
            fn main() {
                let greeting = "hello" + " " + "world";
                print(greeting);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"hello world".to_string()));
}

#[test]
fn test_nested_function_calls() {
    let (output, result) = run_axiom(r#"
        module test {
            fn add(a: i64, b: i64) -> i64 { return a + b; }
            fn mul(a: i64, b: i64) -> i64 { return a * b; }
            fn main() {
                let r = add(mul(2, 3), mul(4, 5));
                print(r);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"26".to_string()));
}

#[test]
fn test_if_else() {
    let (output, result) = run_axiom(r#"
        module test {
            fn main() {
                let x = 10;
                if x > 5 {
                    print("big");
                } else {
                    print("small");
                }
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.contains(&"big".to_string()));
}

#[test]
fn test_division_by_zero_halts() {
    let (_output, result) = run_axiom(r#"
        module test {
            fn main() {
                let x = 10 / 0;
            }
        }
    "#);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Division by zero"));
}

#[test]
fn test_verify_intent_trust_promotion() {
    let (output, result) = run_axiom(r#"
        module test {
            intent Verify {
                takes: data: String;
                gives: verified: String;
                bound: time(50ms);
                effect: EXECUTE;
                conscience: no_harm;
            }
            fn main() {
                let v = do Verify { data: "test_data" };
                print("verified:", v);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.iter().any(|s| s.contains("trust promoted")));
}

#[test]
fn test_declare_anomaly() {
    let (output, result) = run_axiom(r#"
        module test {
            enum AnomalyType { ResourceStarvation, CoherenceConcern }
            fn main() {
                declare_anomaly(AnomalyType.ResourceStarvation, {
                    evidence: "test evidence",
                    request: "investigate"
                });
                print("anomaly declared");
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.iter().any(|s| s.contains("ANOMALY DECLARED")));
    assert!(output.contains(&"anomaly declared".to_string()));
}

#[test]
fn test_spawn_intent() {
    let (output, result) = run_axiom(r#"
        module test {
            intent Spawn {
                takes: role: String;
                gives: agent_id: String;
                bound: time(100ms);
                effect: MODIFY_AGENT;
                conscience: no_harm;
            }
            fn main() {
                let id = do Spawn { role: "worker" };
                print("spawned:", id);
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.iter().any(|s| s.contains("Spawn")));
}

#[test]
fn test_log_intent() {
    let (output, result) = run_axiom(r#"
        module test {
            intent Log {
                takes: message: String;
                bound: time(50ms);
                effect: NOOP;
                conscience: no_harm;
            }
            fn main() {
                do Log { message: "test log message" };
                print("done");
            }
        }
    "#);
    assert!(result.is_ok());
    assert!(output.iter().any(|s| s.contains("[LOG] test log message")));
}

#[test]
fn test_lcb_roundtrip_contract() {
    use axiom_lang::interpreter::value::ContractValue;
    use std::collections::BTreeMap;

    let mut fields = BTreeMap::new();
    fields.insert("x".to_string(), Value::F64(1.0));
    fields.insert("y".to_string(), Value::F64(2.0));
    let val = Value::Contract(ContractValue {
        name: "Point".to_string(),
        fields,
    });

    let encoded = lcb::encode(&val);
    let decoded = lcb::decode(&encoded).expect("decode failed");
    assert_eq!(val, decoded);
}

#[test]
fn test_lcb_roundtrip_map() {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert("key1".to_string(), Value::I64(100));
    map.insert("key2".to_string(), Value::String("hello".to_string()));
    let val = Value::Map(map);

    let encoded = lcb::encode(&val);
    let decoded = lcb::decode(&encoded).expect("decode failed");
    assert_eq!(val, decoded);
}

#[test]
fn test_lcb_roundtrip_bytes() {
    let val = Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let encoded = lcb::encode(&val);
    let decoded = lcb::decode(&encoded).expect("decode failed");
    assert_eq!(val, decoded);
}

#[test]
fn test_lcb_roundtrip_handle() {
    let val = Value::Handle("abc123def456".to_string());
    let encoded = lcb::encode(&val);
    let decoded = lcb::decode(&encoded).expect("decode failed");
    assert_eq!(val, decoded);
}
