use crate::agent::AgentPool;
use crate::builtins;
use crate::governance::{Effect, EffectType, GovernanceRegistry};
use crate::rsi::{AgentMemory, ModificationType, RSIPipeline};
use crate::scale::ScalePool;
use crate::tensor::Tensor;
use crate::{Bytecode, Opcode, RuntimeError, RuntimeResult, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const DEFAULT_SPAWN_RATE_LIMIT: usize = 10;
const DEFAULT_SPAWN_WINDOW_SECS: u64 = 60;
const DEFAULT_MAX_TOTAL_AGENTS: usize = 1000;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub spawn_rate_limit: usize,
    pub spawn_window_secs: u64,
    pub max_total_agents: usize,
    pub max_steps: usize,
    pub register_count: usize,
    pub arg_base_register: usize,
    pub saved_register_count: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            spawn_rate_limit: DEFAULT_SPAWN_RATE_LIMIT,
            spawn_window_secs: DEFAULT_SPAWN_WINDOW_SECS,
            max_total_agents: DEFAULT_MAX_TOTAL_AGENTS,
            max_steps: 1_000_000,
            register_count: 256,
            arg_base_register: 150,
            saved_register_count: 20,
        }
    }
}

#[derive(Debug, Clone)]
struct SpawnRateLimit {
    max_spawns: usize,
    window: Duration,
    spawn_times: Vec<Instant>,
}

impl SpawnRateLimit {
    fn new(max_spawns: usize, window_secs: u64) -> Self {
        SpawnRateLimit {
            max_spawns,
            window: Duration::from_secs(window_secs),
            spawn_times: Vec::new(),
        }
    }

    fn check_and_record(&mut self) -> Result<(), String> {
        let now = Instant::now();

        self.spawn_times
            .retain(|&t| now.duration_since(t) < self.window);

        if self.spawn_times.len() >= self.max_spawns {
            return Err(format!(
                "Agent spawn rate limit exceeded: {}/{} in last {:?}",
                self.spawn_times.len(),
                self.max_spawns,
                self.window
            ));
        }

        self.spawn_times.push(now);
        Ok(())
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.spawn_times.clear();
    }
}

#[derive(Debug, Clone)]
struct CallFrame {
    return_pc: usize,
    base_reg: usize,
    #[allow(dead_code)]
    arg_count: usize,
    saved_regs: Vec<Value>,
}

#[derive(Debug, Clone)]
struct LoopFrame {
    start_pc: usize,
    end_pc: usize,
    max_iters: i64,
    iterations: i64,
}

#[derive(Debug, Clone)]
struct CycleFrame {
    #[allow(dead_code)]
    name: String,
    max_count: u64,
    current: u64,
    start_pc: usize,
}

/// Simple memory entry for HIL learn/recall
#[derive(Debug, Clone)]
pub struct MemEntry {
    pub pattern: String,
    pub confidence: f64,
}

pub struct Vm {
    registers: Vec<Value>,
    call_stack: Vec<CallFrame>,
    loop_stack: Vec<LoopFrame>,
    cycle_stack: Vec<CycleFrame>,
    agent_pool: AgentPool,
    scale_pool: ScalePool,
    governance_registry: GovernanceRegistry,
    rsi_pipeline: RSIPipeline,
    agent_memories: HashMap<u64, AgentMemory>,
    current_agent: Option<u64>,
    current_scale: Option<u64>,
    functions: HashMap<String, (usize, usize)>,
    #[allow(dead_code)]
    globals: HashMap<String, Value>,
    latent_states: HashMap<String, Value>,
    halted: bool,
    max_steps: usize,
    steps: usize,
    spawn_rate_limit: SpawnRateLimit,
    max_total_agents: usize,
    config: RuntimeConfig,
    /// Native function registry for builtins like bond(), mem_store(), mem_query()
    /// Functions get &mut Vm so they can access VM state (memory, etc.)
    natives: HashMap<String, Box<dyn Fn(&mut Vm, Vec<Value>) -> Value + Send + Sync>>,
    /// In-memory storage for HIL learn/recall (Phase 10)
    memory: Vec<MemEntry>,
}

