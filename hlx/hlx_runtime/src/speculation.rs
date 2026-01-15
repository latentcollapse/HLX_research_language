//! HLX-Scale Speculation Runtime
//!
//! Implements parallel agent execution with barrier synchronization and hash verification.
//! This module enables quantum-inspired speculation and MAS-style swarm execution.

use hlx_core::{Value, Result, HlxError, HlxCrate};
use blake3::Hasher;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Barrier as SyncBarrier};
use std::thread;

/// Configuration for speculation execution
#[derive(Debug, Clone)]
pub struct SpeculationConfig {
    /// Number of parallel agents to spawn
    pub agent_count: usize,
    /// Maximum allowed agent count (safety limit)
    pub max_agent_count: usize,
    /// Enable debug logging
    pub debug: bool,
    /// Enable strict hash verification (fail on mismatch)
    pub strict_verification: bool,
}

impl Default for SpeculationConfig {
    fn default() -> Self {
        Self {
            agent_count: 2,
            max_agent_count: 1024,  // Default max (Grok feedback: Q3)
            debug: false,
            strict_verification: true,
        }
    }
}

impl SpeculationConfig {
    /// Create config with clamped agent count
    pub fn with_agent_count(mut self, count: usize) -> Self {
        self.agent_count = count.min(self.max_agent_count);
        if count > self.max_agent_count {
            eprintln!("[HLX-SCALE] Warning: Requested {} agents, capped at max {}",
                     count, self.max_agent_count);
        }
        self
    }

    /// Set maximum agent count
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_agent_count = max;
        self.agent_count = self.agent_count.min(max);
        self
    }
}

/// Agent state snapshot for speculation
#[derive(Debug, Clone)]
pub struct AgentState {
    /// Agent ID
    pub id: usize,
    /// Current register values (simplified - just final result for now)
    pub result: Value,
    /// State hash at barrier
    pub state_hash: Option<String>,
}

impl AgentState {
    /// Compute hash of agent state
    pub fn compute_hash(&self) -> String {
        let mut hasher = Hasher::new();
        // Hash the result value
        let value_bytes = format!("{:?}", self.result);
        hasher.update(value_bytes.as_bytes());
        format!("{}", hasher.finalize().to_hex())
    }
}

/// Speculation coordinator manages parallel agent execution
pub struct SpeculationCoordinator {
    config: SpeculationConfig,
    /// Barrier counters for synchronization
    barrier_names: HashMap<String, usize>,
}

impl SpeculationCoordinator {
    /// Create a new speculation coordinator
    pub fn new(config: SpeculationConfig) -> Self {
        Self {
            config,
            barrier_names: HashMap::new(),
        }
    }

    /// Execute a crate with speculation (fork N agents)
    ///
    /// Each agent will execute the crate independently and results will be verified for consensus.
    pub fn execute_speculative(
        &mut self,
        krate: &HlxCrate,
    ) -> Result<Value> {
        let agent_count = self.config.agent_count.min(self.config.max_agent_count);

        if self.config.debug || std::env::var("RUST_LOG").is_ok() {
            println!("[HLX-SCALE] Starting speculation with {} agents (max: {})",
                    agent_count, self.config.max_agent_count);
        }

        // Shared barrier for synchronization
        let barrier = Arc::new(SyncBarrier::new(agent_count));

        // Shared state for collecting results
        let results = Arc::new(Mutex::new(Vec::new()));

        // Shared error state
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Clone krate for each agent
        let krate = Arc::new(krate.clone());

        // Spawn agents
        let mut handles = vec![];

        for agent_id in 0..agent_count {
            let krate = Arc::clone(&krate);
            let barrier = Arc::clone(&barrier);
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);
            let debug = self.config.debug;

            let handle = thread::spawn(move || {
                let log_enabled = debug || std::env::var("RUST_LOG").is_ok();

                if log_enabled {
                    println!("[HLX-SCALE][AGENT-{}] Forked and starting execution", agent_id);
                }

                // CRITICAL: Disable speculation for nested execution
                // Set env var to prevent infinite recursion
                std::env::set_var("HLX_SCALE_DISABLE", "1");

                // Execute the crate using the standard runtime executor
                let result = match crate::execute(&krate) {
                    Ok(val) => val,
                    Err(e) => {
                        let err_msg = format!("Agent {}: Execution failed: {}", agent_id, e);
                        if log_enabled {
                            eprintln!("[HLX-SCALE][AGENT-{}] ERROR: {}", agent_id, e);
                        }
                        errors.lock().unwrap().push(err_msg);
                        return;
                    }
                };

                if log_enabled {
                    println!("[HLX-SCALE][AGENT-{}] Completed with result: {:?}", agent_id, result);
                }

                // Store result
                let state = AgentState {
                    id: agent_id,
                    result,
                    state_hash: None,
                };

                results.lock().unwrap().push(state);

                // Wait at implicit final barrier
                barrier.wait();
            });

            handles.push(handle);
        }

        // Wait for all agents to complete
        for handle in handles {
            handle.join().expect("Agent thread panicked");
        }

