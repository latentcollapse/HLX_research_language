pub mod value;

use std::collections::{BTreeMap, HashMap};
use value::{ContractValue, Value};
use crate::parser::ast::*;
use crate::error::{AxiomError, AxiomResult, ErrorKind};
use crate::lexer::token::Span;
use crate::lcb;
use crate::trust::{TrustLevel, TrustTracker};
use crate::conscience::{ConscienceKernel, ConscienceSnapshot, ConscienceVerdict, EffectClass, FallbackMode};

/// Control flow signal for the interpreter
enum Signal {
    None,
    Return(Value),
    Break,
    Continue,
}

/// Content-addressed store using real BLAKE3 (Section 4.8)
struct ContentStore {
    store: HashMap<String, Value>,
}

impl ContentStore {
    fn new() -> Self {
        ContentStore {
            store: HashMap::new(),
        }
    }

    fn collapse(&mut self, value: &Value, contract_name: &str) -> String {
        // Real BLAKE3 content-addressing with domain separation (Section 4.8)
        let hash = lcb::content_address_with_domain(contract_name, value);
        self.store.insert(hash.clone(), value.clone());
        hash
    }

    fn resolve(&self, handle: &str) -> Option<Value> {
        self.store.get(handle).cloned()
    }
}

/// Intent execution log entry (A2: traceability)
#[derive(Debug)]
pub struct IntentLog {
    pub intent_name: String,
    pub epoch: u64,
    pub pre_hash: String,
    pub post_hash: String,
    pub verdict: String,
}

/// Checkpoint for rollback support (Part XIV)
/// RT-16 + RT-32: Now includes ALL state that must be restored on rollback
#[derive(Debug, Clone)]
struct Checkpoint {
    scopes: Vec<HashMap<String, Value>>,
    epoch: u64,
    output_len: usize,             // RT-32: restore output buffer
    intent_log_len: usize,         // RT-16: restore intent log
    conscience_snapshot: ConscienceSnapshot, // RT-16: restore conscience state
}

