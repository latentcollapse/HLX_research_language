use crate::{RuntimeError, RuntimeResult, Value};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierState {
    Open,
    Waiting,
    Released,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Barrier {
    pub id: u64,
    pub expected: usize,
    pub arrived: HashSet<u64>,
    pub state: BarrierState,
    pub created_at: Option<Instant>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum BarrierError {
    TimedOut {
        barrier_id: u64,
        arrived: usize,
        expected: usize,
    },
    Cancelled {
        barrier_id: u64,
    },
    AlreadyArrived {
        agent_id: u64,
        barrier_id: u64,
    },
}

impl std::fmt::Display for BarrierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarrierError::TimedOut {
                barrier_id,
                arrived,
                expected,
            } => {
                write!(
                    f,
                    "Barrier {} timed out: {}/{} agents arrived",
                    barrier_id, arrived, expected
                )
            }
            BarrierError::Cancelled { barrier_id } => {
                write!(f, "Barrier {} was cancelled", barrier_id)
            }
            BarrierError::AlreadyArrived {
                agent_id,
                barrier_id,
            } => {
                write!(
                    f,
                    "Agent {} already arrived at barrier {}",
                    agent_id, barrier_id
                )
            }
        }
    }
}

impl std::error::Error for BarrierError {}

impl Barrier {
    pub fn new(id: u64, expected: usize) -> Self {
        Barrier {
            id,
            expected,
            arrived: HashSet::new(),
            state: BarrierState::Open,
            created_at: None,
            timeout: None,
        }
    }

    pub fn with_timeout(id: u64, expected: usize, timeout: Duration) -> Self {
        Barrier {
            id,
            expected,
            arrived: HashSet::new(),
            state: BarrierState::Open,
            created_at: Some(Instant::now()),
            timeout: Some(timeout),
        }
    }

    pub fn arrive(&mut self, agent_id: u64) -> Result<bool, BarrierError> {
        if self.state == BarrierState::Cancelled {
            return Err(BarrierError::Cancelled {
                barrier_id: self.id,
            });
        }

        if self.state == BarrierState::TimedOut {
            return Err(BarrierError::TimedOut {
                barrier_id: self.id,
                arrived: self.arrived.len(),
                expected: self.expected,
            });
        }

        if let (Some(created), Some(timeout)) = (self.created_at, self.timeout) {
            if created.elapsed() > timeout {
                self.state = BarrierState::TimedOut;
                return Err(BarrierError::TimedOut {
                    barrier_id: self.id,
                    arrived: self.arrived.len(),
                    expected: self.expected,
                });
            }
        }

        if self.arrived.contains(&agent_id) {
            return Err(BarrierError::AlreadyArrived {
                agent_id,
                barrier_id: self.id,
            });
        }

        self.arrived.insert(agent_id);
        if self.arrived.len() >= self.expected {
            self.state = BarrierState::Released;
            Ok(true)
        } else {
            self.state = BarrierState::Waiting;
            Ok(false)
        }
    }

    pub fn is_released(&self) -> bool {
        self.state == BarrierState::Released
    }

    pub fn is_timed_out(&self) -> bool {
        if self.state == BarrierState::TimedOut {
            return true;
        }
        if let (Some(created), Some(timeout)) = (self.created_at, self.timeout) {
            return created.elapsed() > timeout;
        }
        false
    }

    pub fn cancel(&mut self) {
        self.state = BarrierState::Cancelled;
    }

    pub fn reset(&mut self) {
        self.arrived.clear();
        self.state = BarrierState::Open;
        self.created_at = Some(Instant::now());
    }

