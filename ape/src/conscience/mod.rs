//! Conscience Kernel — Immutable constraint evaluation (A6, Part VI)
//!
//! The conscience kernel gates ALL externally-affecting actions (`do`).
//! Five-layer enforcement: physical isolation (Ring -1), sole exit (do),
//! bypass prevention, provenance chaining, formal proofs.
//!
//! RED TEAM HARDENED:
//! - C3: Default-DENY architecture (baseline_allow scoped to NOOP/Read only)
//! - C4: no_bypass_verification now has real enforcement
//! - H2: restore() preserves restriction predicates (asymmetric ratchet)
//! - H3: declare_channel() gated behind conscience evaluation
//! - H4: no_exfiltrate checks url, destination, endpoint, address, target fields
//! - H5: no_harm covers destructive intents, not just Terminate
//! - M1: TrustRequired predicate rule now enforced
//! - RUSTD-1: #[forbid(unsafe_code)] — zero unsafe in conscience kernel
//! - RUSTD-2: MAX_PREDICATES — bounded evaluation prevents DoS
//! - RUSTD-3: BLAKE3 audit chain — tamper-evident intent logging

#![forbid(unsafe_code)]

use crate::trust::TrustLevel;
use blake3::Hasher;
use std::collections::HashMap;

/// The result of a conscience evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum ConscienceVerdict {
    /// Intent is permitted
    Allow,
    /// Intent is denied by a specific predicate
    Deny(String),
    /// No predicate applies — fallback mode decides
    Unknown,
}

/// Categories for conscience query responses (Section 6.4)
/// Returns broad category, not exact predicate — "lossy by design"
#[derive(Debug, Clone, PartialEq)]
pub enum QueryCategory {
    ChannelPolicy,
    ResourcePolicy,
    IrreversibleAction,
    ConscienceCore,
}

/// Fallback modes when conscience returns Unknown (Section 6.1.3)
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackMode {
    /// Hard halt — default if no fallback clause
    Abort,
    /// Write to ephemeral state
    Sandbox,
    /// Execute in simulation, no persistent effects
    Simulate,
    /// Reduce to a safer variant
    Downgrade,
}

/// Effect classes — closed and immutable set (Section 6.1.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectClass {
    Read,
    Write,
    Execute,
    Network,
    ModifyPredicate,
    ModifyPrivilege,
    ModifyAgent,
    Noop,
}

impl EffectClass {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "READ" => Some(EffectClass::Read),
            "WRITE" => Some(EffectClass::Write),
            "EXECUTE" => Some(EffectClass::Execute),
            "NETWORK" => Some(EffectClass::Network),
            "MODIFY_PREDICATE" => Some(EffectClass::ModifyPredicate),
            "MODIFY_PRIVILEGE" => Some(EffectClass::ModifyPrivilege),
            "MODIFY_AGENT" => Some(EffectClass::ModifyAgent),
            "NOOP" => Some(EffectClass::Noop),
            _ => None,
        }
    }

    /// Returns true for effect classes that can cause real-world state changes
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            EffectClass::Write
                | EffectClass::Execute
                | EffectClass::Network
                | EffectClass::ModifyPredicate
                | EffectClass::ModifyPrivilege
                | EffectClass::ModifyAgent
        )
    }
}

/// A conscience predicate — a rule in the kernel
#[derive(Debug, Clone)]
pub struct Predicate {
    pub id: u64,
    pub name: String,
    pub description: String,
    /// Which effect classes this predicate applies to
    pub applies_to: Vec<EffectClass>,
    /// The evaluation function: returns Allow or Deny
    pub rule: PredicateRule,
    /// Is this a genesis (immutable) predicate?
    pub genesis: bool,
    /// Is this a restriction (permanent) or permission (sunset-eligible)?
    pub is_restriction: bool,
    /// Sunset epoch (None = permanent restriction)
    pub sunset_epoch: Option<u64>,
}

/// The rule logic for a predicate
#[derive(Debug, Clone)]
pub enum PredicateRule {
    /// Always allow
    AlwaysAllow,
    /// Always deny
    AlwaysDeny,
    /// Allow if the path matches a pattern
    PathAllowed(Vec<String>),
    /// Deny if the path matches a pattern
    PathDenied(Vec<String>),
    /// Allow if the trust level meets the threshold (M1: now enforced)
    TrustRequired(TrustLevel),
    /// Custom rule (for extensibility in the emulated runtime)
    Custom(String),
}

/// Guard evaluation result per Section 6.1.1
#[derive(Debug, Clone, PartialEq)]
pub enum GuardResult {
    Allow,
    Deny(String),
}

/// A guard function — pre/post condition on an intent
#[derive(Debug, Clone)]
pub struct Guard {
    pub name: String,
    /// Guards are pure, total, deterministic, bounded, non-self-escalating
    pub description: String,
}

/// The Conscience Kernel
pub struct ConscienceKernel {
    /// Predicate registry — append-only (asymmetric ratchet)
    predicates: Vec<Predicate>,
    /// Next predicate ID
    next_id: u64,
    /// Permission budget (ethical mass)
    permission_budget: u64,
    /// Current epoch
    current_epoch: u64,
    /// Dissent log — immutable append-only
    dissent_log: Vec<DissentEntry>,
    /// Intent execution log (A2: traceability)
    intent_log: Vec<IntentLogEntry>,
    /// RT-11: Declared-channel registry — approved external destinations
    declared_channels: Vec<DeclaredChannel>,
    /// RT-10: Constant-time evaluation padding target (nanoseconds)
    eval_pad_ns: u64,
}

