//! Visitor pattern for AST traversal
//!
//! Enables safe, structured traversal of the AST without exposing internals.
//! Used by linters, type checkers, and RSI diffing.

use super::rsi::{GovernDef, ModifyDef};
use super::{AgentDef, ClusterDef, Expression, Function, Item, Program, Statement};

/// Result of visiting a node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitResult {
    /// Continue visiting children
    Continue,
    /// Skip children of this node
    SkipChildren,
    /// Stop visiting entirely
    Stop,
}

/// Visitor trait for AST traversal
pub trait Visitor {
    /// Called before visiting children
    fn enter_program(&mut self, _prog: &Program) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_item(&mut self, _item: &Item) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_function(&mut self, _func: &Function) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_agent(&mut self, _agent: &AgentDef) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_cluster(&mut self, _cluster: &ClusterDef) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_statement(&mut self, _stmt: &Statement) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_expression(&mut self, _expr: &Expression) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_govern(&mut self, _govern: &GovernDef) -> VisitResult {
        VisitResult::Continue
    }
    fn enter_modify(&mut self, _modify: &ModifyDef) -> VisitResult {
        VisitResult::Continue
    }

    /// Called after visiting children
    fn exit_program(&mut self, _prog: &Program) {}
    fn exit_item(&mut self, _item: &Item) {}
    fn exit_function(&mut self, _func: &Function) {}
    fn exit_agent(&mut self, _agent: &AgentDef) {}
    fn exit_cluster(&mut self, _cluster: &ClusterDef) {}
    fn exit_statement(&mut self, _stmt: &Statement) {}
    fn exit_expression(&mut self, _expr: &Expression) {}
    fn exit_govern(&mut self, _govern: &GovernDef) {}
    fn exit_modify(&mut self, _modify: &ModifyDef) {}
}

/// Walk the AST with a visitor
pub fn walk_program(visitor: &mut impl Visitor, prog: &Program) {
    if visitor.enter_program(prog) == VisitResult::Stop {
        return;
    }

    for item in &prog.items {
        if walk_item(visitor, item) == VisitResult::Stop {
            return;
        }
    }

    visitor.exit_program(prog);
}

