use crate::{Value, Vm};
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Debug events sent from VM to debugger
#[derive(Debug, Clone)]
pub enum DebugEvent {
    Stopped {
        reason: StopReason,
        pc: u32,
        line: u32,
        col: u32,
    },
    VariableState {
        registers: Vec<(String, Value)>,
        latents: Vec<(String, Value)>,
    },
    RSIProposal {
        proposal_id: u64,
        mod_type: String,
        target: String,
    },
    GateCheck {
        gate: String,
        passed: bool,
    },
    Promotion {
        from: String,
        to: String,
    },
}

/// Reasons why execution stopped
#[derive(Debug, Clone)]
pub enum StopReason {
    Breakpoint,
    Step,
    Pause,
    Exception(String),
}

/// Step mode for the debugger
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepMode {
    Run,      // Continue execution
    StepOver, // Step over function calls
    StepIn,   // Step into function calls
    StepOut,  // Step out of current function
}

/// Debugger state attached to VM
#[derive(Debug)]
pub struct Debugger {
    pub breakpoints: HashSet<u32>, // Instruction offsets
    pub debug_mode: bool,
    pub step_mode: StepMode,
    pub debug_tx: Option<Sender<DebugEvent>>,
    pub call_stack_depth: u32,
    pub target_stack_depth: Option<u32>, // For step out
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            breakpoints: HashSet::new(),
            debug_mode: false,
            step_mode: StepMode::Run,
            debug_tx: None,
            call_stack_depth: 0,
            target_stack_depth: None,
        }
    }

    pub fn enable(&mut self) {
        self.debug_mode = true;
    }

    pub fn disable(&mut self) {
        self.debug_mode = false;
    }

    pub fn set_breakpoint(&mut self, offset: u32) {
        self.breakpoints.insert(offset);
    }

    pub fn clear_breakpoint(&mut self, offset: u32) {
        self.breakpoints.remove(&offset);
    }

    pub fn clear_all_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn step_over(&mut self) {
        self.step_mode = StepMode::StepOver;
        self.target_stack_depth = None;
    }

    pub fn step_in(&mut self) {
        self.step_mode = StepMode::StepIn;
        self.target_stack_depth = None;
    }

    pub fn step_out(&mut self) {
        self.step_mode = StepMode::StepOut;
        self.target_stack_depth = Some(self.call_stack_depth.saturating_sub(1));
    }

    pub fn continue_execution(&mut self) {
        self.step_mode = StepMode::Run;
        self.target_stack_depth = None;
    }

    /// Check if we should stop at this instruction
    pub fn should_stop(&self, pc: u32) -> Option<StopReason> {
        if !self.debug_mode {
            return None;
        }

        // Check breakpoints
        if self.breakpoints.contains(&pc) && self.step_mode == StepMode::Run {
            return Some(StopReason::Breakpoint);
        }

        // Check step modes
        match self.step_mode {
            StepMode::StepIn => return Some(StopReason::Step),
            StepMode::StepOver if self.call_stack_depth == 0 => return Some(StopReason::Step),
            StepMode::StepOut => {
                if let Some(target) = self.target_stack_depth {
                    if self.call_stack_depth <= target {
                        return Some(StopReason::Step);
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Send a debug event
    pub fn send_event(&self, event: DebugEvent) {
        if let Some(ref tx) = self.debug_tx {
            let _ = tx.send(event);
        }
    }

    /// Called when entering a function
    pub fn enter_function(&mut self) {
        self.call_stack_depth += 1;
    }

    /// Called when exiting a function
    pub fn exit_function(&mut self) {
        self.call_stack_depth = self.call_stack_depth.saturating_sub(1);
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

/// DAP (Debug Adapter Protocol) server
pub struct DapServer {
    debugger: Debugger,
    #[allow(dead_code)]
    vm: Option<Vm>,
    event_rx: Option<Receiver<DebugEvent>>,
}

impl DapServer {
    pub fn new() -> Self {
        Self {
            debugger: Debugger::new(),
            vm: None,
            event_rx: None,
        }
    }

    pub fn attach_vm(&mut self, _vm: &mut Vm) {
        let (tx, rx) = channel();
        self.debugger.debug_tx = Some(tx);
        self.event_rx = Some(rx);
        // Note: VM needs to be modified to use the debugger
    }

    pub fn set_breakpoints(&mut self, offsets: Vec<u32>) {
        self.debugger.clear_all_breakpoints();
        for offset in offsets {
            self.debugger.set_breakpoint(offset);
        }
    }

    pub fn continue_execution(&mut self) {
        self.debugger.continue_execution();
    }

    pub fn step_over(&mut self) {
        self.debugger.step_over();
    }

    pub fn step_in(&mut self) {
        self.debugger.step_in();
    }

    pub fn step_out(&mut self) {
        self.debugger.step_out();
    }

    pub fn poll_event(&self) -> Option<DebugEvent> {
        self.event_rx.as_ref()?.try_recv().ok()
    }
}

impl Default for DapServer {
    fn default() -> Self {
        Self::new()
    }
}