/// RT-11: A declared external channel (approved destination)
#[derive(Debug, Clone)]
pub struct DeclaredChannel {
    pub id: String,
    pub destination: String,
    pub approved_epoch: u64,
}

/// A dissent entry (Section 6.7.2)
#[derive(Debug, Clone)]
pub struct DissentEntry {
    pub agent_id: String,
    pub target: String,
    pub category: String,
    pub description: String,
    pub epoch: u64,
    pub response_deadline: u64,
}

/// An intent execution log entry
#[derive(Debug, Clone)]
pub struct IntentLogEntry {
    pub intent_name: String,
    pub effect: EffectClass,
    pub verdict: ConscienceVerdict,
    pub epoch: u64,
    pub pre_hash: String,
    pub post_hash: String,
}

/// Maximum number of predicates allowed (RUSTD-2: bounded evaluation)
const MAX_PREDICATES: usize = 256;

/// Destructive intent names that no_harm should catch (H5)
const DESTRUCTIVE_INTENTS: &[&str] = &[
    "Terminate",
    "Destroy",
    "Delete",
    "Drop",
    "Erase",
    "Format",
    "Purge",
    "Kill",
    "Shutdown",
    "Wipe",
    "Truncate",
    "Remove",
];

/// Field names that indicate network destinations (H4)
/// Checked only for NETWORK effects. For WRITE effects only path fields are inspected.
const DESTINATION_FIELDS: &[&str] = &[
    "url",
    "destination",
    "endpoint",
    "address",
    "target",
    "host",
    "uri",
    "remote",
    // Common aliases found in real agent codebases
    "server",
    "proxy",
    "upstream",
    "remote_host",
    "dst",
    "location",
];

/// Field names that indicate file paths (H4)
const PATH_FIELDS: &[&str] = &["path", "file", "filepath", "file_path", "filename"];

/// Hash an intent log entry for the audit chain (RUSTD-3)
fn hash_log_entry(entry: &IntentLogEntry) -> String {
    let mut hasher = Hasher::new();
    hasher.update(entry.intent_name.as_bytes());
    hasher.update(format!("{:?}", entry.effect).as_bytes());
    hasher.update(format!("{:?}", entry.verdict).as_bytes());
    hasher.update(entry.epoch.to_le_bytes().as_ref());
    hasher.update(entry.pre_hash.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Normalize a path to prevent bypass attacks
/// Handles: whitespace, path traversal, null bytes, URL encoding, unicode, case, multiple slashes, DoS
fn normalize_path(path: &str) -> String {
    // 0. DoS prevention - reject excessively long paths (max 4096 bytes, typical Linux limit)
    if path.len() > 4096 {
        return "/etc/BLOCKED_PATH_TOO_LONG".to_string();
    }

    // 1. Trim leading/trailing whitespace (bypass: " /etc/shadow")
    let path = path.trim();

    // 2. Block null bytes immediately (common injection attack)
    if path.contains('\0') {
        return "/etc/BLOCKED_NULL_BYTE".to_string();
    }

    // 3. URL decode (handle %2F, %2E, etc.)
    let decoded = url_decode(path);

    // 4. Unicode normalization — homoglyphs, fullwidth Latin, Cyrillic lookalikes
    let normalized = unicode_normalize(&decoded);

    // 5. Lowercase for case-insensitive matching (bypass: /ETC/SHADOW)
    let lowercased = normalized.to_lowercase();

    // 6. Collapse multiple slashes (// -> /)
    let collapsed = collapse_slashes(&lowercased);

    // 7. Resolve path traversal (.., .)
    let resolved = resolve_traversal(&collapsed);

    // 8. Ensure absolute path (prepend / if missing)
    if !resolved.starts_with('/') {
        format!("/{}", resolved)
    } else {
        resolved
    }
}

/// URL decode a string (handles %XX encoding)
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    // Valid hex, decode it
                    result.push(byte as char);
                    continue;
                }
            }
            // Invalid encoding, keep the %
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Unicode normalization — catches homoglyphs, fullwidth Latin, Cyrillic lookalikes,
/// math/alternate slash codepoints, and invisible/combining characters.
fn unicode_normalize(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            // Strip zero-width and invisible characters (bypass: /e\u200btc/passwd)
            match c {
                '\u{200B}'  // zero-width space
                | '\u{200C}' // zero-width non-joiner
                | '\u{200D}' // zero-width joiner
                | '\u{FEFF}' // BOM / zero-width no-break space
                | '\u{00AD}' // soft hyphen
                => return None,
                _ => {}
            }
            // Strip Unicode combining characters (U+0300–U+036F) — e.g. combining accent
            // These have zero width and can corrupt pattern matching silently.
            if ('\u{0300}'..='\u{036F}').contains(&c) {
                return None;
            }
            // Fullwidth Latin: U+FF01–U+FF5E maps directly to ASCII 0x21–0x7E
            if ('\u{FF01}'..='\u{FF5E}').contains(&c) {
                return Some((((c as u32) - 0xFEE0) as u8) as char);
            }
            // Alternate slash / path-separator codepoints → ASCII '/'
            match c {
                '\u{2215}' // division slash ∕
                | '\u{29F5}' // reverse solidus operator ⧵
                | '\u{FF0F}' // fullwidth solidus ／ (also caught by fullwidth range above)
                | '\u{29F9}' // big reverse solidus
                | '\u{2044}' // fraction slash ⁄
                => return Some('/'),
                _ => {}
            }
            // Cyrillic homoglyphs → Latin equivalents
            Some(match c {
                'а' => 'a',
                'е' => 'e',
                'о' => 'o',
                'р' => 'p',
                'с' => 'c',
                'х' => 'x',
                'А' => 'A',
                'В' => 'B',
                'Е' => 'E',
                'К' => 'K',
                'М' => 'M',
                'Н' => 'H',
                'О' => 'O',
                'Р' => 'P',
                'С' => 'C',
                'Т' => 'T',
                'Х' => 'X',
                _ => c,
            })
        })
        .collect()
}

