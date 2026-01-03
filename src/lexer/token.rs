//! Token definitions for the Armature lexer.

use std::fmt;

/// Source location for error reporting and debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// All valid token types in Armature v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    // Special
    Eof,
    Newline,
    DocComment,

    // Literals
    String,
    Integer,
    Decimal,
    Identifier,

    // Symbol declarations
    Model,
    Enum,
    Interface,
    Operation,
    Event,
    Config,
    Portal,
    Surface,

    // Declaration keywords
    Namespace,
    Import,
    Project,
    Platform,
    Build,
    Clone,
    Alias,

    // Symbol member keywords
    Field,
    Variant,
    Relation,
    Invariant,
    Method,
    Input,
    Output,
    Error,
    Requires,
    Effect,
    Precondition,
    Postcondition,

    // Portal keywords
    Direction,
    Transport,
    Format,
    Handler,
    DataSource,

    // Surface keywords
    Display,
    Interaction,
    Command,
    Argument,
    Flag,
    Property,
    Accessibility,
    Help,
    State,
    On,
    Mapping,
    Parent,

    // Common keywords
    Context,
    Producer,
    Consumer,

    // Primitive types
    StringType,
    Bytes,
    Char,
    Bool,

    // Integer types
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,

    // Float types
    F32,
    F64,

    // Special types
    Uuid,
    Ulid,
    Timestamp,
    Duration,
    DecimalType,
    Size,
    Ptr,
    Any,
    Void,
    None,

    // Type modifiers
    List,
    Set,
    Map,
    Optional,
    Oneof,
    Ref,

    // Constraint keywords
    Unique,
    FormatKw,
    Pattern,
    Values,
    References,
    Min,
    Max,
    Length,
    SizeKw,
    ComputedBy,
    Custom,
    Sensitive,
    Immutable,
    Readonly,
    Derived,
    Indexed,
    Default,
    Primary,
    Now,

    // HTTP methods
    Get,
    Post,
    Put,
    Patch,
    Delete,

    // Relation types
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,

    // Effect types
    Creates,
    Updates,
    Deletes,
    Emits,
    Calls,

    // Boolean literals
    True,
    False,

    // Architecture keywords (for .ac files)
    Architecture,
    Naming,
    Structure,

    // Punctuation
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    LParen,    // (
    RParen,    // )
    LAngle,    // <
    RAngle,    // >
    Colon,     // :
    Comma,     // ,
    Dot,       // .
    DotDot,    // ..
    Question,  // ?
    Equals,    // =
    Pipe,      // |
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Token value - literals carry their parsed value.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    None,
    String(String),
    Integer(i64),
    Decimal(f64),
}

impl fmt::Display for TokenValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenValue::None => write!(f, ""),
            TokenValue::String(s) => write!(f, "{:?}", s),
            TokenValue::Integer(i) => write!(f, "{}", i),
            TokenValue::Decimal(d) => write!(f, "{}", d),
        }
    }
}

/// A lexical token with type, value, and source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub value: TokenValue,
    pub location: SourceLocation,
}

impl Token {
    pub fn new(token_type: TokenType, value: TokenValue, location: SourceLocation) -> Self {
        Self {
            token_type,
            value,
            location,
        }
    }

    pub fn simple(token_type: TokenType, location: SourceLocation) -> Self {
        Self::new(token_type, TokenValue::None, location)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            TokenValue::None => write!(f, "{:?} @ {}", self.token_type, self.location),
            _ => write!(
                f,
                "{:?}({}) @ {}",
                self.token_type, self.value, self.location
            ),
        }
    }
}
