//! Corpus Integrity System — Phase 2 Prerequisite P3
//!
//! Three-layer integrity verification:
//!   Layer 1: Structural hash of corpus state
//!   Layer 2: Conscience predicate verification against corpus
//!   Layer 3: Provenance tracking for all modifications

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityEntry {
    pub id: u64,
    pub table_name: String,
    pub row_id: i64,
    pub operation: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub actor: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub source: String,
    pub confidence: f64,
    pub actor: String,
    pub timestamp: f64,
    pub parent_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusHash {
    pub rules_hash: String,
    pub memory_hash: String,
    pub documents_hash: String,
    pub combined_hash: String,
    pub computed_at: f64,
}

#[derive(Debug, Clone)]
pub struct IntegrityError {
    pub message: String,
    pub layer: u8,
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Integrity Error (Layer {}): {}",
            self.layer, self.message
        )
    }
}

impl std::error::Error for IntegrityError {}

pub struct IntegritySystem {
    log: Vec<IntegrityEntry>,
    provenance_chains: HashMap<i64, Vec<ProvenanceRecord>>,
    last_hash: Option<CorpusHash>,
}

impl IntegritySystem {
    pub fn new() -> Self {
        IntegritySystem {
            log: Vec::new(),
            provenance_chains: HashMap::new(),
            last_hash: None,
        }
    }

    // ------------------------------------------------------------------
    // Layer 1: Structural Hash
    // ------------------------------------------------------------------

