//! Lifter (Decompiler) - Axiom A2 (Reversibility) Implementation
//!
//! Converts HLX bytecode (HlxCrate) back to AST, enabling:
//! - Bytecode → AST → Source reconstruction
//! - Verification of compilation correctness
//! - Debug and introspection tooling
//!
//! ## Design
//! The lifter performs the inverse of lowering:
//! 1. Parse instruction stream into basic blocks
//! 2. Reconstruct control flow (if/loop/function boundaries)
//! 3. Convert register-based IR to named variables
//! 4. Build AST from instruction patterns
//!
//! ## Register Naming
//! - Use debug symbols if available (from metadata)
//! - Otherwise generate synthetic names: r0, r1, r2, ...
//! - Preserve SSA property by appending versions: x_0, x_1, x_2

use crate::ast::*;
use hlx_core::{HlxCrate, Result as HlxResult, HlxError};
use hlx_core::instruction::{Instruction, Register, DType};
use hlx_core::value::Value;
use std::collections::HashMap;

/// Lift bytecode to AST
pub fn lift_from_crate(krate: &HlxCrate) -> HlxResult<Program> {
    let lifter = Lifter::new(krate);
    lifter.lift()
}

/// Internal lifter state
struct Lifter<'a> {
    krate: &'a HlxCrate,
    /// Map register ID to variable name
    register_names: HashMap<Register, String>,
    /// Current instruction pointer
    ip: usize,
    /// Collected blocks
    blocks: Vec<Block>,
    /// Register version counters (for SSA → versioned names)
    version_counter: HashMap<String, usize>,
}

impl<'a> Lifter<'a> {
    fn new(krate: &'a HlxCrate) -> Self {
        Self {
            krate,
            register_names: HashMap::new(),
            ip: 0,
            blocks: Vec::new(),
            version_counter: HashMap::new(),
        }
    }

    fn lift(mut self) -> HlxResult<Program> {
        // Initialize register names from metadata if available
        self.init_register_names();

        // First pass: collect function metadata (FuncDef instructions come AFTER the body)
        let mut func_defs = Vec::new();
        for (idx, inst) in self.krate.instructions.iter().enumerate() {
            if let Instruction::FuncDef { name, params, body } = inst {
                func_defs.push((idx, name.clone(), params.clone(), *body as usize));
            }
        }

        // Second pass: lift each function using the metadata
        if func_defs.is_empty() {
            // No functions, treat all instructions as main block
            let block = self.lift_all_instructions()?;
            self.blocks.push(block);
        } else {
            // Lift each function
            for (func_def_idx, name, params, body_start) in func_defs {
                // Find the end of this function (where FuncDef is)
                let body_end = func_def_idx;
                self.ip = body_start;

                let block = self.lift_function_range(name, params, body_start, body_end)?;
                self.blocks.push(block);
            }
        }

        Ok(Program {
            name: self.krate.metadata.as_ref()
                .and_then(|m| m.source_file.clone())
                .map(|s| s.trim_end_matches(".hlxl").to_string())
                .unwrap_or_else(|| "main".to_string()),
            imports: Vec::new(),
            modules: Vec::new(),
            blocks: self.blocks,
        })
    }

    /// Initialize register names from metadata or generate synthetic names
    fn init_register_names(&mut self) {
        let max_reg = self.krate.max_register();

        // Generate synthetic names for all registers
        for reg in 0..=max_reg {
            self.register_names.insert(reg, format!("r{}", reg));
        }
    }

    /// Get or generate a name for a register
    fn register_name(&self, reg: Register) -> String {
        self.register_names.get(&reg)
            .cloned()
            .unwrap_or_else(|| format!("r{}", reg))
    }

