//! SCALE — Multi-Agent Coordination (Part IX)
//!
//! Turns a single Axiom process into a society of minds working on DIFFERENT tasks,
//! coordinated through shared physics. Not redundancy voting — a civilization.

use std::collections::{BTreeMap, HashMap};
use crate::interpreter::value::{ContractValue, Value};
use crate::lcb;

/// SCALE operating mode
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleMode {
    /// Agents receive different tasks from a work queue (default)
    Independent,
    /// All agents receive the same task, must produce identical outputs
    Redundant,
}

/// Conflict resolution strategy per-field (Section 9.3.3)
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictStrategy {
    /// Actions don't overlap — both apply
    Compatible,
    /// Later in canonical order wins, earlier logged
    Supersede,
    /// Both conflicting actions rejected, flagged
    RejectBoth,
}

/// Agent lifecycle states (Section 4.5)
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Running,
    Paused,
    Terminated,
}

/// A SCALE agent
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub state: AgentState,
    pub role: Option<String>,
    /// The agent's current contribution for this epoch
    pub contribution: Option<AgentContribution>,
    /// Coherence metric (kernel-internal, NOT agent-readable)
    coherence: f64,
}

/// An agent's contribution at a barrier
#[derive(Debug, Clone)]
pub struct AgentContribution {
    pub agent_id: String,
    pub epoch: u64,
    pub deltas: Vec<StateDelta>,
    pub contribution_hash: String,
}

/// A single state change proposed by an agent
#[derive(Debug, Clone)]
pub struct StateDelta {
    pub field: String,
    pub operation: DeltaOp,
}

#[derive(Debug, Clone)]
pub enum DeltaOp {
    /// Set a key in a Map field
    MapSet(String, Value),
    /// Append to an array field
    ArrayAppend(Value),
    /// Set the entire field
    FieldSet(Value),
}

/// The shared state — common ground truth (Section 9.4)
#[derive(Debug, Clone)]
pub struct SharedState {
    pub version: u64,
    pub fields: BTreeMap<String, Value>,
    /// Content-addressed handle of this state
    pub handle: String,
    /// Conflict strategies per field
    pub conflict_strategies: HashMap<String, ConflictStrategy>,
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            version: 0,
            fields: BTreeMap::new(),
            handle: String::new(),
            conflict_strategies: HashMap::new(),
        }
    }

    /// Define a field with its conflict strategy
    pub fn define_field(&mut self, name: &str, initial: Value, strategy: ConflictStrategy) {
        self.fields.insert(name.to_string(), initial);
        self.conflict_strategies.insert(name.to_string(), strategy);
    }

    /// Get the content-addressed handle
    pub fn compute_handle(&mut self) -> String {
        let state_value = Value::Contract(ContractValue {
            name: "SharedState".to_string(),
            fields: self.fields.clone(),
        });
        let handle = lcb::content_address_with_domain("SharedState", &state_value);
        self.handle = handle.clone();
        handle
    }
}

/// Epoch — the fundamental unit of time in Axiom (Section 9.3.1)
#[derive(Debug)]
pub struct Epoch {
    pub number: u64,
    pub state_before: SharedState,
    pub contributions: Vec<AgentContribution>,
    pub state_after: Option<SharedState>,
}

/// The SCALE coordinator
pub struct ScaleCoordinator {
    pub mode: ScaleMode,
    pub agents: Vec<Agent>,
    pub shared_state: SharedState,
    pub current_epoch: u64,
    pub epochs: Vec<Epoch>,
    pub max_agents: usize,
    /// Audit log of all barrier merges
    pub audit_log: Vec<String>,
}

impl ScaleCoordinator {
    pub fn new(max_agents: usize, mode: ScaleMode) -> Self {
        ScaleCoordinator {
            mode,
            agents: Vec::new(),
            shared_state: SharedState::new(),
            current_epoch: 0,
            epochs: Vec::new(),
            max_agents,
            audit_log: Vec::new(),
        }
    }

