//! Lexer implementation for Armature source files.

use std::iter::Peekable;
use std::str::CharIndices;
use thiserror::Error;

use super::keywords;
use super::token::{SourceLocation, Token, TokenType, TokenValue};

/// Map single-character punctuation to token types.
fn punctuation_token(ch: char) -> Option<TokenType> {
    match ch {
        '{' => Some(TokenType::LBrace),
        '}' => Some(TokenType::RBrace),
        '[' => Some(TokenType::LBracket),
        ']' => Some(TokenType::RBracket),
        '(' => Some(TokenType::LParen),
        ')' => Some(TokenType::RParen),
        '<' => Some(TokenType::LAngle),
        '>' => Some(TokenType::RAngle),
        ':' => Some(TokenType::Colon),
        ',' => Some(TokenType::Comma),
        '.' => Some(TokenType::Dot),
        '?' => Some(TokenType::Question),
        '=' => Some(TokenType::Equals),
        '|' => Some(TokenType::Pipe),
        _ => None,
    }
}

/// Errors that can occur during lexical analysis.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LexerError {
    #[error("{line}:{column}: Unterminated string")]
    UnterminatedString { line: u32, column: u32 },

    #[error("{line}:{column}: Unterminated triple-quoted string")]
    UnterminatedTripleString { line: u32, column: u32 },

    #[error("{line}:{column}: Unterminated block comment")]
    UnterminatedBlockComment { line: u32, column: u32 },

    #[error("{line}:{column}: Invalid escape sequence: \\{ch}")]
    InvalidEscape { line: u32, column: u32, ch: char },

    #[error("{line}:{column}: Newline in single-quoted string")]
    NewlineInString { line: u32, column: u32 },

    #[error("{line}:{column}: Expected exponent digits after 'e'")]
    InvalidScientificNotation { line: u32, column: u32 },

    #[error("{line}:{column}: Unexpected character: '{ch}'")]
    UnexpectedCharacter { line: u32, column: u32, ch: char },
}

