//! Communication Channel — Bit's Message Bus
//!
//! A simple message bus that Bit uses to emit status, ask questions, and respond.
//! This is the interface between Bit and the MCP server / human operators.

use crate::homeostasis::HomeostasisStatus;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum Message {
    Status {
        status: HomeostasisStatus,
    },
    Question {
        id: u64,
        content: String,
        context: String,
    },
    Answer {
        question_id: u64,
        content: String,
    },
    Observation {
        content: String,
        source: String,
    },
    Learned {
        pattern: String,
        confidence: f64,
    },
    Promotion {
        level: String,
        previous_level: String,
    },
    HomeostasisAchieved {
        sustained_for_secs: f64,
    },
}

impl Message {
    pub fn timestamp(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, Clone)]
pub struct TimestampedMessage {
    pub timestamp: Instant,
    pub message: Message,
}

impl TimestampedMessage {
    pub fn new(message: Message) -> Self {
        TimestampedMessage {
            timestamp: Instant::now(),
            message,
        }
    }

    pub fn age_secs(&self) -> f64 {
        self.timestamp.elapsed().as_secs_f64()
    }
}

#[derive(Debug, Clone)]
pub struct CommunicationChannelConfig {
    pub max_inbox: usize,
    pub max_outbox: usize,
}

impl Default for CommunicationChannelConfig {
    fn default() -> Self {
        CommunicationChannelConfig {
            max_inbox: 100,
            max_outbox: 100,
        }
    }
}

#[derive(Debug)]
pub struct CommunicationChannel {
    outbox: Vec<TimestampedMessage>,
    inbox: Vec<TimestampedMessage>,
    config: CommunicationChannelConfig,
    next_question_id: u64,
}

impl CommunicationChannel {
    pub fn new() -> Self {
        CommunicationChannel {
            outbox: Vec::new(),
            inbox: Vec::new(),
            config: CommunicationChannelConfig::default(),
            next_question_id: 0,
        }
    }

    pub fn with_config(mut self, config: CommunicationChannelConfig) -> Self {
        self.config = config;
        self
    }

    pub fn emit_status(&mut self, status: HomeostasisStatus) {
        self.send(Message::Status { status });
    }

    pub fn emit_question(&mut self, content: impl Into<String>, context: impl Into<String>) -> u64 {
        let id = self.next_question_id;
        self.next_question_id += 1;

        self.send(Message::Question {
            id,
            content: content.into(),
            context: context.into(),
        });

        id
    }

    pub fn emit_answer(&mut self, question_id: u64, content: impl Into<String>) {
        self.send(Message::Answer {
            question_id,
            content: content.into(),
        });
    }

    pub fn emit_observation(&mut self, content: impl Into<String>, source: impl Into<String>) {
        self.send(Message::Observation {
            content: content.into(),
            source: source.into(),
        });
    }

    pub fn emit_learned(&mut self, pattern: impl Into<String>, confidence: f64) {
        self.send(Message::Learned {
            pattern: pattern.into(),
            confidence,
        });
    }

    pub fn emit_promotion(&mut self, level: impl Into<String>, previous: impl Into<String>) {
        self.send(Message::Promotion {
            level: level.into(),
            previous_level: previous.into(),
        });
    }

    pub fn emit_homeostasis(&mut self, sustained_for_secs: f64) {
        self.send(Message::HomeostasisAchieved { sustained_for_secs });
    }

    fn send(&mut self, message: Message) {
        if self.outbox.len() >= self.config.max_outbox {
            self.outbox.remove(0);
        }
        self.outbox.push(TimestampedMessage::new(message));
    }

    pub fn receive(&mut self, message: Message) {
        if self.inbox.len() >= self.config.max_inbox {
            self.inbox.remove(0);
        }
        self.inbox.push(TimestampedMessage::new(message));
    }

    pub fn outbox(&self) -> &[TimestampedMessage] {
        &self.outbox
    }

    pub fn inbox(&self) -> &[TimestampedMessage] {
        &self.inbox
    }

    pub fn take_outbox(&mut self) -> Vec<TimestampedMessage> {
        std::mem::take(&mut self.outbox)
    }