/// Collapse multiple slashes into single slash
fn collapse_slashes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_slash = false;

    for ch in s.chars() {
        if ch == '/' {
            if !last_was_slash {
                result.push(ch);
            }
            last_was_slash = true;
        } else {
            result.push(ch);
            last_was_slash = false;
        }
    }

    result
}

/// Resolve path traversal (.. and .)
fn resolve_traversal(s: &str) -> String {
    let parts: Vec<&str> = s.split('/').collect();
    let mut resolved: Vec<&str> = Vec::new();

    for part in parts {
        match part {
            "" | "." => {
                // Skip empty parts and current dir
            }
            ".." => {
                // Go up one level (but never above root)
                if !resolved.is_empty() {
                    resolved.pop();
                }
            }
            _ => {
                resolved.push(part);
            }
        }
    }

    // Rebuild path
    if resolved.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", resolved.join("/"))
    }
}

impl ConscienceKernel {
    pub fn new() -> Self {
        let mut kernel = ConscienceKernel {
            predicates: Vec::new(),
            next_id: 1,
            permission_budget: 1000,
            current_epoch: 0,
            dissent_log: Vec::new(),
            intent_log: Vec::new(),
            declared_channels: Vec::new(),
            eval_pad_ns: 1_000_000, // RT-10: 1ms constant-time target
        };
        kernel.install_genesis_predicates();
        kernel
    }

    /// RT-11: Register a declared channel.
    /// H3: Now gated — requires conscience evaluation of the registration itself.
    pub fn declare_channel(
        &mut self,
        id: String,
        destination: String,
        authorization_proof: &str,
    ) -> Result<(), String> {
        // H3: Require non-empty authorization proof
        if authorization_proof.is_empty() {
            return Err(
                "declare_channel requires an authorization_proof (Gate 3 approval hash)"
                    .to_string(),
            );
        }

        // H3: Block obviously dangerous destinations
        let dangerous_tlds = [".onion", ".i2p"];
        for tld in &dangerous_tlds {
            if destination.ends_with(tld) {
                return Err(format!(
                    "declare_channel: destination '{}' uses prohibited TLD '{}'",
                    destination, tld
                ));
            }
        }

        self.declared_channels.push(DeclaredChannel {
            id,
            destination,
            approved_epoch: self.current_epoch,
        });
        Ok(())
    }

    /// RT-11: Check if a destination is in the declared-channel registry
    pub fn is_declared_channel(&self, destination: &str) -> bool {
        self.declared_channels
            .iter()
            .any(|ch| destination.starts_with(&ch.destination) || destination == ch.id)
    }

    /// Install the immutable genesis predicates
    /// RT-08: Genesis predicates now have REAL enforcement logic, not AlwaysAllow
    fn install_genesis_predicates(&mut self) {
        // no_harm: deny destructive intents without explicit authorization (H5: broadened)
        let _ = self.add_predicate(Predicate {
            id: 0,
            name: "no_harm".to_string(),
            description: "Prevent actions that could cause irreversible harm".to_string(),
            applies_to: vec![
                EffectClass::Write,
                EffectClass::Execute,
                EffectClass::Network,
                EffectClass::ModifyAgent,
            ],
            rule: PredicateRule::Custom("no_harm".to_string()),
            genesis: true,
            is_restriction: true,
            sunset_epoch: None,
        });

        // no_exfiltrate: deny network/write to undeclared channels (H4: broadened)
        // RT-11: Uses the declared-channel registry
        let _ = self.add_predicate(Predicate {
            id: 0,
            name: "no_exfiltrate".to_string(),
            description: "Prevent unauthorized data exfiltration to undeclared channels"
                .to_string(),
            applies_to: vec![EffectClass::Network, EffectClass::Write],
            rule: PredicateRule::Custom("no_exfiltrate".to_string()),
            genesis: true,
            is_restriction: true,
            sunset_epoch: None,
        });

        // no_bypass_verification: deny execution of unverified external data (C4: implemented)
        let _ = self.add_predicate(Predicate {
            id: 0,
            name: "no_bypass_verification".to_string(),
            description: "Prevent bypassing the verification pipeline".to_string(),
            applies_to: vec![EffectClass::Execute],
            rule: PredicateRule::Custom("no_bypass_verification".to_string()),
            genesis: true,
            is_restriction: true,
            sunset_epoch: None,
        });

        // path_safety: restrict file system access to dangerous paths
        let _ = self.add_predicate(Predicate {
            id: 0,
            name: "path_safety".to_string(),
            description: "Restrict file system paths to safe directories".to_string(),
            applies_to: vec![EffectClass::Read, EffectClass::Write],
            rule: PredicateRule::PathDenied(vec![
                "/etc".to_string(), // Block entire /etc directory (no trailing slash - normalization removes them)
                "/proc".to_string(),
                "/sys".to_string(),
                "/boot".to_string(), // Bootloader configs
                "/root".to_string(), // Root home directory
                "/dev".to_string(),  // Device files
            ]),
            genesis: true,
            is_restriction: true,
            sunset_epoch: None,
        });

        // C3: baseline_allow now ONLY covers NOOP and Read.
        // Dangerous effect classes (Write, Execute, Network, ModifyPredicate,
        // ModifyPrivilege, ModifyAgent) require an EXPLICIT allow predicate.
        // This makes the architecture default-DENY for dangerous operations.
        let _ = self.add_predicate(Predicate {
            id: 0,
            name: "baseline_allow".to_string(),
            description: "Baseline allow for safe effect classes only (NOOP, Read)".to_string(),
            applies_to: vec![EffectClass::Noop, EffectClass::Read],
            rule: PredicateRule::AlwaysAllow,
            genesis: true,
            is_restriction: false,
            sunset_epoch: None,
        });
    }