/// Tokenizer for Armature source files.
pub struct Lexer<'a> {
    chars: Peekable<CharIndices<'a>>,
    line: u32,
    column: u32,
    after_namespace_or_import: bool,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source code.
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            after_namespace_or_import: false,
        }
    }

    /// Tokenize the entire source into a list of tokens.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.token_type == TokenType::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Get current source location.
    fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column)
    }

    /// Peek at the current character without consuming it.
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    /// Peek at the character after the current one.
    fn peek_next(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.peek().map(|(_, c)| *c)
    }

    /// Check if the next three characters form a triple-quote (""").
    fn is_triple_quote(&self) -> bool {
        let mut iter = self.chars.clone();
        iter.next().map(|(_, c)| c) == Some('"')
            && iter.next().map(|(_, c)| c) == Some('"')
            && iter.next().map(|(_, c)| c) == Some('"')
    }

    /// Consume and return the current character.
    fn advance(&mut self) -> Option<char> {
        if let Some((_, ch)) = self.chars.next() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    /// Process an escape sequence after a backslash. Returns the escaped character.
    fn process_escape(&mut self, start_loc: SourceLocation) -> Result<char, LexerError> {
        let escape_loc = self.location();
        match self.peek() {
            None => Err(LexerError::UnterminatedString {
                line: start_loc.line,
                column: start_loc.column,
            }),
            Some(ch) => {
                let escaped = match ch {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    _ => {
                        return Err(LexerError::InvalidEscape {
                            line: escape_loc.line,
                            column: escape_loc.column,
                            ch,
                        })
                    }
                };
                self.advance();
                Ok(escaped)
            }
        }
    }

    /// Skip whitespace (not newlines - those can be significant).
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip a line comment (from # to end of line).
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Read a block comment (#* ... *#) and return as DOC_COMMENT token.
    fn read_block_comment(&mut self, start_loc: SourceLocation) -> Result<Token, LexerError> {
        let mut content = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedBlockComment {
                        line: start_loc.line,
                        column: start_loc.column,
                    });
                }
                Some('*') if self.peek_next() == Some('#') => {
                    self.advance();
                    self.advance();
                    let trimmed = content.trim().to_string();
                    return Ok(Token::new(
                        TokenType::DocComment,
                        TokenValue::String(trimmed),
                        start_loc,
                    ));
                }
                Some(ch) => {
                    content.push(ch);
                    self.advance();
                }
            }
        }
    }

    /// Read a string literal (single or triple-quoted).
    fn read_string(&mut self, start_loc: SourceLocation) -> Result<Token, LexerError> {
        self.advance(); // Opening quote

        // Check for triple-quote
        if self.peek() == Some('"') && self.peek_next() == Some('"') {
            self.advance(); // Second quote
            self.advance(); // Third quote
            return self.read_triple_string(start_loc);
        }

        // Single-quoted string
        let mut chars = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedString {
                        line: start_loc.line,
                        column: start_loc.column,
                    });
                }
                Some('\n') => {
                    return Err(LexerError::NewlineInString {
                        line: self.line,
                        column: self.column,
                    });
                }
                Some('"') => {
                    self.advance();
                    return Ok(Token::new(
                        TokenType::String,
                        TokenValue::String(chars),
                        start_loc,
                    ));
                }
                Some('\\') => {
                    self.advance();
                    let ch = self.process_escape(start_loc)?;
                    chars.push(ch);
                }
                Some(ch) => {
                    chars.push(ch);
                    self.advance();
                }
            }
        }
    }

    /// Read a triple-quoted string literal.
    fn read_triple_string(&mut self, start_loc: SourceLocation) -> Result<Token, LexerError> {
        let mut chars = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedTripleString {
                        line: start_loc.line,
                        column: start_loc.column,
                    });
                }
                Some('"') if self.is_triple_quote() => {
                    self.advance();
                    self.advance();
                    self.advance();
                    return Ok(Token::new(
                        TokenType::String,
                        TokenValue::String(chars),
                        start_loc,
                    ));
                }
                Some('"') => {
                    chars.push('"');
                    self.advance();
                }
                Some(ch) => {
                    chars.push(ch);
                    self.advance();
                }
            }
        }
    }

    /// Read a numeric literal (integer, hex integer, decimal, or scientific notation).
    fn read_number(&mut self, start_loc: SourceLocation) -> Result<Token, LexerError> {
        let mut chars = String::new();

        // Optional negative sign
        let is_negative = if self.peek() == Some('-') {
            chars.push('-');
            self.advance();
            true
        } else {
            false
        };

        // Read first digit
        if let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                chars.push(ch);
                self.advance();
            }
        }

        // Check for hex literal: 0x or 0X
        if (chars == "0" || chars == "-0") && matches!(self.peek(), Some('x' | 'X')) {
            self.advance();

            let mut hex_chars = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_hexdigit() {
                    hex_chars.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            if hex_chars.is_empty() {
                return Err(LexerError::UnexpectedCharacter {
                    line: start_loc.line,
                    column: start_loc.column,
                    ch: self.peek().unwrap_or('x'),
                });
            }

            let value = i64::from_str_radix(&hex_chars, 16).expect("validated hex string");
            let final_value = if is_negative { -value } else { value };

            return Ok(Token::new(
                TokenType::Integer,
                TokenValue::Integer(final_value),
                start_loc,
            ));
        }

        // Continue reading decimal digits for integer part
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                chars.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let mut has_decimal = false;

        // Check for decimal point (but not ..)
        if self.peek() == Some('.') && self.peek_next() != Some('.') {
            chars.push('.');
            self.advance();
            has_decimal = true;

            // Decimal part
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    chars.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for exponent (scientific notation: e.g., 1e10, 1.5e-3)
        if let Some(ch) = self.peek() {
            if ch == 'e' || ch == 'E' {
                chars.push(ch);
                self.advance();

                // Optional sign for exponent
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        chars.push(sign);
                        self.advance();
                    }
                }

                // Exponent digits (required)
                if self.peek().map(|c| c.is_ascii_digit()) != Some(true) {
                    return Err(LexerError::InvalidScientificNotation {
                        line: start_loc.line,
                        column: start_loc.column,
                    });
                }

                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        chars.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }

                // Scientific notation is always a decimal
                let value: f64 = chars.parse().expect("validated scientific notation");
                return Ok(Token::new(
                    TokenType::Decimal,
                    TokenValue::Decimal(value),
                    start_loc,
                ));
            }
        }

        if has_decimal {
            let value: f64 = chars.parse().expect("validated decimal literal");
            Ok(Token::new(
                TokenType::Decimal,
                TokenValue::Decimal(value),
                start_loc,
            ))
        } else {
            let value: i64 = chars.parse().expect("validated integer literal");
            Ok(Token::new(
                TokenType::Integer,
                TokenValue::Integer(value),
                start_loc,
            ))
        }
    }

    /// Read an identifier or keyword.
    fn read_identifier_or_keyword(&mut self, start_loc: SourceLocation) -> Token {
        let mut chars = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                chars.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check if it's a keyword
        if let Some(token_type) = keywords::lookup(&chars) {
            // Track if we just saw namespace or import (for newline significance)
            if token_type == TokenType::Namespace || token_type == TokenType::Import {
                self.after_namespace_or_import = true;
            }
            Token::simple(token_type, start_loc)
        } else {
            Token::new(TokenType::Identifier, TokenValue::String(chars), start_loc)
        }
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        loop {
            self.skip_whitespace();

            let loc = self.location();

            match self.peek() {
                // End of file
                None => return Ok(Token::simple(TokenType::Eof, loc)),

                // Newline (significant after namespace/import)
                Some('\n') => {
                    self.advance();
                    if self.after_namespace_or_import {
                        self.after_namespace_or_import = false;
                        return Ok(Token::simple(TokenType::Newline, loc));
                    }
                    // Skip insignificant newlines
                    continue;
                }

                // Comments
                Some('#') => {
                    self.advance();
                    if self.peek() == Some('*') {
                        self.advance();
                        return self.read_block_comment(loc);
                    } else {
                        self.skip_line_comment();
                        continue;
                    }
                }

                // String literals
                Some('"') => return self.read_string(loc),

                // Numbers (including negative)
                Some(ch) if ch.is_ascii_digit() => return self.read_number(loc),
                Some('-') if self.peek_next().map(|c| c.is_ascii_digit()) == Some(true) => {
                    return self.read_number(loc)
                }

                // Identifiers and keywords
                Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                    return Ok(self.read_identifier_or_keyword(loc))
                }

                // Two-character punctuation
                Some('.') if self.peek_next() == Some('.') => {
                    self.advance();
                    self.advance();
                    return Ok(Token::simple(TokenType::DotDot, loc));
                }

                // Single-character punctuation or unknown character
                Some(ch) => {
                    if let Some(tt) = punctuation_token(ch) {
                        self.advance();
                        return Ok(Token::simple(tt, loc));
                    }

                    return Err(LexerError::UnexpectedCharacter {
                        line: loc.line,
                        column: loc.column,
                        ch,
                    });
                }
            }
        }
    }
}

