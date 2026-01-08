//! Type System for HLX
//!
//! Defines types and type operations for static analysis.

use std::fmt;

/// HLX Type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Integer type
    Int,
    /// Float type
    Float,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Null type
    Null,
    /// Array type (element type)
    Array(Box<Type>),
    /// Object type (field name -> type)
    Object,
    /// Function type (param types, return type)
    Function(Vec<Type>, Box<Type>),
    /// Unknown/inferred type
    Unknown,
    /// Any type (used for dynamic operations)
    Any,
}

impl Type {
    /// Check if this type is numeric (Int or Float)
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    /// Check if two types are compatible for operations
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        // Same type is always compatible
        if self == other {
            return true;
        }

        // Any is compatible with everything
        if matches!(self, Type::Any) || matches!(other, Type::Any) {
            return true;
        }

        // Unknown is compatible with everything (for inference)
        if matches!(self, Type::Unknown) || matches!(other, Type::Unknown) {
            return true;
        }

        // Int and Float are compatible for some operations
        match (self, other) {
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => true,
            _ => false,
        }
    }

    /// Get the result type of a binary operation
    pub fn binary_op_result(&self, op: BinaryOp, other: &Type) -> Result<Type, TypeError> {
        match op {
            // Arithmetic operations
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                match (self, other) {
                    // Int op Int = Int
                    (Type::Int, Type::Int) => Ok(Type::Int),
                    // Float op Float = Float
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    // Int op Float = Float
                    (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(Type::Float),
                    // String + String = String (concatenation)
                    (Type::String, Type::String) if op == BinaryOp::Add => Ok(Type::String),
                    // Unknown propagates
                    (Type::Unknown, _) | (_, Type::Unknown) => Ok(Type::Unknown),
                    (Type::Any, _) | (_, Type::Any) => Ok(Type::Any),
                    _ => Err(TypeError::IncompatibleTypes {
                        op: format!("{:?}", op),
                        left: self.clone(),
                        right: other.clone(),
                    }),
                }
            }
            // Comparison operations
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                // Comparisons work on compatible types and return bool
                if self.is_compatible_with(other) {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::IncompatibleTypes {
                        op: format!("{:?}", op),
                        left: self.clone(),
                        right: other.clone(),
                    })
                }
            }
            // Logical operations
            BinaryOp::And | BinaryOp::Or => {
                match (self, other) {
                    (Type::Bool, Type::Bool) => Ok(Type::Bool),
                    (Type::Unknown, _) | (_, Type::Unknown) => Ok(Type::Unknown),
                    (Type::Any, _) | (_, Type::Any) => Ok(Type::Any),
                    _ => Err(TypeError::IncompatibleTypes {
                        op: format!("{:?}", op),
                        left: self.clone(),
                        right: other.clone(),
                    }),
                }
            }
        }
    }

    /// Get the result type of a unary operation
    pub fn unary_op_result(&self, op: UnaryOp) -> Result<Type, TypeError> {
        match op {
            UnaryOp::Neg => {
                match self {
                    Type::Int => Ok(Type::Int),
                    Type::Float => Ok(Type::Float),
                    Type::Unknown => Ok(Type::Unknown),
                    Type::Any => Ok(Type::Any),
                    _ => Err(TypeError::InvalidUnaryOp {
                        op: format!("{:?}", op),
                        operand: self.clone(),
                    }),
                }
            }
            UnaryOp::Not => {
                match self {
                    Type::Bool => Ok(Type::Bool),
                    Type::Unknown => Ok(Type::Unknown),
                    Type::Any => Ok(Type::Any),
                    _ => Err(TypeError::InvalidUnaryOp {
                        op: format!("{:?}", op),
                        operand: self.clone(),
                    }),
                }
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::String => write!(f, "String"),
            Type::Bool => write!(f, "Bool"),
            Type::Null => write!(f, "Null"),
            Type::Array(elem) => write!(f, "Array<{}>", elem),
            Type::Object => write!(f, "Object"),
            Type::Function(params, ret) => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Unknown => write!(f, "?"),
            Type::Any => write!(f, "Any"),
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Type error
#[derive(Debug, Clone)]
pub enum TypeError {
    /// Incompatible types in binary operation
    IncompatibleTypes {
        op: String,
        left: Type,
        right: Type,
    },
    /// Invalid unary operation
    InvalidUnaryOp {
        op: String,
        operand: Type,
    },
    /// Wrong number of arguments
    WrongArgCount {
        expected: usize,
        got: usize,
    },
    /// Wrong argument type
    WrongArgType {
        param_index: usize,
        expected: Type,
        got: Type,
    },
    /// Undefined variable
    UndefinedVariable {
        name: String,
    },
    /// Undefined function
    UndefinedFunction {
        name: String,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::IncompatibleTypes { op, left, right } => {
                write!(f, "Cannot apply operator {} to types {} and {}", op, left, right)
            }
            TypeError::InvalidUnaryOp { op, operand } => {
                write!(f, "Cannot apply operator {} to type {}", op, operand)
            }
            TypeError::WrongArgCount { expected, got } => {
                write!(f, "Expected {} arguments, got {}", expected, got)
            }
            TypeError::WrongArgType { param_index, expected, got } => {
                write!(f, "Argument {} expected type {}, got {}", param_index + 1, expected, got)
            }
            TypeError::UndefinedVariable { name } => {
                write!(f, "Undefined variable: {}", name)
            }
            TypeError::UndefinedFunction { name } => {
                write!(f, "Undefined function: {}", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_compatibility() {
        assert!(Type::Int.is_compatible_with(&Type::Int));
        assert!(Type::Float.is_compatible_with(&Type::Float));
        assert!(Type::Int.is_compatible_with(&Type::Float));
        assert!(Type::Float.is_compatible_with(&Type::Int));
        assert!(!Type::Int.is_compatible_with(&Type::String));
    }

    #[test]
    fn test_binary_op_types() {
        // Int + Int = Int
        assert_eq!(
            Type::Int.binary_op_result(BinaryOp::Add, &Type::Int).unwrap(),
            Type::Int
        );

        // Float + Float = Float
        assert_eq!(
            Type::Float.binary_op_result(BinaryOp::Add, &Type::Float).unwrap(),
            Type::Float
        );

        // Int + Float = Float
        assert_eq!(
            Type::Int.binary_op_result(BinaryOp::Add, &Type::Float).unwrap(),
            Type::Float
        );

        // Int < Float = Bool
        assert_eq!(
            Type::Int.binary_op_result(BinaryOp::Lt, &Type::Float).unwrap(),
            Type::Bool
        );

        // String + String = String
        assert_eq!(
            Type::String.binary_op_result(BinaryOp::Add, &Type::String).unwrap(),
            Type::String
        );
    }

    #[test]
    fn test_binary_op_errors() {
        // String - Int should error
        assert!(Type::String.binary_op_result(BinaryOp::Sub, &Type::Int).is_err());

        // Bool + Int should error
        assert!(Type::Bool.binary_op_result(BinaryOp::Add, &Type::Int).is_err());
    }

    #[test]
    fn test_unary_ops() {
        // -Int = Int
        assert_eq!(Type::Int.unary_op_result(UnaryOp::Neg).unwrap(), Type::Int);

        // -Float = Float
        assert_eq!(Type::Float.unary_op_result(UnaryOp::Neg).unwrap(), Type::Float);

        // !Bool = Bool
        assert_eq!(Type::Bool.unary_op_result(UnaryOp::Not).unwrap(), Type::Bool);

        // -String should error
        assert!(Type::String.unary_op_result(UnaryOp::Neg).is_err());
    }
}