    /// Lift a function with known range
    fn lift_function_range(&mut self, name: String, params: Vec<Register>, start: usize, end: usize) -> HlxResult<Block> {
        // Convert parameter registers to names
        let param_names: Vec<_> = params.iter()
            .map(|&reg| {
                let name = self.register_name(reg);
                (name, None, None) // (name, name_span, type)
            })
            .collect();

        // Collect instructions from start to end (exclusive)
        let mut items = Vec::new();
        self.ip = start;

        while self.ip < end {
            if let Some(stmt) = self.lift_instruction()? {
                items.push(Spanned::dummy(Item::Statement(stmt)));
            }
            self.ip += 1;
        }

        // Remove the automatic null return added by the compiler if present
        // (it's at the last two instructions: Constant null + Return)
        if items.len() >= 2 {
            let len = items.len();
            // Check if last two statements are: let rN = null; return rN;
            let should_remove = matches!(&items[len - 2].node,
                Item::Statement(Statement::Let { value, .. }) if matches!(value.node, Expr::Literal(Literal::Null)))
                && matches!(&items[len - 1].node, Item::Statement(Statement::Return { .. }));

            if should_remove {
                items.truncate(len - 2);
            }
        }

        Ok(Block {
            name,
            attributes: Vec::new(),
            name_span: None,
            fn_keyword_span: None,
            params: param_names,
            return_type: None,
            return_type_span: None,
            items,
        })
    }

    /// Lift top-level block (program entry point)
    fn lift_top_level_block(&mut self) -> HlxResult<Block> {
        let mut items = Vec::new();

        while self.ip < self.krate.instructions.len() {
            let inst = &self.krate.instructions[self.ip];

            // Stop at function definitions
            if matches!(inst, Instruction::FuncDef { .. } | Instruction::ModuleDef { .. }) {
                break;
            }

            if let Some(stmt) = self.lift_instruction()? {
                items.push(Spanned::dummy(Item::Statement(stmt)));
            }

            self.ip += 1;
        }

        Ok(Block {
            name: "main".to_string(),
            attributes: Vec::new(),
            name_span: None,
            fn_keyword_span: None,
            params: Vec::new(),
            return_type: None,
            return_type_span: None,
            items,
        })
    }

    /// Lift all instructions into a single block
    fn lift_all_instructions(&mut self) -> HlxResult<Block> {
        let mut items = Vec::new();

        while self.ip < self.krate.instructions.len() {
            if let Some(stmt) = self.lift_instruction()? {
                items.push(Spanned::dummy(Item::Statement(stmt)));
            }
            self.ip += 1;
        }

        Ok(Block {
            name: "main".to_string(),
            attributes: Vec::new(),
            name_span: None,
            fn_keyword_span: None,
            params: Vec::new(),
            return_type: None,
            return_type_span: None,
            items,
        })
    }