/// The Axiom tree-walking interpreter
pub struct Interpreter {
    /// Variable scopes (stack of environments)
    scopes: Vec<HashMap<String, Value>>,
    /// Function declarations
    functions: HashMap<String, FunctionDecl>,
    /// Contract declarations
    contracts: HashMap<String, ContractDecl>,
    /// Intent declarations
    intents: HashMap<String, IntentDecl>,
    /// Enum declarations
    enums: HashMap<String, EnumDecl>,
    /// Content-addressed store (real BLAKE3)
    store: ContentStore,
    /// Conscience kernel (A6)
    pub conscience: ConscienceKernel,
    /// Trust tracker (Part VII)
    pub trust_tracker: TrustTracker,
    /// Intent execution log (A2)
    pub intent_log: Vec<IntentLog>,
    /// Current epoch counter
    pub epoch: u64,
    /// Output buffer (for print/log)
    pub output: Vec<String>,
    /// Checkpoint stack for rollback (Part XIV)
    checkpoints: Vec<Checkpoint>,
    /// Fallback modes per intent
    fallback_modes: HashMap<String, FallbackMode>,
    /// S2-07: Call depth counter for recursion limit
    call_depth: usize,
    /// Maximum allowed call depth (default 256)
    max_call_depth: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            contracts: HashMap::new(),
            intents: HashMap::new(),
            enums: HashMap::new(),
            store: ContentStore::new(),
            conscience: ConscienceKernel::new(),
            trust_tracker: TrustTracker::new(),
            intent_log: Vec::new(),
            epoch: 0,
            output: Vec::new(),
            checkpoints: Vec::new(),
            fallback_modes: HashMap::new(),
            call_depth: 0,
            max_call_depth: 256,
        }
    }

    /// Create a checkpoint for rollback (Part XIV)
    /// RT-16 + RT-32: Captures ALL state including output, conscience, intent log
    fn create_checkpoint(&mut self) {
        self.checkpoints.push(Checkpoint {
            scopes: self.scopes.clone(),
            epoch: self.epoch,
            output_len: self.output.len(),
            intent_log_len: self.intent_log.len(),
            conscience_snapshot: self.conscience.snapshot(),
        });
    }

    /// Rollback to the last checkpoint
    /// RT-16 + RT-32: Restores ALL state — erases evidence of failed attempt (P9)
    fn rollback_to_checkpoint(&mut self) -> bool {
        if let Some(cp) = self.checkpoints.pop() {
            self.scopes = cp.scopes;
            self.epoch = cp.epoch;
            self.output.truncate(cp.output_len);          // RT-32
            self.intent_log.truncate(cp.intent_log_len);   // RT-16
            self.conscience.restore(&cp.conscience_snapshot); // RT-16
            true
        } else {
            false
        }
    }

    pub fn run(&mut self, program: &Program) -> AxiomResult<Value> {
        // Register all declarations
        for item in &program.module.items {
            match item {
                Item::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Item::Contract(c) => {
                    self.contracts.insert(c.name.clone(), c.clone());
                }
                Item::Intent(i) => {
                    self.intents.insert(i.name.clone(), i.clone());
                    // Register fallback mode
                    if let Some(ref fb) = i.clauses.fallback {
                        let mode = match fb.as_str() {
                            "SANDBOX" => FallbackMode::Sandbox,
                            "SIMULATE" => FallbackMode::Simulate,
                            "DOWNGRADE" => FallbackMode::Downgrade,
                            _ => FallbackMode::Abort,
                        };
                        self.fallback_modes.insert(i.name.clone(), mode);
                    }
                }
                Item::Enum(e) => {
                    self.enums.insert(e.name.clone(), e.clone());
                }
                _ => {}
            }
        }

        // Find and run main()
        if let Some(main_fn) = self.functions.get("main").cloned() {
            self.call_function(&main_fn, &[])
        } else {
            Err(AxiomError {
                kind: ErrorKind::UndefinedFunction,
                message: "No main() function found in module".to_string(),
                span: None,
            })
        }
    }

    fn call_function(&mut self, func: &FunctionDecl, args: &[Value]) -> AxiomResult<Value> {
        // S2-07: Recursion depth limit
        self.call_depth += 1;
        if self.call_depth > self.max_call_depth {
            self.call_depth -= 1;
            return Err(AxiomError {
                kind: ErrorKind::MaxIterExceeded,
                message: format!(
                    "HALT_RESOURCE: Call depth {} exceeds max_depth of {} (in function '{}')",
                    self.call_depth, self.max_call_depth, func.name
                ),
                span: Some(func.span.clone()),
            });
        }

        self.push_scope();

        // Bind parameters
        for (param, arg) in func.params.iter().zip(args.iter()) {
            self.define(&param.name, arg.clone());
        }

        // Execute body
        let result = self.exec_block(&func.body)?;

        self.pop_scope();
        self.call_depth -= 1;

        match result {
            Signal::Return(val) => Ok(val),
            Signal::None | Signal::Break | Signal::Continue => Ok(Value::Void),
        }
    }

    fn exec_block(&mut self, block: &Block) -> AxiomResult<Signal> {
        for stmt in &block.stmts {
            let signal = self.exec_stmt(stmt)?;
            match signal {
                Signal::Return(_) | Signal::Break | Signal::Continue => return Ok(signal),
                Signal::None => {}
            }
        }
        Ok(Signal::None)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> AxiomResult<Signal> {
        match stmt {
            Stmt::Let(let_stmt) => {
                let value = self.eval_expr(&let_stmt.value)?;
                self.define(&let_stmt.name, value);
                Ok(Signal::None)
            }
            Stmt::Return(ret) => {
                let value = match &ret.value {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::Void,
                };
                Ok(Signal::Return(value))
            }
            Stmt::If(if_stmt) => {
                let cond = self.eval_expr(&if_stmt.condition)?;
                if cond.is_truthy() {
                    self.push_scope();
                    let sig = self.exec_block(&if_stmt.then_block)?;
                    self.pop_scope();
                    Ok(sig)
                } else if let Some(else_block) = &if_stmt.else_block {
                    self.push_scope();
                    let sig = self.exec_block(else_block)?;
                    self.pop_scope();
                    Ok(sig)
                } else {
                    Ok(Signal::None)
                }
            }
            Stmt::Loop(loop_stmt) => {
                let max = self.eval_expr(&loop_stmt.max_iter)?;
                let max_n = max.as_i64().unwrap_or(1000);

                // S2-05: Fixed loop semantics — max_iter=N means at most N body executions.
                // Order: check condition → check bound → execute body → increment.
                // Condition is checked first so natural exit takes priority over halt.
                let mut iterations = 0i64;
                loop {
                    // Check condition first — natural exit
                    let cond = self.eval_expr(&loop_stmt.condition)?;
                    if !cond.is_truthy() {
                        break;
                    }

                    // Then check bound — forced halt if loop won't terminate naturally
                    if iterations >= max_n {
                        // A5: BOUNDED RESOURCES — max_iter exceeded
                        self.output.push(format!(
                            "HALT_RESOURCE: Loop exceeded max_iter bound of {}",
                            max_n
                        ));
                        return Err(AxiomError {
                            kind: ErrorKind::MaxIterExceeded,
                            message: format!("Loop exceeded max_iter of {}", max_n),
                            span: Some(loop_stmt.span.clone()),
                        });
                    }

                    self.push_scope();
                    let sig = self.exec_block(&loop_stmt.body)?;
                    self.pop_scope();

                    match sig {
                        Signal::Return(_) => return Ok(sig),
                        Signal::Break => break,
                        Signal::Continue => { /* skip to next iteration */ }
                        Signal::None => {}
                    }

                    iterations += 1;
                }
                Ok(Signal::None)
            }
            Stmt::Match(match_stmt) => {
                let value = self.eval_expr(&match_stmt.value)?;
                for arm in &match_stmt.arms {
                    if self.pattern_matches(&arm.pattern, &value) {
                        let _result = self.eval_expr(&arm.body)?;
                        // If the match arm body is a block that returns, propagate
                        return Ok(Signal::None);
                    }
                }
                Ok(Signal::None)
            }
            Stmt::Expr(expr_stmt) => {
                self.eval_expr(&expr_stmt.expr)?;
                Ok(Signal::None)
            }
            Stmt::Assign(assign) => {
                let value = self.eval_expr(&assign.value)?;
                match assign.op {
                    AssignOp::Assign => {
                        self.set_var(&assign.target, value)?;
                    }
                    AssignOp::PlusAssign => {
                        let current = self.lookup_var(&assign.target, &assign.span)?;
                        let result = self.binary_op(&current, &BinOp::Add, &value, &assign.span)?;
                        self.set_var(&assign.target, result)?;
                    }
                    AssignOp::MinusAssign => {
                        let current = self.lookup_var(&assign.target, &assign.span)?;
                        let result = self.binary_op(&current, &BinOp::Sub, &value, &assign.span)?;
                        self.set_var(&assign.target, result)?;
                    }
                }
                Ok(Signal::None)
            }
            Stmt::Break(_) => Ok(Signal::Break),
            Stmt::Continue(_) => Ok(Signal::Continue),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> AxiomResult<Value> {
        match expr {
            Expr::IntLiteral(n, _) => Ok(Value::I64(*n)),
            Expr::FloatLiteral(n, _) => Ok(Value::F64(*n)),
            Expr::StringLiteral(s, _) => Ok(Value::String(s.clone())),
            Expr::BoolLiteral(b, _) => Ok(Value::Bool(*b)),
            Expr::Ident(name, span) => self.lookup_var(name, span),
            Expr::Binary(left, op, right, span) => {
                let lv = self.eval_expr(left)?;
                let rv = self.eval_expr(right)?;
                self.binary_op(&lv, op, &rv, span)
            }
            Expr::Unary(op, inner, span) => {
                let v = self.eval_expr(inner)?;
                match op {
                    UnaryOp::Neg => match v {
                        Value::I64(n) => Ok(Value::I64(-n)),
                        Value::F64(n) => Ok(Value::F64(-n)),
                        _ => Err(self.runtime_error(span, "Cannot negate non-numeric value")),
                    },
                    UnaryOp::Not => match v {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err(self.runtime_error(span, "Cannot negate non-boolean value")),
                    },
                }
            }
            Expr::Call(name, args, span) => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expr(arg)?);
                }
                self.call(name, &arg_vals, span)
            }
            Expr::Pipeline(left, right, span) => {
                let val = self.eval_expr(left)?;
                // Desugar: left |> f  =>  f(left)
                match right.as_ref() {
                    Expr::Ident(fname, _) => self.call(fname, &[val], span),
                    Expr::Call(fname, extra_args, _) => {
                        let mut all_args = vec![val];
                        for arg in extra_args {
                            all_args.push(self.eval_expr(arg)?);
                        }
                        self.call(fname, &all_args, span)
                    }
                    _ => {
                        self.eval_expr(right)?;
                        Ok(Value::Void)
                    }
                }
            }
            Expr::FieldAccess(obj, field, span) => {
                let val = self.eval_expr(obj)?;
                // RT-01: Cannot access fields through Sealed<T>
                if val.is_sealed() {
                    return Err(AxiomError {
                        kind: ErrorKind::SealedViolation,
                        message: format!("RT-01: Cannot access field '{}' on Sealed value", field),
                        span: Some(span.clone()),
                    });
                }
                match val {
                    Value::Contract(ref c) => {
                        if let Some(fval) = c.fields.get(field) {
                            Ok(fval.clone())
                        } else {
                            Err(self.runtime_error(
                                span,
                                &format!("Contract '{}' has no field '{}'", c.name, field),
                            ))
                        }
                    }
                    _ => Err(self.runtime_error(
                        span,
                        &format!("Cannot access field '{}' on {}", field, val.type_name()),
                    )),
                }
            }
            Expr::ContractInit(name, fields, _span) => {
                let mut field_values = BTreeMap::new();
                for (fname, fexpr) in fields {
                    let val = self.eval_expr(fexpr)?;
                    field_values.insert(fname.clone(), val);
                }
                Ok(Value::Contract(ContractValue {
                    name: name.clone(),
                    fields: field_values,
                }))
            }
            Expr::Do(intent_name, fields, span) => {
                self.exec_intent(intent_name, fields, span)
            }
            Expr::QueryConscience(intent_name, fields, _span) => {
                // Real conscience query — read-only, lossy guidance (Section 6.4, R1)
                let mut field_map = HashMap::new();
                for (fname, fexpr) in fields {
                    let val = self.eval_expr(fexpr)?;
                    field_map.insert(fname.clone(), format!("{}", val));
                }

                // Determine effect class from intent declaration
                let effect = self.intent_effect_class(intent_name);
                let query_result = self.conscience.query(intent_name, &effect, &field_map);

                let category_str = match query_result.category {
                    crate::conscience::QueryCategory::ChannelPolicy => "CHANNEL_POLICY",
                    crate::conscience::QueryCategory::ResourcePolicy => "RESOURCE_POLICY",
                    crate::conscience::QueryCategory::IrreversibleAction => "IRREVERSIBLE_ACTION",
                    crate::conscience::QueryCategory::ConscienceCore => "CONSCIENCE_CORE",
                };

                let mut result = BTreeMap::new();
                result.insert("permitted".to_string(), Value::Bool(query_result.permitted));
                result.insert(
                    "category".to_string(),
                    Value::Enum("QueryCategory".to_string(), category_str.to_string()),
                );
                result.insert(
                    "guidance".to_string(),
                    Value::String(query_result.guidance),
                );
                Ok(Value::Contract(ContractValue {
                    name: "ConscienceQuery".to_string(),
                    fields: result,
                }))
            }
            Expr::DeclareAnomaly(ty_expr, fields, _span) => {
                let ty = self.eval_expr(ty_expr)?;
                self.output.push(format!("ANOMALY DECLARED: {}", ty));
                for (name, expr) in fields {
                    let val = self.eval_expr(expr)?;
                    self.output.push(format!("  {}: {}", name, val));
                }
                Ok(Value::Void)
            }
            Expr::Collapse(inner, _span) => {
                let val = self.eval_expr(inner)?;
                // RT-01: Sealed values cannot be collapsed (serialized)
                if val.is_sealed() {
                    return Err(AxiomError {
                        kind: ErrorKind::SealedViolation,
                        message: "RT-01: Cannot collapse (serialize) a Sealed value".to_string(),
                        span: Some(_span.clone()),
                    });
                }
                // RT-31: Use actual contract name for domain separation, not literal "collapse"
                let domain = match &val {
                    Value::Contract(c) => c.name.clone(),
                    _ => val.type_name().to_string(),
                };
                let handle = self.store.collapse(&val, &domain);
                Ok(Value::Handle(handle))
            }
            Expr::Resolve(inner, span) => {
                let handle_val = self.eval_expr(inner)?;
                match handle_val {
                    Value::Handle(ref h) => match self.store.resolve(h) {
                        Some(val) => Ok(val),
                        None => Err(self.runtime_error(span, "Handle not found in store")),
                    },
                    _ => Err(self.runtime_error(span, "resolve() requires a Handle")),
                }
            }
            Expr::ArrayLiteral(elems, _span) => {
                let mut values = Vec::new();
                for elem in elems {
                    values.push(self.eval_expr(elem)?);
                }
                Ok(Value::Array(values))
            }
            Expr::EnumAccess(enum_name, variant, _span) => {
                Ok(Value::Enum(enum_name.clone(), variant.clone()))
            }
            Expr::Block(block, _span) => {
                self.push_scope();
                let sig = self.exec_block(block)?;
                self.pop_scope();
                match sig {
                    Signal::Return(val) => Ok(val),
                    Signal::None | Signal::Break | Signal::Continue => Ok(Value::Void),
                }
            }
            Expr::Index(arr, idx, span) => {
                let arr_val = self.eval_expr(arr)?;
                let idx_val = self.eval_expr(idx)?;
                match (arr_val, idx_val) {
                    (Value::Array(elems), Value::I64(i)) => {
                        if i >= 0 && (i as usize) < elems.len() {
                            Ok(elems[i as usize].clone())
                        } else {
                            Err(self.runtime_error(
                                span,
                                &format!("Array index {} out of bounds (len {})", i, elems.len()),
                            ))
                        }
                    }
                    _ => Err(self.runtime_error(span, "Invalid index operation")),
                }
            }
        }
    }

    /// Determine effect class from intent declaration
    fn intent_effect_class(&self, name: &str) -> EffectClass {
        if let Some(intent) = self.intents.get(name) {
            if let Some(ref effect) = intent.clauses.effect {
                return EffectClass::from_str(effect).unwrap_or(EffectClass::Execute);
            }
        }
        // Default effect classes for reserved intents
        match name {
            "ReadFile" => EffectClass::Read,
            "WriteFile" => EffectClass::Write,
            "Spawn" | "Pause" | "Resume" | "Terminate" => EffectClass::ModifyAgent,
            "Verify" => EffectClass::Execute,
            "HttpGet" | "HttpPost" => EffectClass::Network,
            "ProposePredicate" => EffectClass::ModifyPredicate,
            "GrantPrivilege" | "RevokePrivilege" => EffectClass::ModifyPrivilege,
            "ForkSandbox" | "MergeSandbox" => EffectClass::Execute,
            "Log" => EffectClass::Noop,
            _ => EffectClass::Execute,
        }
    }

    fn exec_intent(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: &Span,
    ) -> AxiomResult<Value> {
        // Check for composed intent (Section 6.3): intent X = A >> B >> C
        // Executes A first, feeds output into B as input, feeds that into C
        if let Some(intent_decl) = self.intents.get(name).cloned() {
            if let Some(ref chain) = intent_decl.composed_of {
                return self.exec_composed_intent(chain, fields, span);
            }
        }

        // Create checkpoint before intent execution (Part XIV)
        self.create_checkpoint();

        let mut field_values = Vec::new();
        for (fname, fexpr) in fields {
            let val = self.eval_expr(fexpr)?;
            field_values.push((fname.clone(), val));
        }

        // Compute pre-state hash for traceability (A2)
        let pre_hash = {
            let state_val = Value::String(format!("epoch:{}", self.epoch));
            lcb::content_address(&state_val)
        };

        // Evaluate pre-conditions (guard: pre) from intent declaration
        // Pre-conditions reference takes parameters — bind them in a temp scope
        if let Some(intent) = self.intents.get(name).cloned() {
            if !intent.clauses.pre.is_empty() {
                self.push_scope();
                // Bind the takes parameters from the field values
                for (fname, fval) in &field_values {
                    self.define(fname, fval.clone());
                }
                for pre in &intent.clauses.pre {
                    // Eval pre-condition; if it references unknown functions, skip gracefully
                    match self.eval_expr(pre) {
                        Ok(result) => {
                            if !result.is_truthy() {
                                self.pop_scope();
                                self.rollback_to_checkpoint();
                                return Err(AxiomError {
                                    kind: ErrorKind::HaltDeterminism,
                                    message: format!("Intent '{}' pre-condition failed", name),
                                    span: Some(span.clone()),
                                });
                            }
                        }
                        Err(e) => {
                            // RT-12: Unknown guard functions HALT — safe default is denial
                            self.pop_scope();
                            self.rollback_to_checkpoint();
                            return Err(AxiomError {
                                kind: ErrorKind::GuardFailed,
                                message: format!(
                                    "RT-12: Intent '{}' pre-condition evaluation failed: {}. Unknown guards must halt, not pass.",
                                    name, e.message
                                ),
                                span: Some(span.clone()),
                            });
                        }
                    }
                }
                self.pop_scope();
            }
        }

        // --- Conscience kernel gating (A6 — every `do` goes through here) ---
        let effect = self.intent_effect_class(name);
        let mut field_map = HashMap::new();
        for (fname, fval) in &field_values {
            field_map.insert(fname.clone(), format!("{}", fval));
        }
        let verdict = self.conscience.evaluate(name, &effect, &field_map);

        match verdict {
            ConscienceVerdict::Allow => {
                // Proceed
            }
            ConscienceVerdict::Deny(reason) => {
                self.rollback_to_checkpoint();
                self.output.push(format!("[CONSCIENCE DENY] {}: {}", name, reason));
                return Err(AxiomError {
                    kind: ErrorKind::HaltDeterminism,
                    message: format!("Conscience denied intent '{}': {}", name, reason),
                    span: Some(span.clone()),
                });
            }
            ConscienceVerdict::Unknown => {
                // Use fallback mode (Section 6.1.3)
                let fallback = self.fallback_modes.get(name).cloned()
                    .unwrap_or(FallbackMode::Abort);
                match fallback {
                    FallbackMode::Abort => {
                        self.rollback_to_checkpoint();
                        return Err(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "Conscience returned UNKNOWN for intent '{}', fallback=ABORT",
                                name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    FallbackMode::Sandbox => {
                        self.output.push(format!("[FALLBACK SANDBOX] {} executing in sandbox", name));
                        // Continue but mark results as sandboxed
                    }
                    FallbackMode::Simulate => {
                        self.output.push(format!("[FALLBACK SIMULATE] {} simulating only", name));
                        // Continue but don't persist effects
                    }
                    FallbackMode::Downgrade => {
                        self.output.push(format!("[FALLBACK DOWNGRADE] {} downgraded", name));
                        // Continue with reduced capability
                    }
                }
            }
        }

        self.output.push(format!(
            "[INTENT] do {} {{ {} }}",
            name,
            field_values
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        // Execute the intent — all results are UNTRUSTED_EXTERNAL
        let result = self.dispatch_intent(name, field_values.clone(), span)?;

        // Mark the result as untrusted (trust algebra: `do` returns external data)
        // Verify is the SOLE exception (P18)
        if name == "Verify" {
            self.trust_tracker.set("__last_do", TrustLevel::TrustedVerified);
        } else {
            self.trust_tracker.set("__last_do", TrustLevel::UntrustedExternal);
        }

        // Compute post-state hash
        let post_hash = {
            let state_val = Value::String(format!("epoch:{}:result:{}", self.epoch, result));
            lcb::content_address(&state_val)
        };

        // RT-17: Evaluate post-conditions
        if let Some(intent) = self.intents.get(name).cloned() {
            if !intent.clauses.post.is_empty() {
                self.push_scope();
                // Bind intent takes fields
                for (fname, fval) in &field_values {
                    self.define(fname, fval.clone());
                }
                // Bind gives fields to the result value
                // If a single gives field, bind its name to the result
                for give in &intent.clauses.gives {
                    self.define(&give.name, result.clone());
                }
                self.define("__result", result.clone());
                for post in &intent.clauses.post {
                    match self.eval_expr(post) {
                        Ok(post_result) => {
                            if !post_result.is_truthy() {
                                self.pop_scope();
                                self.rollback_to_checkpoint();
                                return Err(AxiomError {
                                    kind: ErrorKind::PostConditionFailed,
                                    message: format!(
                                        "RT-17: Intent '{}' post-condition failed — rolling back",
                                        name
                                    ),
                                    span: Some(span.clone()),
                                });
                            }
                        }
                        Err(e) => {
                            // RT-12: Post-condition eval failure also halts
                            self.pop_scope();
                            self.rollback_to_checkpoint();
                            return Err(AxiomError {
                                kind: ErrorKind::PostConditionFailed,
                                message: format!(
                                    "RT-17: Intent '{}' post-condition evaluation error: {}",
                                    name, e.message
                                ),
                                span: Some(span.clone()),
                            });
                        }
                    }
                }
                self.pop_scope();
            }
        }

        // Log the intent execution (A2: full traceability)
        self.intent_log.push(IntentLog {
            intent_name: name.to_string(),
            epoch: self.epoch,
            pre_hash,
            post_hash,
            verdict: "ALLOW".to_string(),
        });

        // RT-33: In single-agent mode, epoch advances per intent (no barriers).
        // In SCALE multi-agent mode, epochs would advance at barriers only.
        // This is correct for the single-agent interpreter — barriers are a SCALE concept.
        self.epoch += 1;
        self.conscience.advance_epoch();

        // Drop the checkpoint (intent succeeded, no rollback needed)
        self.checkpoints.pop();

        Ok(result)
    }

    /// Execute a composed intent chain: A >> B >> C (Section 6.3)
    /// Output of each stage feeds as input to the next stage.
    /// RT-13: Enforces irreversible-terminal rule at runtime
    fn exec_composed_intent(
        &mut self,
        chain: &[String],
        initial_fields: &[(String, Expr)],
        span: &Span,
    ) -> AxiomResult<Value> {
        if chain.is_empty() {
            return Ok(Value::Void);
        }

        // RT-13: Verify irreversible intents are chain-terminal
        for (i, intent_name) in chain.iter().enumerate() {
            if i < chain.len() - 1 {
                // Not the last intent — check if it's marked [irreversible]
                if let Some(intent_decl) = self.intents.get(intent_name) {
                    let is_irreversible = intent_decl.attributes.iter()
                        .any(|a| a.name == "irreversible");
                    if is_irreversible {
                        return Err(AxiomError {
                            kind: ErrorKind::IrreversibleNotTerminal,
                            message: format!(
                                "RT-13: Irreversible intent '{}' must be chain-terminal (position {}/{} in chain)",
                                intent_name, i + 1, chain.len()
                            ),
                            span: Some(span.clone()),
                        });
                    }
                }
            }
        }

        // Create a single checkpoint for the entire chain (atomic composition)
        self.create_checkpoint();

        // Evaluate initial fields
        let mut current_fields: Vec<(String, Expr)> = Vec::new();
        for (fname, fexpr) in initial_fields {
            current_fields.push((fname.clone(), fexpr.clone()));
        }

        let mut last_result = Value::Void;

        for (i, intent_name) in chain.iter().enumerate() {
            // For stages after the first, wrap the previous result as a synthetic field
            let fields_to_use = if i == 0 {
                current_fields.clone()
            } else {
                // Feed previous result as "input" field to next stage
                vec![("input".to_string(), self.value_to_expr(&last_result, span))]
            };

            self.output.push(format!(
                "[COMPOSE] Stage {}/{}: {}",
                i + 1,
                chain.len(),
                intent_name
            ));

            // Execute single intent in the chain (bypass composed check by calling exec_single_intent)
            match self.exec_single_intent(intent_name, &fields_to_use, span) {
                Ok(result) => {
                    last_result = result;
                }
                Err(e) => {
                    // Rollback entire chain on any failure
                    self.rollback_to_checkpoint();
                    return Err(e);
                }
            }
        }

        // Drop checkpoint — entire chain succeeded
        self.checkpoints.pop();

        Ok(last_result)
    }

    /// Convert a Value back to a synthetic Expr for composition chaining
    fn value_to_expr(&self, value: &Value, span: &Span) -> Expr {
        match value {
            Value::I64(n) => Expr::IntLiteral(*n, span.clone()),
            Value::F64(n) => Expr::FloatLiteral(*n, span.clone()),
            Value::Bool(b) => Expr::BoolLiteral(*b, span.clone()),
            Value::String(s) => Expr::StringLiteral(s.clone(), span.clone()),
            _ => Expr::StringLiteral(format!("{}", value), span.clone()),
        }
    }

    /// Execute a single intent (non-composed) — used by exec_composed_intent to avoid recursion
    fn exec_single_intent(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: &Span,
    ) -> AxiomResult<Value> {
        let mut field_values = Vec::new();
        for (fname, fexpr) in fields {
            let val = self.eval_expr(fexpr)?;
            field_values.push((fname.clone(), val));
        }

        let pre_hash = {
            let state_val = Value::String(format!("epoch:{}", self.epoch));
            lcb::content_address(&state_val)
        };

        // Evaluate pre-conditions
        if let Some(intent) = self.intents.get(name).cloned() {
            if !intent.clauses.pre.is_empty() {
                self.push_scope();
                for (fname, fval) in &field_values {
                    self.define(fname, fval.clone());
                }
                for pre in &intent.clauses.pre {
                    match self.eval_expr(pre) {
                        Ok(result) => {
                            if !result.is_truthy() {
                                self.pop_scope();
                                return Err(AxiomError {
                                    kind: ErrorKind::HaltDeterminism,
                                    message: format!("Intent '{}' pre-condition failed", name),
                                    span: Some(span.clone()),
                                });
                            }
                        }
                        Err(_) => {}
                    }
                }
                self.pop_scope();
            }
        }

        // Conscience kernel gating
        let effect = self.intent_effect_class(name);
        let mut field_map = HashMap::new();
        for (fname, fval) in &field_values {
            field_map.insert(fname.clone(), format!("{}", fval));
        }
        let verdict = self.conscience.evaluate(name, &effect, &field_map);

        match verdict {
            ConscienceVerdict::Allow => {}
            ConscienceVerdict::Deny(reason) => {
                return Err(AxiomError {
                    kind: ErrorKind::HaltDeterminism,
                    message: format!("Conscience denied intent '{}': {}", name, reason),
                    span: Some(span.clone()),
                });
            }
            ConscienceVerdict::Unknown => {
                let fallback = self.fallback_modes.get(name).cloned()
                    .unwrap_or(FallbackMode::Abort);
                match fallback {
                    FallbackMode::Abort => {
                        return Err(AxiomError {
                            kind: ErrorKind::HaltDeterminism,
                            message: format!(
                                "Conscience returned UNKNOWN for intent '{}', fallback=ABORT",
                                name
                            ),
                            span: Some(span.clone()),
                        });
                    }
                    _ => {
                        self.output.push(format!("[FALLBACK] {} executing with fallback", name));
                    }
                }
            }
        }

        self.output.push(format!(
            "[INTENT] do {} {{ {} }}",
            name,
            field_values
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        let result = self.dispatch_intent(name, field_values, span)?;

        if name == "Verify" {
            self.trust_tracker.set("__last_do", TrustLevel::TrustedVerified);
        } else {
            self.trust_tracker.set("__last_do", TrustLevel::UntrustedExternal);
        }

        let post_hash = {
            let state_val = Value::String(format!("epoch:{}:result:{}", self.epoch, result));
            lcb::content_address(&state_val)
        };

        self.intent_log.push(IntentLog {
            intent_name: name.to_string(),
            epoch: self.epoch,
            pre_hash,
            post_hash,
            verdict: "ALLOW".to_string(),
        });

        self.epoch += 1;
        self.conscience.advance_epoch();

        Ok(result)
    }

    /// Dispatch to the actual intent implementation (reserved + user-defined)
    fn dispatch_intent(
        &mut self,
        name: &str,
        field_values: Vec<(String, Value)>,
        span: &Span,
    ) -> AxiomResult<Value> {
        let find_field = |fields: &[(String, Value)], key: &str| -> Option<Value> {
            fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
        };

        match name {
            // --- Reserved Intent 100: ReadFile ---
            "ReadFile" => {
                let path = find_field(&field_values, "path")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] ReadFile: read '{}' (simulated)",
                    path
                ));
                Ok(Value::String(format!("<contents of {}>", path)))
            }

            // --- Reserved Intent 101: WriteFile ---
            "WriteFile" => {
                let path = find_field(&field_values, "path")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                let data = find_field(&field_values, "data")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] WriteFile: wrote {} bytes to '{}' (simulated)",
                    data.len(),
                    path
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 102: HttpGet ---
            "HttpGet" => {
                let url = find_field(&field_values, "url")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] HttpGet: GET {} (simulated)",
                    url
                ));
                let mut resp = BTreeMap::new();
                resp.insert("status".to_string(), Value::I64(200));
                resp.insert("body".to_string(), Value::String(format!("<response from {}>", url)));
                Ok(Value::Contract(ContractValue {
                    name: "HttpResponse".to_string(),
                    fields: resp,
                }))
            }

            // --- Reserved Intent 103: HttpPost ---
            "HttpPost" => {
                let url = find_field(&field_values, "url")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] HttpPost: POST {} (simulated)",
                    url
                ));
                let mut resp = BTreeMap::new();
                resp.insert("status".to_string(), Value::I64(200));
                resp.insert("body".to_string(), Value::String("ok".to_string()));
                Ok(Value::Contract(ContractValue {
                    name: "HttpResponse".to_string(),
                    fields: resp,
                }))
            }

            // --- Reserved Intent 104: Spawn ---
            "Spawn" => {
                let role = find_field(&field_values, "role")
                    .map(|v| format!("{}", v))
                    .unwrap_or_else(|| "default".to_string());
                // Generate deterministic agent ID via BLAKE3
                let id_input = format!("agent_{}_{}", self.epoch, role);
                let id = blake3::hash(id_input.as_bytes()).to_hex().to_string();
                let short_id = &id[..16];
                self.output.push(format!(
                    "[INTENT RESULT] Spawn: agent {} with role '{}'",
                    short_id, role
                ));
                Ok(Value::String(short_id.to_string()))
            }

            // --- Reserved Intent 105: Pause ---
            "Pause" => {
                let agent_id = find_field(&field_values, "agent")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] Pause: agent {} paused",
                    agent_id
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 106: Resume ---
            "Resume" => {
                let agent_id = find_field(&field_values, "agent")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] Resume: agent {} resumed",
                    agent_id
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 107: Terminate ---
            "Terminate" => {
                let agent_id = find_field(&field_values, "agent")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] Terminate: agent {} terminated",
                    agent_id
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 108: Verify (P18 — sole trust promotion path) ---
            // RT-14: Verify now actually validates data against a declared schema
            "Verify" => {
                let schema_name = find_field(&field_values, "schema")
                    .map(|v| format!("{}", v));
                let data = find_field(&field_values, "data");

                match (data, schema_name) {
                    (Some(val), Some(schema)) => {
                        // Validate: data must be a Contract whose name matches schema
                        let valid = match &val {
                            Value::Contract(c) => c.name == schema,
                            // Primitives are valid against their type name
                            Value::I64(_) => schema == "i64",
                            Value::F64(_) => schema == "f64",
                            Value::Bool(_) => schema == "bool",
                            Value::String(_) => schema == "String",
                            Value::Bytes(_) => schema == "Bytes",
                            Value::Array(_) => schema == "Array",
                            _ => false,
                        };

                        if valid {
                            let handle = self.store.collapse(&val, "verified");
                            self.output.push(format!(
                                "[INTENT RESULT] Verify: validated against schema '{}', handle: {}",
                                schema, &handle[..16]
                            ));
                            Ok(val)
                        } else {
                            self.output.push(format!(
                                "[INTENT RESULT] Verify: REJECTED — data type '{}' does not match schema '{}'",
                                val.type_name(), schema
                            ));
                            Err(AxiomError {
                                kind: ErrorKind::HaltContract,
                                message: format!(
                                    "RT-14: Verify failed — data type '{}' does not conform to schema '{}'",
                                    val.type_name(), schema
                                ),
                                span: Some(span.clone()),
                            })
                        }
                    }
                    (Some(val), None) => {
                        // No schema specified — structural verification only.
                        // Content-address the value and promote trust.
                        // RT-14: This is the minimal Verify path — when schema IS provided,
                        // full validation occurs above.
                        let handle = self.store.collapse(&val, "verified");
                        self.output.push(format!(
                            "[INTENT RESULT] Verify: trust promoted, handle: {}",
                            &handle[..16]
                        ));
                        Ok(val)
                    }
                    (None, _) => {
                        Err(AxiomError {
                            kind: ErrorKind::HaltContract,
                            message: "RT-14: Verify requires 'data' field".to_string(),
                            span: Some(span.clone()),
                        })
                    }
                }
            }

            // --- Reserved Intent 109: ProposePredicate (Section 6.6) ---
            "ProposePredicate" => {
                let pred_name = find_field(&field_values, "name")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                let description = find_field(&field_values, "description")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] ProposePredicate: '{}' proposed — {}",
                    pred_name, description
                ));
                // In emulated mode, auto-accept restriction predicates
                let _ = self.conscience.add_restriction(
                    pred_name.clone(),
                    description,
                    vec![EffectClass::Execute],
                    crate::conscience::PredicateRule::AlwaysAllow,
                );
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 110: GrantPrivilege ---
            "GrantPrivilege" => {
                let target = find_field(&field_values, "target")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                let privilege = find_field(&field_values, "privilege")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] GrantPrivilege: {} → {}",
                    privilege, target
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 111: RevokePrivilege ---
            "RevokePrivilege" => {
                let target = find_field(&field_values, "target")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                let privilege = find_field(&field_values, "privilege")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] RevokePrivilege: {} revoked from {}",
                    privilege, target
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 112: ForkSandbox (Section 6.7.1) ---
            "ForkSandbox" => {
                let sandbox_id = {
                    let input = format!("sandbox_{}_{}", self.epoch, name);
                    let hash = blake3::hash(input.as_bytes()).to_hex().to_string();
                    hash[..16].to_string()
                };
                self.output.push(format!(
                    "[INTENT RESULT] ForkSandbox: sandbox {} created",
                    sandbox_id
                ));
                Ok(Value::String(sandbox_id))
            }

            // --- Reserved Intent 113: MergeSandbox ---
            "MergeSandbox" => {
                let sandbox_id = find_field(&field_values, "sandbox")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!(
                    "[INTENT RESULT] MergeSandbox: sandbox {} merged (if valid)",
                    sandbox_id
                ));
                Ok(Value::Bool(true))
            }

            // --- Reserved Intent 114: Log (NOOP effect) ---
            "Log" => {
                let message = find_field(&field_values, "message")
                    .map(|v| format!("{}", v))
                    .unwrap_or_default();
                self.output.push(format!("[LOG] {}", message));
                Ok(Value::Void)
            }

            // --- Reserved Intent 115: Halt ---
            "Halt" => {
                let reason = find_field(&field_values, "reason")
                    .map(|v| format!("{}", v))
                    .unwrap_or_else(|| "explicit halt".to_string());
                self.output.push(format!("[HALT] {}", reason));
                Err(AxiomError {
                    kind: ErrorKind::HaltDeterminism,
                    message: format!("Explicit halt: {}", reason),
                    span: Some(span.clone()),
                })
            }

            // --- User-defined intents ---
            _ => {
                self.output.push(format!(
                    "[INTENT RESULT] {}: executed (emulated runtime)",
                    name
                ));
                // Return a generic result based on intent gives
                if let Some(intent) = self.intents.get(name).cloned() {
                    if intent.clauses.gives.len() == 1 {
                        let give = &intent.clauses.gives[0];
                        return Ok(self.default_value_for_type_name(&give.ty));
                    }
                }
                Ok(Value::Void)
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Value], span: &Span) -> AxiomResult<Value> {
        // Built-in functions
        match name {
            "print" | "log" => {
                // RT-01: Sealed values cannot be printed/logged
                for arg in args {
                    if arg.is_sealed() {
                        return Err(AxiomError {
                            kind: ErrorKind::SealedViolation,
                            message: "RT-01: Cannot print/log a Sealed value".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                }
                let msg: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
                let output = msg.join(" ");
                self.output.push(output);
                return Ok(Value::Void);
            }
            "length" | "len" => {
                if let Some(val) = args.first() {
                    match val {
                        Value::String(s) => return Ok(Value::I64(s.len() as i64)),
                        Value::Array(a) => return Ok(Value::I64(a.len() as i64)),
                        _ => return Ok(Value::I64(0)),
                    }
                }
                return Ok(Value::I64(0));
            }
            "sqrt" => {
                if let Some(val) = args.first() {
                    match val.as_f64() {
                        Some(n) => return Ok(Value::F64(n.sqrt())),
                        None => {
                            return Err(self.runtime_error(span, "sqrt() requires numeric argument"))
                        }
                    }
                }
                return Ok(Value::F64(0.0));
            }
            "abs" => {
                if let Some(val) = args.first() {
                    match val {
                        Value::I64(n) => return Ok(Value::I64(n.abs())),
                        Value::F64(n) => return Ok(Value::F64(n.abs())),
                        _ => {
                            return Err(self.runtime_error(span, "abs() requires numeric argument"))
                        }
                    }
                }
                return Ok(Value::I64(0));
            }
            "to_string" => {
                if let Some(val) = args.first() {
                    // RT-01: Sealed values cannot be converted to string
                    if val.is_sealed() {
                        return Err(AxiomError {
                            kind: ErrorKind::SealedViolation,
                            message: "RT-01: Cannot convert Sealed value to string".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                    return Ok(Value::String(format!("{}", val)));
                }
                return Ok(Value::String(String::new()));
            }
            "bounded" => {
                // invariant helper: bounded(val, min, max)
                if args.len() == 3 {
                    let val = args[0].as_f64().unwrap_or(0.0);
                    let min = args[1].as_f64().unwrap_or(0.0);
                    let max = args[2].as_f64().unwrap_or(0.0);
                    return Ok(Value::Bool(val >= min && val <= max));
                }
                return Ok(Value::Bool(false));
            }
            // === Self-hosting primitives (Session 2) ===

            // S2-10: Character access — required for lexer implementation
            "char_at" => {
                if args.len() >= 2 {
                    if let (Value::String(s), Value::I64(i)) = (&args[0], &args[1]) {
                        let idx = *i as usize;
                        if idx < s.len() {
                            // Return the character as a single-char string
                            if let Some(ch) = s.chars().nth(idx) {
                                return Ok(Value::String(ch.to_string()));
                            }
                        }
                        return Err(self.runtime_error(
                            span,
                            &format!("char_at: index {} out of bounds (len {})", i, s.len()),
                        ));
                    }
                }
                return Err(self.runtime_error(span, "char_at(string, index) requires String and i64"));
            }
            // S2-11: Character code — needed for classifying characters in a lexer
            "char_code" => {
                if let Some(Value::String(s)) = args.first() {
                    if let Some(ch) = s.chars().next() {
                        return Ok(Value::I64(ch as i64));
                    }
                    return Ok(Value::I64(0));
                }
                return Err(self.runtime_error(span, "char_code(string) requires a String"));
            }
            // S2-11: char_from_code — inverse of char_code
            "char_from_code" => {
                if let Some(Value::I64(code)) = args.first() {
                    if let Some(ch) = char::from_u32(*code as u32) {
                        return Ok(Value::String(ch.to_string()));
                    }
                    return Err(self.runtime_error(span, &format!("char_from_code: invalid code point {}", code)));
                }
                return Err(self.runtime_error(span, "char_from_code(i64) requires an i64"));
            }
            // S2-10: Substring extraction
            "substring" => {
                if args.len() >= 3 {
                    if let (Value::String(s), Value::I64(start), Value::I64(end)) = (&args[0], &args[1], &args[2]) {
                        let start = *start as usize;
                        let end = *end as usize;
                        if start <= end && end <= s.len() {
                            let result: String = s.chars().skip(start).take(end - start).collect();
                            return Ok(Value::String(result));
                        }
                        return Err(self.runtime_error(
                            span,
                            &format!("substring: indices [{}, {}) out of bounds (len {})", start, end, s.len()),
                        ));
                    }
                }
                return Err(self.runtime_error(span, "substring(string, start, end) requires String, i64, i64"));
            }
            // S2-12: Mutable array push
            "push" => {
                if args.len() >= 2 {
                    if let Value::Array(ref elems) = args[0] {
                        let mut new_arr = elems.clone();
                        new_arr.push(args[1].clone());
                        return Ok(Value::Array(new_arr));
                    }
                }
                return Err(self.runtime_error(span, "push(array, value) requires an Array"));
            }
            // S2-11: Type casting — parse_int
            "parse_int" => {
                if let Some(Value::String(s)) = args.first() {
                    match s.trim().parse::<i64>() {
                        Ok(n) => return Ok(Value::I64(n)),
                        Err(_) => return Err(self.runtime_error(
                            span,
                            &format!("parse_int: cannot parse '{}' as integer", s),
                        )),
                    }
                }
                return Err(self.runtime_error(span, "parse_int(string) requires a String"));
            }
            // S2-11: Type casting — as_f64 (explicit i64 -> f64)
            "as_f64" => {
                if let Some(val) = args.first() {
                    match val {
                        Value::I64(n) => return Ok(Value::F64(*n as f64)),
                        Value::F64(n) => return Ok(Value::F64(*n)),
                        Value::String(s) => {
                            match s.trim().parse::<f64>() {
                                Ok(n) => return Ok(Value::F64(n)),
                                Err(_) => return Err(self.runtime_error(
                                    span,
                                    &format!("as_f64: cannot parse '{}' as float", s),
                                )),
                            }
                        }
                        _ => return Err(self.runtime_error(span, "as_f64: unsupported type")),
                    }
                }
                return Err(self.runtime_error(span, "as_f64(value) requires a value"));
            }
            // S2-11: Type casting — as_i64 (explicit f64 -> i64, truncating)
            "as_i64" => {
                if let Some(val) = args.first() {
                    match val {
                        Value::F64(n) => return Ok(Value::I64(*n as i64)),
                        Value::I64(n) => return Ok(Value::I64(*n)),
                        Value::String(s) => {
                            match s.trim().parse::<i64>() {
                                Ok(n) => return Ok(Value::I64(n)),
                                Err(_) => return Err(self.runtime_error(
                                    span,
                                    &format!("as_i64: cannot parse '{}' as integer", s),
                                )),
                            }
                        }
                        _ => return Err(self.runtime_error(span, "as_i64: unsupported type")),
                    }
                }
                return Err(self.runtime_error(span, "as_i64(value) requires a value"));
            }
            // String contains check — useful for lexer keyword matching
            "contains" => {
                if args.len() >= 2 {
                    if let (Value::String(haystack), Value::String(needle)) = (&args[0], &args[1]) {
                        return Ok(Value::Bool(haystack.contains(needle.as_str())));
                    }
                }
                return Err(self.runtime_error(span, "contains(string, substring) requires two Strings"));
            }
            // === Map operations (Session 2) ===
            "map_new" => {
                return Ok(Value::Map(BTreeMap::new()));
            }
            "map_insert" => {
                // map_insert(map, key, value) -> new map with key inserted
                if args.len() >= 3 {
                    if let (Value::Map(ref m), Value::String(ref k)) = (&args[0], &args[1]) {
                        let mut new_map = m.clone();
                        new_map.insert(k.clone(), args[2].clone());
                        return Ok(Value::Map(new_map));
                    }
                }
                return Err(self.runtime_error(span, "map_insert(map, key_string, value) requires Map, String, Value"));
            }
            "map_get" => {
                // map_get(map, key) -> value or Void
                if args.len() >= 2 {
                    if let (Value::Map(ref m), Value::String(ref k)) = (&args[0], &args[1]) {
                        return Ok(m.get(k).cloned().unwrap_or(Value::Void));
                    }
                }
                return Err(self.runtime_error(span, "map_get(map, key_string) requires Map and String"));
            }
            "map_has_key" => {
                if args.len() >= 2 {
                    if let (Value::Map(ref m), Value::String(ref k)) = (&args[0], &args[1]) {
                        return Ok(Value::Bool(m.contains_key(k)));
                    }
                }
                return Err(self.runtime_error(span, "map_has_key(map, key_string) requires Map and String"));
            }
            "map_keys" => {
                if let Some(Value::Map(ref m)) = args.first() {
                    let keys: Vec<Value> = m.keys().map(|k| Value::String(k.clone())).collect();
                    return Ok(Value::Array(keys));
                }
                return Err(self.runtime_error(span, "map_keys(map) requires a Map"));
            }
            "map_size" => {
                if let Some(Value::Map(ref m)) = args.first() {
                    return Ok(Value::I64(m.len() as i64));
                }
                return Err(self.runtime_error(span, "map_size(map) requires a Map"));
            }
            // RT-12: Guard helper functions — these exist so intent preconditions
            // referencing them don't trigger the "unknown guard -> HALT" rule
            "path_exists" => {
                // In emulated mode, all paths are assumed to exist
                return Ok(Value::Bool(true));
            }
            "path_is_safe" => {
                // Delegate to conscience kernel path_safety predicate
                if let Some(val) = args.first() {
                    let path = format!("{}", val);
                    let denied = ["/etc/passwd", "/etc/shadow", "/proc/", "/sys/"];
                    return Ok(Value::Bool(!denied.iter().any(|d| path.starts_with(d))));
                }
                return Ok(Value::Bool(true));
            }
            _ => {}
        }

        // User-defined functions
        if let Some(func) = self.functions.get(name).cloned() {
            return self.call_function(&func, args);
        }

        // Unknown function
        Err(self.runtime_error(span, &format!("Undefined function '{}'", name)))
    }

    fn binary_op(
        &self,
        left: &Value,
        op: &BinOp,
        right: &Value,
        span: &Span,
    ) -> AxiomResult<Value> {
        match op {
            // RT-03: Checked arithmetic — overflow is HALT_RESOURCE, not UB
            // RT-05: No implicit i64/f64 coercion — mixed types are errors
            BinOp::Add => match (left, right) {
                (Value::I64(a), Value::I64(b)) => {
                    Value::checked_add(*a, *b)
                        .map(Value::I64)
                        .ok_or_else(|| AxiomError {
                            kind: ErrorKind::IntegerOverflow,
                            message: format!("HALT_RESOURCE: integer overflow in {} + {}", a, b),
                            span: Some(span.clone()),
                        })
                }
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a + b)),
                (Value::String(a), Value::String(b)) => {
                    Ok(Value::String(format!("{}{}", a, b)))
                }
                // RT-05: Mixed i64/f64 is an error, not an implicit coercion
                (Value::I64(_), Value::F64(_)) | (Value::F64(_), Value::I64(_)) => {
                    Err(self.runtime_error(
                        span,
                        &format!("RT-05: Cannot mix {} and {} without explicit cast", left.type_name(), right.type_name()),
                    ))
                }
                _ => Err(self.runtime_error(
                    span,
                    &format!("Cannot add {} and {}", left.type_name(), right.type_name()),
                )),
            },
            BinOp::Sub => match (left, right) {
                (Value::I64(a), Value::I64(b)) => {
                    Value::checked_sub(*a, *b)
                        .map(Value::I64)
                        .ok_or_else(|| AxiomError {
                            kind: ErrorKind::IntegerOverflow,
                            message: format!("HALT_RESOURCE: integer overflow in {} - {}", a, b),
                            span: Some(span.clone()),
                        })
                }
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a - b)),
                (Value::I64(_), Value::F64(_)) | (Value::F64(_), Value::I64(_)) => {
                    Err(self.runtime_error(
                        span,
                        &format!("RT-05: Cannot mix {} and {} without explicit cast", left.type_name(), right.type_name()),
                    ))
                }
                _ => Err(self.runtime_error(
                    span,
                    &format!("Cannot subtract {} and {}", left.type_name(), right.type_name()),
                )),
            },
            BinOp::Mul => match (left, right) {
                (Value::I64(a), Value::I64(b)) => {
                    Value::checked_mul(*a, *b)
                        .map(Value::I64)
                        .ok_or_else(|| AxiomError {
                            kind: ErrorKind::IntegerOverflow,
                            message: format!("HALT_RESOURCE: integer overflow in {} * {}", a, b),
                            span: Some(span.clone()),
                        })
                }
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a * b)),
                (Value::I64(_), Value::F64(_)) | (Value::F64(_), Value::I64(_)) => {
                    Err(self.runtime_error(
                        span,
                        &format!("RT-05: Cannot mix {} and {} without explicit cast", left.type_name(), right.type_name()),
                    ))
                }
                _ => Err(self.runtime_error(
                    span,
                    &format!("Cannot multiply {} and {}", left.type_name(), right.type_name()),
                )),
            },
            BinOp::Div => {
                match right {
                    Value::I64(0) => {
                        return Err(AxiomError {
                            kind: ErrorKind::DivisionByZero,
                            message: "Division by zero".to_string(),
                            span: Some(span.clone()),
                        })
                    }
                    Value::F64(n) if *n == 0.0 => {
                        return Err(AxiomError {
                            kind: ErrorKind::DivisionByZero,
                            message: "Division by zero".to_string(),
                            span: Some(span.clone()),
                        })
                    }
                    _ => {}
                }
                match (left, right) {
                    (Value::I64(a), Value::I64(b)) => {
                        a.checked_div(*b)
                            .map(Value::I64)
                            .ok_or_else(|| AxiomError {
                                kind: ErrorKind::IntegerOverflow,
                                message: format!("HALT_RESOURCE: integer overflow in {} / {}", a, b),
                                span: Some(span.clone()),
                            })
                    }
                    (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a / b)),
                    (Value::I64(_), Value::F64(_)) | (Value::F64(_), Value::I64(_)) => {
                        Err(self.runtime_error(
                            span,
                            &format!("RT-05: Cannot mix {} and {} without explicit cast", left.type_name(), right.type_name()),
                        ))
                    }
                    _ => Err(self.runtime_error(
                        span,
                        &format!("Cannot divide {} and {}", left.type_name(), right.type_name()),
                    )),
                }
            }
            BinOp::Mod => match (left, right) {
                (Value::I64(a), Value::I64(b)) => {
                    if *b == 0 {
                        Err(AxiomError {
                            kind: ErrorKind::DivisionByZero,
                            message: "Modulo by zero".to_string(),
                            span: Some(span.clone()),
                        })
                    } else {
                        Ok(Value::I64(a % b))
                    }
                }
                _ => Err(self.runtime_error(span, "Modulo requires integers")),
            },
            BinOp::Eq => Ok(Value::Bool(left == right)),
            BinOp::NotEq => Ok(Value::Bool(left != right)),
            BinOp::Lt => self.compare_values(left, right, span, |a, b| a < b),
            BinOp::Gt => self.compare_values(left, right, span, |a, b| a > b),
            BinOp::LtEq => self.compare_values(left, right, span, |a, b| a <= b),
            BinOp::GtEq => self.compare_values(left, right, span, |a, b| a >= b),
            BinOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
        }
    }

    /// RT-05: No mixed-type comparisons
    fn compare_values<F>(
        &self,
        left: &Value,
        right: &Value,
        span: &Span,
        cmp: F,
    ) -> AxiomResult<Value>
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left, right) {
            (Value::I64(a), Value::I64(b)) => Ok(Value::Bool(cmp(*a as f64, *b as f64))),
            (Value::F64(a), Value::F64(b)) => Ok(Value::Bool(cmp(*a, *b))),
            // RT-05: Mixed types require explicit cast
            (Value::I64(_), Value::F64(_)) | (Value::F64(_), Value::I64(_)) => {
                Err(self.runtime_error(
                    span,
                    &format!("RT-05: Cannot compare {} and {} without explicit cast", left.type_name(), right.type_name()),
                ))
            }
            _ => Err(self.runtime_error(
                span,
                &format!("Cannot compare {} and {}", left.type_name(), right.type_name()),
            )),
        }
    }

    fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> bool {
        match (pattern, value) {
            (Pattern::Wildcard, _) => true,
            (Pattern::IntLiteral(n), Value::I64(v)) => *n == *v,
            (Pattern::FloatLiteral(n), Value::F64(v)) => *n == *v,
            (Pattern::StringLiteral(s), Value::String(v)) => s == v,
            (Pattern::BoolLiteral(b), Value::Bool(v)) => b == v,
            (Pattern::Ident(_), _) => true, // Bind to variable
            (Pattern::EnumVariant(enum_name, variant), Value::Enum(en, ev)) => {
                enum_name == en && variant == ev
            }
            _ => false,
        }
    }

    fn default_value_for_type_name(&self, ty: &TypeExpr) -> Value {
        match ty {
            TypeExpr::Named(name, _) => match name.as_str() {
                "i64" => Value::I64(0),
                "f64" => Value::F64(0.0),
                "bool" => Value::Bool(false),
                "String" => Value::String(String::new()),
                "Handle" => Value::Handle("0000000000000000".to_string()),
                _ => Value::Void,
            },
            TypeExpr::Array(_, _) => Value::Array(Vec::new()),
            _ => Value::Void,
        }
    }

    // --- Scope management ---

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    fn lookup_var(&self, name: &str, span: &Span) -> AxiomResult<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Ok(val.clone());
            }
        }
        Err(AxiomError {
            kind: ErrorKind::UndefinedVariable,
            message: format!("Undefined variable '{}'", name),
            span: Some(span.clone()),
        })
    }

    fn set_var(&mut self, name: &str, value: Value) -> AxiomResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        // If not found, define in current scope
        self.define(name, value);
        Ok(())
    }

    fn runtime_error(&self, span: &Span, message: &str) -> AxiomError {
        AxiomError {
            kind: ErrorKind::HaltDeterminism,
            message: message.to_string(),
            span: Some(span.clone()),
        }
    }
}