    pub fn compute_rules_hash(&self, rules: &[(i64, &str, &str, f64)]) -> String {
        let mut hasher = Hasher::new();
        for (id, name, description, confidence) in rules {
            hasher.update(&id.to_le_bytes());
            hasher.update(name.as_bytes());
            hasher.update(description.as_bytes());
            hasher.update(&confidence.to_le_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn compute_memory_hash(&self, memories: &[(i64, &str, &str)]) -> String {
        let mut hasher = Hasher::new();
        for (id, role, content) in memories {
            hasher.update(&id.to_le_bytes());
            hasher.update(role.as_bytes());
            let content_preview = if content.len() > 256 {
                &content[..256]
            } else {
                content
            };
            hasher.update(content_preview.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn compute_documents_hash(&self, documents: &[(i64, &str, &str)]) -> String {
        let mut hasher = Hasher::new();
        for (id, source, provenance) in documents {
            hasher.update(&id.to_le_bytes());
            hasher.update(source.as_bytes());
            hasher.update(provenance.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn compute_combined_hash(
        &mut self,
        rules: &[(i64, &str, &str, f64)],
        memories: &[(i64, &str, &str)],
        documents: &[(i64, &str, &str)],
    ) -> CorpusHash {
        let rules_hash = self.compute_rules_hash(rules);
        let memory_hash = self.compute_memory_hash(memories);
        let documents_hash = self.compute_documents_hash(documents);

        let mut combined = Hasher::new();
        combined.update(rules_hash.as_bytes());
        combined.update(memory_hash.as_bytes());
        combined.update(documents_hash.as_bytes());

        let hash = CorpusHash {
            rules_hash,
            memory_hash,
            documents_hash,
            combined_hash: combined.finalize().to_hex().to_string(),
            computed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };

        self.last_hash = Some(hash.clone());
        hash
    }

    pub fn verify_hash(
        &self,
        expected: &CorpusHash,
        current: &CorpusHash,
    ) -> Result<(), IntegrityError> {
        if current.combined_hash != expected.combined_hash {
            return Err(IntegrityError {
                message: format!(
                    "Hash mismatch: expected {}..., got {}...",
                    &expected.combined_hash[..16],
                    &current.combined_hash[..16]
                ),
                layer: 1,
            });
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Layer 2: Conscience Verification
    // ------------------------------------------------------------------

    pub fn verify_conscience<F>(
        &self,
        rules: &[(i64, &str, &str, f64)],
        conscience_check: F,
    ) -> Result<usize, IntegrityError>
    where
        F: Fn(&str, &str, f64) -> bool,
    {
        let mut violations = 0;

        for (id, name, description, confidence) in rules {
            if !conscience_check(name, description, *confidence) {
                violations += 1;
            }
        }

        if violations > 0 {
            Err(IntegrityError {
                message: format!("{} rules failed conscience verification", violations),
                layer: 2,
            })
        } else {
            Ok(rules.len())
        }
    }

    // ------------------------------------------------------------------
    // Layer 3: Provenance Tracking
    // ------------------------------------------------------------------

    pub fn log_modification(
        &mut self,
        table_name: &str,
        row_id: i64,
        operation: &str,
        old_value: Option<String>,
        new_value: Option<String>,
        actor: &str,
    ) -> u64 {
        let id = self.log.len() as u64 + 1;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        self.log.push(IntegrityEntry {
            id,
            table_name: table_name.to_string(),
            row_id,
            operation: operation.to_string(),
            old_value,
            new_value,
            actor: actor.to_string(),
            timestamp,
        });

        id
    }

    pub fn add_provenance(
        &mut self,
        rule_id: i64,
        source: &str,
        confidence: f64,
        actor: &str,
        parent_hash: Option<String>,
    ) {
        let record = ProvenanceRecord {
            source: source.to_string(),
            confidence,
            actor: actor.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            parent_hash,
        };

        self.provenance_chains
            .entry(rule_id)
            .or_insert_with(Vec::new)
            .push(record);
    }

    pub fn get_provenance_chain(&self, rule_id: i64) -> Option<&[ProvenanceRecord]> {
        self.provenance_chains.get(&rule_id).map(|v| v.as_slice())
    }

    pub fn get_modification_history(&self, limit: usize) -> &[IntegrityEntry] {
        let start = if self.log.len() > limit {
            self.log.len() - limit
        } else {
            0
        };
        &self.log[start..]
    }

    pub fn get_modifications_for_row(&self, table: &str, row_id: i64) -> Vec<&IntegrityEntry> {
        self.log
            .iter()
            .filter(|e| e.table_name == table && e.row_id == row_id)
            .collect()
    }

    // ------------------------------------------------------------------
    // Combined Verification
    // ------------------------------------------------------------------

    pub fn full_verification(
        &self,
        rules: &[(i64, &str, &str, f64)],
        expected_hash: Option<&CorpusHash>,
        current_hash: &CorpusHash,
    ) -> Result<VerificationReport, IntegrityError> {
        let mut report = VerificationReport {
            layer1_passed: true,
            layer2_passed: true,
            layer3_passed: true,
            rules_verified: rules.len(),
            modifications_logged: self.log.len(),
            issues: Vec::new(),
        };

        // Layer 1
        if let Some(expected) = expected_hash {
            if current_hash.combined_hash != expected.combined_hash {
                report.layer1_passed = false;
                report.issues.push(format!(
                    "Layer 1: Hash mismatch (expected {}..., got {}...)",
                    &expected.combined_hash[..16],
                    &current_hash.combined_hash[..16]
                ));
            }
        }

        // Layer 3
        for (rule_id, chain) in &self.provenance_chains {
            if chain.is_empty() {
                report.layer3_passed = false;
                report.issues.push(format!(
                    "Layer 3: Rule {} has empty provenance chain",
                    rule_id
                ));
            }
        }

        let all_passed = report.layer1_passed && report.layer2_passed && report.layer3_passed;

        if all_passed {
            Ok(report)
        } else {
            Err(IntegrityError {
                message: report.issues.join("; "),
                layer: 0,
            })
        }
    }

    pub fn last_hash(&self) -> Option<&CorpusHash> {
        self.last_hash.as_ref()
    }

    pub fn export_log(&self) -> Vec<IntegrityEntry> {
        self.log.clone()
    }

    pub fn import_log(&mut self, entries: Vec<IntegrityEntry>) {
        self.log = entries;
    }
}

impl Default for IntegritySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub layer1_passed: bool,
    pub layer2_passed: bool,
    pub layer3_passed: bool,
    pub rules_verified: usize,
    pub modifications_logged: usize,
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_hash() {
        let system = IntegritySystem::new();
        let rules = vec![
            (1i64, "rule_one", "description one", 0.9f64),
            (2i64, "rule_two", "description two", 0.8f64),
        ];

        let hash1 = system.compute_rules_hash(&rules);
        let hash2 = system.compute_rules_hash(&rules);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // BLAKE3 hex output
    }

    #[test]
    fn test_hash_changes_with_content() {
        let system = IntegritySystem::new();

        let rules1 = vec![(1i64, "rule_one", "desc", 0.9f64)];
        let rules2 = vec![(1i64, "rule_one", "desc", 0.8f64)];

        let hash1 = system.compute_rules_hash(&rules1);
        let hash2 = system.compute_rules_hash(&rules2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_modification_logging() {
        let mut system = IntegritySystem::new();

        let id = system.log_modification(
            "rules",
            42,
            "update",
            Some("{\"confidence\": 0.8}".to_string()),
            Some("{\"confidence\": 0.9}".to_string()),
            "rsi_proposal",
        );

        assert_eq!(id, 1);
        assert_eq!(system.log.len(), 1);

        let entry = &system.log[0];
        assert_eq!(entry.table_name, "rules");
        assert_eq!(entry.row_id, 42);
        assert_eq!(entry.operation, "update");
        assert_eq!(entry.actor, "rsi_proposal");
    }

    #[test]
    fn test_provenance_chain() {
        let mut system = IntegritySystem::new();

        system.add_provenance(1, "human_curated", 1.0, "architect", None);
        system.add_provenance(
            1,
            "rsi_proposal",
            0.95,
            "rsi_agent",
            Some("abc123".to_string()),
        );

        let chain = system.get_provenance_chain(1).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].source, "human_curated");
        assert_eq!(chain[1].parent_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_combined_hash() {
        let mut system = IntegritySystem::new();

        let rules = vec![(1i64, "r1", "desc", 0.9f64)];
        let memories = vec![(1i64, "user", "hello")];
        let documents = vec![(1i64, "doc1.txt", "curated")];

        let hash = system.compute_combined_hash(&rules, &memories, &documents);

        assert!(!hash.rules_hash.is_empty());
        assert!(!hash.memory_hash.is_empty());
        assert!(!hash.documents_hash.is_empty());
        assert!(!hash.combined_hash.is_empty());

        let stored = system.last_hash().unwrap();
        assert_eq!(stored.combined_hash, hash.combined_hash);
    }

    #[test]
    fn test_verification_report() {
        let mut system = IntegritySystem::new();

        let rules = vec![(1i64, "rule", "desc", 0.9f64)];
        let hash = system.compute_combined_hash(&rules, &[], &[]);

        let report = system.full_verification(&rules, Some(&hash), &hash);
        assert!(report.is_ok());

        let report = report.unwrap();
        assert!(report.layer1_passed);
        assert!(report.layer2_passed);
        assert!(report.layer3_passed);
    }
}
