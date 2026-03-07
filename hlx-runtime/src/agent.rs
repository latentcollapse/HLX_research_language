use crate::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Defined,
    Running,
    Halted,
    Dissolved,
    Archived,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: u64,
    pub name: String,
    pub state: AgentState,
    pub latent_states: HashMap<String, Value>,
    pub cycle_counters: HashMap<String, u64>,
    pub max_cycles: HashMap<String, u64>,
    pub step_count: u64,
    pub max_steps: u64,
    pub confidence: f64,
    pub halt_threshold: f64,
}

impl Agent {
    pub fn new(id: u64, name: String) -> Self {
        Agent {
            id,
            name,
            state: AgentState::Defined,
            latent_states: HashMap::new(),
            cycle_counters: HashMap::new(),
            max_cycles: HashMap::new(),
            step_count: 0,
            max_steps: 10000,
            confidence: 0.0,
            halt_threshold: 0.95,
        }
    }

    pub fn spawn(&mut self) {
        self.state = AgentState::Running;
        self.step_count = 0;
        self.confidence = 0.0;
    }

    pub fn halt(&mut self) {
        self.state = AgentState::Halted;
    }

    pub fn dissolve(&mut self) {
        self.state = AgentState::Dissolved;
    }

    pub fn archive(&mut self) {
        self.state = AgentState::Archived;
    }

    pub fn is_running(&self) -> bool {
        self.state == AgentState::Running
    }

    pub fn should_halt(&self) -> bool {
        self.confidence >= self.halt_threshold || self.step_count >= self.max_steps
    }

    pub fn begin_cycle(&mut self, cycle_name: &str) {
        *self
            .cycle_counters
            .entry(cycle_name.to_string())
            .or_insert(0) = 0;
    }

    pub fn advance_cycle(&mut self, cycle_name: &str) -> bool {
        let current = self.cycle_counters.get(cycle_name).copied().unwrap_or(0);
        let max = self.max_cycles.get(cycle_name).copied().unwrap_or(1);

        if current < max {
            self.cycle_counters
                .insert(cycle_name.to_string(), current + 1);
            true
        } else {
            false
        }
    }

    pub fn end_cycle(&mut self, _cycle_name: &str) {
        self.step_count += 1;
    }

    pub fn set_latent(&mut self, name: &str, value: Value) {
        self.latent_states.insert(name.to_string(), value);
    }

    pub fn get_latent(&self, name: &str) -> Option<&Value> {
        self.latent_states.get(name)
    }

    pub fn set_max_cycle(&mut self, cycle_name: &str, max: u64) {
        self.max_cycles.insert(cycle_name.to_string(), max);
    }
}

#[derive(Debug)]
pub struct AgentPool {
    agents: HashMap<u64, Agent>,
    next_id: u64,
}

impl AgentPool {
    pub fn new() -> Self {
        AgentPool {
            agents: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn spawn(&mut self, name: &str) -> u64 {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("Agent ID overflow: cannot spawn more than 2^64 agents");
        let mut agent = Agent::new(id, name.to_string());
        agent.spawn();

        self.agents.insert(id, agent);
        id
    }

    pub fn all_agent_ids(&self) -> Vec<u64> {
        self.agents.keys().copied().collect()
    }

    pub fn get(&self, id: u64) -> Option<&Agent> {
        self.agents.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Agent> {
        self.agents.get_mut(&id)
    }

    pub fn halt(&mut self, id: u64) -> bool {
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.halt();
            true
        } else {
            false
        }
    }

    pub fn dissolve(&mut self, id: u64) -> Option<Agent> {
        if let Some(mut agent) = self.agents.remove(&id) {
            agent.dissolve();
            Some(agent)
        } else {
            None
        }
    }

    pub fn running_count(&self) -> usize {
        self.agents.values().filter(|a| a.is_running()).count()
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }

    pub fn all_halted(&self) -> bool {
        self.agents.values().all(|a| !a.is_running())
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_lifecycle() {
        let mut pool = AgentPool::new();

        let id = pool.spawn("TestAgent");
        assert!(pool.get(id).unwrap().is_running());

        pool.halt(id);
        assert!(!pool.get(id).unwrap().is_running());

        let agent = pool.dissolve(id);
        assert!(agent.is_some());
        assert!(pool.get(id).is_none());
    }

    #[test]
    fn test_cycle_execution() {
        let mut agent = Agent::new(0, "Test".to_string());
        agent.spawn();

        agent.set_max_cycle("outer", 3);
        agent.set_max_cycle("inner", 6);

        let mut outer_count = 0;
        let mut inner_count = 0;

        agent.begin_cycle("outer");
        while agent.advance_cycle("outer") {
            agent.begin_cycle("inner");
            while agent.advance_cycle("inner") {
                inner_count += 1;
            }
            agent.end_cycle("inner");
            outer_count += 1;
        }
        agent.end_cycle("outer");

        assert_eq!(outer_count, 3);
        assert_eq!(inner_count, 18);
    }

    #[test]
    fn test_halt_condition() {
        let mut agent = Agent::new(0, "Test".to_string());
        agent.spawn();
        agent.halt_threshold = 0.9;

        assert!(!agent.should_halt());

        agent.confidence = 0.95;
        assert!(agent.should_halt());
    }
}
