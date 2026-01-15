//! HLX-Scale Speculation Runtime
//!
//! Implements parallel agent execution with barrier synchronization and hash verification.
//! This module enables quantum-inspired speculation and MAS-style swarm execution.

use hlx_core::{Value, Result, HlxError, HlxCrate};
use blake3::Hasher;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Barrier as SyncBarrier};
use std::thread;
use tracing::{info, warn, error, debug};

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
            warn!("HLX-SCALE:  Requested {} agents, capped at max {}",
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

/// Barrier point tracking agent states at synchronization
#[derive(Debug, Clone)]
pub struct BarrierPoint {
    /// Barrier name
    pub name: String,
    /// Agent states at this barrier
    pub agent_states: Vec<(usize, String)>, // (agent_id, state_hash)
    /// Whether consensus was reached
    pub consensus: bool,
}

/// Multi-barrier coordinator for intermediate verification
pub struct BarrierCoordinator {
    /// Barriers encountered during execution
    barriers: Arc<Mutex<HashMap<String, BarrierPoint>>>,
    /// Synchronization primitive for each barrier
    sync_barriers: Arc<Mutex<HashMap<String, Arc<SyncBarrier>>>>,
    /// Agent count
    agent_count: usize,
    /// Debug logging enabled
    debug: bool,
}

impl BarrierCoordinator {
    pub fn new(agent_count: usize, debug: bool) -> Self {
        Self {
            barriers: Arc::new(Mutex::new(HashMap::new())),
            sync_barriers: Arc::new(Mutex::new(HashMap::new())),
            agent_count,
            debug,
        }
    }

    /// Register an agent at a barrier with its state hash
    pub fn wait_at_barrier(&self, barrier_name: &str, agent_id: usize, state_hash: String) -> Result<()> {
        let log_enabled = self.debug || std::env::var("RUST_LOG").is_ok();

        // Get or create the sync barrier for this named barrier
        let sync_barrier = {
            let mut barriers = self.sync_barriers.lock().unwrap();
            barriers.entry(barrier_name.to_string())
                .or_insert_with(|| Arc::new(SyncBarrier::new(self.agent_count)))
                .clone()
        };

        // Record this agent's state
        {
            let mut barriers = self.barriers.lock().unwrap();
            let point = barriers.entry(barrier_name.to_string())
                .or_insert_with(|| BarrierPoint {
                    name: barrier_name.to_string(),
                    agent_states: Vec::new(),
                    consensus: false,
                });
            point.agent_states.push((agent_id, state_hash.clone()));

            if log_enabled {
                info!("HLX-SCALE: [AGENT-{}] Reached barrier '{}' with hash: {}",
                         agent_id, barrier_name, &state_hash[..16]);
            }
        }

        // Wait for all agents to reach this barrier
        sync_barrier.wait();

        // First agent to wake verifies consensus
        let mut barriers = self.barriers.lock().unwrap();
        let point = barriers.get_mut(barrier_name).unwrap();

        if point.agent_states.len() == self.agent_count && !point.consensus {
            // Verify all hashes match
            let first_hash = &point.agent_states[0].1;
            let all_match = point.agent_states.iter().all(|(_, h)| h == first_hash);

            if all_match {
                point.consensus = true;
                if log_enabled {
                    info!("HLX-SCALE: [BARRIER] '{}': All {} agents agree (hash: {})",
                             barrier_name, self.agent_count, &first_hash[..16]);
                }
            } else {
                // Divergence detected!
                let msg = format!(
                    "Divergence detected at barrier '{}':\n{}",
                    barrier_name,
                    point.agent_states.iter()
                        .map(|(id, h)| format!("  Agent {}: {} {}", id, &h[..16],
                            if h == first_hash { "" } else { "<- DIVERGENT" }))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                error!("HLX-SCALE/BARRIER:  {}", msg);
                return Err(HlxError::BackendError { message: msg });
            }
        }

        Ok(())
    }

    pub fn get_barrier_stats(&self) -> Vec<BarrierPoint> {
        self.barriers.lock().unwrap().values().cloned().collect()
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
        let log_enabled = self.config.debug || std::env::var("RUST_LOG").is_ok();

        if log_enabled {
            info!("HLX-SCALE:  Starting speculation with {} agents (max: {})",
                    agent_count, self.config.max_agent_count);
        }

        // Create barrier coordinator for intermediate verification
        let barrier_coordinator = Arc::new(BarrierCoordinator::new(agent_count, self.config.debug));

        // Shared barrier for final synchronization
        let barrier = Arc::new(SyncBarrier::new(agent_count));

        // Shared state for collecting results
        let results = Arc::new(Mutex::new(Vec::new()));

        // Shared error state (includes barrier divergence)
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Clone krate for each agent
        let krate = Arc::new(krate.clone());

        // Spawn agents
        let mut handles = vec![];

        for agent_id in 0..agent_count {
            let krate = Arc::clone(&krate);
            let barrier = Arc::clone(&barrier);
            let barrier_coordinator = Arc::clone(&barrier_coordinator);
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);
            let debug = self.config.debug;

            let handle = thread::spawn(move || {
                // CRITICAL: Disable speculation for nested execution (prevents infinite recursion)
                // This is thread-local and safe across all threads
                crate::disable_speculation();

                // Set barrier coordinator for this agent thread
                crate::set_barrier_coordinator(Some(barrier_coordinator), Some(agent_id));

                let log_enabled = debug || std::env::var("RUST_LOG").is_ok();

                if log_enabled {
                    info!("HLX-SCALE: [AGENT-{}] Forked and starting execution", agent_id);
                }

                // Execute the crate using the standard runtime executor
                let result = match crate::execute(&krate) {
                    Ok(val) => val,
                    Err(e) => {
                        let err_msg = format!("Agent {}: Execution failed: {}", agent_id, e);
                        if log_enabled {
                            error!(agent_id, error = %e, "HLX-SCALE/AGENT: Execution failed");
                        }
                        errors.lock().unwrap().push(err_msg);
                        return;
                    }
                };

                if log_enabled {
                    info!("HLX-SCALE: [AGENT-{}] Completed with result: {:?}", agent_id, result);
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

        // Check for errors (including barrier divergence)
        let errors = errors.lock().unwrap();
        let has_errors = !errors.is_empty();
        let error_messages = errors.clone();
        drop(errors);

        if has_errors {
            // Check if any error is a barrier divergence
            let has_divergence = error_messages.iter()
                .any(|msg| msg.contains("Divergence detected") || msg.contains("Hash mismatch"));

            if has_divergence {
                warn!("HLX-SCALE:  Barrier divergence detected during speculation");
                warn!("HLX-SCALE:  Falling back to serial execution...");

                // Fallback to serial execution
                // Temporarily disable speculation for serial fallback
                crate::disable_speculation();
                let serial_result = crate::execute(krate.as_ref());

                // Note: speculation is already disabled in this thread, no need to re-enable
                return serial_result.map(|result| {
                    if log_enabled {
                        warn!("HLX-SCALE:  Serial fallback completed successfully");
                    }
                    result
                });
            } else {
                // Non-divergence errors (execution failures)
                return Err(HlxError::BackendError {
                    message: format!("Speculation failed with errors:\n{}", error_messages.join("\n"))
                });
            }
        }

        // Verify consensus on final results
        let results = results.lock().unwrap();
        let result = self.verify_and_merge(results.as_slice())?;

        if log_enabled {
            info!("HLX-SCALE:  Speculation complete with consensus result: {:?}", result);
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
                    info!("HLX-SCALE: [AGENT-{}] State hash: {}", agent.id, hash);
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
                error!("HLX-SCALE:  {}", msg);
                warn!("HLX-SCALE:  Falling back to serial execution would be implemented here.");
                return Err(HlxError::BackendError { message: msg });
            } else {
                warn!("HLX-SCALE:  {}", msg);
                warn!("HLX-SCALE:  Continuing with first agent result (strict_verification=false)");
            }
        } else if log_enabled {
            info!("HLX-SCALE: [CONSENSUS] All {} agents agree (hash: {})", agents.len(), first_hash);
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
            debug!("BARRIER:  '{}' hit {} times", barrier_name, count);
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