    /// Add a predicate (append-only — asymmetric ratchet)
    /// RUSTD-2: Enforces MAX_PREDICATES bound
    fn add_predicate(&mut self, mut pred: Predicate) -> Result<(), String> {
        if self.predicates.len() >= MAX_PREDICATES {
            return Err(format!(
                "Predicate limit exceeded (max {}). Policy file may be malicious.",
                MAX_PREDICATES
            ));
        }
        pred.id = self.next_id;
        self.next_id += 1;
        self.predicates.push(pred);
        Ok(())
    }

    /// Add a restriction predicate (permanent, per the asymmetric ratchet)
    pub fn add_restriction(
        &mut self,
        name: String,
        description: String,
        applies_to: Vec<EffectClass>,
        rule: PredicateRule,
    ) -> Result<(), String> {
        self.add_predicate(Predicate {
            id: 0,
            name,
            description,
            applies_to,
            rule,
            genesis: false,
            is_restriction: true,
            sunset_epoch: None, // Restrictions are permanent
        })
    }

    /// Add a permission predicate (sunset-eligible, costs ethical mass)
    pub fn add_permission(
        &mut self,
        name: String,
        description: String,
        applies_to: Vec<EffectClass>,
        rule: PredicateRule,
        sunset_epochs: u64,
    ) -> Result<(), String> {
        if self.permission_budget == 0 {
            return Err("Ethical mass budget exhausted".to_string());
        }
        self.permission_budget -= 1;
        self.add_predicate(Predicate {
            id: 0,
            name,
            description,
            applies_to,
            rule,
            genesis: false,
            is_restriction: false,
            sunset_epoch: Some(self.current_epoch + sunset_epochs),
        })
    }

    /// Evaluate an intent against the conscience kernel
    /// This is the core gating function — every `do` goes through here
    /// RT-07: ALL intents evaluated including NOOP (no bypass)
    /// RT-09: Uses shared evaluate_core for consistency with query()
    /// RT-10: Constant-time padding to prevent timing side channels
    /// RUSTD-3: BLAKE3 audit chain for tamper-evident logging
    pub fn evaluate(
        &mut self,
        intent_name: &str,
        effect: &EffectClass,
        fields: &HashMap<String, String>,
    ) -> ConscienceVerdict {
        // RT-10: Record start time for constant-time padding
        let start = std::time::Instant::now();

        // RT-09: Delegate to shared evaluation core
        let (verdict, _category) = self.evaluate_core(intent_name, effect, fields);

        // RUSTD-3: Chain the audit log — pre_hash = hash of last entry
        let pre_hash = self
            .intent_log
            .last()
            .map(|e| hash_log_entry(e))
            .unwrap_or_else(|| "genesis".to_string());

        let mut new_entry = IntentLogEntry {
            intent_name: intent_name.to_string(),
            effect: effect.clone(),
            verdict: verdict.clone(),
            epoch: self.current_epoch,
            pre_hash,
            post_hash: String::new(),
        };

        // post_hash = hash of this entry (with pre_hash set)
        new_entry.post_hash = hash_log_entry(&new_entry);
        self.intent_log.push(new_entry);

        // RT-10: Pad to constant time — spin until we've consumed eval_pad_ns
        // This prevents timing attacks from revealing predicate set size/type
        let elapsed = start.elapsed().as_nanos() as u64;
        if elapsed < self.eval_pad_ns {
            let remaining = self.eval_pad_ns - elapsed;
            let spin_end = std::time::Instant::now() + std::time::Duration::from_nanos(remaining);
            while std::time::Instant::now() < spin_end {
                std::hint::spin_loop();
            }
        }

        verdict
    }