    /// Spawn an agent (A5: capped count)
    pub fn spawn_agent(&mut self, role: Option<String>) -> Result<String, String> {
        if self.agents.len() >= self.max_agents {
            return Err(format!(
                "A5 BOUNDED RESOURCES: Agent count capped at {}",
                self.max_agents
            ));
        }

        let id = format!(
            "agent_{:016x}",
            blake3::hash(
                format!("{}_{}", self.current_epoch, self.agents.len()).as_bytes()
            )
            .as_bytes()[..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64)
        );

        self.agents.push(Agent {
            id: id.clone(),
            state: AgentState::Running,
            role,
            contribution: None,
            coherence: 1.0,
        });

        Ok(id)
    }

    /// Submit an agent's contribution for the current epoch
    pub fn submit_contribution(
        &mut self,
        agent_id: &str,
        deltas: Vec<StateDelta>,
    ) -> Result<(), String> {
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

        if agent.state != AgentState::Running {
            return Err(format!("Agent '{}' is not running", agent_id));
        }

        // Compute contribution hash for canonical ordering
        let hash_input = format!("{}_{}", agent_id, self.current_epoch);
        let contribution_hash = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

        agent.contribution = Some(AgentContribution {
            agent_id: agent_id.to_string(),
            epoch: self.current_epoch,
            deltas,
            contribution_hash,
        });

        Ok(())
    }

    /// Execute a barrier — deterministic merge (Section 9.3.2)
    pub fn barrier(&mut self, name: &str) -> Result<SharedState, String> {
        self.audit_log.push(format!(
            "BARRIER '{}' at epoch {}",
            name, self.current_epoch
        ));

        // Collect all contributions
        let mut contributions: Vec<AgentContribution> = self
            .agents
            .iter()
            .filter_map(|a| a.contribution.clone())
            .collect();

        // Sort by canonical ordering: lexicographic sort by action_id
        // action_id = BLAKE3(agent_id || epoch_number || contribution_hash)
        contributions.sort_by(|a, b| {
            let aid = blake3::hash(
                format!("{}||{}||{}", a.agent_id, a.epoch, a.contribution_hash).as_bytes(),
            );
            let bid = blake3::hash(
                format!("{}||{}||{}", b.agent_id, b.epoch, b.contribution_hash).as_bytes(),
            );
            aid.as_bytes().cmp(bid.as_bytes())
        });

        // State(N+1) = Fold(OrderedActionSet(N), State(N))
        let mut new_state = self.shared_state.clone();
        new_state.version = self.current_epoch + 1;

        // Track conflicts per field
        let mut field_writers: HashMap<String, Vec<String>> = HashMap::new();

        for contrib in &contributions {
            for delta in &contrib.deltas {
                field_writers
                    .entry(delta.field.clone())
                    .or_default()
                    .push(contrib.agent_id.clone());

                let strategy = new_state
                    .conflict_strategies
                    .get(&delta.field)
                    .cloned()
                    .unwrap_or(ConflictStrategy::RejectBoth);

                let writers = &field_writers[&delta.field];
                if writers.len() > 1 {
                    // Conflict detected
                    match strategy {
                        ConflictStrategy::Compatible => {
                            // For arrays: concurrent appends concatenated in canonical order
                            // For maps: writes to different keys both apply
                            self.apply_delta(&mut new_state, delta)?;
                        }
                        ConflictStrategy::Supersede => {
                            // Later in canonical order wins (this one, since sorted)
                            self.apply_delta(&mut new_state, delta)?;
                            self.audit_log.push(format!(
                                "  SUPERSEDE on field '{}': {} wins",
                                delta.field, contrib.agent_id
                            ));
                        }
                        ConflictStrategy::RejectBoth => {
                            self.audit_log.push(format!(
                                "  REJECT_BOTH on field '{}': conflict between agents",
                                delta.field
                            ));
                            // Don't apply — both rejected
                        }
                    }
                } else {
                    // No conflict — apply directly
                    self.apply_delta(&mut new_state, delta)?;
                }
            }
        }

        // Compute handle for new state
        new_state.compute_handle();

        // Record the epoch
        self.epochs.push(Epoch {
            number: self.current_epoch,
            state_before: self.shared_state.clone(),
            contributions: contributions.clone(),
            state_after: Some(new_state.clone()),
        });

        // Advance epoch
        self.shared_state = new_state.clone();
        self.current_epoch += 1;

        // Clear agent contributions
        for agent in &mut self.agents {
            agent.contribution = None;
        }

        self.audit_log.push(format!(
            "  New state version: {}, handle: {}",
            new_state.version,
            &new_state.handle[..16]
        ));

        Ok(new_state)
    }