impl Vm {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        Vm {
            registers: vec![Value::Nil; config.register_count],
            call_stack: Vec::new(),
            loop_stack: Vec::new(),
            cycle_stack: Vec::new(),
            agent_pool: AgentPool::new(),
            scale_pool: ScalePool::new(),
            governance_registry: GovernanceRegistry::new(),
            rsi_pipeline: RSIPipeline::new(),
            agent_memories: HashMap::new(),
            current_agent: None,
            current_scale: None,
            functions: HashMap::new(),
            globals: HashMap::new(),
            latent_states: HashMap::new(),
            halted: false,
            max_steps: config.max_steps,
            steps: 0,
            spawn_rate_limit: SpawnRateLimit::new(
                config.spawn_rate_limit,
                config.spawn_window_secs,
            ),
            max_total_agents: config.max_total_agents,
            config,
            natives: HashMap::new(),
            memory: Vec::new(),
        }
    }

    /// Register a native function that can be called from HLX code
    /// Native functions receive &mut Vm so they can access VM state (memory, etc.)
    pub fn register_native(
        &mut self,
        name: &str,
        f: impl Fn(&mut Vm, Vec<Value>) -> Value + Send + Sync + 'static,
    ) {
        self.natives.insert(name.to_string(), Box::new(f));
    }

    /// Store a pattern in memory (for HIL learn) - called from native dispatch
    pub fn mem_store(&mut self, pattern: String, confidence: f64) -> bool {
        self.memory.push(MemEntry {
            pattern,
            confidence,
        });
        true
    }

    /// Query memory for patterns matching query (for HIL recall) - called from native dispatch
    pub fn mem_query(&self, query: &str, limit: usize) -> Vec<String> {
        let mut matches: Vec<(String, f64)> = self
            .memory
            .iter()
            .filter(|entry| entry.pattern.contains(query))
            .map(|entry| (entry.pattern.clone(), entry.confidence))
            .collect();
        // Sort by confidence descending
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.into_iter().take(limit).map(|(p, _)| p).collect()
    }

    /// Get mutable reference to memory (for native function implementations)
    pub fn memory_mut(&mut self) -> &mut Vec<MemEntry> {
        &mut self.memory
    }

    /// Get immutable reference to memory
    pub fn memory(&self) -> &[MemEntry] {
        &self.memory
    }

    pub fn with_max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    pub fn with_spawn_rate_limit(mut self, max_spawns: usize, window_secs: u64) -> Self {
        self.spawn_rate_limit = SpawnRateLimit::new(max_spawns, window_secs);
        self
    }

    pub fn with_max_agents(mut self, max: usize) -> Self {
        self.max_total_agents = max;
        self
    }

    pub fn load_functions(&mut self, funcs: &HashMap<String, (u32, u32)>) {
        for (name, &(start_pc, params)) in funcs {
            self.functions
                .insert(name.clone(), (start_pc as usize, params as usize));
        }
    }

    pub fn get_register(&self, idx: usize) -> &Value {
        self.registers.get(idx).unwrap_or(&Value::Nil)
    }

    pub fn get_register_cloned(&self, idx: usize) -> Value {
        self.registers.get(idx).cloned().unwrap_or(Value::Nil)
    }

    pub fn set_register(&mut self, idx: usize, val: Value) {
        if idx < self.registers.len() {
            self.registers[idx] = val;
        }
    }

    pub fn run(&mut self, bytecode: &Bytecode) -> RuntimeResult<Value> {
        let mut pc: usize = 0;

        while pc < bytecode.code.len() && !self.halted {
            self.steps += 1;
            if self.steps > self.max_steps {
                return Err(RuntimeError::new("Max steps exceeded", pc));
            }

            let op_byte = bytecode.read_u16(&mut pc)?;
            let op = Opcode::from_u16(op_byte)
                .ok_or_else(|| RuntimeError::new(format!("Unknown opcode: {}", op_byte), pc))?;

            match op {
                Opcode::Nop => {}

                Opcode::Halt => {
                    self.halted = true;
                    return Ok(self.get_register_cloned(0));
                }

                Opcode::Const => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let idx = bytecode.read_u32(&mut pc)? as usize;
                    let val = bytecode.constants.get(idx).cloned().unwrap_or(Value::Nil);
                    self.set_register(dst, val);
                }

                Opcode::Move => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let val = self.get_register(src).clone();
                    self.set_register(dst, val);
                }

                Opcode::Add => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.binary_add(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Sub => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.binary_sub(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Mul => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.binary_mul(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Div => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.binary_div(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Mod => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.binary_mod(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Neg => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let result = match self.get_register(src) {
                        Value::I64(n) => Value::I64(-n),
                        Value::F64(n) => Value::F64(-n),
                        _ => return Err(RuntimeError::new("Cannot negate non-numeric", pc)),
                    };
                    self.set_register(dst, result);
                }

                Opcode::Eq => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = Value::Bool(self.get_register(a) == self.get_register(b));
                    self.set_register(dst, result);
                }

                Opcode::Ne => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = Value::Bool(self.get_register(a) != self.get_register(b));
                    self.set_register(dst, result);
                }

                Opcode::Lt => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.compare_lt(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Le => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.compare_le(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Gt => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.compare_gt(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::Ge => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = self.compare_ge(self.get_register(a), self.get_register(b))?;
                    self.set_register(dst, result);
                }

                Opcode::And => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = Value::Bool(
                        self.get_register(a).is_truthy() && self.get_register(b).is_truthy(),
                    );
                    self.set_register(dst, result);
                }

                Opcode::Or => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = Value::Bool(
                        self.get_register(a).is_truthy() || self.get_register(b).is_truthy(),
                    );
                    self.set_register(dst, result);
                }

                Opcode::Not => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let result = Value::Bool(!self.get_register(src).is_truthy());
                    self.set_register(dst, result);
                }

                Opcode::Jump => {
                    let target = bytecode.read_u32(&mut pc)? as usize;
                    pc = target;
                }

                Opcode::JumpIf => {
                    let cond = bytecode.read_u8(&mut pc)? as usize;
                    let target = bytecode.read_u32(&mut pc)? as usize;
                    if self.get_register(cond).is_truthy() {
                        pc = target;
                    }
                }

                Opcode::JumpIfNot => {
                    let cond = bytecode.read_u8(&mut pc)? as usize;
                    let target = bytecode.read_u32(&mut pc)? as usize;
                    if !self.get_register(cond).is_truthy() {
                        pc = target;
                    }
                }

                Opcode::Return => {
                    let return_val = self.registers[0].clone();
                    if let Some(frame) = self.call_stack.pop() {
                        for (i, val) in frame.saved_regs.iter().enumerate() {
                            self.registers[i] = val.clone();
                        }
                        self.registers[frame.base_reg] = return_val;
                        pc = frame.return_pc;
                    } else {
                        self.halted = true;
                        return Ok(return_val);
                    }
                }

                Opcode::Push => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let val = bytecode.read_u8(&mut pc)? as usize;
                    let arr = self.get_register_cloned(dst);
                    let new_arr = builtins::builtin_push(&[arr, self.get_register_cloned(val)])?;
                    self.set_register(dst, new_arr);
                }

                Opcode::Get => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let arr = bytecode.read_u8(&mut pc)? as usize;
                    let idx = bytecode.read_u8(&mut pc)? as usize;
                    let val = builtins::builtin_get_at(&[
                        self.get_register_cloned(arr),
                        self.get_register_cloned(idx),
                    ])?;
                    self.set_register(dst, val);
                }

                Opcode::Set => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let idx = bytecode.read_u8(&mut pc)? as usize;
                    let val = bytecode.read_u8(&mut pc)? as usize;
                    let arr = self.get_register_cloned(dst);
                    let new_arr = builtins::builtin_set_at(&[
                        arr,
                        self.get_register_cloned(idx),
                        self.get_register_cloned(val),
                    ])?;
                    self.set_register(dst, new_arr);
                }

                Opcode::Len => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let len = builtins::builtin_array_len(&[self.get_register_cloned(src)])?;
                    self.set_register(dst, len);
                }

                Opcode::Print => {
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    print!("{}", self.get_register(src));
                }

                Opcode::PrintInt => {
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    match &self.registers[src] {
                        Value::I64(n) => print!("{}", n),
                        _ => print!("{}", self.get_register(src)),
                    }
                }

                Opcode::PrintChar => {
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    if let Value::I64(n) = &self.registers[src] {
                        if let Some(c) = char::from_u32(*n as u32) {
                            print!("{}", c);
                        }
                    }
                }

                Opcode::StrLen => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let len = builtins::builtin_strlen(&[self.get_register_cloned(src)])?;
                    self.set_register(dst, len);
                }

                Opcode::Substring => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let s = bytecode.read_u8(&mut pc)? as usize;
                    let start = bytecode.read_u8(&mut pc)? as usize;
                    let len = bytecode.read_u8(&mut pc)? as usize;
                    let result = builtins::builtin_substring(&[
                        self.get_register_cloned(s),
                        self.get_register_cloned(start),
                        self.get_register_cloned(len),
                    ])?;
                    self.set_register(dst, result);
                }

                Opcode::Concat => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = builtins::builtin_concat(&[
                        self.get_register_cloned(a),
                        self.get_register_cloned(b),
                    ])?;
                    self.set_register(dst, result);
                }

                Opcode::StrCmp => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;
                    let result = builtins::builtin_strcmp(&[
                        self.get_register_cloned(a),
                        self.get_register_cloned(b),
                    ])?;
                    self.set_register(dst, result);
                }

                Opcode::Ord => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let result = builtins::builtin_ord(&[self.get_register_cloned(src)])?;
                    self.set_register(dst, result);
                }

                Opcode::Char => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let result = builtins::builtin_char(&[self.get_register_cloned(src)])?;
                    self.set_register(dst, result);
                }

                Opcode::Loop => {
                    let max_iters = bytecode.read_i64(&mut pc)?;
                    let end_pc = bytecode.read_u32(&mut pc)? as usize;
                    self.loop_stack.push(LoopFrame {
                        start_pc: pc,
                        end_pc,
                        max_iters,
                        iterations: 0,
                    });
                }

                Opcode::Break => {
                    if let Some(frame) = self.loop_stack.pop() {
                        pc = frame.end_pc;
                    }
                }

                Opcode::Continue => {
                    if let Some(frame) = self.loop_stack.last_mut() {
                        frame.iterations += 1;
                        if frame.iterations >= frame.max_iters {
                            let end = frame.end_pc;
                            self.loop_stack.pop();
                            pc = end;
                        } else {
                            pc = frame.start_pc;
                        }
                    }
                }

                Opcode::CycleBegin => {
                    let level = bytecode.read_u8(&mut pc)?;
                    let count = bytecode.read_u8(&mut pc)? as u64;

                    let name = format!("cycle_{}", level);
                    if let Some(agent_id) = self.current_agent {
                        if let Some(agent) = self.agent_pool.get_mut(agent_id) {
                            agent.set_max_cycle(&name, count);
                            agent.begin_cycle(&name);
                        }
                    }

                    self.cycle_stack.push(CycleFrame {
                        name: name.clone(),
                        max_count: count,
                        current: 0,
                        start_pc: pc,
                    });
                }

                Opcode::CycleEnd => {
                    let level = bytecode.read_u8(&mut pc)?;
                    let name = format!("cycle_{}", level);

                    let should_continue = if let Some(frame) = self.cycle_stack.last_mut() {
                        frame.current += 1;
                        if frame.current < frame.max_count {
                            pc = frame.start_pc;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !should_continue {
                        self.cycle_stack.pop();
                    }

                    if let Some(agent_id) = self.current_agent {
                        if let Some(agent) = self.agent_pool.get_mut(agent_id) {
                            agent.end_cycle(&name);
                        }
                    }
                }

                Opcode::LatentGet => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let name = bytecode.strings.get(name_idx).ok_or_else(|| {
                        RuntimeError::new(
                            format!("LatentGet: invalid string index {}", name_idx),
                            pc,
                        )
                    })?;
                    let val = if let Some(agent_id) = self.current_agent {
                        self.agent_pool
                            .get(agent_id)
                            .and_then(|a| a.get_latent(name).cloned())
                            .unwrap_or(Value::Nil)
                    } else {
                        self.latent_states.get(name).cloned().unwrap_or(Value::Nil)
                    };
                    self.set_register(dst, val);
                }

                Opcode::LatentSet => {
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let name = bytecode.strings.get(name_idx).ok_or_else(|| {
                        RuntimeError::new(
                            format!("LatentSet: invalid string index {}", name_idx),
                            pc,
                        )
                    })?;
                    let val = self.get_register_cloned(src);
                    if let Some(agent_id) = self.current_agent {
                        if let Some(agent) = self.agent_pool.get_mut(agent_id) {
                            agent.set_latent(name, val);
                        }
                    } else {
                        self.latent_states.insert(name.clone(), val);
                    }
                }

                Opcode::AgentSpawn => {
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let _latent_count = bytecode.read_u32(&mut pc)? as usize;

                    if self.agent_pool.count() >= self.max_total_agents {
                        return Err(RuntimeError::new(
                            format!("Maximum agent count reached: {}", self.max_total_agents),
                            pc,
                        ));
                    }

                    self.spawn_rate_limit
                        .check_and_record()
                        .map_err(|e| RuntimeError::new(e, pc))?;

                    let name = bytecode
                        .strings
                        .get(name_idx)
                        .cloned()
                        .unwrap_or_else(|| "AnonymousAgent".to_string());

                    let id = self.agent_pool.spawn(&name);
                    self.current_agent = Some(id);
                    self.set_register(0, Value::I64(id as i64));
                }

                Opcode::AgentHalt => {
                    let condition = bytecode.read_u8(&mut pc)? as usize;
                    if self.get_register(condition).is_truthy() {
                        self.halted = true;
                        return Ok(self.get_register_cloned(0));
                    }
                }

                Opcode::AgentDissolve => {
                    if let Some(agent_id) = self.current_agent {
                        self.agent_memories.remove(&agent_id);
                    }
                    self.halted = true;
                    return Ok(self.get_register_cloned(0));
                }

                Opcode::ScaleCreate => {
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                    let id = self.scale_pool.create(&name);
                    self.current_scale = Some(id);
                    self.set_register(0, Value::I64(id as i64));
                }

                Opcode::ScaleAddAgent => {
                    let scale_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = bytecode.read_u8(&mut pc)? as usize;
                    let agent = match &self.registers[agent_id] {
                        Value::I64(id) => *id as u64,
                        _ => return Err(RuntimeError::new("ScaleAddAgent requires agent ID", pc)),
                    };
                    if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                        scale.add_agent(agent);
                    }
                }

                Opcode::ScaleRemoveAgent => {
                    let scale_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = bytecode.read_u8(&mut pc)? as usize;
                    let agent = match &self.registers[agent_id] {
                        Value::I64(id) => *id as u64,
                        _ => {
                            return Err(RuntimeError::new("ScaleRemoveAgent requires agent ID", pc))
                        }
                    };
                    if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                        scale.remove_agent(agent);
                    }
                }

                Opcode::BarrierCreate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let expected = bytecode.read_u8(&mut pc)? as usize;
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                            let barrier_id = scale.create_barrier(expected);
                            self.set_register(dst, Value::I64(barrier_id as i64));
                        }
                    }
                }

                Opcode::BarrierArrive => {
                    let barrier_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = bytecode.read_u8(&mut pc)? as usize;
                    let agent = match &self.registers[agent_id] {
                        Value::I64(id) => *id as u64,
                        _ => return Err(RuntimeError::new("BarrierArrive requires agent ID", pc)),
                    };
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                            let released = scale.arrive_barrier(barrier_id, agent)?;
                            self.set_register(0, Value::Bool(released));
                        }
                    }
                }

                Opcode::BarrierCheck => {
                    let barrier_id = bytecode.read_u32(&mut pc)? as u64;
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get(scale_id) {
                            let released = scale.check_barrier(barrier_id)?;
                            self.set_register(0, Value::Bool(released));
                        }
                    }
                }

                Opcode::ConsensusCreate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let expected = bytecode.read_u8(&mut pc)? as usize;
                    let threshold_raw = bytecode.read_u8(&mut pc)?;
                    let threshold = threshold_raw as f64 / 100.0;
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                            let consensus_id = scale.create_consensus(expected, threshold);
                            self.set_register(dst, Value::I64(consensus_id as i64));
                        }
                    }
                }

                Opcode::ConsensusVote => {
                    let consensus_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = bytecode.read_u8(&mut pc)? as usize;
                    let value_src = bytecode.read_u8(&mut pc)? as usize;
                    let agent = match &self.registers[agent_id] {
                        Value::I64(id) => *id as u64,
                        _ => return Err(RuntimeError::new("ConsensusVote requires agent ID", pc)),
                    };
                    let value = self.registers[value_src].clone();
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get_mut(scale_id) {
                            scale.vote(consensus_id, agent, value)?;
                        }
                    }
                }

                Opcode::ConsensusResult => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let consensus_id = bytecode.read_u32(&mut pc)? as u64;
                    if let Some(scale_id) = self.current_scale {
                        if let Some(scale) = self.scale_pool.get(scale_id) {
                            let result = scale.consensus_result(consensus_id)?;
                            let map = std::collections::BTreeMap::from([
                                ("winner".to_string(), Value::String(result.winning_value)),
                                ("agreement".to_string(), Value::F64(result.agreement)),
                                ("agreed".to_string(), Value::Bool(result.agreed)),
                                ("votes".to_string(), Value::I64(result.total_votes as i64)),
                            ]);
                            self.set_register(dst, Value::Map(map));
                        }
                    }
                }

                Opcode::GovernCheck => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let effect_type_raw = bytecode.read_u8(&mut pc)?;
                    let effect_type = EffectType::from_u8(effect_type_raw)
                        .ok_or_else(|| RuntimeError::new("Invalid effect type", pc))?;
                    let desc_idx = bytecode.read_u32(&mut pc)? as usize;
                    let description = bytecode.strings.get(desc_idx).cloned().unwrap_or_default();

                    let mut effect = Effect::new(effect_type, &description);

                    if let Some(agent_id) = self.current_agent {
                        if let Some(gov) = self.governance_registry.get_mut(agent_id) {
                            let allowed = gov.check_effect(&mut effect)?;
                            self.set_register(dst, Value::Bool(allowed));
                        } else {
                            self.set_register(dst, Value::Bool(true));
                        }
                    } else {
                        self.set_register(dst, Value::Bool(true));
                    }
                }

                Opcode::GovernRegister => {
                    if let Some(agent_id) = self.current_agent {
                        self.governance_registry.create(agent_id);
                    }
                }

                Opcode::GovernSetConfidence => {
                    let confidence_raw = bytecode.read_u8(&mut pc)?;
                    let confidence = confidence_raw as f64 / 100.0;
                    if let Some(agent_id) = self.current_agent {
                        if let Some(gov) = self.governance_registry.get_mut(agent_id) {
                            gov.set_confidence(confidence);
                        }
                    }
                }

                Opcode::GovernSetCycleDepth => {
                    let depth = bytecode.read_u8(&mut pc)? as u32;
                    if let Some(agent_id) = self.current_agent {
                        if let Some(gov) = self.governance_registry.get_mut(agent_id) {
                            gov.set_cycle_depth(depth);
                        }
                    }
                }

                Opcode::GovernAdvanceStep => {
                    if let Some(agent_id) = self.current_agent {
                        if let Some(gov) = self.governance_registry.get_mut(agent_id) {
                            gov.advance_step();
                        }
                    }
                }

                Opcode::EffectCreate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let effect_type_raw = bytecode.read_u8(&mut pc)?;
                    let effect_type = EffectType::from_u8(effect_type_raw)
                        .ok_or_else(|| RuntimeError::new("Invalid effect type", pc))?;
                    let desc_idx = bytecode.read_u32(&mut pc)? as usize;
                    let description = bytecode.strings.get(desc_idx).cloned().unwrap_or_default();

                    let effect = Effect::new(effect_type, &description);
                    let map = std::collections::BTreeMap::from([
                        ("type".to_string(), Value::I64(effect_type_raw as i64)),
                        ("description".to_string(), Value::String(description)),
                        ("severity".to_string(), Value::F64(effect.severity)),
                        ("reversible".to_string(), Value::Bool(effect.reversible)),
                    ]);
                    self.set_register(dst, Value::Map(map));
                }

                Opcode::TensorCreate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let rank = bytecode.read_u8(&mut pc)? as usize;
                    let mut shape = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        let dim = bytecode.read_u32(&mut pc)? as usize;
                        shape.push(dim);
                    }
                    let tensor = Tensor::zeros(shape);
                    self.set_register(dst, Value::Tensor(tensor));
                }

                Opcode::TensorFromData => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let data_src = bytecode.read_u8(&mut pc)? as usize;
                    let shape_src = bytecode.read_u8(&mut pc)? as usize;

                    let data = match &self.registers[data_src] {
                        Value::Array(arr) => arr.iter().filter_map(|v| v.as_f64()).collect(),
                        _ => {
                            return Err(RuntimeError::new(
                                "TensorFromData requires array of f64",
                                pc,
                            ))
                        }
                    };
                    let shape = match &self.registers[shape_src] {
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|v| v.as_i64().map(|n| n as usize))
                            .collect(),
                        _ => {
                            return Err(RuntimeError::new(
                                "TensorFromData requires shape array",
                                pc,
                            ))
                        }
                    };

                    match Tensor::from_data(shape, data) {
                        Ok(tensor) => self.set_register(dst, Value::Tensor(tensor)),
                        Err(e) => return Err(e),
                    }
                }

                Opcode::TensorGet => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let idx_src = bytecode.read_u8(&mut pc)? as usize;

                    let indices: Vec<usize> = match &self.registers[idx_src] {
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|v| v.as_i64().map(|n| n as usize))
                            .collect(),
                        _ => return Err(RuntimeError::new("TensorGet requires index array", pc)),
                    };

                    match &self.registers[src] {
                        Value::Tensor(t) => match t.get(&indices) {
                            Ok(val) => self.set_register(dst, Value::F64(val)),
                            Err(e) => return Err(e),
                        },
                        _ => return Err(RuntimeError::new("TensorGet requires tensor", pc)),
                    }
                }

                Opcode::TensorSet => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let idx_src = bytecode.read_u8(&mut pc)? as usize;
                    let val_src = bytecode.read_u8(&mut pc)? as usize;

                    let indices: Vec<usize> = match &self.registers[idx_src] {
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|v| v.as_i64().map(|n| n as usize))
                            .collect(),
                        _ => return Err(RuntimeError::new("TensorSet requires index array", pc)),
                    };
                    let value = match &self.registers[val_src] {
                        Value::F64(v) => *v,
                        Value::I64(v) => *v as f64,
                        _ => return Err(RuntimeError::new("TensorSet requires numeric value", pc)),
                    };

                    match &mut self.registers[dst] {
                        Value::Tensor(t) => match t.set(&indices, value) {
                            Ok(()) => {}
                            Err(e) => return Err(e),
                        },
                        _ => return Err(RuntimeError::new("TensorSet requires tensor", pc)),
                    }
                }

                Opcode::TensorAdd => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;

                    let tensor_a = match &self.registers[a] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorAdd requires tensor", pc)),
                    };
                    let tensor_b = match &self.registers[b] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorAdd requires tensor", pc)),
                    };

                    match tensor_a.add(&tensor_b) {
                        Ok(result) => self.set_register(dst, Value::Tensor(result)),
                        Err(e) => return Err(e),
                    }
                }

                Opcode::TensorMul => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;

                    let tensor_a = match &self.registers[a] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorMul requires tensor", pc)),
                    };
                    let tensor_b = match &self.registers[b] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorMul requires tensor", pc)),
                    };

                    match tensor_a.mul(&tensor_b) {
                        Ok(result) => self.set_register(dst, Value::Tensor(result)),
                        Err(e) => return Err(e),
                    }
                }

                Opcode::TensorMatmul => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let a = bytecode.read_u8(&mut pc)? as usize;
                    let b = bytecode.read_u8(&mut pc)? as usize;

                    let tensor_a = match &self.registers[a] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorMatmul requires tensor", pc)),
                    };
                    let tensor_b = match &self.registers[b] {
                        Value::Tensor(t) => t.clone(),
                        _ => return Err(RuntimeError::new("TensorMatmul requires tensor", pc)),
                    };

                    match tensor_a.matmul(&tensor_b) {
                        Ok(result) => self.set_register(dst, Value::Tensor(result)),
                        Err(e) => return Err(e),
                    }
                }

                Opcode::TensorReshape => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;
                    let shape_src = bytecode.read_u8(&mut pc)? as usize;

                    let new_shape: Vec<usize> = match &self.registers[shape_src] {
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|v| v.as_i64().map(|n| n as usize))
                            .collect(),
                        _ => {
                            return Err(RuntimeError::new("TensorReshape requires shape array", pc))
                        }
                    };

                    match &self.registers[src] {
                        Value::Tensor(t) => match t.reshape(new_shape) {
                            Ok(result) => self.set_register(dst, Value::Tensor(result)),
                            Err(e) => return Err(e),
                        },
                        _ => return Err(RuntimeError::new("TensorReshape requires tensor", pc)),
                    }
                }

                Opcode::TensorSoftmax => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;

                    match &self.registers[src] {
                        Value::Tensor(t) => match t.softmax() {
                            Ok(result) => self.set_register(dst, Value::Tensor(result)),
                            Err(e) => return Err(e),
                        },
                        _ => return Err(RuntimeError::new("TensorSoftmax requires tensor", pc)),
                    }
                }

                Opcode::TensorRelu => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let src = bytecode.read_u8(&mut pc)? as usize;

                    match &self.registers[src] {
                        Value::Tensor(t) => {
                            let result = t.relu();
                            self.set_register(dst, Value::Tensor(result));
                        }
                        _ => return Err(RuntimeError::new("TensorRelu requires tensor", pc)),
                    }
                }

                Opcode::RSIPropose => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let mod_type_raw = bytecode.read_u8(&mut pc)?;
                    let confidence_raw = bytecode.read_u8(&mut pc)?;
                    let confidence = confidence_raw as f64 / 100.0;

                    let modification = match mod_type_raw {
                        0 => {
                            let name_idx = bytecode.read_u32(&mut pc)? as usize;
                            let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                            let old_val = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            let new_val = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            ModificationType::ParameterUpdate {
                                name,
                                old_value: old_val,
                                new_value: new_val,
                            }
                        }
                        1 => {
                            let h = bytecode.read_u8(&mut pc)? as u32;
                            let l = bytecode.read_u8(&mut pc)? as u32;
                            ModificationType::CycleConfigChange {
                                h_cycles: h,
                                l_cycles: l,
                            }
                        }
                        2 => {
                            // BehaviorAdd: pattern_len (u8), pattern data, response_len (u8), response data
                            let pattern_len = bytecode.read_u8(&mut pc)? as usize;
                            let mut pattern = Vec::with_capacity(pattern_len);
                            for _ in 0..pattern_len {
                                let val = bytecode.read_f64(&mut pc)?;
                                pattern.push(val);
                            }
                            let response_len = bytecode.read_u8(&mut pc)? as usize;
                            let mut response = Vec::with_capacity(response_len);
                            for _ in 0..response_len {
                                let val = bytecode.read_f64(&mut pc)?;
                                response.push(val);
                            }
                            ModificationType::BehaviorAdd { pattern, response }
                        }
                        3 => {
                            // BehaviorRemove: index (u32)
                            let idx = bytecode.read_u32(&mut pc)? as usize;
                            ModificationType::BehaviorRemove { index: idx }
                        }
                        4 => {
                            // ThresholdChange: name_idx (u32), old_val (u8), new_val (u8)
                            let name_idx = bytecode.read_u32(&mut pc)? as usize;
                            let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                            let old_val = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            let new_val = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            ModificationType::ThresholdChange {
                                name,
                                old_value: old_val,
                                new_value: new_val,
                            }
                        }
                        5 => {
                            // WeightMatrixUpdate: layer (u32), delta_len (u8), delta data
                            let layer = bytecode.read_u32(&mut pc)? as usize;
                            let delta_len = bytecode.read_u8(&mut pc)? as usize;
                            let mut delta = Vec::with_capacity(delta_len);
                            for _ in 0..delta_len {
                                let val = bytecode.read_f64(&mut pc)?;
                                delta.push(val);
                            }
                            ModificationType::WeightMatrixUpdate { layer, delta }
                        }
                        6 => {
                            // RuleAdd: name_idx (u32), desc_idx (u32), confidence (u8)
                            let name_idx = bytecode.read_u32(&mut pc)? as usize;
                            let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                            let desc_idx = bytecode.read_u32(&mut pc)? as usize;
                            let description =
                                bytecode.strings.get(desc_idx).cloned().unwrap_or_default();
                            let conf = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            ModificationType::RuleAdd {
                                name,
                                description,
                                confidence: conf,
                            }
                        }
                        7 => {
                            // RuleRemove: name_idx (u32)
                            let name_idx = bytecode.read_u32(&mut pc)? as usize;
                            let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                            ModificationType::RuleRemove { name }
                        }
                        8 => {
                            // RuleUpdate: name_idx (u32), desc_idx (u32), confidence (u8)
                            let name_idx = bytecode.read_u32(&mut pc)? as usize;
                            let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                            let desc_idx = bytecode.read_u32(&mut pc)? as usize;
                            let description =
                                bytecode.strings.get(desc_idx).cloned().unwrap_or_default();
                            let conf = bytecode.read_u8(&mut pc)? as f64 / 100.0;
                            ModificationType::RuleUpdate {
                                name,
                                description,
                                confidence: conf,
                            }
                        }
                        _ => return Err(RuntimeError::new("Unknown modification type", pc)),
                    };

                    let agent_id = self.current_agent.unwrap_or(0);
                    match self
                        .rsi_pipeline
                        .create_proposal(agent_id, modification, confidence)
                    {
                        Ok(id) => self.set_register(dst, Value::I64(id as i64)),
                        Err(e) => return Err(e),
                    }
                }

                Opcode::RSIVote => {
                    let proposal_id = bytecode.read_u32(&mut pc)? as u64;
                    let approve_raw = bytecode.read_u8(&mut pc)?;
                    let approve = approve_raw != 0;
                    let agent_id = self.current_agent.unwrap_or(0);
                    self.rsi_pipeline.vote(proposal_id, agent_id, approve)?;
                }

                Opcode::RSIValidate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let proposal_id = bytecode.read_u32(&mut pc)? as u64;

                    if let Some(agent_id) = self.current_agent {
                        if let Some(gov) = self.governance_registry.get_mut(agent_id) {
                            let valid = self.rsi_pipeline.validate_proposal(proposal_id, gov)?;
                            self.set_register(dst, Value::Bool(valid));
                        }
                    }
                }

                Opcode::RSIApply => {
                    let proposal_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = self.current_agent.unwrap_or(0);

                    let memory = self
                        .agent_memories
                        .entry(agent_id)
                        .or_insert_with(AgentMemory::new);
                    self.rsi_pipeline.apply_proposal(proposal_id, memory)?;

                    // Phase 4.4: Auto-promotion after successful RSIApply
                    // Check if promotion criteria are met
                    if let Some(new_level) = self.rsi_pipeline.check_promotion() {
                        // Promotion occurred - log it (could emit an event in the future)
                        let _ = new_level; // Use the promotion level if needed
                    }
                }

                Opcode::RSIRollback => {
                    let proposal_id = bytecode.read_u32(&mut pc)? as u64;
                    let agent_id = self.current_agent.unwrap_or(0);

                    if let Some(memory) = self.agent_memories.get_mut(&agent_id) {
                        self.rsi_pipeline.rollback_proposal(proposal_id, memory)?;
                    }
                }

                Opcode::RSIGetStatus => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let proposal_id = bytecode.read_u32(&mut pc)? as u64;

                    if let Some(proposal) = self.rsi_pipeline.get_proposal(proposal_id) {
                        let status = match proposal.status {
                            crate::rsi::ProposalStatus::Pending => 0,
                            crate::rsi::ProposalStatus::Validating => 1,
                            crate::rsi::ProposalStatus::Approved => 2,
                            crate::rsi::ProposalStatus::Rejected => 3,
                            crate::rsi::ProposalStatus::Applied => 4,
                            crate::rsi::ProposalStatus::RolledBack => 5,
                        };
                        self.set_register(dst, Value::I64(status));
                    }
                }

                Opcode::MemoryGet => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();

                    let agent_id = self.current_agent.unwrap_or(0);
                    if let Some(memory) = self.agent_memories.get(&agent_id) {
                        let val = memory.parameters.get(&name).copied().unwrap_or(0.0);
                        self.set_register(dst, Value::F64(val));
                    }
                }

                Opcode::MemorySet => {
                    let name_idx = bytecode.read_u32(&mut pc)? as usize;
                    let name = bytecode.strings.get(name_idx).cloned().unwrap_or_default();
                    let val_src = bytecode.read_u8(&mut pc)? as usize;
                    let val = match &self.registers[val_src] {
                        Value::F64(v) => *v,
                        Value::I64(v) => *v as f64,
                        _ => return Err(RuntimeError::new("MemorySet requires numeric value", pc)),
                    };

                    let agent_id = self.current_agent.unwrap_or(0);
                    let memory = self
                        .agent_memories
                        .entry(agent_id)
                        .or_insert_with(AgentMemory::new);
                    memory.parameters.insert(name, val);
                }

                Opcode::MemoryAddBehavior => {
                    let pattern_src = bytecode.read_u8(&mut pc)? as usize;
                    let response_src = bytecode.read_u8(&mut pc)? as usize;

                    let pattern: Vec<f64> = match &self.registers[pattern_src] {
                        Value::Array(arr) => arr.iter().filter_map(|v| v.as_f64()).collect(),
                        _ => {
                            return Err(RuntimeError::new(
                                "MemoryAddBehavior requires pattern array",
                                pc,
                            ))
                        }
                    };
                    let response: Vec<f64> = match &self.registers[response_src] {
                        Value::Array(arr) => arr.iter().filter_map(|v| v.as_f64()).collect(),
                        _ => {
                            return Err(RuntimeError::new(
                                "MemoryAddBehavior requires response array",
                                pc,
                            ))
                        }
                    };

                    let agent_id = self.current_agent.unwrap_or(0);
                    let memory = self
                        .agent_memories
                        .entry(agent_id)
                        .or_insert_with(AgentMemory::new);
                    memory.behaviors.push((pattern, response));
                }

                Opcode::MemoryAddWeight => {
                    let tensor_src = bytecode.read_u8(&mut pc)? as usize;

                    match &self.registers[tensor_src] {
                        Value::Tensor(t) => {
                            let agent_id = self.current_agent.unwrap_or(0);
                            let memory = self
                                .agent_memories
                                .entry(agent_id)
                                .or_insert_with(AgentMemory::new);
                            memory.weight_matrices.push(t.clone());
                        }
                        _ => return Err(RuntimeError::new("MemoryAddWeight requires tensor", pc)),
                    }
                }

                Opcode::MapCreate => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    self.set_register(dst, Value::Map(std::collections::BTreeMap::new()));
                }

                Opcode::MapSet => {
                    let dst = bytecode.read_u8(&mut pc)? as usize;
                    let key_reg = bytecode.read_u8(&mut pc)? as usize;
                    let val_reg = bytecode.read_u8(&mut pc)? as usize;
                    let key = match self.get_register_cloned(key_reg) {
                        Value::String(s) => s,
                        other => format!("{:?}", other),
                    };
                    let val = self.get_register_cloned(val_reg);
                    if let Value::Map(ref mut map) = self.registers[dst] {
                        map.insert(key, val);
                    }
                }

                Opcode::Call => {
                    let func_idx = bytecode.read_u32(&mut pc)? as usize;
                    let arg_count = bytecode.read_u8(&mut pc)? as usize;
                    let dst = bytecode.read_u8(&mut pc)? as usize;

                    if let Some(func_name) = bytecode.strings.get(func_idx) {
                        if let Some(&(start_pc, param_count)) = self.functions.get(func_name) {
                            let saved_regs: Vec<Value> =
                                self.registers[..self.config.saved_register_count].to_vec();

                            let arg_base = self.config.arg_base_register;
                            let max_args = arg_count.min(param_count);
                            if arg_base + max_args > self.registers.len() {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Function call arg_base ({}) + arg_count ({}) exceeds register limit ({})",
                                        arg_base, max_args, self.registers.len()
                                    ),
                                    pc,
                                ));
                            }
                            for i in 0..max_args {
                                self.registers[i + 1] = self.registers[arg_base + i].clone();
                            }

                            self.call_stack.push(CallFrame {
                                return_pc: pc,
                                base_reg: dst,
                                arg_count,
                                saved_regs,
                            });
                            pc = start_pc;
                        } else if self.natives.contains_key(func_name) {
                            // Native function (e.g., bond(), mem_store(), mem_query()) - call with VM access
                            let arg_base = self.config.arg_base_register;
                            if arg_base + arg_count > self.registers.len() {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Native call arg_base ({}) + arg_count ({}) exceeds register limit ({})",
                                        arg_base, arg_count, self.registers.len()
                                    ),
                                    pc,
                                ));
                            }
                            // Collect args first
                            let args: Vec<Value> = (0..arg_count)
                                .map(|i| self.registers[arg_base + i].clone())
                                .collect();
                            // Take ownership of the native function to avoid borrow issues
                            let func_name_owned = func_name.clone();
                            if let Some(native) = self.natives.remove(&func_name_owned) {
                                let result = native(self, args);
                                self.registers[dst] = result;
                                // Re-insert the native function for future calls
                                self.natives.insert(func_name_owned, native);
                            }
                        } else {
                            let arg_base = self.config.arg_base_register;
                            if arg_base + arg_count > self.registers.len() {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Builtin call arg_base ({}) + arg_count ({}) exceeds register limit ({})",
                                        arg_base, arg_count, self.registers.len()
                                    ),
                                    pc,
                                ));
                            }
                            let args: Vec<Value> = (0..arg_count)
                                .map(|i| self.registers[arg_base + i].clone())
                                .collect();
                            let result = self.call_builtin_by_name(func_name, &args)?;
                            self.registers[dst] = result;
                        }
                    }
                }

                Opcode::CallAddr => {
                    // Call by direct PC address (used for cross-module imports)
                    // Read 32-bit target PC (matches the u32 written by patch_forward_calls)
                    let target_pc = bytecode.read_u32(&mut pc)? as usize;
                    let arg_count = bytecode.read_u8(&mut pc)? as usize;
                    let dst = bytecode.read_u8(&mut pc)? as usize;

                    // Find the function at this PC to get param_count
                    let mut param_count = 0;
                    for (_, &(start_pc, params)) in &self.functions {
                        if start_pc == target_pc as usize {
                            param_count = params as usize;
                            break;
                        }
                    }

                    let saved_regs: Vec<Value> =
                        self.registers[..self.config.saved_register_count].to_vec();

                    let arg_base = self.config.arg_base_register;
                    let max_args = arg_count.min(param_count);
                    if arg_base + max_args > self.registers.len() {
                        return Err(RuntimeError::new(
                            format!(
                                "CallAddr arg_base ({}) + arg_count ({}) exceeds register limit ({})",
                                arg_base, max_args, self.registers.len()
                            ),
                            pc,
                        ));
                    }
                    for i in 0..max_args {
                        self.registers[i + 1] = self.registers[arg_base + i].clone();
                    }

                    self.call_stack.push(CallFrame {
                        return_pc: pc,
                        base_reg: dst,
                        arg_count,
                        saved_regs,
                    });
                    pc = target_pc as usize;
                }
            }
        }

        Ok(self.registers[0].clone())
    }

    fn binary_add(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::I64(x + y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::F64(x + y)),
            (Value::String(x), Value::String(y)) => Ok(Value::String(format!("{}{}", x, y))),
            _ => Err(RuntimeError::new(
                format!("Cannot add {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn binary_sub(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::I64(x - y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::F64(x - y)),
            _ => Err(RuntimeError::new(
                format!("Cannot subtract {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn binary_mul(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::I64(x * y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::F64(x * y)),
            _ => Err(RuntimeError::new(
                format!("Cannot multiply {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn binary_div(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => {
                if *y == 0 {
                    return Err(RuntimeError::new("Division by zero", 0));
                }
                Ok(Value::I64(x / y))
            }
            (Value::F64(x), Value::F64(y)) => Ok(Value::F64(x / y)),
            _ => Err(RuntimeError::new(
                format!("Cannot divide {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn binary_mod(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => {
                if *y == 0 {
                    return Err(RuntimeError::new("Modulo by zero", 0));
                }
                Ok(Value::I64(x % y))
            }
            _ => Err(RuntimeError::new(
                format!("Cannot modulo {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn compare_lt(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::Bool(x < y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::Bool(x < y)),
            (Value::String(x), Value::String(y)) => Ok(Value::Bool(x < y)),
            _ => Err(RuntimeError::new(
                format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn compare_le(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::Bool(x <= y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::Bool(x <= y)),
            (Value::String(x), Value::String(y)) => Ok(Value::Bool(x <= y)),
            _ => Err(RuntimeError::new(
                format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn compare_gt(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::Bool(x > y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::Bool(x > y)),
            (Value::String(x), Value::String(y)) => Ok(Value::Bool(x > y)),
            _ => Err(RuntimeError::new(
                format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    fn compare_ge(&self, a: &Value, b: &Value) -> RuntimeResult<Value> {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => Ok(Value::Bool(x >= y)),
            (Value::F64(x), Value::F64(y)) => Ok(Value::Bool(x >= y)),
            (Value::String(x), Value::String(y)) => Ok(Value::Bool(x >= y)),
            _ => Err(RuntimeError::new(
                format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
                0,
            )),
        }
    }

    pub fn register_function(&mut self, name: &str, start_pc: usize, params: usize) {
        self.functions.insert(name.to_string(), (start_pc, params));
    }

    fn call_builtin_by_name(&mut self, name: &str, args: &[Value]) -> RuntimeResult<Value> {
        match name {
            "strlen" => builtins::builtin_strlen(args),
            "substring" => builtins::builtin_substring(args),
            "concat" => builtins::builtin_concat(args),
            "strcmp" => builtins::builtin_strcmp(args),
            "ord" => builtins::builtin_ord(args),
            "char" => builtins::builtin_char(args),
            "push" => builtins::builtin_push(args),
            "get_at" => builtins::builtin_get_at(args),
            "set_at" => builtins::builtin_set_at(args),
            "array_len" | "len" => builtins::builtin_array_len(args),
            "print" => builtins::builtin_print(args),
            "println" => builtins::builtin_println(args),
            "image_load" => builtins::builtin_image_load(args),
            "image_save" => builtins::builtin_image_save(args),
            "image_process" => builtins::builtin_image_process(args),
            "image_info" => builtins::builtin_image_info(args),
            "audio_load" => builtins::builtin_audio_load(args),
            "audio_save" => builtins::builtin_audio_save(args),
            "audio_info" => builtins::builtin_audio_info(args),
            "audio_resample" => builtins::builtin_audio_resample(args),
            "audio_normalize" => builtins::builtin_audio_normalize(args),
            // Bit's builtins (Phase 2)
            "zeros" => builtins::builtin_zeros(args),
            "i64_to_str" => builtins::builtin_i64_to_str(args),
            "f64_to_str" => builtins::builtin_f64_to_str(args),
            "str_contains" => builtins::builtin_str_contains(args),
            "str_equals" => builtins::builtin_str_equals(args),
            "sqrt" => builtins::builtin_sqrt(args),
            "hash" => builtins::builtin_hash(args),
            // Phase 1.4: Math builtins
            "abs" => builtins::builtin_abs(args),
            "floor" => builtins::builtin_floor(args),
            "ceil" => builtins::builtin_ceil(args),
            "round" => builtins::builtin_round(args),
            "min" => builtins::builtin_min(args),
            "max" => builtins::builtin_max(args),
            "pow" => builtins::builtin_pow(args),
            "rand" => builtins::builtin_rand(args),
            "rand_range" => builtins::builtin_rand_range(args),
            // Phase 1.4: Type conversion builtins
            "f64_to_i64" => builtins::builtin_f64_to_i64(args),
            "i64_to_f64" => builtins::builtin_i64_to_f64(args),
            "parse_i64" => builtins::builtin_parse_i64(args),
            "parse_f64" => builtins::builtin_parse_f64(args),
            "type_of" => builtins::builtin_type_of(args),
            // Phase 1.4: String builtins
            "str_split" => builtins::builtin_str_split(args),
            "str_trim" => builtins::builtin_str_trim(args),
            "str_replace" => builtins::builtin_str_replace(args),
            "str_to_upper" => builtins::builtin_str_to_upper(args),
            "str_to_lower" => builtins::builtin_str_to_lower(args),
            "str_starts_with" => builtins::builtin_str_starts_with(args),
            "str_ends_with" => builtins::builtin_str_ends_with(args),
            "str_index_of" => builtins::builtin_str_index_of(args),
            // Phase 1.4: Array builtins
            "array_slice" => builtins::builtin_array_slice(args),
            "array_concat" => builtins::builtin_array_concat(args),
            "array_contains" => builtins::builtin_array_contains(args),
            "array_pop" => builtins::builtin_array_pop(args),
            "array_reverse" => builtins::builtin_array_reverse(args),
            "array_sort" => builtins::builtin_array_sort(args),
            // Phase 1.4: Map builtins
            "map_get" => builtins::builtin_map_get(args),
            "map_set" => builtins::builtin_map_set(args),
            "map_keys" => builtins::builtin_map_keys(args),
            "map_values" => builtins::builtin_map_values(args),
            "map_contains" => builtins::builtin_map_contains(args),
            "map_remove" => builtins::builtin_map_remove(args),
            // Phase 1.4: I/O builtins
            "read_file" => builtins::builtin_read_file(args),
            "write_file" => builtins::builtin_write_file(args),
            "clock_ms" => builtins::builtin_clock_ms(args),
            // Phase 4.3: Homeostasis and promotion builtins
            "homeostasis_pressure" => builtins::builtin_homeostasis_pressure(args),
            "promotion_level" => builtins::builtin_promotion_level(args),
            "can_modify_self" => builtins::builtin_can_modify_self(args),
            "rsi_history" => builtins::builtin_rsi_history(args),
            // Phase 4.5: Fitness evaluation hooks
            "evaluate_fitness" => builtins::builtin_evaluate_fitness(args),
            "fitness_snapshot" => builtins::builtin_fitness_snapshot(args),
            "fitness_compare" => builtins::builtin_fitness_compare(args),
            // Phase 5.3: Bond builtin
            "bond" => builtins::builtin_bond(args),
            _ => Err(RuntimeError::new(format!("Unknown function: {}", name), 0)),
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let mut bc = Bytecode::new();
        let idx = bc.add_constant(Value::I64(42));

        bc.emit(Opcode::Const);
        bc.emit_u8(0);
        bc.emit_u32(idx);
        bc.emit(Opcode::Halt);

        let mut vm = Vm::new();
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_addition() {
        let mut bc = Bytecode::new();
        let a = bc.add_constant(Value::I64(10));
        let b = bc.add_constant(Value::I64(32));

        bc.emit(Opcode::Const);
        bc.emit_u8(1);
        bc.emit_u32(a);

        bc.emit(Opcode::Const);
        bc.emit_u8(2);
        bc.emit_u32(b);

        bc.emit(Opcode::Add);
        bc.emit_u8(0);
        bc.emit_u8(1);
        bc.emit_u8(2);

        bc.emit(Opcode::Halt);

        let mut vm = Vm::new();
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_agent_spawn_rate_limit() {
        let mut bc = Bytecode::new();

        for _ in 0..15 {
            let name = bc.add_string("TestAgent".to_string());
            bc.emit(Opcode::AgentSpawn);
            bc.emit_u32(name);
            bc.emit_u32(0);
        }
        bc.emit(Opcode::Halt);

        let mut vm = Vm::new().with_spawn_rate_limit(10, 60);

        let result = vm.run(&bc);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("rate limit"));
    }

    #[test]
    fn test_max_agent_count() {
        let mut bc = Bytecode::new();

        for _ in 0..15 {
            let name = bc.add_string("TestAgent".to_string());
            bc.emit(Opcode::AgentSpawn);
            bc.emit_u32(name);
            bc.emit_u32(0);
        }
        bc.emit(Opcode::Halt);

        let mut vm = Vm::new().with_max_agents(10);

        let result = vm.run(&bc);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Maximum agent count"));
    }
}
