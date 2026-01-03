//! Lexer module for tokenizing Armature source files.

mod keywords;
mod lexer;
mod token;

pub use lexer::{tokenize, Lexer, LexerError};
pub use token::{SourceLocation, Token, TokenType, TokenValue};
