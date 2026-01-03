//! Parser module for Armature v2 source files.

mod ast;
mod parser;

pub use ast::*;
pub use parser::{parse, parse_ac, parse_file, FileKind, Parser, ParseError};
