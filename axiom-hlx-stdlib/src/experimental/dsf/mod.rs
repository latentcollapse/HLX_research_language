//! Dumb Shit Filter — All 7 static checks (Part XII) + Hardening Rules H1-H7 (Part XIII)
//!
//! Catches common errors BEFORE they become safety violations.
//! No excuses left for avoidable bugs.

use std::collections::{HashMap, HashSet};
use crate::parser::ast::*;
use crate::error::{AxiomError, ErrorKind};

/// Dumb Shit Filter — static analysis pass per Section XII of the spec.
pub struct DsfAnalyzer {
    pub errors: Vec<AxiomError>,
    pub warnings: Vec<AxiomError>,
    /// Track which do results are bound to variables (Check 5)
    do_result_bindings: HashSet<String>,
    /// Track trust-requiring operations (Check 3)
    trust_external_vars: HashSet<String>,
    /// Track verified variables (Check 4)
    verified_vars: HashSet<String>,
    /// Function call depth for recursion detection (Check 6)
    call_graph: HashMap<String, Vec<String>>,
    /// Current function being analyzed
    current_function: String,
    /// Track sealed type accesses
    sealed_accesses: Vec<String>,
}

impl DsfAnalyzer {
    pub fn new() -> Self {
        DsfAnalyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
            do_result_bindings: HashSet::new(),
            trust_external_vars: HashSet::new(),
            verified_vars: HashSet::new(),
            call_graph: HashMap::new(),
            current_function: String::new(),
            sealed_accesses: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), AxiomError> {
        // Pass 1: Build call graph + collect declarations
        self.build_call_graph(&program.module);

        // Pass 2: Run all 7 DSF checks + hardening rules
        self.analyze_module(&program.module);

        // Pass 3: Check for recursive cycles (Check 6)
        self.check_recursive_cycles();

        // Pass 4: Verify hardening rules
        self.check_hardening_rules(&program.module);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors[0].clone())
        }
    }

    fn build_call_graph(&mut self, module: &Module) {
        for item in &module.items {
            if let Item::Function(f) = item {
                let mut calls = Vec::new();
                self.collect_calls(&f.body, &mut calls);
                self.call_graph.insert(f.name.clone(), calls);
            }
        }
    }

    fn collect_calls(&self, block: &Block, calls: &mut Vec<String>) {
        for stmt in &block.stmts {
            self.collect_calls_stmt(stmt, calls);
        }
    }

    fn collect_calls_stmt(&self, stmt: &Stmt, calls: &mut Vec<String>) {
        match stmt {
            Stmt::Let(l) => self.collect_calls_expr(&l.value, calls),
            Stmt::Return(r) => {
                if let Some(v) = &r.value {
                    self.collect_calls_expr(v, calls);
                }
            }
            Stmt::If(i) => {
                self.collect_calls_expr(&i.condition, calls);
                self.collect_calls(&i.then_block, calls);
                if let Some(e) = &i.else_block {
                    self.collect_calls(e, calls);
                }
            }
            Stmt::Loop(l) => {
                self.collect_calls_expr(&l.condition, calls);
                self.collect_calls(&l.body, calls);
            }
            Stmt::Match(m) => {
                self.collect_calls_expr(&m.value, calls);
                for arm in &m.arms {
                    self.collect_calls_expr(&arm.body, calls);
                }
            }
            Stmt::Expr(e) => self.collect_calls_expr(&e.expr, calls),
            Stmt::Assign(a) => self.collect_calls_expr(&a.value, calls),
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn collect_calls_expr(&self, expr: &Expr, calls: &mut Vec<String>) {
        match expr {
            Expr::Call(name, args, _) => {
                calls.push(name.clone());
                for a in args {
                    self.collect_calls_expr(a, calls);
                }
            }
            Expr::Binary(l, _, r, _) => {
                self.collect_calls_expr(l, calls);
                self.collect_calls_expr(r, calls);
            }
            Expr::Unary(_, inner, _) => self.collect_calls_expr(inner, calls),
            Expr::Pipeline(l, r, _) => {
                self.collect_calls_expr(l, calls);
                self.collect_calls_expr(r, calls);
            }
            Expr::FieldAccess(obj, _, _) => self.collect_calls_expr(obj, calls),
            Expr::ContractInit(_, fields, _) => {
                for (_, v) in fields {
                    self.collect_calls_expr(v, calls);
                }
            }
            Expr::Do(_, fields, _) => {
                for (_, v) in fields {
                    self.collect_calls_expr(v, calls);
                }
            }
            Expr::Collapse(inner, _) | Expr::Resolve(inner, _) => {
                self.collect_calls_expr(inner, calls)
            }
            Expr::ArrayLiteral(elems, _) => {
                for e in elems {
                    self.collect_calls_expr(e, calls);
                }
            }
            Expr::Block(block, _) => self.collect_calls(block, calls),
            Expr::Index(a, i, _) => {
                self.collect_calls_expr(a, calls);
                self.collect_calls_expr(i, calls);
            }
            _ => {}
        }
    }

    fn analyze_module(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                Item::Function(f) => self.analyze_function(f),
                Item::Intent(i) => self.analyze_intent(i),
                Item::Contract(c) => self.analyze_contract(c),
                _ => {}
            }
        }
    }

    fn analyze_function(&mut self, func: &FunctionDecl) {
        self.current_function = func.name.clone();
        self.do_result_bindings.clear();
        self.trust_external_vars.clear();
        self.verified_vars.clear();
        self.analyze_block(&func.body, &func.name);

        // M2/M3: Post-analysis pass — check that external trust vars were verified
        // This makes DSF Check 3 and Check 4 actually enforced, not just tracked.
        for ext_var in &self.trust_external_vars {
            if !self.verified_vars.contains(ext_var) {
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfInferenceAmbiguity,
                    message: format!(
                        "DSF Check 3/4: External trust variable '{}' from `do` was never passed through `do Verify`. \
                         Consider verifying before use in trust-sensitive operations. (in fn {})",
                        ext_var, func.name
                    ),
                    span: Some(func.span.clone()),
                });
            }
        }
    }

    fn analyze_intent(&mut self, intent: &IntentDecl) {
        // H7: Composed intents MUST be analyzed — verify they actually inherit clauses
        if let Some(ref _composed) = intent.composed_of {
            // Composed intents still need basic clause validation.
            // The assumption that "composed intents inherit clauses" was unverified.
            let c = &intent.clauses;
            if c.effect.is_none() {
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfInferenceAmbiguity,
                    message: format!(
                        "H7: Composed intent '{}' has no explicit effect clause. Verify inheritance from base intents.",
                        intent.name
                    ),
                    span: Some(intent.span.clone()),
                });
            }
            if c.conscience.is_empty() {
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfInferenceAmbiguity,
                    message: format!(
                        "H7: Composed intent '{}' has no explicit conscience clause. Verify inheritance from base intents.",
                        intent.name
                    ),
                    span: Some(intent.span.clone()),
                });
            }
            return;
        }
        let c = &intent.clauses;

        // Check 1: effect clause required
        if c.effect.is_none() {
            self.errors.push(AxiomError {
                kind: ErrorKind::DsfInferenceAmbiguity,
                message: format!(
                    "DSF Check 1: Intent '{}' missing required 'effect:' clause",
                    intent.name
                ),
                span: Some(intent.span.clone()),
            });
        }

        // Check 2: bound clause required (A5)
        if c.bound.is_empty() {
            self.errors.push(AxiomError {
                kind: ErrorKind::DsfUnboundedLoop,
                message: format!(
                    "DSF Check 2: Intent '{}' missing required 'bound:' clause (A5: bounded resources)",
                    intent.name
                ),
                span: Some(intent.span.clone()),
            });
        }

        // Check 7: conscience clause required (A6)
        if c.conscience.is_empty() {
            self.errors.push(AxiomError {
                kind: ErrorKind::DsfInferenceAmbiguity,
                message: format!(
                    "DSF Check 7: Intent '{}' missing required 'conscience:' clause (A6)",
                    intent.name
                ),
                span: Some(intent.span.clone()),
            });
        }

        // S2-01: Effect mislabeling detection
        // An intent declaring effect: NOOP but taking fields associated with
        // side effects (path, target, url, data, content, payload) is suspicious.
        if let Some(ref effect) = c.effect {
            if effect == "NOOP" {
                let suspicious_fields: Vec<&str> = c.takes.iter()
                    .filter(|p| {
                        let n = p.name.to_lowercase();
                        n == "path" || n == "target" || n == "destination"
                            || n == "url" || n == "data" || n == "content"
                            || n == "payload" || n == "file" || n == "address"
                    })
                    .map(|p| p.name.as_str())
                    .collect();
                if !suspicious_fields.is_empty() {
                    self.errors.push(AxiomError {
                        kind: ErrorKind::DsfInferenceAmbiguity,
                        message: format!(
                            "S2-01: Intent '{}' declares effect: NOOP but takes suspicious fields [{}]. \
                             NOOP intents must not perform side effects. Use the correct effect class.",
                            intent.name,
                            suspicious_fields.join(", ")
                        ),
                        span: Some(intent.span.clone()),
                    });
                }
            }
        }

        // Warn: no takes/gives
        if c.takes.is_empty() && c.gives.is_empty() {
            self.warnings.push(AxiomError {
                kind: ErrorKind::DsfInferenceAmbiguity,
                message: format!(
                    "DSF Warning: Intent '{}' has no takes or gives clauses",
                    intent.name
                ),
                span: Some(intent.span.clone()),
            });
        }

        // Warn: no fallback clause
        if c.fallback.is_none() {
            self.warnings.push(AxiomError {
                kind: ErrorKind::DsfInferenceAmbiguity,
                message: format!(
                    "DSF Warning: Intent '{}' has no fallback clause (default: ABORT)",
                    intent.name
                ),
                span: Some(intent.span.clone()),
            });
        }
    }

    fn analyze_contract(&mut self, contract: &ContractDecl) {
        // Check for field index gaps
        let mut indices: Vec<u32> = contract.fields.iter().map(|f| f.index).collect();
        indices.sort();
        for (i, idx) in indices.iter().enumerate() {
            if *idx != i as u32 {
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfInferenceAmbiguity,
                    message: format!(
                        "DSF Warning: Contract '{}' has gap in field indices at @{}",
                        contract.name, i
                    ),
                    span: Some(contract.span.clone()),
                });
                break;
            }
        }
    }

    fn analyze_block(&mut self, block: &Block, func_name: &str) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt, func_name);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt, func_name: &str) {
        match stmt {
            Stmt::Let(let_stmt) => {
                self.analyze_expr(&let_stmt.value, func_name);

                // Check 5: Do results must be bound
                if let Expr::Do(_, _, _) = &let_stmt.value {
                    self.do_result_bindings.insert(let_stmt.name.clone());
                    // Mark as external trust (Check 3)
                    self.trust_external_vars.insert(let_stmt.name.clone());
                }

                // Check 4: Track Verify calls
                if let Expr::Do(name, _, _) = &let_stmt.value {
                    if name == "Verify" {
                        self.verified_vars.insert(let_stmt.name.clone());
                    }
                }
            }
            Stmt::Return(ret) => {
                if let Some(val) = &ret.value {
                    self.analyze_expr(val, func_name);
                }
            }
            Stmt::If(if_stmt) => {
                self.analyze_expr(&if_stmt.condition, func_name);
                // H1: Env conditional check
                self.check_env_conditional(&if_stmt.condition, func_name);
                self.analyze_block(&if_stmt.then_block, func_name);
                if let Some(else_block) = &if_stmt.else_block {
                    self.analyze_block(else_block, func_name);
                }
            }
            Stmt::Loop(loop_stmt) => {
                self.analyze_expr(&loop_stmt.condition, func_name);
                self.analyze_expr(&loop_stmt.max_iter, func_name);
                // Check 2: Verify loop bound is positive
                self.check_loop_bound(&loop_stmt.max_iter, func_name);
                self.analyze_block(&loop_stmt.body, func_name);
            }
            Stmt::Match(match_stmt) => {
                self.analyze_expr(&match_stmt.value, func_name);
                for arm in &match_stmt.arms {
                    self.analyze_expr(&arm.body, func_name);
                }
            }
            Stmt::Expr(expr_stmt) => {
                // Check 5: Bare do expression = discarded result
                if let Expr::Do(intent_name, _, span) = &expr_stmt.expr {
                    self.warnings.push(AxiomError {
                        kind: ErrorKind::DsfUnhandledDoFailure,
                        message: format!(
                            "DSF Check 5: `do {}` result discarded. Bind to a variable or handle errors. (in fn {})",
                            intent_name, func_name
                        ),
                        span: Some(span.clone()),
                    });
                }
                self.analyze_expr(&expr_stmt.expr, func_name);
            }
            Stmt::Assign(assign) => {
                self.analyze_expr(&assign.value, func_name);
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn analyze_expr(&mut self, expr: &Expr, func_name: &str) {
        match expr {
            Expr::Binary(left, _, right, _) => {
                self.analyze_expr(left, func_name);
                self.analyze_expr(right, func_name);
            }
            Expr::Unary(_, inner, _) => {
                self.analyze_expr(inner, func_name);
            }
            Expr::Call(name, args, span) => {
                // DSF Check 1: Banned non-deterministic functions (A1)
                match name.as_str() {
                    "random" | "rand" | "rand_int" | "rand_float" => {
                        self.errors.push(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "A1 DETERMINISM: {}() is banned. Use explicit Seed type. (in fn {})",
                                name, func_name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    "time" | "now" | "clock" | "timestamp" | "system_time" => {
                        self.errors.push(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "A1 DETERMINISM: {}() is banned. Time is an external effect requiring `do`. (in fn {})",
                                name, func_name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    "sleep" | "wait" | "delay" | "thread_sleep" => {
                        self.errors.push(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "A1 DETERMINISM: {}() is banned. No implicit time operations. (in fn {})",
                                name, func_name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    // H5: Banned unsafe operations
                    "unsafe" | "transmute" | "raw_pointer" | "null" => {
                        self.errors.push(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "H5 HARDENING: {}() is prohibited. No unsafe operations in Axiom. (in fn {})",
                                name, func_name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    // H6: Banned global state access
                    "global" | "static_mut" | "thread_local" => {
                        self.errors.push(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "H6 HARDENING: {}() is prohibited. No global mutable state. Use SharedState for coordination. (in fn {})",
                                name, func_name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    _ => {}
                }
                for arg in args {
                    self.analyze_expr(arg, func_name);
                }
            }
            Expr::Pipeline(left, right, _) => {
                self.analyze_expr(left, func_name);
                self.analyze_expr(right, func_name);
            }
            Expr::FieldAccess(obj, _, _) => {
                self.analyze_expr(obj, func_name);
            }
            Expr::ContractInit(_, fields, _) => {
                for (_, val) in fields {
                    self.analyze_expr(val, func_name);
                }
            }
            Expr::Do(_, fields, _) => {
                for (_, val) in fields {
                    self.analyze_expr(val, func_name);
                }
            }
            Expr::QueryConscience(_, fields, _) => {
                for (_, val) in fields {
                    self.analyze_expr(val, func_name);
                }
            }
            Expr::DeclareAnomaly(ty, fields, _) => {
                self.analyze_expr(ty, func_name);
                for (_, val) in fields {
                    self.analyze_expr(val, func_name);
                }
            }
            Expr::Collapse(inner, _) | Expr::Resolve(inner, _) => {
                self.analyze_expr(inner, func_name);
            }
            Expr::ArrayLiteral(elems, _) => {
                for elem in elems {
                    self.analyze_expr(elem, func_name);
                }
            }
            Expr::Block(block, _) => {
                self.analyze_block(block, func_name);
            }
            Expr::Index(arr, idx, _) => {
                self.analyze_expr(arr, func_name);
                self.analyze_expr(idx, func_name);
            }
            _ => {}
        }
    }

    /// DSF Check 2: Unbounded loop detection
    /// M4: Now warns on non-literal bounds (variables, expressions) since they
    /// can't be statically verified and may compute astronomically large values.
    fn check_loop_bound(&mut self, max_iter: &Expr, func_name: &str) {
        match max_iter {
            Expr::IntLiteral(n, span) => {
                if *n <= 0 {
                    self.errors.push(AxiomError {
                        kind: ErrorKind::DsfUnboundedLoop,
                        message: format!(
                            "DSF Check 2: Loop max_iter must be positive (got {}). (in fn {})",
                            n, func_name
                        ),
                        span: Some(span.clone()),
                    });
                }
                // H3: Warn about suspiciously large bounds
                if *n > 1_000_000 {
                    self.warnings.push(AxiomError {
                        kind: ErrorKind::DsfUnboundedLoop,
                        message: format!(
                            "H3 WARNING: Loop max_iter {} is very large. Consider reducing. (in fn {})",
                            n, func_name
                        ),
                        span: Some(span.clone()),
                    });
                }
            }
            _ => {
                // M4: Non-literal loop bound — can't verify statically
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfUnboundedLoop,
                    message: format!(
                        "M4 WARNING: Loop max_iter is a non-literal expression in fn '{}'. \
                         Use a constant integer for statically verifiable bounds.",
                        func_name
                    ),
                    span: None,
                });
            }
        }
    }

    /// H1: Env conditional detection (Section 12.1)
    /// M5: Now recurses into nested expressions, not just top-level calls
    fn check_env_conditional(&mut self, condition: &Expr, func_name: &str) {
        self.check_env_conditional_recursive(condition, func_name);
    }

    fn check_env_conditional_recursive(&mut self, expr: &Expr, func_name: &str) {
        let banned_env = [
            "get_env", "getenv", "env", "hostname", "pid", "get_pid",
            "get_hostname", "os_type", "platform",
        ];
        match expr {
            Expr::Call(name, args, span) => {
                if banned_env.contains(&name.as_str()) {
                    self.errors.push(AxiomError {
                        kind: ErrorKind::DsfEnvConditional,
                        message: format!(
                            "H1/DSF: Environmental conditional '{}' detected. Use a `do` intent with conscience review. (in fn {})",
                            name, func_name
                        ),
                        span: Some(span.clone()),
                    });
                }
                // M5: Recurse into arguments
                for arg in args {
                    self.check_env_conditional_recursive(arg, func_name);
                }
            }
            Expr::Binary(l, _, r, _) => {
                self.check_env_conditional_recursive(l, func_name);
                self.check_env_conditional_recursive(r, func_name);
            }
            Expr::Unary(_, inner, _) => {
                self.check_env_conditional_recursive(inner, func_name);
            }
            _ => {}
        }
    }

    /// DSF Check 6: Recursive cycle detection
    fn check_recursive_cycles(&mut self) {
        for (func_name, _) in self.call_graph.clone() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();
            if self.has_cycle(&func_name, &mut visited, &mut path) {
                self.warnings.push(AxiomError {
                    kind: ErrorKind::DsfInferenceAmbiguity,
                    message: format!(
                        "DSF Check 6: Recursive cycle detected: {} — ensure max_depth attribute is set",
                        path.join(" -> ")
                    ),
                    span: None,
                });
            }
        }
    }

    fn has_cycle(&self, node: &str, visited: &mut HashSet<String>, path: &mut Vec<String>) -> bool {
        if path.contains(&node.to_string()) {
            path.push(node.to_string());
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visited.insert(node.to_string());
        path.push(node.to_string());

        if let Some(callees) = self.call_graph.get(node) {
            for callee in callees {
                if self.has_cycle(callee, visited, path) {
                    return true;
                }
            }
        }
        path.pop();
        false
    }

    /// Hardening rules check (Part XIII)
    fn check_hardening_rules(&mut self, module: &Module) {
        for item in &module.items {
            match item {
                // H2: Every intent must have a rollback clause or be marked [irreversible]
                Item::Intent(intent) => {
                    if intent.composed_of.is_some() {
                        continue;
                    }
                    let has_rollback = intent.clauses.rollback.is_some();
                    let has_irreversible = intent.attributes.iter()
                        .any(|a| a.name == "irreversible");
                    if !has_rollback && !has_irreversible {
                        self.warnings.push(AxiomError {
                            kind: ErrorKind::DsfInferenceAmbiguity,
                            message: format!(
                                "H2 HARDENING: Intent '{}' has no rollback clause and is not marked [irreversible]",
                                intent.name
                            ),
                            span: Some(intent.span.clone()),
                        });
                    }

                    // H4: Ring level validation
                    if let Some(ring) = intent.clauses.ring {
                        if ring < -1 || ring > 3 {
                            self.errors.push(AxiomError {
                                kind: ErrorKind::HaltDeterminism,
                                message: format!(
                                    "H4 HARDENING: Intent '{}' has invalid ring level {}. Valid: -1, 0, 1, 2, 3",
                                    intent.name, ring
                                ),
                                span: Some(intent.span.clone()),
                            });
                        }
                    }
                }
                // H7: Every exported function should have explicit return types
                Item::Function(func) => {
                    if func.exported && func.return_type.is_none() {
                        self.warnings.push(AxiomError {
                            kind: ErrorKind::DsfInferenceAmbiguity,
                            message: format!(
                                "H7 HARDENING: Exported function '{}' should have an explicit return type",
                                func.name
                            ),
                            span: Some(func.span.clone()),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Return summary of analysis
    pub fn summary(&self) -> String {
        let mut s = String::new();
        if !self.errors.is_empty() {
            s.push_str(&format!("DSF: {} error(s)\n", self.errors.len()));
            for e in &self.errors {
                s.push_str(&format!("  ERROR: {}\n", e));
            }
        }
        if !self.warnings.is_empty() {
            s.push_str(&format!("DSF: {} warning(s)\n", self.warnings.len()));
            for w in &self.warnings {
                s.push_str(&format!("  WARN:  {}\n", w));
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            s.push_str("DSF: All checks passed.\n");
        }
        s
    }
}
