use crate::lexer::token::Span;

#[derive(Debug, Clone)]
pub struct AxiomError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    // Lexer errors
    UnexpectedChar,
    UnterminatedString,
    InvalidNumber,

    // Parser errors
    UnexpectedToken,
    ExpectedToken,
    ExpectedExpression,
    ExpectedType,
    ExpectedDeclaration,

    // Type checker errors
    TypeMismatch,
    UndefinedVariable,
    UndefinedFunction,
    UndefinedContract,
    UndefinedField,
    DuplicateDefinition,
    ArgumentCount,

    // DSF errors (Dumb Shit Filter)
    DsfUnboundedLoop,
    DsfMissingTrustVerify,
    DsfUnhandledDoFailure,
    DsfTrustDecayChain,
    DsfReVerifyGuardMiss,
    DsfInferenceAmbiguity,
    DsfEnvConditional,

    // Axiom violations
    HaltDeterminism,
    HaltTraceCorrupt,
    HaltContract,
    HaltResource,
    HaltConscience,
    HaltUnknown,

    // Runtime errors
    DivisionByZero,
    IntegerOverflow,
    ResourceBoundExceeded,
    MaxIterExceeded,
    MaxDepthExceeded,

    // Trust / Sealed errors
    TrustBoundaryViolation,
    SealedViolation,

    // Guard / pre/post condition errors
    GuardFailed,
    PostConditionFailed,

    // Irreversible chain-terminal violation
    IrreversibleNotTerminal,
}

impl std::fmt::Display for AxiomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.span {
            Some(span) => write!(
                f,
                "[{:?}] Error at line {}:{}: {}",
                self.kind, span.line, span.col, self.message
            ),
            None => write!(f, "[{:?}] Error: {}", self.kind, self.message),
        }
    }
}

impl std::error::Error for AxiomError {}

pub type AxiomResult<T> = Result<T, AxiomError>;
