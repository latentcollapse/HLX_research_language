use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub timestamp_ms: u64,
    pub source: String,
    pub content: String,
    pub relevance_score: f64,
    pub embedding: Option<Vec<f32>>,
}

impl Observation {
    pub fn new(source: impl Into<String>, content: impl Into<String>) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
         Observation {
            timestamp_ms: now,
            source: source.into(),
            content: content.into(),
            relevance_score: 1.0,
            embedding: None,
        }
    }

    pub fn with_relevance(mut self, score: f64) -> Self {
        self.relevance_score = score.clamp(0.0, 1.0);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        let mut v = embedding;
        normalize_vector(&mut v);
        self.embedding = Some(v);
        self
    }

    pub fn similarity_with(&self, other: &[f32]) -> f32 {
        match &self.embedding {
            Some(emb) => cosine_similarity(emb, other),
            None => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: u64,
    pub timestamp_ms: u64,
    pub content: String,
    pub context: String,
    pub answered: bool,
    pub answer: Option<String>,
}

impl Question {
    pub fn new(id: u64, content: impl Into<String>, context: impl Into<String>) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        Question {
            id,
            timestamp_ms: now,
            content: content.into(),
            context: context.into(),
            answered: false,
            answer: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern: String,
    pub confidence: f64,
    pub timestamp_ms: u64,
    pub hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub role: ExchangeRole,
    pub content: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub observations: usize,
    pub patterns: usize,
    pub unanswered_questions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn add_observation_with_relevance(&mut self, obs: Observation, _relevance: f64) {
        self.observations.push(obs);
        if self.observations.len() > self.config.max_observations {
            self.observations.remove(0);
        }
    }

    pub fn query_by_embedding(&self, query: &[f32], top_k: usize) -> Vec<(&Observation, f32)> {
        let mut results: Vec<(&Observation, f32)> = self.observations
            .iter()
            .map(|o| (o, o.similarity_with(query)))
            .filter(|(_, score)| *score >= self.config.min_relevance_threshold as f32)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    pub fn record_exchange(&mut self, role: ExchangeRole, content: impl Into<String>) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        self.conversation_history.push(Exchange {
            role,
            content: content.into(),
            timestamp_ms: now,
        });
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a <= 0.0 || norm_b <= 0.0 { return 0.0; }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

pub fn normalize_vector(v: &mut [f32]) {
    let mag: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 { for val in v.iter_mut() { *val /= mag; } }
}
