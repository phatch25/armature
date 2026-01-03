//! Compiler for converting Armature AST to SQLite database.
//!
//! The compiler takes a parsed and analyzed AST and produces a SQLite database
//! that can be queried by the MCP server for agent consumption.

mod compiler;
mod schema;

pub use compiler::{compile, compile_ac, Compiler, AcCompiler, CompileError};
pub use schema::{SCHEMA_SQL, SCHEMA_VERSION};
