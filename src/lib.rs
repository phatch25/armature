//! Armature Schema Language compiler
//!
//! Armature is a focus mechanism for AI agents - it specifies *what* to build,
//! not *how* to build it. The compiler tokenizes `.arm` and `.ac` files,
//! parses them into an AST, and compiles to a SQLite database for agent consumption.
//!
//! ## Pipeline
//!
//! `.arm` file → Lexer → Parser → Expander → Analyzer → Compiler → `spec.db`
//!
//! ## Modules
//!
//! - [`lexer`] - Tokenizes source into a stream of tokens
//! - [`parser`] - Builds an AST from tokens
//! - [`expand`] - Expands clones into standalone symbols
//! - [`analyzer`] - Performs semantic analysis and validation
//! - [`compiler`] - Generates SQLite database from validated AST
//! - [`types`] - Shared type definitions (primitives, constants)

pub mod analyzer;
pub mod compiler;
pub mod expand;
pub mod lexer;
pub mod parser;
pub mod types;
