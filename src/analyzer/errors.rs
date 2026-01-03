//! Semantic analysis error and warning types.

use crate::lexer::SourceLocation;
use thiserror::Error;

/// Semantic analysis error.
#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("{location}: Duplicate symbol '{name}'")]
    DuplicateSymbol { location: SourceLocation, name: String },

    #[error("{location}: Undefined reference '{name}'")]
    UndefinedReference { location: SourceLocation, name: String },

    #[error("{location}: Clone base '{base}' not found")]
    CloneBaseNotFound { location: SourceLocation, base: String },

    #[error("{location}: Chained clone not allowed - '{base}' is itself a clone")]
    ChainedClone { location: SourceLocation, base: String },

    #[error("{location}: Invalid type modifier '{modifier}' - expected {expected} type argument(s), got {got}")]
    InvalidModifierArity {
        location: SourceLocation,
        modifier: String,
        expected: usize,
        got: usize,
    },

    #[error("{location}: Constraint '{constraint}' not valid for type '{type_name}'")]
    InvalidConstraint {
        location: SourceLocation,
        constraint: String,
        type_name: String,
    },

    #[error("{location}: Effect target '{target}' not found")]
    EffectTargetNotFound { location: SourceLocation, target: String },

    #[error("{location}: Required interface '{interface}' not found")]
    RequiredInterfaceNotFound {
        location: SourceLocation,
        interface: String,
    },

    #[error("{location}: Producer '{producer}' not found")]
    ProducerNotFound { location: SourceLocation, producer: String },

    #[error("{location}: Handler '{handler}' not found")]
    HandlerNotFound { location: SourceLocation, handler: String },

    #[error("{location}: Relation target '{target}' not found")]
    RelationTargetNotFound { location: SourceLocation, target: String },

    #[error("{location}: Union type must have at least 2 members, got {count}")]
    UnionTooFewMembers { location: SourceLocation, count: usize },

    #[error("{location}: ref<T> should not be used in signatures (method parameters, returns, or operation inputs/outputs)")]
    RefInSignature { location: SourceLocation },
}

/// Semantic analysis warning.
#[derive(Debug)]
pub enum AnalysisWarning {
    MissingContext {
        location: SourceLocation,
        symbol_name: String,
    },
    ContextTooLong {
        location: SourceLocation,
        symbol_name: String,
        length: usize,
        max_length: usize,
    },
    UnusedSymbol {
        location: SourceLocation,
        symbol_name: String,
    },
}

impl std::fmt::Display for AnalysisWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisWarning::MissingContext {
                location,
                symbol_name,
            } => {
                write!(f, "{}: Warning: Symbol '{}' has no context", location, symbol_name)
            }
            AnalysisWarning::ContextTooLong {
                location,
                symbol_name,
                length,
                max_length,
            } => {
                write!(
                    f,
                    "{}: Warning: Context for '{}' is {} chars (recommended max: {})",
                    location, symbol_name, length, max_length
                )
            }
            AnalysisWarning::UnusedSymbol {
                location,
                symbol_name,
            } => {
                write!(f, "{}: Warning: Symbol '{}' is never referenced", location, symbol_name)
            }
        }
    }
}

/// Result of semantic analysis.
#[derive(Debug)]
pub struct AnalysisResult {
    pub errors: Vec<AnalysisError>,
    pub warnings: Vec<AnalysisWarning>,
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}