    /// Lift a single instruction to a statement
    fn lift_instruction(&self) -> HlxResult<Option<Statement>> {
        let inst = &self.krate.instructions[self.ip];

        match inst {
            // Value operations
            Instruction::Constant { out, val } => {
                let var_name = self.register_name(*out);
                let expr = self.value_to_expr(val);
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(expr),
                }))
            }

            Instruction::Move { out, src } => {
                let var_name = self.register_name(*out);
                let src_name = self.register_name(*src);
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(Expr::Ident(src_name)),
                }))
            }

            // Arithmetic operations
            Instruction::Add { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Add)
            }
            Instruction::Sub { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Sub)
            }
            Instruction::Mul { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Mul)
            }
            Instruction::Div { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Div)
            }
            Instruction::Mod { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Mod)
            }
            Instruction::Neg { out, src } => {
                self.lift_unaryop(*out, *src, UnaryOp::Neg)
            }

            // Comparison operations
            Instruction::Eq { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Eq)
            }
            Instruction::Ne { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Ne)
            }
            Instruction::Lt { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Lt)
            }
            Instruction::Le { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Le)
            }
            Instruction::Gt { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Gt)
            }
            Instruction::Ge { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Ge)
            }

            // Logical operations
            Instruction::And { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::And)
            }
            Instruction::Or { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Or)
            }
            Instruction::Not { out, src } => {
                self.lift_unaryop(*out, *src, UnaryOp::Not)
            }

            // Bitwise operations
            Instruction::BitAnd { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::BitAnd)
            }
            Instruction::BitOr { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::BitOr)
            }
            Instruction::BitXor { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::BitXor)
            }
            Instruction::Shl { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Shl)
            }
            Instruction::Shr { out, lhs, rhs } => {
                self.lift_binop(*out, *lhs, *rhs, BinOp::Shr)
            }

            // Array operations
            Instruction::ArrayCreate { out, elements, .. } => {
                let var_name = self.register_name(*out);
                let elem_exprs: Vec<_> = elements.iter()
                    .map(|&reg| Spanned::dummy(Expr::Ident(self.register_name(reg))))
                    .collect();
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(Expr::Array(elem_exprs)),
                }))
            }

            Instruction::Index { out, container, index } => {
                let var_name = self.register_name(*out);
                let container_name = self.register_name(*container);
                let index_name = self.register_name(*index);
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(Expr::Index {
                        object: Box::new(Spanned::dummy(Expr::Ident(container_name))),
                        index: Box::new(Spanned::dummy(Expr::Ident(index_name))),
                    }),
                }))
            }

            Instruction::ArrayLen { out, array } => {
                let var_name = self.register_name(*out);
                let array_name = self.register_name(*array);
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(Expr::Call {
                        func: Box::new(Spanned::dummy(Expr::Field {
                            object: Box::new(Spanned::dummy(Expr::Ident(array_name))),
                            field: "length".to_string(),
                        })),
                        args: Vec::new(),
                    }),
                }))
            }

            // Function calls
            Instruction::Call { out, func, args } => {
                let var_name = self.register_name(*out);
                let arg_exprs: Vec<_> = args.iter()
                    .map(|&reg| Spanned::dummy(Expr::Ident(self.register_name(reg))))
                    .collect();
                Ok(Some(Statement::Let {
                    keyword_span: None,
                    name: var_name,
                    name_span: None,
                    type_annotation: None,
                    type_span: None,
                    value: Spanned::dummy(Expr::Call {
                        func: Box::new(Spanned::dummy(Expr::Ident(func.clone()))),
                        args: arg_exprs,
                    }),
                }))
            }

            // Control flow
            Instruction::Return { val } => {
                let val_name = self.register_name(*val);
                Ok(Some(Statement::Return {
                    keyword_span: None,
                    value: Spanned::dummy(Expr::Ident(val_name)),
                }))
            }

            Instruction::Break => Ok(Some(Statement::Break)),
            Instruction::Continue => Ok(Some(Statement::Continue)),

            // Debug
            Instruction::Print { val } => {
                let val_name = self.register_name(*val);
                Ok(Some(Statement::Expr(Spanned::dummy(Expr::Call {
                    func: Box::new(Spanned::dummy(Expr::Ident("print".to_string()))),
                    args: vec![Spanned::dummy(Expr::Ident(val_name))],
                }))))
            }

            // No-op and metadata instructions
            Instruction::Nop => Ok(None),
            Instruction::FuncDef { .. } => Ok(None),
            Instruction::ModuleDef { .. } => Ok(None),

            // Complex instructions that need special handling
            Instruction::If { .. } | Instruction::Loop { .. } | Instruction::Jump { .. } => {
                // TODO: Implement control flow reconstruction
                Ok(None)
            }

            // Tensor operations - represent as function calls
            Instruction::MatMul { out, lhs, rhs } => {
                self.lift_tensor_call(*out, "matmul", vec![*lhs, *rhs])
            }
            Instruction::LayerNorm { out, input, gamma, beta, eps } => {
                self.lift_tensor_call_with_float(*out, "layernorm", vec![*input, *gamma, *beta], *eps)
            }
            Instruction::Softmax { out, input, .. } => {
                self.lift_tensor_call(*out, "softmax", vec![*input])
            }
            Instruction::Gelu { out, input } => {
                self.lift_tensor_call(*out, "gelu", vec![*input])
            }
            Instruction::Relu { out, input } => {
                self.lift_tensor_call(*out, "relu", vec![*input])
            }

            // Other instructions not yet implemented
            _ => {
                // For unimplemented instructions, generate a comment
                Ok(None)
            }
        }
    }

    /// Lift a binary operation
    fn lift_binop(&self, out: Register, lhs: Register, rhs: Register, op: BinOp) -> HlxResult<Option<Statement>> {
        let var_name = self.register_name(out);
        let lhs_name = self.register_name(lhs);
        let rhs_name = self.register_name(rhs);

        Ok(Some(Statement::Let {
            keyword_span: None,
            name: var_name,
            name_span: None,
            type_annotation: None,
            type_span: None,
            value: Spanned::dummy(Expr::BinOp {
                op,
                lhs: Box::new(Spanned::dummy(Expr::Ident(lhs_name))),
                rhs: Box::new(Spanned::dummy(Expr::Ident(rhs_name))),
            }),
        }))
    }

    /// Lift a unary operation
    fn lift_unaryop(&self, out: Register, src: Register, op: UnaryOp) -> HlxResult<Option<Statement>> {
        let var_name = self.register_name(out);
        let src_name = self.register_name(src);

        Ok(Some(Statement::Let {
            keyword_span: None,
            name: var_name,
            name_span: None,
            type_annotation: None,
            type_span: None,
            value: Spanned::dummy(Expr::UnaryOp {
                op,
                operand: Box::new(Spanned::dummy(Expr::Ident(src_name))),
            }),
        }))
    }

    /// Lift a tensor operation as a function call
    fn lift_tensor_call(&self, out: Register, func_name: &str, args: Vec<Register>) -> HlxResult<Option<Statement>> {
        let var_name = self.register_name(out);
        let arg_exprs: Vec<_> = args.iter()
            .map(|&reg| Spanned::dummy(Expr::Ident(self.register_name(reg))))
            .collect();

        Ok(Some(Statement::Let {
            keyword_span: None,
            name: var_name,
            name_span: None,
            type_annotation: None,
            type_span: None,
            value: Spanned::dummy(Expr::Call {
                func: Box::new(Spanned::dummy(Expr::Ident(func_name.to_string()))),
                args: arg_exprs,
            }),
        }))
    }

    /// Lift a tensor operation with a float parameter
    fn lift_tensor_call_with_float(&self, out: Register, func_name: &str, args: Vec<Register>, float_param: f64) -> HlxResult<Option<Statement>> {
        let var_name = self.register_name(out);
        let mut arg_exprs: Vec<_> = args.iter()
            .map(|&reg| Spanned::dummy(Expr::Ident(self.register_name(reg))))
            .collect();
        arg_exprs.push(Spanned::dummy(Expr::Literal(Literal::Float(float_param))));

        Ok(Some(Statement::Let {
            keyword_span: None,
            name: var_name,
            name_span: None,
            type_annotation: None,
            type_span: None,
            value: Spanned::dummy(Expr::Call {
                func: Box::new(Spanned::dummy(Expr::Ident(func_name.to_string()))),
                args: arg_exprs,
            }),
        }))
    }

    /// Convert a Value to an Expr
    fn value_to_expr(&self, val: &Value) -> Expr {
        Expr::Literal(val.clone().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hlx_core::value::Value;

    #[test]
    fn test_lift_simple_add() {
        // Bytecode: r0 = 5; r1 = 3; r2 = r0 + r1; return r2
        let instructions = vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ];

        let krate = HlxCrate::new(instructions);
        let program = lift_from_crate(&krate).unwrap();

        assert_eq!(program.blocks.len(), 1);
        assert_eq!(program.blocks[0].items.len(), 4);
    }

    #[test]
    fn test_lift_function() {
        // Bytecode: fn add(r0, r1) { r2 = r0 + r1; return r2; }
        let instructions = vec![
            Instruction::FuncDef {
                name: "add".to_string(),
                params: vec![0, 1],
                body: 1,
            },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ];

        let krate = HlxCrate::new(instructions);
        let program = lift_from_crate(&krate).unwrap();

        assert_eq!(program.blocks.len(), 1);
        assert_eq!(program.blocks[0].name, "add");
        assert_eq!(program.blocks[0].params.len(), 2);
    }

    #[test]
    fn test_lift_array_operations() {
        // Bytecode: r0 = [1, 2, 3]; r1 = r0[0]
        let instructions = vec![
            Instruction::Constant { out: 0, val: Value::Integer(1) },
            Instruction::Constant { out: 1, val: Value::Integer(2) },
            Instruction::Constant { out: 2, val: Value::Integer(3) },
            Instruction::ArrayCreate { out: 3, elements: vec![0, 1, 2], element_type: None },
            Instruction::Constant { out: 4, val: Value::Integer(0) },
            Instruction::Index { out: 5, container: 3, index: 4 },
        ];

        let krate = HlxCrate::new(instructions);
        let program = lift_from_crate(&krate).unwrap();

        assert_eq!(program.blocks.len(), 1);
        assert!(program.blocks[0].items.len() >= 4);
    }
}