/// Convenience function to tokenize source code.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_source() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_simple_identifier() {
        let tokens = tokenize("hello").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].value, TokenValue::String("hello".to_string()));
    }

    #[test]
    fn test_keyword() {
        let tokens = tokenize("model").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, TokenType::Model);
    }

    #[test]
    fn test_integer() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(42));
    }

    #[test]
    fn test_negative_integer() {
        let tokens = tokenize("-17").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(-17));
    }

    #[test]
    fn test_decimal() {
        let tokens = tokenize("3.14").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Decimal);
        if let TokenValue::Decimal(v) = tokens[0].value {
            assert!((v - 3.14).abs() < 1e-10);
        } else {
            panic!("Expected decimal");
        }
    }

    #[test]
    fn test_scientific_notation() {
        let tokens = tokenize("1e10").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Decimal);
        if let TokenValue::Decimal(v) = tokens[0].value {
            assert!((v - 1e10).abs() < 1e5);
        } else {
            panic!("Expected decimal");
        }
    }

    #[test]
    fn test_scientific_notation_negative_exponent() {
        let tokens = tokenize("6.02e-23").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Decimal);
        if let TokenValue::Decimal(v) = tokens[0].value {
            assert!((v - 6.02e-23).abs() < 1e-30);
        } else {
            panic!("Expected decimal");
        }
    }

    #[test]
    fn test_string() {
        let tokens = tokenize("\"hello\"").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::String);
        assert_eq!(tokens[0].value, TokenValue::String("hello".to_string()));
    }

    #[test]
    fn test_string_with_escapes() {
        let tokens = tokenize("\"hello\\nworld\"").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::String);
        assert_eq!(
            tokens[0].value,
            TokenValue::String("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_triple_quoted_string() {
        let tokens = tokenize("\"\"\"hello\nworld\"\"\"").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::String);
        assert_eq!(
            tokens[0].value,
            TokenValue::String("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_line_comment() {
        let tokens = tokenize("model # comment\nUser").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Model);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
    }

    #[test]
    fn test_block_comment() {
        let tokens = tokenize("#* doc comment *#").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::DocComment);
        assert_eq!(
            tokens[0].value,
            TokenValue::String("doc comment".to_string())
        );
    }

    #[test]
    fn test_punctuation() {
        let tokens = tokenize("{}[]()<>:,.?=|").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::LBrace);
        assert_eq!(tokens[1].token_type, TokenType::RBrace);
        assert_eq!(tokens[2].token_type, TokenType::LBracket);
        assert_eq!(tokens[3].token_type, TokenType::RBracket);
        assert_eq!(tokens[4].token_type, TokenType::LParen);
        assert_eq!(tokens[5].token_type, TokenType::RParen);
        assert_eq!(tokens[6].token_type, TokenType::LAngle);
        assert_eq!(tokens[7].token_type, TokenType::RAngle);
        assert_eq!(tokens[8].token_type, TokenType::Colon);
        assert_eq!(tokens[9].token_type, TokenType::Comma);
        assert_eq!(tokens[10].token_type, TokenType::Dot);
        assert_eq!(tokens[11].token_type, TokenType::Question);
        assert_eq!(tokens[12].token_type, TokenType::Equals);
        assert_eq!(tokens[13].token_type, TokenType::Pipe);
    }

    #[test]
    fn test_dotdot() {
        let tokens = tokenize("1..10").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[1].token_type, TokenType::DotDot);
        assert_eq!(tokens[2].token_type, TokenType::Integer);
    }

    #[test]
    fn test_namespace_newline_significant() {
        let tokens = tokenize("namespace app.auth\nmodel").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Namespace);
        assert_eq!(tokens[1].token_type, TokenType::Identifier); // app
        assert_eq!(tokens[2].token_type, TokenType::Dot);
        assert_eq!(tokens[3].token_type, TokenType::Identifier); // auth
        assert_eq!(tokens[4].token_type, TokenType::Newline);
        assert_eq!(tokens[5].token_type, TokenType::Model);
    }

    #[test]
    fn test_import_newline_significant() {
        // Import paths are strings in v2 syntax
        let tokens = tokenize("import \"./file.arm\"\nmodel").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Import);
        assert_eq!(tokens[1].token_type, TokenType::String);
        assert_eq!(tokens[2].token_type, TokenType::Newline);
        assert_eq!(tokens[3].token_type, TokenType::Model);
    }

    #[test]
    fn test_type_keywords() {
        let tokens = tokenize("u8 u16 u32 u64 i32 f64 uuid timestamp").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::U8);
        assert_eq!(tokens[1].token_type, TokenType::U16);
        assert_eq!(tokens[2].token_type, TokenType::U32);
        assert_eq!(tokens[3].token_type, TokenType::U64);
        assert_eq!(tokens[4].token_type, TokenType::I32);
        assert_eq!(tokens[5].token_type, TokenType::F64);
        assert_eq!(tokens[6].token_type, TokenType::Uuid);
        assert_eq!(tokens[7].token_type, TokenType::Timestamp);
    }

    #[test]
    fn test_unterminated_string_error() {
        let result = tokenize("\"hello");
        assert!(result.is_err());
        if let Err(LexerError::UnterminatedString { .. }) = result {
            // OK
        } else {
            panic!("Expected UnterminatedString error");
        }
    }

    #[test]
    fn test_invalid_escape_error() {
        let result = tokenize("\"hello\\x\"");
        assert!(result.is_err());
        if let Err(LexerError::InvalidEscape { ch, .. }) = result {
            assert_eq!(ch, 'x');
        } else {
            panic!("Expected InvalidEscape error");
        }
    }

    #[test]
    fn test_unexpected_character_error() {
        let result = tokenize("@");
        assert!(result.is_err());
        if let Err(LexerError::UnexpectedCharacter { ch, .. }) = result {
            assert_eq!(ch, '@');
        } else {
            panic!("Expected UnexpectedCharacter error");
        }
    }

    #[test]
    fn test_hex_integer() {
        let tokens = tokenize("0xFF").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(255));
    }

    #[test]
    fn test_hex_integer_lowercase() {
        let tokens = tokenize("0xdeadbeef").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(0xdeadbeef));
    }

    #[test]
    fn test_hex_integer_uppercase() {
        let tokens = tokenize("0xDEADBEEF").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(0xDEADBEEF));
    }

    #[test]
    fn test_hex_integer_large() {
        // 0x18EEFF00 - used in CAN bus message IDs
        let tokens = tokenize("0x18EEFF00").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Integer);
        assert_eq!(tokens[0].value, TokenValue::Integer(0x18EEFF00));
    }
}
