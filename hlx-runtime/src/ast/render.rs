//! AST to Source Code Rendering
//!
//! Converts AST back to HLX source code.
//! Used by RSI to show what changed and for debugging.

use super::expr::{ExprKind, Pattern};
use super::rsi::{GovernDef, ModifyDef};
use super::{
    AgentDef, ClusterDef, Function, Item, Literal, LoopStmt, MatchPattern, MatchStmt, Parameter,
    Program, Statement, StmtKind, SwitchStmt,
};

/// Render AST to source code
pub trait Render {
    fn render(&self, indent: usize) -> String;
}

impl Render for Program {
    fn render(&self, indent: usize) -> String {
        let mut output = String::new();
        let ind = "    ".repeat(indent);

        output.push_str(&format!("{}program {} {{\n", ind, self.name));

        for item in &self.items {
            output.push_str(&item.render(indent + 1));
            output.push('\n');
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for Item {
    fn render(&self, indent: usize) -> String {
        match self {
            Item::Function(f) => f.render(indent),
            Item::Agent(a) => a.render(indent),
            Item::Cluster(c) => c.render(indent),
            Item::Module(m) => {
                let ind = "    ".repeat(indent);
                let mut output = format!("{}module {} {{\n", ind, m.name);
                for item in &m.items {
                    output.push_str(&item.render(indent + 1));
                    output.push('\n');
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            Item::Import(i) => {
                let ind = "    ".repeat(indent);
                let mut output = format!("{}import ", ind);
                if i.items.len() == 1
                    && matches!(i.items.first(), Some(super::ImportItem::Wildcard))
                {
                    output.push_str(&format!("{}::*", i.module));
                } else {
                    output.push_str(&format!("{}::{{ ", i.module));
                    for (idx, item) in i.items.iter().enumerate() {
                        if idx > 0 {
                            output.push_str(", ");
                        }
                        match item {
                            super::ImportItem::Named(n) => output.push_str(n),
                            super::ImportItem::Aliased { name, alias } => {
                                output.push_str(&format!("{} as {}", name, alias));
                            }
                            super::ImportItem::Wildcard => output.push('*'),
                        }
                    }
                    output.push_str(" }");
                }
                output.push_str(";\n");
                output
            }
            Item::Export(e) => {
                let ind = "    ".repeat(indent);
                format!("{}export {}\n", ind, e.item.render(indent))
            }
            Item::Struct(s) => {
                let ind = "    ".repeat(indent);
                let mut output = String::new();
                output.push_str(&format!("{}struct {} {{\n", ind, s.name));
                for field in &s.fields {
                    output.push_str(&format!("{}    {}: String,\n", ind, field.name));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            Item::ExternFunction(f) => {
                let ind = "    ".repeat(indent);
                let mut output = format!("{}extern fn {}(", ind, f.name);
                for (idx, param) in f.parameters.iter().enumerate() {
                    if idx > 0 { output.push_str(", "); }
                    output.push_str(&param.name);
                    output.push_str(": ");
                    if let Some(ref ty) = param.ty {
                        output.push_str(&ty.render(0));
                    } else {
                        output.push_str("Any");
                    }
                }
                output.push_str(")");
                if let Some(ref ty) = f.return_type {
                    output.push_str(" -> ");
                    output.push_str(&ty.render(0));
                }
                output.push_str(";\n");
                output
            }
            Item::Global(s) => s.render(indent),
        }
    }
}

impl Render for super::TypeAnnotation {
    fn render(&self, _indent: usize) -> String {
        if self.parameters.is_empty() {
            self.name.clone()
        } else {
            let mut output = format!("{}<", self.name);
            for (idx, param) in self.parameters.iter().enumerate() {
                if idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(&param.render(0));
            }
            output.push('>');
            output
        }
    }
}

impl Render for Function {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        // Attributes
        for attr in &self.attributes {
            output.push_str(&format!("{}#[{}]\n", ind, attr.name));
            if !attr.arguments.is_empty() {
                output.push_str(&format!(
                    "{}#[{}({})]\n",
                    ind,
                    attr.name,
                    attr.arguments.join(", ")
                ));
            }
        }

        // Signature
        if self.is_exported {
            output.push_str(&format!("{}export ", ind));
        }
        output.push_str(&format!("{}fn {}(", ind, self.name));

        for (idx, param) in self.parameters.iter().enumerate() {
            if idx > 0 {
                output.push_str(", ");
            }
            output.push_str(&param.render(indent));
        }

        output.push(')');

        if let Some(ref ret) = self.return_type {
            output.push_str(&format!(" -> {}", ret.name));
        }

        output.push_str(" {\n");

        // Body
        for stmt in &self.body {
            output.push_str(&stmt.render(indent + 1));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for Parameter {
    fn render(&self, _indent: usize) -> String {
        let mut output = self.name.clone();
        if let Some(ref ty) = self.ty {
            output.push_str(&format!(": {}", ty.name));
        }
        output
    }
}

impl Render for AgentDef {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}recursive agent {} {{\n", ind, self.name));

        // Latents
        for latent in &self.latents {
            output.push_str(&format!("{}    latent {}", ind, latent.name));
            output.push_str(&format!(": {}", latent.ty.name));
            if let Some(ref init) = latent.initializer {
                output.push_str(&format!(" = {}", init.render(0)));
            }
            output.push_str(";\n");
        }

        // Takes
        if !self.takes.is_empty() {
            output.push_str(&format!("{}    takes: ", ind));
            for (idx, param) in self.takes.iter().enumerate() {
                if idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(&param.name);
            }
            output.push_str(";\n");
        }

        // Gives
        if let Some(ref gives) = self.gives {
            output.push_str(&format!("{}    gives: {};\n", ind, gives.name));
        }

        // Cycles
        for cycle in &self.cycles {
            output.push_str(&cycle.render(indent + 1));
        }

        // Body
        for stmt in &self.body {
            output.push_str(&stmt.render(indent + 1));
        }

        // Govern
        if let Some(ref govern) = self.govern {
            output.push_str(&govern.render(indent + 1));
        }

        // Modify
        if let Some(ref modify) = self.modify {
            output.push_str(&modify.render(indent + 1));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for LoopStmt {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}loop(", ind));
        // Note: would need condition expression rendering
        output.push_str(&format!("{}, ", self.max_iterations.unwrap_or(1000)));
        output.push_str(") {\n");

        for stmt in &self.body {
            output.push_str(&stmt.render(indent + 1));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for super::CycleDef {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!(
            "{}cycle {}({}) {{\n",
            ind,
            self.level.name(),
            self.iterations
        ));

        for stmt in &self.body {
            output.push_str(&stmt.render(indent + 1));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for GovernDef {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}govern {{\n", ind));
        output.push_str(&format!("{}    effect: {};\n", ind, self.effect.name()));

        if !self.conscience.is_empty() {
            output.push_str(&format!("{}    conscience: [", ind));
            for (idx, pred) in self.conscience.iter().enumerate() {
                if idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(&pred.name);
            }
            output.push_str("];\n");
        }

        output.push_str(&format!("{}    trust: {};\n", ind, self.trust_threshold));
        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for ModifyDef {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}modify self {{\n", ind));

        for gate in &self.gates {
            output.push_str(&format!("{}    gate {:?};\n", ind, gate));
        }

        output.push_str(&format!("{}    cooldown: {};\n", ind, self.cooldown_steps));

        for proposal in &self.proposals {
            output.push_str(&format!("{}    proposal {:?};\n", ind, proposal.kind));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for ClusterDef {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = String::new();

        output.push_str(&format!("{}scale cluster {} {{\n", ind, self.name));

        if !self.agents.is_empty() {
            output.push_str(&format!("{}    agents: [", ind));
            for (idx, agent) in self.agents.iter().enumerate() {
                if idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(&agent.name);
            }
            output.push_str("];\n");
        }

        for barrier in self.barriers.iter() {
            output.push_str(&format!(
                "{}    barrier {}({});\n",
                ind, barrier.name, barrier.expected
            ));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for Statement {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);

        match &self.kind {
            StmtKind::Let {
                name,
                ty,
                mutable,
                value,
            } => {
                let mut output = format!("{}let ", ind);
                if *mutable {
                    output.push_str("mut ");
                }
                output.push_str(name);
                if let Some(t) = ty {
                    output.push_str(&format!(": {}", t.name));
                }
                if let Some(val) = value {
                    output.push_str(&format!(" = {};\n", val.render(0)));
                } else {
                    output.push_str(";\n");
                }
                output
            }
            StmtKind::Assign { target, value } => {
                format!("{}{} = {};\n", ind, target.render(0), value.render(0))
            }
            StmtKind::CompoundAssign { target, op, value } => {
                format!(
                    "{}{} {}= {};\n",
                    ind,
                    target.render(0),
                    op.symbol(),
                    value.render(0)
                )
            }
            StmtKind::Expr(expr) => {
                format!("{}{};\n", ind, expr.render(0))
            }
            StmtKind::Return(Some(expr)) => {
                format!("{}return {};\n", ind, expr.render(0))
            }
            StmtKind::Return(None) => {
                format!("{}return;\n", ind)
            }
            StmtKind::If(if_stmt) => {
                let mut output = format!("{}if ({}) {{\n", ind, if_stmt.condition.render(0));
                for stmt in &if_stmt.then_body {
                    output.push_str(&stmt.render(indent + 1));
                }

                if !if_stmt.else_body.is_empty() {
                    output.push_str(&format!("{}}} else {{\n", ind));
                    for stmt in &if_stmt.else_body {
                        output.push_str(&stmt.render(indent + 1));
                    }
                }

                output.push_str(&format!("{}}}\n", ind));
                output
            }
            StmtKind::Loop(loop_stmt) => loop_stmt.render(indent),
            StmtKind::While { condition, body } => {
                let mut output = format!("{}while ({}) {{\n", ind, condition.render(0));
                for stmt in body {
                    output.push_str(&stmt.render(indent + 1));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                let mut output = format!(
                    "{}for {} in {} {{\n",
                    ind,
                    pattern.render(0),
                    iterable.render(0)
                );
                for stmt in body {
                    output.push_str(&stmt.render(indent + 1));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            StmtKind::Break => format!("{}break;\n", ind),
            StmtKind::Continue => format!("{}continue;\n", ind),
            StmtKind::Block(stmts) => {
                let mut output = format!("{}{{\n", ind);
                for stmt in stmts {
                    output.push_str(&stmt.render(indent + 1));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            StmtKind::Switch(switch) => switch.render(indent),
            StmtKind::Match(m) => m.render(indent),
            StmtKind::Module(m) => {
                let mut output = format!("{}module {} {{\n", ind, m.name);
                for item in &m.items {
                    output.push_str(&item.render(indent + 1));
                }
                output.push_str(&format!("{}}}\n", ind));
                output
            }
            StmtKind::Import(i) => {
                format!("{}import {};\n", ind, i.module)
            }
            StmtKind::Export(e) => {
                format!("{}export {};\n", ind, e.item.render(0))
            }
            StmtKind::Migrate { agent, target } => {
                format!("{}migrate {} to {};\n", ind, agent, target)
            }
        }
    }
}

impl Render for SwitchStmt {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = format!("{}switch {} {{\n", ind, self.discriminant.render(0));

        for case in &self.cases {
            output.push_str(&format!(
                "{}    case {} => {{\n",
                ind,
                case.pattern.render(0)
            ));
            for stmt in &case.body {
                output.push_str(&stmt.render(indent + 2));
            }
            output.push_str(&format!("{}    }}\n", ind));
        }

        if !self.default_body.is_empty() {
            output.push_str(&format!("{}    default => {{\n", ind));
            for stmt in &self.default_body {
                output.push_str(&stmt.render(indent + 2));
            }
            output.push_str(&format!("{}    }}\n", ind));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for MatchStmt {
    fn render(&self, indent: usize) -> String {
        let ind = "    ".repeat(indent);
        let mut output = format!("{}match {} {{\n", ind, self.subject.render(0));

        for arm in &self.arms {
            let pattern_str = match &arm.pattern {
                MatchPattern::Literal(lit) => match lit {
                    Literal::Int(n) => n.to_string(),
                    Literal::Float(f) => f.to_string(),
                    Literal::String(s) => format!("\"{}\"", s),
                    Literal::Bool(b) => b.to_string(),
                    Literal::Nil => "nil".to_string(),
                },
                MatchPattern::Wildcard => "_".to_string(),
                MatchPattern::Binding(name) => name.clone(),
                MatchPattern::Range { start, end } => format!("{}..{}", start, end),
            };

            let guard_str = if let Some(g) = &arm.guard {
                format!(" if {}", g.render(0))
            } else {
                String::new()
            };

            output.push_str(&format!("{}    {} =>{}", ind, pattern_str, guard_str));
            for stmt in &arm.body {
                output.push_str(&stmt.render(indent + 2));
            }
            output.push_str(&format!("{}    }}\n", ind));
        }

        output.push_str(&format!("{}}}\n", ind));
        output
    }
}

impl Render for super::Expression {
    fn render(&self, _indent: usize) -> String {
        match &self.kind {
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Float(n) => n.to_string(),
            ExprKind::String(s) => format!("\"{}\"", s),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Identifier(name) => name.clone(),
            ExprKind::BinaryOp { op, left, right } => {
                format!("({} {} {})", left.render(0), op.symbol(), right.render(0))
            }
            ExprKind::UnaryOp { op, operand } => {
                format!("{}{}", op.symbol(), operand.render(0))
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.render(0)).collect();
                format!("{}({})", function, args.join(", "))
            }
            ExprKind::Array(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| e.render(0)).collect();
                format!("[{}]", elems.join(", "))
            }
            ExprKind::Dict(pairs) => {
                let entries: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k.render(0), v.render(0)))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            ExprKind::Index { array, index } => {
                format!("{}[{}]", array.render(0), index.render(0))
            }
            ExprKind::FieldAccess { object, field } => {
                format!("{}.{}", object.render(0), field)
            }
            ExprKind::Nil => "nil".to_string(),
            ExprKind::Void => "void".to_string(),
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                if *inclusive {
                    format!("{}..={}", start.render(0), end.render(0))
                } else {
                    format!("{}..{}", start.render(0), end.render(0))
                }
            }
            ExprKind::Contract {
                contract_type,
                fields,
            } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, val)| format!("{}: {}", name, val.render(0)))
                    .collect();
                format!("{} {{ {} }}", contract_type, fields_str.join(", "))
            }
            ExprKind::MethodCall {
                object,
                method,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.render(0)).collect();
                format!("{}.{}({})", object.render(0), method, args.join(", "))
            }
            ExprKind::Lambda { parameters, body } => {
                format!("|{}| {}", parameters.join(", "), body.render(0))
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                format!(
                    "if {} then {} else {}",
                    condition.render(0),
                    then_expr.render(0),
                    else_expr.render(0)
                )
            }
            ExprKind::Match { value, cases } => {
                let cases_str: Vec<String> = cases
                    .iter()
                    .map(|c| format!("{} => {}", c.pattern.render(0), c.body.render(0)))
                    .collect();
                format!("match {} {{ {} }}", value.render(0), cases_str.join(", "))
            }
            ExprKind::Collapse(inner) => format!("collapse {}", inner.render(0)),
            ExprKind::Resolve(inner) => format!("resolve {}", inner.render(0)),
            ExprKind::Cast { expr, target_type } => {
                format!("{} as {}", expr.render(0), target_type)
            }
            ExprKind::Do {
                intent_name,
                fields,
            } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.render(0)))
                    .collect();
                format!("do {} {{ {} }}", intent_name, fields_str.join(", "))
            }
        }
    }
}

impl Render for Pattern {
    fn render(&self, indent: usize) -> String {
        let _ind = "    ".repeat(indent);
        match self {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Int(n) => n.to_string(),
            Pattern::String(s) => format!("\"{}\"", s),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Destructure {
                type_name,
                fields,
                rest,
            } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, pat)| {
                        if let Some(p) = pat {
                            format!("{}: {}", name, p.render(indent))
                        } else {
                            name.clone()
                        }
                    })
                    .collect();
                let rest_str = if *rest { ", .." } else { "" };
                format!("{} {{ {}{} }}", type_name, fields_str.join(", "), rest_str)
            }
            Pattern::Or(patterns) => {
                let patterns_str: Vec<String> = patterns.iter().map(|p| p.render(indent)).collect();
                patterns_str.join(" | ")
            }
            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                if *inclusive {
                    format!("{}..={}", start, end)
                } else {
                    format!("{}..{}", start, end)
                }
            }
        }
    }
}