        // Check for errors
        let errors = errors.lock().unwrap();
        if !errors.is_empty() {
            return Err(HlxError::BackendError {
                message: format!("Speculation failed with errors:\n{}", errors.join("\n"))
            });
        }

        // Verify consensus
        let results = results.lock().unwrap();
        let result = self.verify_and_merge(results.as_slice())?;

        if self.config.debug {
            println!("[SPECULATION] Execution complete with consensus result: {:?}", result);
        }

        Ok(result)
    }

    /// Verify consensus and merge agent results
    fn verify_and_merge(&self, agents: &[AgentState]) -> Result<Value> {
        if agents.is_empty() {
            return Err(HlxError::validation("No agents completed execution"));
        }

        let log_enabled = self.config.debug || std::env::var("RUST_LOG").is_ok();

        // Compute hashes for all agents
        let hashes: Vec<String> = agents.iter()
            .map(|agent| {
                let hash = agent.compute_hash();
                if log_enabled {
                    println!("[HLX-SCALE][AGENT-{}] State hash: {}", agent.id, hash);
                }
                hash
            })
            .collect();

        // Check consensus (all hashes match)
        let first_hash = &hashes[0];
        let consensus = hashes.iter().all(|h| h == first_hash);

        if !consensus {
            let msg = format!(
                "Swarm mismatch detected! Expected all agents to agree.\n{}",
                hashes.iter().enumerate()
                    .map(|(i, h)| format!("  Agent {}: {} {}", i, h,
                        if h == first_hash { "" } else { "<- DIVERGENT" }))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            if self.config.strict_verification {
                eprintln!("[HLX-SCALE] ERROR: {}", msg);
                eprintln!("[HLX-SCALE] Falling back to serial execution would be implemented here.");
                return Err(HlxError::BackendError { message: msg });
            } else {
                eprintln!("[HLX-SCALE] WARNING: {}", msg);
                eprintln!("[HLX-SCALE] WARNING: Continuing with first agent result (strict_verification=false)");
            }
        } else if log_enabled {
            println!("[HLX-SCALE][CONSENSUS] All {} agents agree (hash: {})", agents.len(), first_hash);
        }

        // Return first agent's result (all should be identical due to determinism)
        Ok(agents[0].result.clone())
    }

    /// Record a barrier hit (for future per-barrier verification)
    pub fn record_barrier(&mut self, name: Option<String>) {
        let barrier_name = name.unwrap_or_else(|| format!("barrier_{}", self.barrier_names.len()));
        let count = self.barrier_names.entry(barrier_name.clone()).or_insert(0);
        *count += 1;

        if self.config.debug {
            println!("[BARRIER] '{}' hit {} times", barrier_name, count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hlx_core::Instruction;

    #[test]
    fn test_agent_state_hash() {
        let state1 = AgentState {
            id: 0,
            result: Value::Integer(42),
            state_hash: None,
        };

        let state2 = AgentState {
            id: 1,
            result: Value::Integer(42),
            state_hash: None,
        };

        // Same result should produce same hash
        assert_eq!(state1.compute_hash(), state2.compute_hash());

        let state3 = AgentState {
            id: 2,
            result: Value::Integer(43),
            state_hash: None,
        };

        // Different result should produce different hash
        assert_ne!(state1.compute_hash(), state3.compute_hash());
    }

    #[test]
    fn test_speculation_coordinator_consensus() {
        let config = SpeculationConfig {
            agent_count: 4,
            debug: true,
            strict_verification: true,
        };

        let coordinator = SpeculationCoordinator::new(config);

        // Create 4 agents with identical results
        let agents = vec![
            AgentState { id: 0, result: Value::Integer(100), state_hash: None },
            AgentState { id: 1, result: Value::Integer(100), state_hash: None },
            AgentState { id: 2, result: Value::Integer(100), state_hash: None },
            AgentState { id: 3, result: Value::Integer(100), state_hash: None },
        ];

        let result = coordinator.verify_and_merge(&agents);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(100));
    }

    #[test]
    fn test_speculation_coordinator_mismatch() {
        let config = SpeculationConfig {
            agent_count: 4,
            debug: false,
            strict_verification: true,
        };

        let coordinator = SpeculationCoordinator::new(config);

        // Create agents with divergent results
        let agents = vec![
            AgentState { id: 0, result: Value::Integer(100), state_hash: None },
            AgentState { id: 1, result: Value::Integer(100), state_hash: None },
            AgentState { id: 2, result: Value::Integer(101), state_hash: None }, // Divergent!
            AgentState { id: 3, result: Value::Integer(100), state_hash: None },
        ];

        let result = coordinator.verify_and_merge(&agents);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Hash mismatch"));
    }

    #[test]
    fn test_basic_speculation_execution() {
        // Create a simple deterministic crate: 5 + 3 = 8
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);

        let config = SpeculationConfig {
            agent_count: 4,
            debug: true,
            strict_verification: true,
        };

        let mut coordinator = SpeculationCoordinator::new(config);

        // Execute with speculation
        let result = coordinator.execute_speculative(&krate);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(8));
    }
}