    pub fn time_remaining(&self) -> Option<Duration> {
        if let (Some(created), Some(timeout)) = (self.created_at, self.timeout) {
            let elapsed = created.elapsed();
            if elapsed < timeout {
                Some(timeout - elapsed)
            } else {
                Some(Duration::ZERO)
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Consensus {
    pub id: u64,
    pub votes: HashMap<u64, Value>,
    pub expected: usize,
    pub threshold: f64,
}

impl Consensus {
    pub fn new(id: u64, expected: usize, threshold: f64) -> Self {
        Consensus {
            id,
            votes: HashMap::new(),
            expected,
            threshold,
        }
    }

    pub fn vote(&mut self, agent_id: u64, value: Value) {
        self.votes.insert(agent_id, value);
    }

    pub fn is_complete(&self) -> bool {
        self.votes.len() >= self.expected
    }

    pub fn result(&self) -> RuntimeResult<ConsensusResult> {
        if !self.is_complete() {
            return Err(RuntimeError::new("Consensus not complete", 0));
        }

        let total = self.votes.len();
        if total == 0 {
            return Ok(ConsensusResult {
                winning_value: "abstain".to_string(),
                agreement: 0.0,
                agreed: false,
                total_votes: 0,
            });
        }

        let vote_counts: HashMap<String, usize> = self
            .votes
            .values()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Bool(b) => Some(b.to_string()),
                Value::I64(n) => Some(n.to_string()),
                _ => None,
            })
            .fold(HashMap::new(), |mut acc, key| {
                *acc.entry(key).or_insert(0) += 1;
                acc
            });

        let (winning_value, winning_count) = vote_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(k, &c)| (k.clone(), c))
            .unwrap_or(("abstain".to_string(), 0));

        let agreement = winning_count as f64 / total as f64;
        let agreed = agreement >= self.threshold;

        Ok(ConsensusResult {
            winning_value,
            agreement,
            agreed,
            total_votes: total,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub winning_value: String,
    pub agreement: f64,
    pub agreed: bool,
    pub total_votes: usize,
}

#[derive(Debug, Clone)]
pub struct Scale {
    pub id: u64,
    pub name: String,
    pub agents: Vec<u64>,
    pub barriers: HashMap<u64, Barrier>,
    pub consensuses: HashMap<u64, Consensus>,
    pub next_barrier_id: u64,
    pub next_consensus_id: u64,
}

impl Scale {
    pub fn new(id: u64, name: String) -> Self {
        Scale {
            id,
            name,
            agents: Vec::new(),
            barriers: HashMap::new(),
            consensuses: HashMap::new(),
            next_barrier_id: 0,
            next_consensus_id: 0,
        }
    }

    pub fn add_agent(&mut self, agent_id: u64) {
        if !self.agents.contains(&agent_id) {
            self.agents.push(agent_id);
        }
    }

    pub fn remove_agent(&mut self, agent_id: u64) {
        self.agents.retain(|&id| id != agent_id);
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn create_barrier(&mut self, expected: usize) -> u64 {
        let id = self.next_barrier_id;
        self.next_barrier_id += 1;
        self.barriers.insert(id, Barrier::new(id, expected));
        id
    }

    pub fn create_barrier_with_timeout(&mut self, expected: usize, timeout: Duration) -> u64 {
        let id = self.next_barrier_id;
        self.next_barrier_id += 1;
        self.barriers
            .insert(id, Barrier::with_timeout(id, expected, timeout));
        id
    }

    pub fn arrive_barrier(&mut self, barrier_id: u64, agent_id: u64) -> RuntimeResult<bool> {
        let barrier = self
            .barriers
            .get_mut(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        barrier
            .arrive(agent_id)
            .map_err(|e| RuntimeError::new(e.to_string(), 0))
    }

    pub fn check_barrier(&self, barrier_id: u64) -> RuntimeResult<bool> {
        let barrier = self
            .barriers
            .get(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        Ok(barrier.is_released())
    }

    pub fn is_barrier_timed_out(&self, barrier_id: u64) -> RuntimeResult<bool> {
        let barrier = self
            .barriers
            .get(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        Ok(barrier.is_timed_out())
    }

    pub fn cancel_barrier(&mut self, barrier_id: u64) -> RuntimeResult<()> {
        let barrier = self
            .barriers
            .get_mut(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        barrier.cancel();
        Ok(())
    }

    pub fn barrier_time_remaining(&self, barrier_id: u64) -> RuntimeResult<Option<Duration>> {
        let barrier = self
            .barriers
            .get(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        Ok(barrier.time_remaining())
    }

    pub fn reset_barrier(&mut self, barrier_id: u64) -> RuntimeResult<()> {
        let barrier = self
            .barriers
            .get_mut(&barrier_id)
            .ok_or_else(|| RuntimeError::new(format!("Barrier {} not found", barrier_id), 0))?;
        barrier.reset();
        Ok(())
    }

    pub fn create_consensus(&mut self, expected: usize, threshold: f64) -> u64 {
        let id = self.next_consensus_id;
        self.next_consensus_id += 1;
        self.consensuses
            .insert(id, Consensus::new(id, expected, threshold));
        id
    }

    pub fn vote(&mut self, consensus_id: u64, agent_id: u64, value: Value) -> RuntimeResult<()> {
        let consensus = self
            .consensuses
            .get_mut(&consensus_id)
            .ok_or_else(|| RuntimeError::new(format!("Consensus {} not found", consensus_id), 0))?;
        consensus.vote(agent_id, value);
        Ok(())
    }

    pub fn consensus_result(&self, consensus_id: u64) -> RuntimeResult<ConsensusResult> {
        let consensus = self
            .consensuses
            .get(&consensus_id)
            .ok_or_else(|| RuntimeError::new(format!("Consensus {} not found", consensus_id), 0))?;
        consensus.result()
    }

    pub fn broadcast(&self, _message: &Value) -> Vec<u64> {
        self.agents.clone()
    }
}

#[derive(Debug)]
pub struct ScalePool {
    scales: HashMap<u64, Scale>,
    next_id: u64,
}

impl ScalePool {
    pub fn new() -> Self {
        ScalePool {
            scales: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create(&mut self, name: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.scales.insert(id, Scale::new(id, name.to_string()));
        id
    }

    pub fn get(&self, id: u64) -> Option<&Scale> {
        self.scales.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Scale> {
        self.scales.get_mut(&id)
    }

    pub fn destroy(&mut self, id: u64) -> Option<Scale> {
        self.scales.remove(&id)
    }

    pub fn count(&self) -> usize {
        self.scales.len()
    }
}

impl Default for ScalePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier_basic() {
        let mut barrier = Barrier::new(0, 3);

        assert!(!barrier.arrive(1).unwrap());
        assert!(!barrier.arrive(2).unwrap());
        assert!(barrier.arrive(3).unwrap());
        assert!(barrier.is_released());
    }

    #[test]
    fn test_barrier_timeout() {
        let mut barrier = Barrier::with_timeout(0, 3, Duration::from_millis(10));

        barrier.arrive(1).unwrap();
        barrier.arrive(2).unwrap();

        std::thread::sleep(Duration::from_millis(20));

        let result = barrier.arrive(3);
        assert!(result.is_err());
        assert!(matches!(result, Err(BarrierError::TimedOut { .. })));
        assert!(barrier.is_timed_out());
    }

    #[test]
    fn test_barrier_cancel() {
        let mut barrier = Barrier::new(0, 3);

        barrier.arrive(1).unwrap();
        barrier.cancel();

        let result = barrier.arrive(2);
        assert!(matches!(result, Err(BarrierError::Cancelled { .. })));
    }

    #[test]
    fn test_barrier_time_remaining() {
        let barrier = Barrier::with_timeout(0, 3, Duration::from_secs(60));

        let remaining = barrier.time_remaining().unwrap();
        assert!(remaining > Duration::from_secs(58));
        assert!(remaining <= Duration::from_secs(60));
    }

    #[test]
    fn test_barrier_no_timeout() {
        let barrier = Barrier::new(0, 3);
        assert!(!barrier.is_timed_out());
        assert!(barrier.time_remaining().is_none());
    }

    #[test]
    fn test_barrier_reset() {
        let mut barrier = Barrier::new(0, 3);
        barrier.arrive(1).unwrap();
        barrier.arrive(2).unwrap();

        barrier.reset();

        assert_eq!(barrier.arrived.len(), 0);
        assert_eq!(barrier.state, BarrierState::Open);
    }

    #[test]
    fn test_scale_barrier_with_timeout() {
        let mut pool = ScalePool::new();
        let scale_id = pool.create("test");
        let scale = pool.get_mut(scale_id).unwrap();

        let barrier_id = scale.create_barrier_with_timeout(3, Duration::from_secs(60));

        assert!(scale.barrier_time_remaining(barrier_id).unwrap().is_some());
    }

    #[test]
    fn test_consensus_agreement() {
        let mut consensus = Consensus::new(0, 3, 0.6);

        consensus.vote(1, Value::Bool(true));
        consensus.vote(2, Value::Bool(true));
        consensus.vote(3, Value::Bool(false));

        let result = consensus.result().unwrap();
        assert!(result.agreed);
        assert_eq!(result.winning_value, "true");
        assert!((result.agreement - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_consensus_no_agreement() {
        let mut consensus = Consensus::new(0, 3, 0.8);

        consensus.vote(1, Value::Bool(true));
        consensus.vote(2, Value::Bool(false));
        consensus.vote(3, Value::Bool(false));

        let result = consensus.result().unwrap();
        assert!(!result.agreed);
        assert_eq!(result.winning_value, "false");
    }

    #[test]
    fn test_scale_coordination() {
        let mut pool = ScalePool::new();
        let scale_id = pool.create("test_scale");

        {
            let scale = pool.get_mut(scale_id).unwrap();
            scale.add_agent(1);
            scale.add_agent(2);
            scale.add_agent(3);

            let barrier_id = scale.create_barrier(3);
            assert!(!scale.arrive_barrier(barrier_id, 1).unwrap());
            assert!(!scale.arrive_barrier(barrier_id, 2).unwrap());
            assert!(scale.arrive_barrier(barrier_id, 3).unwrap());
        }

        assert_eq!(pool.get(scale_id).unwrap().agent_count(), 3);
    }
}