    pub fn clear_inbox(&mut self) {
        self.inbox.clear();
    }

    pub fn pending_questions(&self) -> Vec<u64> {
        self.outbox
            .iter()
            .filter_map(|tm| match &tm.message {
                Message::Question { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    pub fn stats(&self) -> ChannelStats {
        ChannelStats {
            outbox_count: self.outbox.len(),
            inbox_count: self.inbox.len(),
            pending_questions: self.pending_questions().len(),
        }
    }
}

impl Default for CommunicationChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelStats {
    pub outbox_count: usize,
    pub inbox_count: usize,
    pub pending_questions: usize,
}

impl std::fmt::Display for ChannelStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "outbox={} inbox={} pending_questions={}",
            self.outbox_count, self.inbox_count, self.pending_questions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::HomeostasisGate;

    #[test]
    fn test_emit_status() {
        let mut channel = CommunicationChannel::new();
        let status = HomeostasisGate::new().status();

        channel.emit_status(status);
        assert_eq!(channel.outbox().len(), 1);
    }

    #[test]
    fn test_emit_question() {
        let mut channel = CommunicationChannel::new();

        let id = channel.emit_question("What is HLX?", "Context");
        assert_eq!(id, 0);
        assert_eq!(channel.outbox().len(), 1);

        let id2 = channel.emit_question("Another question", "Context 2");
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_emit_answer() {
        let mut channel = CommunicationChannel::new();

        channel.emit_answer(42, "This is the answer");
        assert_eq!(channel.outbox().len(), 1);

        if let Message::Answer {
            question_id,
            content,
        } = &channel.outbox()[0].message
        {
            assert_eq!(*question_id, 42);
            assert_eq!(content, "This is the answer");
        } else {
            panic!("Expected Answer message");
        }
    }

    #[test]
    fn test_receive_message() {
        let mut channel = CommunicationChannel::new();

        channel.receive(Message::Answer {
            question_id: 1,
            content: "Response".into(),
        });

        assert_eq!(channel.inbox().len(), 1);
    }

    #[test]
    fn test_take_outbox() {
        let mut channel = CommunicationChannel::new();

        channel.emit_question("Q1", "ctx");
        channel.emit_question("Q2", "ctx");

        let messages = channel.take_outbox();
        assert_eq!(messages.len(), 2);
        assert_eq!(channel.outbox().len(), 0);
    }

    #[test]
    fn test_buffer_limits() {
        let config = CommunicationChannelConfig {
            max_inbox: 5,
            max_outbox: 5,
        };
        let mut channel = CommunicationChannel::new().with_config(config);

        for i in 0..10 {
            channel.emit_observation(format!("obs {}", i), "source");
        }

        assert_eq!(channel.outbox().len(), 5, "outbox should be limited");

        if let Message::Observation { content, .. } = &channel.outbox()[0].message {
            assert!(content.contains("obs 5"), "oldest should be evicted");
        } else {
            panic!("Expected Observation message");
        }
    }

    #[test]
    fn test_pending_questions() {
        let mut channel = CommunicationChannel::new();

        channel.emit_question("Q1", "ctx");
        channel.emit_observation("obs", "src");
        channel.emit_question("Q2", "ctx");

        let pending = channel.pending_questions();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0], 0);
        assert_eq!(pending[1], 1);
    }

    #[test]
    fn test_message_age() {
        let mut channel = CommunicationChannel::new();
        channel.emit_question("Test", "ctx");

        let age = channel.outbox()[0].age_secs();
        assert!(age < 1.0, "age should be less than 1 second");
    }

    #[test]
    fn test_channel_stats() {
        let mut channel = CommunicationChannel::new();

        channel.emit_question("Q1", "ctx");
        channel.emit_observation("obs", "src");
        channel.receive(Message::Answer {
            question_id: 1,
            content: "A".into(),
        });

        let stats = channel.stats();
        assert_eq!(stats.outbox_count, 2);
        assert_eq!(stats.inbox_count, 1);
        assert_eq!(stats.pending_questions, 1);
    }
}