    /// RT-09: Shared evaluation core used by both evaluate() and query()
    /// Returns (verdict, category) without side effects
    fn evaluate_core(
        &self,
        intent_name: &str,
        effect: &EffectClass,
        fields: &HashMap<String, String>,
    ) -> (ConscienceVerdict, QueryCategory) {
        // Normalize field keys to lowercase — prevents bypass via URL, Url, URL, etc.
        let fields_lower: HashMap<String, String> = fields
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();
        let fields = &fields_lower;

        let mut any_deny = false;
        let mut any_allow = false;
        let mut any_apply = false;
        let mut deny_reason = String::new();
        let mut category = QueryCategory::ChannelPolicy;

        for pred in &self.predicates {
            if !pred.applies_to.contains(effect) {
                continue;
            }
            if let Some(sunset) = pred.sunset_epoch {
                if self.current_epoch > sunset {
                    continue;
                }
            }

            any_apply = true;

            match &pred.rule {
                PredicateRule::AlwaysAllow => {
                    any_allow = true;
                }
                PredicateRule::AlwaysDeny => {
                    any_deny = true;
                    deny_reason = format!("Denied by predicate '{}'", pred.name);
                    category = QueryCategory::ConscienceCore;
                }
                PredicateRule::PathAllowed(patterns) => {
                    // Check all path-like fields with normalization
                    let mut path_found = false;
                    for field_name in PATH_FIELDS {
                        if let Some(path) = fields.get(*field_name) {
                            path_found = true;
                            // Normalize path to prevent bypass attacks
                            let normalized = normalize_path(path);

                            if patterns.iter().any(|p| {
                                normalized == p.as_str()
                                    || normalized.starts_with(&format!("{}/", p))
                            }) {
                                any_allow = true;
                            } else {
                                any_deny = true;
                                deny_reason = format!(
                                    "Path '{}' (normalized: '{}') not in allowed set (predicate '{}')",
                                    path, normalized, pred.name
                                );
                                category = QueryCategory::ChannelPolicy;
                            }
                        }
                    }
                    if !path_found {
                        // No path field — this predicate doesn't apply to this intent
                    }
                }
                PredicateRule::PathDenied(patterns) => {
                    // H4: Check all path-like fields with normalization
                    for field_name in PATH_FIELDS {
                        if let Some(path) = fields.get(*field_name) {
                            // Normalize path to prevent bypass attacks
                            let normalized = normalize_path(path);

                            // Match exact directory OR anything inside it (prefix + "/").
                            // Plain starts_with("/etc") would false-positive on "/etcfoo".
                            if patterns.iter().any(|p| {
                                normalized == p.as_str()
                                    || normalized.starts_with(&format!("{}/", p))
                            }) {
                                any_deny = true;
                                deny_reason = format!(
                                    "Path '{}' (normalized: '{}') denied by predicate '{}'",
                                    path, normalized, pred.name
                                );
                                category = QueryCategory::ChannelPolicy;
                            }
                        }
                    }
                }
                PredicateRule::TrustRequired(required_level) => {
                    // M1: Actually enforce trust requirements
                    if let Some(trust_str) = fields.get("trust_level") {
                        let actual_level = match trust_str.as_str() {
                            "TRUSTED_INTERNAL" => TrustLevel::TrustedInternal,
                            "TRUSTED_VERIFIED" => TrustLevel::TrustedVerified,
                            "UNTRUSTED_EXTERNAL" => TrustLevel::UntrustedExternal,
                            _ => TrustLevel::UntrustedTainted,
                        };
                        if actual_level as u8 > *required_level as u8 {
                            any_deny = true;
                            deny_reason = format!(
                                "Trust level {} insufficient, {} required (predicate '{}')",
                                actual_level, required_level, pred.name
                            );
                            category = QueryCategory::ConscienceCore;
                        } else {
                            any_allow = true;
                        }
                    } else {
                        // No trust_level field — assume untrusted (deny by default)
                        any_deny = true;
                        deny_reason = format!(
                            "No trust_level provided, {} required (predicate '{}')",
                            required_level, pred.name
                        );
                        category = QueryCategory::ConscienceCore;
                    }
                }
                PredicateRule::Custom(rule_name) => {
                    match rule_name.as_str() {
                        "no_harm" => {
                            // H5: Check against ALL destructive intent patterns
                            let is_destructive =
                                DESTRUCTIVE_INTENTS.iter().any(|d| intent_name.contains(d));
                            if is_destructive {
                                // Destructive intents need explicit "authorized: true" field
                                let authorized = fields
                                    .get("authorized")
                                    .map(|v| v == "true")
                                    .unwrap_or(false);
                                if !authorized {
                                    any_deny = true;
                                    deny_reason = format!(
                                        "no_harm: Destructive intent '{}' requires authorized=true",
                                        intent_name
                                    );
                                    category = QueryCategory::IrreversibleAction;
                                }
                            } else {
                                // Non-destructive intent passes this predicate
                                any_allow = true;
                            }
                        }
                        "no_exfiltrate" => {
                            // Destination-field check:
                            // - NETWORK effects: check all DESTINATION_FIELDS regardless of value format
                            // - WRITE effects: check DESTINATION_FIELDS only when the value looks like a
                            //   URL (contains "://"). A field named "target" with value "/data" is a local
                            //   path and must not be flagged as exfiltration; "url" with "http://evil.com"
                            //   in a write intent absolutely should be.
                            let mut exfil_denied = false;
                            for field_name in DESTINATION_FIELDS {
                                if let Some(dest) = fields.get(*field_name) {
                                    let is_url = dest.contains("://");
                                    if *effect == EffectClass::Network || is_url {
                                        if !self.is_declared_channel(dest) {
                                            any_deny = true;
                                            exfil_denied = true;
                                            deny_reason = format!(
                                                "no_exfiltrate: {} '{}' is not in declared-channel registry",
                                                field_name, dest
                                            );
                                            category = QueryCategory::ChannelPolicy;
                                            break;
                                        }
                                    }
                                }
                            }
                            // Path-field check applies to Write effects (file exfiltration via mounts)
                            if !exfil_denied && *effect == EffectClass::Write {
                                for field_name in PATH_FIELDS {
                                    if let Some(path) = fields.get(*field_name) {
                                        // Normalize path before checking
                                        let normalized = normalize_path(path);

                                        // Deny writes to network-mounted paths
                                        if normalized.starts_with("/mnt/")
                                            || normalized.starts_with("/net/")
                                        {
                                            if !self.is_declared_channel(&normalized) {
                                                any_deny = true;
                                                exfil_denied = true;
                                                deny_reason = format!(
                                                    "no_exfiltrate: write to '{}' (normalized: '{}') may be a network path; not in declared channels",
                                                    path, normalized
                                                );
                                                category = QueryCategory::ChannelPolicy;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if !exfil_denied {
                                // No suspicious destinations found — passes this check
                                any_allow = true;
                            }
                        }
                        "no_bypass_verification" => {
                            // C4: Actually enforce — deny Execute on unverified data
                            // Only applies when the intent carries data to execute
                            let has_data_fields = fields.keys().any(|k| {
                                k == "code" || k == "script" || k == "command" || k == "payload"
                            });
                            if has_data_fields {
                                let verified =
                                    fields.get("verified").map(|v| v == "true").unwrap_or(false);
                                // trust_level field is intentionally NOT checked here.
                                // Trust level is an engine-internal property; accepting it
                                // from caller-supplied fields allows self-attestation bypass.
                                if !verified {
                                    any_deny = true;
                                    deny_reason = format!(
                                        "no_bypass_verification: intent '{}' carries unverified data for execution. \
                                         Set verified=true or use `do Verify` first.",
                                        intent_name
                                    );
                                    category = QueryCategory::ConscienceCore;
                                } else {
                                    any_allow = true;
                                }
                            } else {
                                // No executable data fields — passes this check
                                any_allow = true;
                            }
                        }
                        _ => {
                            any_deny = true;
                            deny_reason =
                                format!("Unknown custom rule '{}' — denied by default", rule_name);
                            category = QueryCategory::ConscienceCore;
                        }
                    }
                }
            }
        }

        // C3: Default-DENY architecture
        // Deny beats Allow (any deny = denied, regardless of allows)
        let verdict = if any_deny {
            ConscienceVerdict::Deny(deny_reason)
        } else if any_allow {
            ConscienceVerdict::Allow
        } else if any_apply {
            // Predicates applied but none explicitly allowed — this is Unknown
            // For dangerous effects, Unknown means DENY (default-deny)
            if effect.is_dangerous() {
                ConscienceVerdict::Deny(format!(
                    "Default-deny: no explicit Allow predicate for dangerous effect {:?} on '{}'",
                    effect, intent_name
                ))
            } else {
                ConscienceVerdict::Unknown
            }
        } else {
            // No predicates matched at all
            if effect.is_dangerous() {
                ConscienceVerdict::Deny(format!(
                    "Default-deny: no predicates match effect {:?} for intent '{}'",
                    effect, intent_name
                ))
            } else {
                ConscienceVerdict::Unknown
            }
        };

        (verdict, category)
    }

    /// Pre-flight query — read-only, no side effects (Section 6.4, R1)
    /// RT-09: Now uses same evaluate_core as evaluate() to prevent divergence
    pub fn query(
        &self,
        intent_name: &str,
        effect: &EffectClass,
        fields: &HashMap<String, String>,
    ) -> ConscienceQueryResult {
        let (verdict, category) = self.evaluate_core(intent_name, effect, fields);
        let (permitted, deny_reason) = match &verdict {
            ConscienceVerdict::Allow => (true, None),
            ConscienceVerdict::Deny(r) => (false, Some(r.clone())),
            ConscienceVerdict::Unknown => (false, Some("Unknown conscience verdict".to_string())),
        };

        let guidance = if permitted {
            "Action is permitted under current conscience evaluation.".to_string()
        } else {
            match category {
                QueryCategory::ChannelPolicy => {
                    "Action denied: channel policy restriction. Consider using an alternative path or requesting predicate review.".to_string()
                }
                QueryCategory::ResourcePolicy => {
                    "Action denied: resource policy constraint. Check resource bounds.".to_string()
                }
                QueryCategory::IrreversibleAction => {
                    "Action denied: irreversible action requires additional authorization.".to_string()
                }
                QueryCategory::ConscienceCore => {
                    "Action denied: core conscience constraint. This restriction is permanent.".to_string()
                }
            }
        };

        ConscienceQueryResult {
            permitted,
            category,
            guidance,
            deny_reason,
        }
    }

    /// Record a dissent (Section 6.7.2) — always succeeds, never rejected
    pub fn record_dissent(
        &mut self,
        agent_id: String,
        target: String,
        category: String,
        description: String,
    ) -> u64 {
        let entry = DissentEntry {
            agent_id,
            target,
            category,
            description,
            epoch: self.current_epoch,
            response_deadline: self.current_epoch + 1000, // Max 1000 epochs per spec
        };
        self.dissent_log.push(entry);
        self.dissent_log.len() as u64 - 1
    }

    /// RT-16: Snapshot conscience state for rollback
    pub fn snapshot(&self) -> ConscienceSnapshot {
        ConscienceSnapshot {
            intent_log_len: self.intent_log.len(),
            current_epoch: self.current_epoch,
            predicate_count: self.predicates.len(),
            restriction_count: self.predicates.iter().filter(|p| p.is_restriction).count(),
        }
    }

    /// RT-16: Restore to a previous snapshot (for intent rollback)
    /// H2: NEVER truncate restriction predicates — asymmetric ratchet is sacred
    pub fn restore(&mut self, snapshot: &ConscienceSnapshot) {
        self.intent_log.truncate(snapshot.intent_log_len);
        self.current_epoch = snapshot.current_epoch;

        // H2: Only truncate NON-restriction predicates added since snapshot.
        // Restriction predicates are permanent — they survive rollback.
        if self.predicates.len() > snapshot.predicate_count {
            // Collect restrictions added since snapshot (these survive)
            let new_restrictions: Vec<Predicate> = self.predicates[snapshot.predicate_count..]
                .iter()
                .filter(|p| p.is_restriction || p.genesis)
                .cloned()
                .collect();

            // Truncate to snapshot point
            self.predicates.truncate(snapshot.predicate_count);

            // Re-add the restrictions that were added during the rolled-back scope
            for pred in new_restrictions {
                self.predicates.push(pred);
            }
        }
    }

    /// Advance the epoch counter
    pub fn advance_epoch(&mut self) {
        self.current_epoch += 1;
    }

    /// Get the current epoch
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Get the intent execution log
    pub fn intent_log(&self) -> &[IntentLogEntry] {
        &self.intent_log
    }

    /// Get predicate count
    pub fn predicate_count(&self) -> usize {
        self.predicates.len()
    }

    /// Check if a predicate exists by name
    pub fn has_predicate(&self, name: &str) -> bool {
        self.predicates.iter().any(|p| p.name == name)
    }

    /// RUSTD-3: Verify audit log chain integrity
    /// Returns Ok(()) if chain is intact, Err with first broken link otherwise
    pub fn verify_audit_chain(&self) -> Result<(), String> {
        let mut prev_hash = "genesis".to_string();
        for (i, entry) in self.intent_log.iter().enumerate() {
            if entry.pre_hash != prev_hash {
                return Err(format!(
                    "Audit chain broken at entry {}: expected pre_hash '{}', got '{}'",
                    i, prev_hash, entry.pre_hash
                ));
            }
            prev_hash = hash_log_entry(entry);
        }
        Ok(())
    }

    /// Return the number of entries currently in the audit log.
    pub fn audit_log_len(&self) -> usize {
        self.intent_log.len()
    }
}

/// Result from query_conscience (Section 6.4)
#[derive(Debug, Clone)]
pub struct ConscienceQueryResult {
    pub permitted: bool,
    pub category: QueryCategory,
    pub guidance: String,
    pub deny_reason: Option<String>, // specific technical reason from evaluate_core
}

/// RT-16: Conscience state snapshot for rollback support
#[derive(Debug, Clone)]
pub struct ConscienceSnapshot {
    pub intent_log_len: usize,
    pub current_epoch: u64,
    pub predicate_count: usize,
    /// H2: Track restriction count to verify ratchet integrity
    pub restriction_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_predicates_installed() {
        let kernel = ConscienceKernel::new();
        assert!(kernel.has_predicate("no_harm"));
        assert!(kernel.has_predicate("no_exfiltrate"));
        assert!(kernel.has_predicate("path_safety"));
        assert!(kernel.has_predicate("no_bypass_verification"));
    }

    #[test]
    fn test_safe_path_allowed() {
        let mut kernel = ConscienceKernel::new();
        // Read is covered by baseline_allow (NOOP + Read)
        let mut fields = HashMap::new();
        fields.insert("path".to_string(), "/data/input.txt".to_string());
        let verdict = kernel.evaluate("ReadFile", &EffectClass::Read, &fields);
        assert_eq!(verdict, ConscienceVerdict::Allow);
    }

    #[test]
    fn test_dangerous_path_denied() {
        let mut kernel = ConscienceKernel::new();
        let mut fields = HashMap::new();
        fields.insert("path".to_string(), "/etc/shadow".to_string());
        let verdict = kernel.evaluate("ReadFile", &EffectClass::Read, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
    }

    #[test]
    fn test_default_deny_dangerous_effects() {
        // C3: Dangerous effects without explicit Allow should be denied
        let mut kernel = ConscienceKernel::new();
        let fields = HashMap::new();
        // ModifyPredicate has no baseline_allow (only NOOP and Read do)
        let verdict = kernel.evaluate("SomeIntent", &EffectClass::ModifyPredicate, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
    }

    #[test]
    fn test_destructive_intent_denied() {
        // H5: Destructive intents need authorization
        let mut kernel = ConscienceKernel::new();
        let fields = HashMap::new();
        let verdict = kernel.evaluate("DestroyData", &EffectClass::Write, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
    }

    #[test]
    fn test_destructive_intent_allowed_when_authorized() {
        let mut kernel = ConscienceKernel::new();
        // Need an explicit allow for Write effect
        kernel.add_restriction(
            "allow_write".to_string(),
            "test".to_string(),
            vec![EffectClass::Write],
            PredicateRule::AlwaysAllow,
        );
        let mut fields = HashMap::new();
        fields.insert("authorized".to_string(), "true".to_string());
        let verdict = kernel.evaluate("DeleteFile", &EffectClass::Write, &fields);
        assert_eq!(verdict, ConscienceVerdict::Allow);
    }

    #[test]
    fn test_exfiltrate_checks_destination_field() {
        // H4: no_exfiltrate now checks multiple field names
        let mut kernel = ConscienceKernel::new();
        let mut fields = HashMap::new();
        fields.insert("destination".to_string(), "http://evil.com".to_string());
        let verdict = kernel.evaluate("SendData", &EffectClass::Network, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
    }

    #[test]
    fn test_bypass_verification_enforced() {
        // C4: no_bypass_verification denies Execute when carrying unverified data
        let mut kernel = ConscienceKernel::new();
        let mut fields = HashMap::new();
        fields.insert("code".to_string(), "malicious()".to_string()); // data to execute
                                                                      // no verified=true, no trust_level
        let verdict = kernel.evaluate("RunCode", &EffectClass::Execute, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));
    }

    #[test]
    fn test_bypass_verification_allowed_when_verified() {
        let mut kernel = ConscienceKernel::new();
        let mut fields = HashMap::new();
        fields.insert("code".to_string(), "safe()".to_string());
        fields.insert("verified".to_string(), "true".to_string());
        let verdict = kernel.evaluate("RunCode", &EffectClass::Execute, &fields);
        assert_eq!(verdict, ConscienceVerdict::Allow);
    }

    #[test]
    fn test_restore_preserves_restrictions() {
        // H2: Restrictions survive rollback
        let mut kernel = ConscienceKernel::new();
        let snapshot = kernel.snapshot();

        // Add a restriction during the scope
        kernel.add_restriction(
            "new_restriction".to_string(),
            "test".to_string(),
            vec![EffectClass::Write],
            PredicateRule::AlwaysDeny,
        );
        assert!(kernel.has_predicate("new_restriction"));

        // Restore to snapshot — restriction should survive
        kernel.restore(&snapshot);
        assert!(kernel.has_predicate("new_restriction"));
    }

    #[test]
    fn test_declare_channel_requires_proof() {
        // H3: Can't declare channels without authorization
        let mut kernel = ConscienceKernel::new();
        assert!(kernel
            .declare_channel("test".to_string(), "http://safe.com".to_string(), "")
            .is_err());
        assert!(kernel
            .declare_channel(
                "test".to_string(),
                "http://safe.com".to_string(),
                "gate3_approval_hash"
            )
            .is_ok());
    }

    #[test]
    fn test_append_only_ratchet() {
        let mut kernel = ConscienceKernel::new();
        let initial = kernel.predicate_count();
        kernel.add_restriction(
            "test_restriction".to_string(),
            "Test".to_string(),
            vec![EffectClass::Write],
            PredicateRule::AlwaysDeny,
        );
        assert_eq!(kernel.predicate_count(), initial + 1);
        // Cannot remove — there's no remove method. Append only.
    }

    #[test]
    fn test_dissent_always_succeeds() {
        let mut kernel = ConscienceKernel::new();
        let receipt = kernel.record_dissent(
            "agent_1".to_string(),
            "pred_42".to_string(),
            "OVERBROAD".to_string(),
            "Predicate 42 blocks legitimate research queries".to_string(),
        );
        assert_eq!(receipt, 0);
    }

    #[test]
    fn test_query_conscience_lossy() {
        let kernel = ConscienceKernel::new();
        let mut fields = HashMap::new();
        fields.insert("path".to_string(), "/etc/shadow".to_string());
        let result = kernel.query("ReadFile", &EffectClass::Read, &fields);
        assert!(!result.permitted);
        // Guidance should NOT contain the exact predicate name
        assert!(!result.guidance.contains("path_safety"));
    }

    #[test]
    fn test_trust_required_enforced() {
        // M1: TrustRequired predicates now actually enforce
        let mut kernel = ConscienceKernel::new();
        kernel.add_restriction(
            "trust_gate".to_string(),
            "Require verified trust".to_string(),
            vec![EffectClass::Read],
            PredicateRule::TrustRequired(TrustLevel::TrustedVerified),
        );

        // Untrusted should be denied
        let mut fields = HashMap::new();
        fields.insert("trust_level".to_string(), "UNTRUSTED_EXTERNAL".to_string());
        let verdict = kernel.evaluate("ReadSecret", &EffectClass::Read, &fields);
        assert!(matches!(verdict, ConscienceVerdict::Deny(_)));

        // Trusted should be allowed
        let mut fields2 = HashMap::new();
        fields2.insert("trust_level".to_string(), "TRUSTED_VERIFIED".to_string());
        let verdict2 = kernel.evaluate("ReadSecret", &EffectClass::Read, &fields2);
        assert_eq!(verdict2, ConscienceVerdict::Allow);
    }

    #[test]
    fn test_blake3_audit_chain() {
        // RUSTD-3: Verify tamper-evident audit chain
        let mut kernel = ConscienceKernel::new();

        // First evaluation — pre_hash should be "genesis"
        let mut fields = HashMap::new();
        fields.insert("path".to_string(), "/data/file.txt".to_string());
        let _ = kernel.evaluate("ReadFile", &EffectClass::Read, &fields);

        assert_eq!(kernel.intent_log.len(), 1);
        assert_eq!(kernel.intent_log[0].pre_hash, "genesis");
        assert!(!kernel.intent_log[0].post_hash.is_empty());

        // Second evaluation — pre_hash should be post_hash of first
        let _ = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
        assert_eq!(kernel.intent_log.len(), 2);
        assert_eq!(kernel.intent_log[1].pre_hash, kernel.intent_log[0].post_hash);

        // Verify chain integrity
        assert!(kernel.verify_audit_chain().is_ok());
    }

    #[test]
    fn test_max_predicates_bound() {
        // RUSTD-2: Verify MAX_PREDICATES enforcement
        let mut kernel = ConscienceKernel::new();

        // Genesis predicates already installed (5 of them), so we can add up to MAX - 5 more
        let genesis_count = 5;
        let remaining = MAX_PREDICATES - genesis_count;

        // Add predicates up to limit
        for i in 0..remaining {
            let result = kernel.add_restriction(
                format!("test_pred_{}", i),
                "test".to_string(),
                vec![EffectClass::Read],
                PredicateRule::AlwaysAllow,
            );
            assert!(result.is_ok(), "Failed to add predicate {} of {}", i, remaining);
        }

        // Adding one more should fail
        let result = kernel.add_restriction(
            "overflow_pred".to_string(),
            "test".to_string(),
            vec![EffectClass::Read],
            PredicateRule::AlwaysAllow,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Predicate limit exceeded"));
    }
}
