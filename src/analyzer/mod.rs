//! Semantic analysis for Armature AST.
//!
//! The analyzer performs:
//! - Symbol table construction
//! - Reference resolution
//! - Type checking (modifier arity)
//! - Clone validation
//! - Warnings for missing context, unused symbols, etc.

mod analyzer;
mod errors;
mod scope;

pub use analyzer::{analyze, Analyzer};
pub use errors::{AnalysisError, AnalysisWarning, AnalysisResult};
pub use scope::{SymbolTable, SymbolInfo, SymbolKind};