    fn apply_delta(&self, state: &mut SharedState, delta: &StateDelta) -> Result<(), String> {
        match &delta.operation {
            DeltaOp::FieldSet(val) => {
                state.fields.insert(delta.field.clone(), val.clone());
            }
            DeltaOp::MapSet(key, val) => {
                if let Some(Value::Contract(ref mut c)) = state.fields.get_mut(&delta.field) {
                    c.fields.insert(key.clone(), val.clone());
                } else {
                    // Create new map-like contract
                    let mut fields = BTreeMap::new();
                    fields.insert(key.clone(), val.clone());
                    state.fields.insert(
                        delta.field.clone(),
                        Value::Contract(ContractValue {
                            name: delta.field.clone(),
                            fields,
                        }),
                    );
                }
            }
            DeltaOp::ArrayAppend(val) => {
                if let Some(Value::Array(ref mut arr)) = state.fields.get_mut(&delta.field) {
                    arr.push(val.clone());
                } else {
                    state
                        .fields
                        .insert(delta.field.clone(), Value::Array(vec![val.clone()]));
                }
            }
        }
        Ok(())
    }

    /// Pause an agent (preserves state)
    pub fn pause_agent(&mut self, agent_id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

        if agent.state != AgentState::Running {
            return Err(format!("Agent '{}' is not running", agent_id));
        }
        agent.state = AgentState::Paused;
        Ok(())
    }

    /// Resume a paused agent
    pub fn resume_agent(&mut self, agent_id: &str) -> Result<(), String> {
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;

        if agent.state != AgentState::Paused {
            return Err(format!("Agent '{}' is not paused", agent_id));
        }
        agent.state = AgentState::Running;
        Ok(())
    }

    /// Get running agent count
    pub fn running_agents(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.state == AgentState::Running)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_agents() {
        let mut coord = ScaleCoordinator::new(5, ScaleMode::Independent);
        let id1 = coord.spawn_agent(Some("codegen".to_string())).unwrap();
        let id2 = coord.spawn_agent(Some("review".to_string())).unwrap();
        assert_eq!(coord.running_agents(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_cap() {
        let mut coord = ScaleCoordinator::new(2, ScaleMode::Independent);
        coord.spawn_agent(None).unwrap();
        coord.spawn_agent(None).unwrap();
        assert!(coord.spawn_agent(None).is_err());
    }

    #[test]
    fn test_barrier_deterministic_merge() {
        let mut coord = ScaleCoordinator::new(3, ScaleMode::Independent);

        // Setup shared state
        coord
            .shared_state
            .define_field("results", Value::Array(vec![]), ConflictStrategy::Compatible);

        let id1 = coord.spawn_agent(None).unwrap();
        let id2 = coord.spawn_agent(None).unwrap();

        // Both agents append to results (COMPATIBLE on arrays)
        coord
            .submit_contribution(
                &id1,
                vec![StateDelta {
                    field: "results".to_string(),
                    operation: DeltaOp::ArrayAppend(Value::String("result_1".to_string())),
                }],
            )
            .unwrap();

        coord
            .submit_contribution(
                &id2,
                vec![StateDelta {
                    field: "results".to_string(),
                    operation: DeltaOp::ArrayAppend(Value::String("result_2".to_string())),
                }],
            )
            .unwrap();

        let new_state = coord.barrier("phase_1").unwrap();

        // Both results should be present
        if let Some(Value::Array(results)) = new_state.fields.get("results") {
            assert_eq!(results.len(), 2);
        } else {
            panic!("Expected array results");
        }
    }

    #[test]
    fn test_agent_pause_resume() {
        let mut coord = ScaleCoordinator::new(5, ScaleMode::Independent);
        let id = coord.spawn_agent(None).unwrap();
        assert_eq!(coord.running_agents(), 1);
        coord.pause_agent(&id).unwrap();
        assert_eq!(coord.running_agents(), 0);
        coord.resume_agent(&id).unwrap();
        assert_eq!(coord.running_agents(), 1);
    }
}