pub fn walk_item(visitor: &mut impl Visitor, item: &Item) -> VisitResult {
    if visitor.enter_item(item) == VisitResult::Stop {
        return VisitResult::Stop;
    }

    let result = match item {
        Item::Function(func) => walk_function(visitor, func),
        Item::Agent(agent) => walk_agent(visitor, agent),
        Item::Cluster(cluster) => walk_cluster(visitor, cluster),
        Item::Module(m) => {
            for item in &m.items {
                if walk_item(visitor, item) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        Item::Import(_) | Item::Export(_) | Item::Struct(_) => VisitResult::Continue,
        Item::ExternFunction(func) => walk_function(visitor, func),
        Item::Global(stmt) => walk_statement(visitor, stmt),
    };

    if result == VisitResult::Stop {
        return VisitResult::Stop;
    }

    visitor.exit_item(item);
    VisitResult::Continue
}

pub fn walk_function(visitor: &mut impl Visitor, func: &Function) -> VisitResult {
    if visitor.enter_function(func) == VisitResult::Stop {
        return VisitResult::Stop;
    }

    for stmt in &func.body {
        if walk_statement(visitor, stmt) == VisitResult::Stop {
            return VisitResult::Stop;
        }
    }

    visitor.exit_function(func);
    VisitResult::Continue
}

pub fn walk_agent(visitor: &mut impl Visitor, agent: &AgentDef) -> VisitResult {
    if visitor.enter_agent(agent) == VisitResult::Stop {
        return VisitResult::Stop;
    }

    for cycle in &agent.cycles {
        for stmt in &cycle.body {
            if walk_statement(visitor, stmt) == VisitResult::Stop {
                return VisitResult::Stop;
            }
        }
    }

    for stmt in &agent.body {
        if walk_statement(visitor, stmt) == VisitResult::Stop {
            return VisitResult::Stop;
        }
    }

    if let Some(ref govern) = agent.govern {
        if visitor.enter_govern(govern) == VisitResult::Stop {
            return VisitResult::Stop;
        }
        visitor.exit_govern(govern);
    }

    if let Some(ref modify) = agent.modify {
        if visitor.enter_modify(modify) == VisitResult::Stop {
            return VisitResult::Stop;
        }
        visitor.exit_modify(modify);
    }

    visitor.exit_agent(agent);
    VisitResult::Continue
}

pub fn walk_cluster(visitor: &mut impl Visitor, cluster: &ClusterDef) -> VisitResult {
    if visitor.enter_cluster(cluster) == VisitResult::Stop {
        return VisitResult::Stop;
    }
    visitor.exit_cluster(cluster);
    VisitResult::Continue
}

pub fn walk_statement(visitor: &mut impl Visitor, stmt: &Statement) -> VisitResult {
    if visitor.enter_statement(stmt) == VisitResult::Stop {
        return VisitResult::Stop;
    }

    use super::StmtKind;
    let result = match &stmt.kind {
        StmtKind::Let { value, .. } => {
            if let Some(val) = value {
                walk_expression(visitor, val)
            } else {
                VisitResult::Continue
            }
        }
        StmtKind::Assign { target, value } => {
            if walk_expression(visitor, target) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, value)
        }
        StmtKind::CompoundAssign { target, value, .. } => {
            if walk_expression(visitor, target) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, value)
        }
        StmtKind::Expr(expr) => walk_expression(visitor, expr),
        StmtKind::Return(Some(expr)) => walk_expression(visitor, expr),
        StmtKind::Return(None) => VisitResult::Continue,
        StmtKind::If(if_stmt) => {
            if walk_expression(visitor, &if_stmt.condition) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for s in &if_stmt.then_body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            for s in &if_stmt.else_body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::Loop(loop_stmt) => {
            if walk_expression(visitor, &loop_stmt.condition) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for s in &loop_stmt.body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::While { condition, body } => {
            if walk_expression(visitor, condition) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for s in body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::For { iterable, body, .. } => {
            if walk_expression(visitor, iterable) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for s in body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::Break | StmtKind::Continue => VisitResult::Continue,
        StmtKind::Block(stmts) => {
            for s in stmts {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::Switch(switch) => {
            if walk_expression(visitor, &switch.discriminant) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for case in &switch.cases {
                for s in &case.body {
                    if walk_statement(visitor, s) == VisitResult::Stop {
                        return VisitResult::Stop;
                    }
                }
            }
            for s in &switch.default_body {
                if walk_statement(visitor, s) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::Match(m) => {
            if walk_expression(visitor, &m.subject) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for arm in &m.arms {
                if let Some(guard) = &arm.guard {
                    if walk_expression(visitor, guard) == VisitResult::Stop {
                        return VisitResult::Stop;
                    }
                }
                for s in &arm.body {
                    if walk_statement(visitor, s) == VisitResult::Stop {
                        return VisitResult::Stop;
                    }
                }
            }
            VisitResult::Continue
        }
        StmtKind::Module(m) => {
            for item in &m.items {
                if walk_item(visitor, item) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        StmtKind::Import(_) | StmtKind::Export(_) => VisitResult::Continue,
        StmtKind::Migrate { .. } => VisitResult::Continue,
    };

    if result == VisitResult::Stop {
        return VisitResult::Stop;
    }

    visitor.exit_statement(stmt);
    VisitResult::Continue
}

pub fn walk_expression(visitor: &mut impl Visitor, expr: &Expression) -> VisitResult {
    if visitor.enter_expression(expr) == VisitResult::Stop {
        return VisitResult::Stop;
    }

    use super::ExprKind;
    let result = match &expr.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Identifier(_)
        | ExprKind::Nil
        | ExprKind::Void => VisitResult::Continue,

        ExprKind::BinaryOp { left, right, .. } => {
            if walk_expression(visitor, left) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, right)
        }
        ExprKind::UnaryOp { operand, .. } => walk_expression(visitor, operand),
        ExprKind::Call { arguments, .. } => {
            for arg in arguments {
                if walk_expression(visitor, arg) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::Array(elements) => {
            for elem in elements {
                if walk_expression(visitor, elem) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::Index { array, index } => {
            if walk_expression(visitor, array) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, index)
        }
        ExprKind::Range { start, end, .. } => {
            if walk_expression(visitor, start) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, end)
        }
        ExprKind::Dict(pairs) => {
            for (key, val) in pairs {
                if walk_expression(visitor, key) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
                if walk_expression(visitor, val) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::Contract { fields, .. } => {
            for (_, value) in fields {
                if walk_expression(visitor, value) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::FieldAccess { object, .. } => walk_expression(visitor, object),
        ExprKind::MethodCall {
            object, arguments, ..
        } => {
            if walk_expression(visitor, object) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for arg in arguments {
                if walk_expression(visitor, arg) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::Lambda { body, .. } => walk_expression(visitor, body),
        ExprKind::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            if walk_expression(visitor, condition) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            if walk_expression(visitor, then_expr) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            walk_expression(visitor, else_expr)
        }
        ExprKind::Match { value, cases } => {
            if walk_expression(visitor, value) == VisitResult::Stop {
                return VisitResult::Stop;
            }
            for case in cases {
                if walk_expression(visitor, &case.body) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
        ExprKind::Collapse(inner) | ExprKind::Resolve(inner) => walk_expression(visitor, inner),
        ExprKind::Cast { expr, .. } => walk_expression(visitor, expr),
        ExprKind::Do { fields, .. } => {
            for (_, value) in fields {
                if walk_expression(visitor, value) == VisitResult::Stop {
                    return VisitResult::Stop;
                }
            }
            VisitResult::Continue
        }
    };

    if result == VisitResult::Stop {
        return VisitResult::Stop;
    }

    visitor.exit_expression(expr);
    VisitResult::Continue
}

/// Collect all expressions matching a predicate
#[allow(dead_code)]
pub struct ExpressionCollector<F> {
    predicate: F,
    collected: Vec<Expression>,
}

#[allow(dead_code)]
impl<F> ExpressionCollector<F>
where
    F: Fn(&Expression) -> bool,
{
    pub fn new(predicate: F) -> Self {
        ExpressionCollector {
            predicate,
            collected: Vec::new(),
        }
    }

    pub fn collect(mut self, prog: &Program) -> Vec<Expression> {
        walk_program(&mut self, prog);
        self.collected
    }
}

impl<F> Visitor for ExpressionCollector<F>
where
    F: Fn(&Expression) -> bool,
{
    fn enter_expression(&mut self, expr: &Expression) -> VisitResult {
        if (self.predicate)(expr) {
            self.collected.push(expr.clone());
        }
        VisitResult::Continue
    }
}

/// Count nodes in an AST
pub struct NodeCounter {
    pub programs: usize,
    pub functions: usize,
    pub agents: usize,
    pub statements: usize,
    pub expressions: usize,
}

impl NodeCounter {
    pub fn new() -> Self {
        NodeCounter {
            programs: 0,
            functions: 0,
            agents: 0,
            statements: 0,
            expressions: 0,
        }
    }

    pub fn count(mut self, prog: &Program) -> Self {
        walk_program(&mut self, prog);
        self
    }
}

impl Default for NodeCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for NodeCounter {
    fn enter_program(&mut self, _prog: &Program) -> VisitResult {
        self.programs += 1;
        VisitResult::Continue
    }

    fn enter_function(&mut self, _func: &Function) -> VisitResult {
        self.functions += 1;
        VisitResult::Continue
    }

    fn enter_agent(&mut self, _agent: &AgentDef) -> VisitResult {
        self.agents += 1;
        VisitResult::Continue
    }

    fn enter_statement(&mut self, _stmt: &Statement) -> VisitResult {
        self.statements += 1;
        VisitResult::Continue
    }

    fn enter_expression(&mut self, _expr: &Expression) -> VisitResult {
        self.expressions += 1;
        VisitResult::Continue
    }
}
