//! Memory Pool for Bit — Working Memory Storage
//!
//! A managed memory space where Bit stores observations, questions, and learned patterns.
//! NOT the Klyntar corpus (that's her rules/conscience). This is her working memory.
//!
//! Key properties:
//! - Observations are pruned by relevance when pool is full
//! - Learned patterns are append-only with BLAKE3 integrity
//! - Questions can be promoted to observations once answered
//! - All protected by existing security hardening

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Observation {
    pub timestamp: Instant,
    pub source: String,
    pub content: String,
    pub relevance_score: f64,
}

impl Observation {
    pub fn new(source: impl Into<String>, content: impl Into<String>) -> Self {
        Observation {
            timestamp: Instant::now(),
            source: source.into(),
            content: content.into(),
            relevance_score: 1.0,
        }
    }

    pub fn with_relevance(mut self, score: f64) -> Self {
        self.relevance_score = score.clamp(0.0, 1.0);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Question {
    pub id: u64,
    pub timestamp: Instant,
    pub content: String,
    pub context: String,
    pub answered: bool,
    pub answer: Option<String>,
    pub answer_timestamp: Option<Instant>,
}

impl Question {
    pub fn new(id: u64, content: impl Into<String>, context: impl Into<String>) -> Self {
        Question {
            id,
            timestamp: Instant::now(),
            content: content.into(),
            context: context.into(),
            answered: false,
            answer: None,
            answer_timestamp: None,
        }
    }

    pub fn answer(&mut self, answer: impl Into<String>) {
        self.answered = true;
        self.answer = Some(answer.into());
        self.answer_timestamp = Some(Instant::now());
    }

    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub timestamp: Instant,
    pub pattern: String,
    pub confidence: f64,
    pub hash: [u8; 32],
    pub observation_count: u32,
}

impl Pattern {
    pub fn new(pattern: impl Into<String>, confidence: f64) -> Self {
        let pattern_str = pattern.into();
        let hash = Self::compute_hash(&pattern_str);
        Pattern {
            timestamp: Instant::now(),
            pattern: pattern_str,
            confidence: confidence.clamp(0.0, 1.0),
            hash,
            observation_count: 1,
        }
    }

    fn compute_hash(content: &str) -> [u8; 32] {
        *blake3::hash(content.as_bytes()).as_bytes()
    }

    pub fn verify_integrity(&self) -> bool {
        self.hash == Self::compute_hash(&self.pattern)
    }

    pub fn strengthen(&mut self, delta: f64) {
        self.confidence = (self.confidence + delta).min(1.0);
        self.observation_count += 1;
    }
}

#[derive(Debug, Clone)]
pub struct Exchange {
    pub timestamp: Instant,
    pub role: ExchangeRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeRole {
    Bit,
    Human,
    System,
}

impl Exchange {
    pub fn new(role: ExchangeRole, content: impl Into<String>) -> Self {
        Exchange {
            timestamp: Instant::now(),
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
    pub max_observations: usize,
    pub max_patterns: usize,
    pub max_history: usize,
    pub max_questions: usize,
    pub min_relevance_threshold: f64,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        MemoryPoolConfig {
            max_observations: 1000,
            max_patterns: 500,
            max_history: 500,
            max_questions: 100,
            min_relevance_threshold: 0.1,
        }
    }
}

#[derive(Debug)]
pub struct MemoryPool {
    observations: Vec<Observation>,
    pending_questions: Vec<Question>,
    learned_patterns: Vec<Pattern>,
    conversation_history: Vec<Exchange>,
    config: MemoryPoolConfig,
    next_question_id: u64,
}

impl MemoryPool {
    pub fn new() -> Self {
        MemoryPool {
            observations: Vec::new(),
            pending_questions: Vec::new(),
            learned_patterns: Vec::new(),
            conversation_history: Vec::new(),
            config: MemoryPoolConfig::default(),
            next_question_id: 0,
        }
    }

    pub fn with_config(mut self, config: MemoryPoolConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_observation(&mut self, source: impl Into<String>, content: impl Into<String>) {
        let obs = Observation::new(source, content);
        self.add_observation_with_relevance(obs, 1.0);
    }

    pub fn add_observation_with_score(
        &mut self,
        source: impl Into<String>,
        content: impl Into<String>,
        relevance: f64,
    ) {
        let obs = Observation::new(source, content).with_relevance(relevance);
        self.add_observation_with_relevance(obs, relevance);
    }

    fn add_observation_with_relevance(&mut self, obs: Observation, _relevance: f64) {
        if self.observations.len() >= self.config.max_observations {
            self.prune_observations();
        }
        self.observations.push(obs);
    }

    fn prune_observations(&mut self) {
        self.observations
            .retain(|o| o.relevance_score >= self.config.min_relevance_threshold);

        if self.observations.len() >= self.config.max_observations {
            self.observations.sort_by(|a, b| {
                b.relevance_score
                    .partial_cmp(&a.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.observations
                .truncate(self.config.max_observations * 3 / 4);
        }
    }

    pub fn ask_question(&mut self, content: impl Into<String>, context: impl Into<String>) -> u64 {
        if self.pending_questions.len() >= self.config.max_questions {
            self.pending_questions.retain(|q| !q.answered);
            if self.pending_questions.len() >= self.config.max_questions {
                self.pending_questions.remove(0);
            }
        }

        let id = self.next_question_id;
        self.next_question_id += 1;

        let question = Question::new(id, content, context);
        self.pending_questions.push(question);
        id
    }

    pub fn answer_question(&mut self, question_id: u64, answer: impl Into<String>) -> bool {
        if let Some(q) = self
            .pending_questions
            .iter_mut()
            .find(|q| q.id == question_id)
        {
            q.answer(answer);
            return true;
        }
        false
    }

    pub fn promote_answer_to_observation(&mut self, question_id: u64) -> bool {
        if let Some(pos) = self
            .pending_questions
            .iter()
            .position(|q| q.id == question_id)
        {
            let q = &self.pending_questions[pos];
            if q.answered {
                if let Some(ref answer) = q.answer {
                    let source = format!("question_{}", question_id);
                    self.add_observation(source, format!("Q: {} A: {}", q.content, answer));
                    return true;
                }
            }
        }
        false
    }

    pub fn learn_pattern(&mut self, pattern: impl Into<String>, confidence: f64) {
        let pattern_str = pattern.into();

        if let Some(existing) = self
            .learned_patterns
            .iter_mut()
            .find(|p| p.pattern == pattern_str)
        {
            existing.strengthen(0.05);
            return;
        }

        if self.learned_patterns.len() >= self.config.max_patterns {
            self.learned_patterns.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.learned_patterns.truncate(self.config.max_patterns - 1);
        }

        let pattern = Pattern::new(pattern_str, confidence);
        self.learned_patterns.push(pattern);
    }

    pub fn verify_pattern_integrity(&self) -> Vec<usize> {
        self.learned_patterns
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.verify_integrity())
            .map(|(i, _)| i)
            .collect()
    }

    pub fn record_exchange(&mut self, role: ExchangeRole, content: impl Into<String>) {
        if self.conversation_history.len() >= self.config.max_history {
            self.conversation_history.remove(0);
        }
        self.conversation_history.push(Exchange::new(role, content));
    }

    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }

    pub fn questions(&self) -> &[Question] {
        &self.pending_questions
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.learned_patterns
    }

    pub fn history(&self) -> &[Exchange] {
        &self.conversation_history
    }

    pub fn stats(&self) -> MemoryStats {
        let unanswered = self
            .pending_questions
            .iter()
            .filter(|q| !q.answered)
            .count();
        MemoryStats {
            observation_count: self.observations.len(),
            pattern_count: self.learned_patterns.len(),
            history_count: self.conversation_history.len(),
            question_count: self.pending_questions.len(),
            unanswered_questions: unanswered,
            avg_relevance: if self.observations.is_empty() {
                0.0
            } else {
                self.observations
                    .iter()
                    .map(|o| o.relevance_score)
                    .sum::<f64>()
                    / self.observations.len() as f64
            },
            avg_pattern_confidence: if self.learned_patterns.is_empty() {
                0.0
            } else {
                self.learned_patterns
                    .iter()
                    .map(|p| p.confidence)
                    .sum::<f64>()
                    / self.learned_patterns.len() as f64
            },
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub observation_count: usize,
    pub pattern_count: usize,
    pub history_count: usize,
    pub question_count: usize,
    pub unanswered_questions: usize,
    pub avg_relevance: f64,
    pub avg_pattern_confidence: f64,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "observations={} patterns={} history={} questions={} ({} unanswered) avg_rel={:.2} avg_conf={:.2}",
            self.observation_count,
            self.pattern_count,
            self.history_count,
            self.question_count,
            self.unanswered_questions,
            self.avg_relevance,
            self.avg_pattern_confidence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_creation() {
        let obs = Observation::new("test_source", "test content");
        assert_eq!(obs.source, "test_source");
        assert_eq!(obs.content, "test content");
        assert_eq!(obs.relevance_score, 1.0);
    }

    #[test]
    fn test_observation_relevance() {
        let obs = Observation::new("src", "content").with_relevance(0.5);
        assert!((obs.relevance_score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_pattern_integrity() {
        let pattern = Pattern::new("test pattern", 0.9);
        assert!(pattern.verify_integrity());

        let mut corrupted = pattern.clone();
        corrupted.pattern = "corrupted pattern".to_string();
        assert!(!corrupted.verify_integrity());
    }

    #[test]
    fn test_pattern_strengthen() {
        let mut pattern = Pattern::new("test", 0.5);
        pattern.strengthen(0.1);
        assert!((pattern.confidence - 0.6).abs() < 0.001);
        assert_eq!(pattern.observation_count, 2);

        pattern.strengthen(0.5);
        assert!((pattern.confidence - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_question_answer() {
        let mut q = Question::new(1, "What is HLX?", "Context");
        assert!(!q.answered);

        q.answer("A governed inference runtime");
        assert!(q.answered);
        assert!(q.answer.is_some());
    }

    #[test]
    fn test_memory_pool_add_observation() {
        let mut pool = MemoryPool::new();
        pool.add_observation("source", "content");

        assert_eq!(pool.observations().len(), 1);
    }

    #[test]
    fn test_memory_pool_pruning() {
        let config = MemoryPoolConfig {
            max_observations: 10,
            min_relevance_threshold: 0.3,
            ..Default::default()
        };
        let mut pool = MemoryPool::new().with_config(config);

        for i in 0..15 {
            let relevance = if i < 5 { 0.1 } else { 0.8 };
            pool.add_observation_with_score("source", format!("content {}", i), relevance);
        }

        assert!(pool.observations().len() <= 10, "pool should have pruned");
        assert!(
            pool.observations().iter().all(|o| o.relevance_score >= 0.3),
            "low relevance observations should be pruned"
        );
    }

    #[test]
    fn test_memory_pool_questions() {
        let mut pool = MemoryPool::new();

        let qid = pool.ask_question("What is Bit?", "Context");
        assert_eq!(pool.questions().len(), 1);
        assert!(!pool.questions()[0].answered);

        pool.answer_question(qid, "Bit is a growing AI in HLX");
        assert!(pool.questions()[0].answered);
    }

    #[test]
    fn test_question_promotion_to_observation() {
        let mut pool = MemoryPool::new();

        let qid = pool.ask_question("What is HLX?", "Context");
        pool.answer_question(qid, "A governed runtime");

        let obs_count_before = pool.observations().len();
        pool.promote_answer_to_observation(qid);

        assert_eq!(pool.observations().len(), obs_count_before + 1);
        assert!(pool
            .observations()
            .last()
            .unwrap()
            .content
            .contains("What is HLX?"));
    }

    #[test]
    fn test_memory_pool_patterns() {
        let mut pool = MemoryPool::new();

        pool.learn_pattern("pattern1", 0.5);
        pool.learn_pattern("pattern2", 0.7);

        assert_eq!(pool.patterns().len(), 2);

        pool.learn_pattern("pattern1", 0.6);
        assert_eq!(
            pool.patterns().len(),
            2,
            "duplicate pattern should strengthen existing"
        );
        assert_eq!(pool.patterns()[0].observation_count, 2);
    }

    #[test]
    fn test_memory_pool_history() {
        let mut pool = MemoryPool::new();

        pool.record_exchange(ExchangeRole::Human, "Hello");
        pool.record_exchange(ExchangeRole::Bit, "Hi there");

        assert_eq!(pool.history().len(), 2);
    }

    #[test]
    fn test_memory_stats() {
        let mut pool = MemoryPool::new();

        pool.add_observation("s1", "c1");
        pool.add_observation_with_score("s2", "c2", 0.5);
        pool.learn_pattern("p1", 0.8);
        pool.ask_question("Q1", "ctx");

        let stats = pool.stats();
        assert_eq!(stats.observation_count, 2);
        assert_eq!(stats.pattern_count, 1);
        assert_eq!(stats.question_count, 1);
        assert_eq!(stats.unanswered_questions, 1);
    }

    #[test]
    fn test_verify_all_patterns_integrity() {
        let mut pool = MemoryPool::new();

        pool.learn_pattern("valid", 0.9);
        pool.learn_pattern("also_valid", 0.8);

        let corrupted = pool.verify_pattern_integrity();
        assert!(corrupted.is_empty(), "all patterns should be valid");
    }

    #[test]
    fn test_history_rolling_window() {
        let config = MemoryPoolConfig {
            max_history: 5,
            ..Default::default()
        };
        let mut pool = MemoryPool::new().with_config(config);

        for i in 0..10 {
            pool.record_exchange(ExchangeRole::Human, format!("Message {}", i));
        }

        assert_eq!(pool.history().len(), 5);
        assert!(
            pool.history()[0].content.contains("5"),
            "oldest should be message 5"
        );
        assert!(
            pool.history()[4].content.contains("9"),
            "newest should be message 9"
        );
    }
}
