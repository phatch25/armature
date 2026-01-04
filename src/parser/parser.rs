//! Recursive descent parser for Armature v2.

use thiserror::Error;

use crate::lexer::{tokenize, LexerError, SourceLocation, Token, TokenType, TokenValue};

use super::ast::*;

/// Errors that can occur during parsing.
#[derive(Debug, Error, Clone)]
pub enum ParseError {
    #[error("{line}:{column}: {message}")]
    Syntax {
        line: u32,
        column: u32,
        message: String,
    },

    #[error("{line}:{column}: Expected {expected}, got {found}")]
    UnexpectedToken {
        line: u32,
        column: u32,
        expected: String,
        found: String,
    },

    #[error("Lexer error: {0}")]
    Lexer(#[from] LexerError),
}

impl ParseError {
    fn at(loc: SourceLocation, message: impl Into<String>) -> Self {
        ParseError::Syntax {
            line: loc.line,
            column: loc.column,
            message: message.into(),
        }
    }

    fn unexpected(loc: SourceLocation, expected: impl Into<String>, found: TokenType) -> Self {
        ParseError::UnexpectedToken {
            line: loc.line,
            column: loc.column,
            expected: expected.into(),
            found: format!("{:?}", found),
        }
    }
}

/// Recursive descent parser for Armature v2.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a new parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Get the current token.
    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    /// Get the current token type.
    fn current_type(&self) -> TokenType {
        self.current().token_type
    }

    /// Get the current location.
    fn location(&self) -> SourceLocation {
        self.current().location
    }

    /// Consume and return current token.
    fn advance(&mut self) -> &Token {
        let prev_pos = self.pos;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        &self.tokens[prev_pos]
    }

    /// Expect a specific token type, or return error.
    fn expect(&mut self, expected: TokenType) -> Result<&Token, ParseError> {
        if self.current_type() != expected {
            return Err(ParseError::unexpected(
                self.location(),
                format!("{:?}", expected),
                self.current_type(),
            ));
        }
        Ok(self.advance())
    }

    /// Check if current token matches any of the given types.
    fn matches(&self, types: &[TokenType]) -> bool {
        types.contains(&self.current_type())
    }

    fn skip_newlines(&mut self) {
        while self.current_type() == TokenType::Newline {
            self.advance();
        }
    }

    fn should_continue_block(&mut self) -> bool {
        self.skip_newlines();
        self.current_type() != TokenType::RBrace
    }

    /// Skip newlines and doc comments, returning the last doc comment seen.
    fn skip_newlines_and_comments(&mut self) -> Option<String> {
        let mut last_doc: Option<String> = None;
        while self.matches(&[TokenType::Newline, TokenType::DocComment]) {
            if self.current_type() == TokenType::DocComment {
                if let TokenValue::String(s) = &self.current().value {
                    last_doc = Some(s.clone());
                }
            }
            self.advance();
        }
        last_doc
    }

    // =========================================================================
    // TOP LEVEL
    // =========================================================================

    /// Parse entire file.
    pub fn parse_file(&mut self) -> Result<File, ParseError> {
        let loc = self.location();
        self.skip_newlines();

        // Optional namespace
        let namespace = if self.current_type() == TokenType::Namespace {
            Some(self.parse_namespace()?)
        } else {
            None
        };

        // Imports
        let mut imports = Vec::new();
        while self.current_type() == TokenType::Import {
            imports.push(self.parse_import()?);
        }

        // Symbol definitions
        let mut symbols = Vec::new();
        while self.current_type() != TokenType::Eof {
            symbols.push(self.parse_symbol()?);
            self.skip_newlines();
        }

        Ok(File {
            location: loc,
            namespace,
            imports,
            symbols,
        })
    }

    /// Parse namespace declaration.
    fn parse_namespace(&mut self) -> Result<NamespaceDecl, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Namespace)?;
        let name = self.parse_dotted_name()?;
        self.expect(TokenType::Newline)?;
        self.skip_newlines();
        Ok(NamespaceDecl { location: loc, name })
    }

    /// Parse import declaration.
    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Import)?;

        // Import path can be a string or a dotted name
        let path = if self.current_type() == TokenType::String {
            self.expect_string()?
        } else {
            self.parse_dotted_name()?
        };

        self.expect(TokenType::Newline)?;
        self.skip_newlines();
        Ok(ImportDecl { location: loc, path })
    }

    fn parse_dotted_name(&mut self) -> Result<String, ParseError> {
        let mut parts = vec![self.expect_identifier()?];
        while self.current_type() == TokenType::Dot {
            self.advance();
            parts.push(self.expect_identifier()?);
        }
        Ok(parts.join("."))
    }

    fn parse_dotted_identifier(&mut self) -> Result<String, ParseError> {
        let mut parts = vec![self.expect_name()?];
        while self.current_type() == TokenType::Dot {
            self.advance();
            parts.push(self.expect_name()?);
        }
        Ok(parts.join("."))
    }

    /// Expect an identifier and return its value.
    /// Provides helpful error messages when a keyword is used where an identifier is expected.
    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        if self.current_type() == TokenType::Identifier {
            if let TokenValue::String(s) = &self.current().value {
                let name = s.clone();
                self.advance();
                return Ok(name);
            }
        }
        // Check if user accidentally used a reserved keyword as an identifier
        let keyword_name = self.get_keyword_as_string();
        if let Some(kw) = keyword_name {
            return Err(ParseError::at(
                self.location(),
                format!("'{}' is a reserved keyword and cannot be used as an identifier. Try a different name.", kw),
            ));
        }
        Err(ParseError::unexpected(
            self.location(),
            "identifier",
            self.current_type(),
        ))
    }

    /// If current token is a keyword, return its string representation.
    fn get_keyword_as_string(&self) -> Option<&'static str> {
        match self.current_type() {
            // Symbol keywords
            TokenType::Model => Some("model"),
            TokenType::Enum => Some("enum"),
            TokenType::Interface => Some("interface"),
            TokenType::Operation => Some("operation"),
            TokenType::Event => Some("event"),
            TokenType::Config => Some("config"),
            TokenType::Portal => Some("portal"),
            TokenType::Surface => Some("surface"),
            // Declaration keywords
            TokenType::Namespace => Some("namespace"),
            TokenType::Import => Some("import"),
            TokenType::Project => Some("project"),
            TokenType::Platform => Some("platform"),
            TokenType::Build => Some("build"),
            TokenType::Clone => Some("clone"),
            TokenType::Alias => Some("alias"),
            // Member keywords
            TokenType::Field => Some("field"),
            TokenType::Variant => Some("variant"),
            TokenType::Relation => Some("relation"),
            TokenType::Invariant => Some("invariant"),
            TokenType::Method => Some("method"),
            TokenType::Input => Some("input"),
            TokenType::Output => Some("output"),
            TokenType::Error => Some("error"),
            TokenType::Requires => Some("requires"),
            TokenType::Effect => Some("effect"),
            TokenType::Precondition => Some("precondition"),
            TokenType::Postcondition => Some("postcondition"),
            // Portal/Surface keywords
            TokenType::Direction => Some("direction"),
            TokenType::Transport => Some("transport"),
            TokenType::Format => Some("format"),
            TokenType::Handler => Some("handler"),
            TokenType::DataSource => Some("data_source"),
            TokenType::Display => Some("display"),
            TokenType::Interaction => Some("interaction"),
            TokenType::Command => Some("command"),
            TokenType::Argument => Some("argument"),
            TokenType::Flag => Some("flag"),
            TokenType::Property => Some("property"),
            TokenType::Accessibility => Some("accessibility"),
            TokenType::Help => Some("help"),
            TokenType::State => Some("state"),
            TokenType::On => Some("on"),
            TokenType::Mapping => Some("mapping"),
            TokenType::Parent => Some("parent"),
            TokenType::Context => Some("context"),
            TokenType::Producer => Some("producer"),
            TokenType::Consumer => Some("consumer"),
            _ => None,
        }
    }

    /// Convert a keyword TokenType to its string representation for use as a name.
    /// Returns Some if the keyword can be used as a name (field names, variant names, etc.),
    /// None if it cannot.
    fn keyword_as_name(token_type: TokenType) -> Option<&'static str> {
        match token_type {
            // HTTP methods
            TokenType::Get => Some("get"),
            TokenType::Post => Some("post"),
            TokenType::Put => Some("put"),
            TokenType::Patch => Some("patch"),
            TokenType::Delete => Some("delete"),
            // Primitive types
            TokenType::StringType => Some("string"),
            TokenType::Bool => Some("bool"),
            TokenType::Bytes => Some("bytes"),
            TokenType::Char => Some("char"),
            TokenType::U8 => Some("u8"),
            TokenType::U16 => Some("u16"),
            TokenType::U32 => Some("u32"),
            TokenType::U64 => Some("u64"),
            TokenType::U128 => Some("u128"),
            TokenType::I8 => Some("i8"),
            TokenType::I16 => Some("i16"),
            TokenType::I32 => Some("i32"),
            TokenType::I64 => Some("i64"),
            TokenType::I128 => Some("i128"),
            TokenType::F32 => Some("f32"),
            TokenType::F64 => Some("f64"),
            TokenType::Uuid => Some("uuid"),
            TokenType::Ulid => Some("ulid"),
            TokenType::Timestamp => Some("timestamp"),
            TokenType::Duration => Some("duration"),
            TokenType::DecimalType => Some("decimal"),
            TokenType::Size => Some("size"),
            TokenType::Ptr => Some("ptr"),
            TokenType::Any => Some("any"),
            TokenType::Void => Some("void"),
            TokenType::None => Some("none"),
            // Type modifiers
            TokenType::List => Some("list"),
            TokenType::Set => Some("set"),
            TokenType::Map => Some("map"),
            TokenType::Optional => Some("optional"),
            TokenType::Oneof => Some("oneof"),
            TokenType::Ref => Some("ref"),
            // Constraints
            TokenType::Unique => Some("unique"),
            TokenType::FormatKw => Some("format"),
            TokenType::Pattern => Some("pattern"),
            TokenType::Min => Some("min"),
            TokenType::Max => Some("max"),
            TokenType::Length => Some("length"),
            TokenType::Sensitive => Some("sensitive"),
            TokenType::Immutable => Some("immutable"),
            TokenType::Readonly => Some("readonly"),
            TokenType::Derived => Some("derived"),
            TokenType::Indexed => Some("indexed"),
            TokenType::Default => Some("default"),
            TokenType::Primary => Some("primary"),
            TokenType::Values => Some("values"),
            TokenType::References => Some("references"),
            TokenType::SizeKw => Some("size"),
            TokenType::ComputedBy => Some("computed_by"),
            TokenType::Custom => Some("custom"),
            // Symbol keywords
            TokenType::Model => Some("model"),
            TokenType::Enum => Some("enum"),
            TokenType::Interface => Some("interface"),
            TokenType::Operation => Some("operation"),
            TokenType::Event => Some("event"),
            TokenType::Config => Some("config"),
            TokenType::Portal => Some("portal"),
            TokenType::Surface => Some("surface"),
            // Member keywords
            TokenType::Field => Some("field"),
            TokenType::Variant => Some("variant"),
            TokenType::Method => Some("method"),
            TokenType::Input => Some("input"),
            TokenType::Output => Some("output"),
            TokenType::Error => Some("error"),
            TokenType::Context => Some("context"),
            TokenType::Requires => Some("requires"),
            TokenType::Effect => Some("effect"),
            TokenType::Relation => Some("relation"),
            TokenType::Invariant => Some("invariant"),
            TokenType::Precondition => Some("precondition"),
            TokenType::Postcondition => Some("postcondition"),
            // Declaration keywords
            TokenType::Namespace => Some("namespace"),
            TokenType::Import => Some("import"),
            TokenType::Clone => Some("clone"),
            TokenType::Project => Some("project"),
            TokenType::Platform => Some("platform"),
            TokenType::Build => Some("build"),
            TokenType::Alias => Some("alias"),
            // Portal keywords
            TokenType::Direction => Some("direction"),
            TokenType::Transport => Some("transport"),
            TokenType::Format => Some("format"),
            TokenType::Handler => Some("handler"),
            TokenType::DataSource => Some("data_source"),
            // Surface keywords
            TokenType::Display => Some("display"),
            TokenType::Interaction => Some("interaction"),
            TokenType::Command => Some("command"),
            TokenType::Argument => Some("argument"),
            TokenType::Flag => Some("flag"),
            TokenType::Property => Some("property"),
            TokenType::Accessibility => Some("accessibility"),
            TokenType::Help => Some("help"),
            TokenType::State => Some("state"),
            TokenType::On => Some("on"),
            TokenType::Mapping => Some("mapping"),
            TokenType::Parent => Some("parent"),
            // Common keywords
            TokenType::Producer => Some("producer"),
            TokenType::Consumer => Some("consumer"),
            // Relation types
            TokenType::HasOne => Some("has_one"),
            TokenType::HasMany => Some("has_many"),
            TokenType::BelongsTo => Some("belongs_to"),
            TokenType::ManyToMany => Some("many_to_many"),
            // Effect types
            TokenType::Creates => Some("creates"),
            TokenType::Updates => Some("updates"),
            TokenType::Deletes => Some("deletes"),
            TokenType::Emits => Some("emits"),
            TokenType::Calls => Some("calls"),
            // Boolean/special literals
            TokenType::True => Some("true"),
            TokenType::False => Some("false"),
            TokenType::Now => Some("now"),
            // Architecture keywords
            TokenType::Architecture => Some("architecture"),
            TokenType::Naming => Some("naming"),
            TokenType::Structure => Some("structure"),
            // Everything else cannot be used as a name
            _ => None,
        }
    }

    /// Expect a name that can be an identifier OR a keyword.
    /// Used for variant names, field names, etc. where keywords are valid names.
    fn expect_name(&mut self) -> Result<String, ParseError> {
        // Try identifier first
        if self.current_type() == TokenType::Identifier {
            if let TokenValue::String(s) = &self.current().value {
                let name = s.clone();
                self.advance();
                return Ok(name);
            }
        }

        // Allow keywords as names
        if let Some(name) = Self::keyword_as_name(self.current_type()) {
            self.advance();
            return Ok(name.to_string());
        }

        Err(ParseError::unexpected(
            self.location(),
            "name",
            self.current_type(),
        ))
    }

    /// Check if current token is a keyword that can be used as a name.
    fn is_keyword_that_can_be_name(&self) -> bool {
        Self::keyword_as_name(self.current_type()).is_some()
    }

    /// Expect a string literal and return its value.
    fn expect_string(&mut self) -> Result<String, ParseError> {
        if self.current_type() == TokenType::String {
            if let TokenValue::String(s) = &self.current().value {
                let value = s.clone();
                self.advance();
                return Ok(value);
            }
        }
        Err(ParseError::unexpected(
            self.location(),
            "string",
            self.current_type(),
        ))
    }

    // =========================================================================
    // SYMBOL DISPATCH
    // =========================================================================

    /// Parse any symbol definition.
    fn parse_symbol(&mut self) -> Result<SymbolDef, ParseError> {
        let doc_comment = self.skip_newlines_and_comments();

        let result = match self.current_type() {
            TokenType::Model => SymbolDef::Model(self.parse_model(doc_comment)?),
            TokenType::Enum => SymbolDef::Enum(self.parse_enum(doc_comment)?),
            TokenType::Interface => SymbolDef::Interface(self.parse_interface(doc_comment)?),
            TokenType::Operation => SymbolDef::Operation(self.parse_operation(doc_comment)?),
            TokenType::Event => SymbolDef::Event(self.parse_event(doc_comment)?),
            TokenType::Config => SymbolDef::Config(self.parse_config(doc_comment)?),
            TokenType::Portal => SymbolDef::Portal(self.parse_portal(doc_comment)?),
            TokenType::Surface => SymbolDef::Surface(self.parse_surface(doc_comment)?),
            _ => {
                return Err(ParseError::at(
                    self.location(),
                    format!("Expected symbol definition, got {:?}", self.current_type()),
                ))
            }
        };

        Ok(result)
    }

    // =========================================================================
    // MODEL
    // =========================================================================

    /// Parse model definition.
    fn parse_model(&mut self, doc_comment: Option<String>) -> Result<ModelDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Model)?;
        let name = self.expect_name()?;

        // Check for clone
        let base = if self.current_type() == TokenType::Clone {
            self.advance();
            Some(self.expect_name()?)
        } else {
            None
        };

        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut fields = Vec::new();
        let mut relations = Vec::new();
        let mut invariants = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Field => {
                    fields.push(self.parse_field_def()?);
                }
                TokenType::Relation => {
                    relations.push(self.parse_relation_def()?);
                }
                TokenType::Invariant => {
                    invariants.push(self.parse_named_string()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in model: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(ModelDef {
            location: loc,
            name,
            doc_comment,
            context,
            base,
            fields,
            relations,
            invariants,
        })
    }

    /// Parse field definition: `field name: type [constraints]? = default?`
    /// Field names can be keywords (e.g., `field state: LexerState`)
    fn parse_field_def(&mut self) -> Result<FieldDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Field)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let field_type = self.parse_type_expr()?;

        // Optional constraints (before default value)
        let constraints = if self.current_type() == TokenType::LBracket {
            self.parse_constraints()?
        } else {
            Vec::new()
        };

        // Optional default value (after constraints)
        let default = if self.current_type() == TokenType::Equals {
            self.advance();
            Some(self.parse_literal()?)
        } else {
            None
        };

        Ok(FieldDef {
            location: loc,
            name,
            field_type,
            default,
            constraints,
        })
    }

    /// Parse constraints: `[unique, min: 0, length: 1..100]`
    fn parse_constraints(&mut self) -> Result<Vec<Constraint>, ParseError> {
        self.expect(TokenType::LBracket)?;
        let mut constraints = Vec::new();

        while self.current_type() != TokenType::RBracket {
            constraints.push(self.parse_constraint()?);
            if self.current_type() == TokenType::Comma {
                self.advance();
            }
        }

        self.expect(TokenType::RBracket)?;
        Ok(constraints)
    }

    /// Parse a single constraint.
    fn parse_constraint(&mut self) -> Result<Constraint, ParseError> {
        let loc = self.location();

        // Constraint name can be an identifier or a keyword like "format", "unique", etc.
        let name = self.expect_constraint_name()?;

        let value = if self.current_type() == TokenType::Colon {
            self.advance();
            Some(self.parse_constraint_value()?)
        } else {
            None
        };

        Ok(Constraint {
            location: loc,
            name,
            value,
        })
    }

    fn expect_constraint_name(&mut self) -> Result<String, ParseError> {
        if self.current_type() == TokenType::Identifier {
            return self.expect_name();
        }

        let name = match self.current_type() {
            TokenType::Unique => "unique",
            TokenType::FormatKw | TokenType::Format => "format",
            TokenType::Pattern => "pattern",
            TokenType::Min => "min",
            TokenType::Max => "max",
            TokenType::Length => "length",
            TokenType::Sensitive => "sensitive",
            TokenType::Immutable => "immutable",
            TokenType::Readonly => "readonly",
            TokenType::Derived => "derived",
            TokenType::Indexed => "indexed",
            TokenType::Default => "default",
            TokenType::Primary => "primary",
            TokenType::References => "references",
            TokenType::Values => "values",
            TokenType::ComputedBy => "computed_by",
            TokenType::Custom => "custom",
            _ => {
                return Err(ParseError::unexpected(
                    self.location(),
                    "constraint name",
                    self.current_type(),
                ))
            }
        };

        self.advance();
        Ok(name.to_string())
    }

    /// Parse constraint value.
    fn parse_constraint_value(&mut self) -> Result<ConstraintValue, ParseError> {
        match self.current_type() {
            TokenType::Integer => {
                let start = self.expect_integer()?;
                if self.current_type() == TokenType::DotDot {
                    self.advance();
                    let end = self.expect_integer()?;
                    Ok(ConstraintValue::Range { start, end })
                } else {
                    Ok(ConstraintValue::Integer(start))
                }
            }
            TokenType::Decimal => {
                if let TokenValue::Decimal(d) = self.current().value {
                    self.advance();
                    Ok(ConstraintValue::Decimal(d))
                } else {
                    Err(ParseError::at(self.location(), "Expected decimal"))
                }
            }
            TokenType::String => Ok(ConstraintValue::String(self.expect_string()?)),
            TokenType::Identifier => {
                let name = self.parse_dotted_identifier()?;
                Ok(ConstraintValue::Identifier(name))
            }
            _ if self.is_keyword_that_can_be_name() => {
                let name = self.parse_dotted_identifier()?;
                Ok(ConstraintValue::Identifier(name))
            }
            TokenType::LBracket => {
                // List of strings for "values" constraint
                self.advance();
                let mut list = Vec::new();
                while self.current_type() != TokenType::RBracket {
                    list.push(self.expect_string()?);
                    if self.current_type() == TokenType::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenType::RBracket)?;
                Ok(ConstraintValue::List(list))
            }
            _ => Err(ParseError::unexpected(
                self.location(),
                "constraint value",
                self.current_type(),
            )),
        }
    }

    fn expect_integer(&mut self) -> Result<i64, ParseError> {
        if self.current_type() == TokenType::Integer {
            if let TokenValue::Integer(i) = self.current().value {
                self.advance();
                return Ok(i);
            }
        }
        Err(ParseError::unexpected(
            self.location(),
            "integer",
            self.current_type(),
        ))
    }

    /// Parse relation definition: `relation sessions: has_many<Session>`
    fn parse_relation_def(&mut self) -> Result<RelationDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Relation)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;

        // Relation kind
        let kind = match self.current_type() {
            TokenType::HasOne => RelationKind::HasOne,
            TokenType::HasMany => RelationKind::HasMany,
            TokenType::BelongsTo => RelationKind::BelongsTo,
            TokenType::ManyToMany => RelationKind::ManyToMany,
            _ => {
                return Err(ParseError::unexpected(
                    self.location(),
                    "relation type (has_one, has_many, belongs_to, many_to_many)",
                    self.current_type(),
                ))
            }
        };
        self.advance();

        self.expect(TokenType::LAngle)?;
        let target = self.parse_type_expr()?;
        self.expect(TokenType::RAngle)?;

        Ok(RelationDef {
            location: loc,
            name,
            kind,
            target,
        })
    }

    /// Parse named string: `invariant name "description"` or `error name "message"`
    fn parse_named_string(&mut self) -> Result<NamedString, ParseError> {
        let loc = self.location();
        self.advance(); // Skip the keyword (invariant, error, etc.)
        let name = self.expect_name()?;
        let value = self.expect_string()?;
        Ok(NamedString {
            location: loc,
            name,
            value,
        })
    }

    // =========================================================================
    // ENUM
    // =========================================================================

    /// Parse enum definition.
    fn parse_enum(&mut self, doc_comment: Option<String>) -> Result<EnumDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Enum)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut variants = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Variant => {
                    variants.push(self.parse_variant_def()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in enum: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(EnumDef {
            location: loc,
            name,
            doc_comment,
            context,
            variants,
        })
    }

    /// Parse variant definition: `variant EOF` or `variant Model(ModelSpec)` for sum types
    /// Keywords are allowed as names (e.g., `variant GET`).
    fn parse_variant_def(&mut self) -> Result<VariantDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Variant)?;
        let name = self.expect_name()?;

        // Check for optional associated data type: variant Name(Type)
        let data_type = if self.current_type() == TokenType::LParen {
            self.advance();
            let type_expr = self.parse_type_expr()?;
            self.expect(TokenType::RParen)?;
            Some(type_expr)
        } else {
            None
        };

        Ok(VariantDef {
            location: loc,
            name,
            data_type,
        })
    }

    // =========================================================================
    // INTERFACE
    // =========================================================================

    /// Parse interface definition.
    fn parse_interface(&mut self, doc_comment: Option<String>) -> Result<InterfaceDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Interface)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut methods = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Method => {
                    methods.push(self.parse_method_def()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in interface: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(InterfaceDef {
            location: loc,
            name,
            doc_comment,
            context,
            methods,
        })
    }

    /// Parse method definition.
    /// Shorthand: `method findById(id: uuid): User?`
    /// Block: `method findById { input id: uuid output: User? }`
    fn parse_method_def(&mut self) -> Result<MethodDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Method)?;
        let name = self.expect_name()?;

        // Check for shorthand form: method name(args): return
        if self.current_type() == TokenType::LParen {
            self.advance();
            let mut params = Vec::new();

            while self.current_type() != TokenType::RParen {
                let param_loc = self.location();
                let param_name = self.expect_name()?;
                self.expect(TokenType::Colon)?;
                let param_type = self.parse_type_expr()?;
                params.push(MethodParam {
                    location: param_loc,
                    name: param_name,
                    param_type,
                });
                if self.current_type() == TokenType::Comma {
                    self.advance();
                }
            }
            self.expect(TokenType::RParen)?;

            let return_type = if self.current_type() == TokenType::Colon {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };

            // Shorthand form doesn't support context or errors
            return Ok(MethodDef {
                location: loc,
                name,
                context: None,
                params,
                return_type,
                errors: Vec::new(),
            });
        }

        // Block form: method name { context ... input ... output: ... error ... }
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut params = Vec::new();
        let mut return_type = None;
        let mut errors = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Input => {
                    self.advance();
                    let param_loc = self.location();
                    let param_name = self.expect_name()?;
                    self.expect(TokenType::Colon)?;
                    let param_type = self.parse_type_expr()?;
                    params.push(MethodParam {
                        location: param_loc,
                        name: param_name,
                        param_type,
                    });
                }
                TokenType::Output => {
                    self.advance();
                    self.expect(TokenType::Colon)?;
                    return_type = Some(self.parse_type_expr()?);
                }
                TokenType::Error => {
                    errors.push(self.parse_named_string()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in method: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(MethodDef {
            location: loc,
            name,
            context,
            params,
            return_type,
            errors,
        })
    }

    // =========================================================================
    // OPERATION
    // =========================================================================

    /// Parse operation definition.
    fn parse_operation(&mut self, doc_comment: Option<String>) -> Result<OperationDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Operation)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut inputs = Vec::new();
        let mut output = None;
        let mut errors = Vec::new();
        let mut requires = Vec::new();
        let mut effects = Vec::new();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Input => {
                    inputs.push(self.parse_input_def()?);
                }
                TokenType::Output => {
                    self.advance();
                    self.expect(TokenType::Colon)?;
                    output = Some(self.parse_type_expr()?);
                }
                TokenType::Error => {
                    errors.push(self.parse_named_string()?);
                }
                TokenType::Requires => {
                    self.advance();
                    requires.push(self.expect_name()?);
                }
                TokenType::Effect => {
                    effects.push(self.parse_effect_def()?);
                }
                TokenType::Precondition => {
                    preconditions.push(self.parse_named_string()?);
                }
                TokenType::Postcondition => {
                    postconditions.push(self.parse_named_string()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in operation: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(OperationDef {
            location: loc,
            name,
            doc_comment,
            context,
            inputs,
            output,
            errors,
            requires,
            effects,
            preconditions,
            postconditions,
        })
    }

    /// Parse input definition: `input name: type = default`
    /// Input names can be keywords (e.g., `input state: LexerState`)
    fn parse_input_def(&mut self) -> Result<FieldDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Input)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let field_type = self.parse_type_expr()?;

        let default = if self.current_type() == TokenType::Equals {
            self.advance();
            Some(self.parse_literal()?)
        } else {
            None
        };

        Ok(FieldDef {
            location: loc,
            name,
            field_type,
            default,
            constraints: Vec::new(),
        })
    }

    /// Parse effect definition: `effect creates User`
    fn parse_effect_def(&mut self) -> Result<EffectDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Effect)?;

        let kind = match self.current_type() {
            TokenType::Creates => EffectKind::Creates,
            TokenType::Updates => EffectKind::Updates,
            TokenType::Deletes => EffectKind::Deletes,
            TokenType::Emits => EffectKind::Emits,
            TokenType::Calls => EffectKind::Calls,
            _ => {
                return Err(ParseError::unexpected(
                    self.location(),
                    "effect type (creates, updates, deletes, emits, calls)",
                    self.current_type(),
                ))
            }
        };
        self.advance();

        let target = self.expect_name()?;

        Ok(EffectDef {
            location: loc,
            kind,
            target,
        })
    }

    // =========================================================================
    // EVENT
    // =========================================================================

    /// Parse event definition.
    fn parse_event(&mut self, doc_comment: Option<String>) -> Result<EventDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Event)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut fields = Vec::new();
        let mut producer = None;

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Field => {
                    fields.push(self.parse_field_def()?);
                }
                TokenType::Producer => {
                    self.advance();
                    producer = Some(self.expect_name()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in event: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(EventDef {
            location: loc,
            name,
            doc_comment,
            context,
            fields,
            producer,
        })
    }

    // =========================================================================
    // CONFIG
    // =========================================================================

    /// Parse config definition.
    fn parse_config(&mut self, doc_comment: Option<String>) -> Result<ConfigDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Config)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut fields = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Field => {
                    fields.push(self.parse_field_def()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in config: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(ConfigDef {
            location: loc,
            name,
            doc_comment,
            context,
            fields,
        })
    }

    // =========================================================================
    // PORTAL
    // =========================================================================

    /// Parse portal definition.
    fn parse_portal(&mut self, doc_comment: Option<String>) -> Result<PortalDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Portal)?;
        let name = self.expect_name()?;

        // Check for clone
        let base = if self.current_type() == TokenType::Clone {
            self.advance();
            Some(self.expect_name()?)
        } else {
            None
        };

        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut properties = Vec::new();

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                _ => {
                    properties.push(self.parse_portal_property()?);
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(PortalDef {
            location: loc,
            name,
            doc_comment,
            context,
            base,
            properties,
        })
    }

    /// Parse portal property: `direction: input` or `transport: http` or `command: "nvidia-smi"`
    fn parse_portal_property(&mut self) -> Result<PortalProperty, ParseError> {
        let loc = self.location();

        // Property name can be a keyword or identifier
        let name = match self.current_type() {
            TokenType::Direction => {
                self.advance();
                "direction".to_string()
            }
            TokenType::Transport => {
                self.advance();
                "transport".to_string()
            }
            TokenType::Format => {
                self.advance();
                "format".to_string()
            }
            TokenType::Handler => {
                self.advance();
                "handler".to_string()
            }
            TokenType::DataSource => {
                self.advance();
                "data_source".to_string()
            }
            // Portal-specific properties for subprocess transport
            TokenType::Command => {
                self.advance();
                "command".to_string()
            }
            TokenType::Identifier => self.expect_name()?,
            // Allow any keyword as property name
            _ if self.is_keyword_that_can_be_name() => self.expect_name()?,
            _ => {
                return Err(ParseError::unexpected(
                    self.location(),
                    "portal property name",
                    self.current_type(),
                ))
            }
        };

        self.expect(TokenType::Colon)?;

        let value = match self.current_type() {
            TokenType::String => PortalValue::String(self.expect_string()?),
            TokenType::Integer => PortalValue::Integer(self.expect_integer()?),
            TokenType::Identifier => PortalValue::Identifier(self.expect_name()?),
            // Allow keywords as values (e.g., direction: input, transport: http)
            _ if self.is_keyword_that_can_be_name() => {
                PortalValue::Identifier(self.expect_name()?)
            }
            _ => {
                return Err(ParseError::unexpected(
                    self.location(),
                    "portal property value",
                    self.current_type(),
                ))
            }
        };

        Ok(PortalProperty {
            location: loc,
            name,
            value,
        })
    }

    // =========================================================================
    // SURFACE
    // =========================================================================

    /// Parse surface definition.
    /// Supports web, mobile, CLI, and embedded display surfaces.
    fn parse_surface(&mut self, doc_comment: Option<String>) -> Result<SurfaceDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Surface)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut context = None;
        let mut parent = None;
        let mut displays = Vec::new();
        let mut interactions = Vec::new();
        let mut commands = Vec::new();
        let mut arguments = Vec::new();
        let mut flags = Vec::new();
        let mut outputs = Vec::new();
        let mut properties = Vec::new();
        let mut states = Vec::new();
        let mut events = Vec::new();
        let mut help = None;
        let mut accessibility = None;
        let mut mapping = None;

        while self.should_continue_block() {
            match self.current_type() {
                TokenType::Context => {
                    self.advance();
                    context = Some(self.expect_string()?);
                }
                TokenType::Parent => {
                    self.advance();
                    parent = Some(self.expect_name()?);
                }
                TokenType::Display => {
                    displays.push(self.parse_display_def()?);
                }
                TokenType::Interaction => {
                    interactions.push(self.parse_named_string()?);
                }
                TokenType::Command => {
                    commands.push(self.parse_command_def()?);
                }
                TokenType::Argument => {
                    arguments.push(self.parse_argument_def()?);
                }
                TokenType::Flag => {
                    flags.push(self.parse_flag_def()?);
                }
                TokenType::Output => {
                    outputs.push(self.parse_output_def()?);
                }
                TokenType::Property => {
                    properties.push(self.parse_surface_property()?);
                }
                TokenType::State => {
                    states.push(self.parse_state_def()?);
                }
                TokenType::On => {
                    events.push(self.parse_surface_event()?);
                }
                TokenType::Help => {
                    self.advance();
                    help = Some(self.expect_string()?);
                }
                TokenType::Accessibility => {
                    accessibility = Some(self.parse_accessibility_def()?);
                }
                TokenType::Mapping => {
                    mapping = Some(self.parse_mapping_def()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unexpected token in surface: {:?}", self.current_type()),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(SurfaceDef {
            location: loc,
            name,
            doc_comment,
            context,
            parent,
            displays,
            interactions,
            commands,
            arguments,
            flags,
            outputs,
            properties,
            states,
            events,
            help,
            accessibility,
            mapping,
        })
    }

    /// Parse display definition: `display tasks: list<Task>`
    fn parse_display_def(&mut self) -> Result<DisplayDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Display)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let display_type = self.parse_type_expr()?;

        Ok(DisplayDef {
            location: loc,
            name,
            display_type,
        })
    }

    /// Parse command definition: `command "list"`
    fn parse_command_def(&mut self) -> Result<CommandDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Command)?;
        let name = self.expect_string()?;

        Ok(CommandDef { location: loc, name })
    }

    /// Parse surface property: `property layout: vertical`
    fn parse_surface_property(&mut self) -> Result<SurfaceProperty, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Property)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let value = self.expect_name()?;

        Ok(SurfaceProperty {
            location: loc,
            name,
            value,
        })
    }

    /// Parse state definition: `state loading: bool = false` or `state selected: Task?`
    fn parse_state_def(&mut self) -> Result<FieldDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::State)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let field_type = self.parse_type_expr()?;

        let default = if self.current_type() == TokenType::Equals {
            self.advance();
            Some(self.parse_literal()?)
        } else {
            None
        };

        Ok(FieldDef {
            location: loc,
            name,
            field_type,
            default,
            constraints: Vec::new(),
        })
    }

    /// Parse surface event: `on complete { task_id: uuid }` or `on submit { email: string, password: string }`
    fn parse_surface_event(&mut self) -> Result<SurfaceEvent, ParseError> {
        let loc = self.location();
        self.expect(TokenType::On)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while self.should_continue_block() {
            // Fields in surface events don't have 'field' keyword
            let field_loc = self.location();
            let field_name = self.expect_name()?;
            self.expect(TokenType::Colon)?;
            let field_type = self.parse_type_expr()?;
            fields.push(FieldDef {
                location: field_loc,
                name: field_name,
                field_type,
                default: None,
                constraints: Vec::new(),
            });
            // Skip comma separator if present
            if self.current_type() == TokenType::Comma {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(SurfaceEvent {
            location: loc,
            name,
            fields,
        })
    }

    /// Parse CLI argument definition: `argument attractor: string?`
    fn parse_argument_def(&mut self) -> Result<ArgumentDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Argument)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let arg_type = self.parse_type_expr()?;

        Ok(ArgumentDef {
            location: loc,
            name,
            arg_type,
        })
    }

    /// Parse CLI flag definition: `flag verbose: bool`
    fn parse_flag_def(&mut self) -> Result<FlagDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Flag)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let flag_type = self.parse_type_expr()?;

        Ok(FlagDef {
            location: loc,
            name,
            flag_type,
        })
    }

    /// Parse output definition: `output format: table` or `output pin: gpio_2`
    fn parse_output_def(&mut self) -> Result<OutputDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Output)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;
        let value = self.expect_name()?;

        Ok(OutputDef {
            location: loc,
            name,
            value,
        })
    }

    /// Parse accessibility block: `accessibility { label "..." hint "..." role: main }`
    fn parse_accessibility_def(&mut self) -> Result<AccessibilityDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Accessibility)?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut label = None;
        let mut hint = None;
        let mut role = None;

        while self.should_continue_block() {

            // Accessibility properties are "label"/"hint" (strings) or "role:" (identifier)
            let prop_name = self.expect_name()?;
            match prop_name.as_str() {
                "label" => {
                    label = Some(self.expect_string()?);
                }
                "hint" => {
                    hint = Some(self.expect_string()?);
                }
                "role" => {
                    self.expect(TokenType::Colon)?;
                    role = Some(self.expect_name()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!("Unknown accessibility property: {}", prop_name),
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(AccessibilityDef {
            location: loc,
            label,
            hint,
            role,
        })
    }

    /// Parse mapping block: `mapping { idle: off, running: blink_slow }`
    fn parse_mapping_def(&mut self) -> Result<MappingDef, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Mapping)?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut entries = Vec::new();

        while self.should_continue_block() {

            let entry_loc = self.location();
            let key = self.expect_name()?;
            self.expect(TokenType::Colon)?;
            let value = self.expect_name()?;

            entries.push(MappingEntry {
                location: entry_loc,
                key,
                value,
            });

            // Skip comma separator if present
            if self.current_type() == TokenType::Comma {
                self.advance();
            }
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(MappingDef {
            location: loc,
            entries,
        })
    }

    // =========================================================================
    // TYPES
    // =========================================================================

    /// Parse a type expression, including the optional `?` suffix.
    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let loc = self.location();
        let mut base_type = self.parse_base_type()?;

        // Check for T? sugar (optional suffix)
        if self.current_type() == TokenType::Question {
            self.advance();
            base_type = TypeExpr::Optional {
                location: loc,
                inner: Box::new(base_type),
            };
        }

        Ok(base_type)
    }

    /// Parse a base type expression (without the optional `?` suffix).
    fn parse_base_type(&mut self) -> Result<TypeExpr, ParseError> {
        let loc = self.location();

        // Primitive types
        let primitive = match self.current_type() {
            TokenType::StringType => Some("string"),
            TokenType::Bytes => Some("bytes"),
            TokenType::Char => Some("char"),
            TokenType::Bool => Some("bool"),
            TokenType::U8 => Some("u8"),
            TokenType::U16 => Some("u16"),
            TokenType::U32 => Some("u32"),
            TokenType::U64 => Some("u64"),
            TokenType::U128 => Some("u128"),
            TokenType::I8 => Some("i8"),
            TokenType::I16 => Some("i16"),
            TokenType::I32 => Some("i32"),
            TokenType::I64 => Some("i64"),
            TokenType::I128 => Some("i128"),
            TokenType::F32 => Some("f32"),
            TokenType::F64 => Some("f64"),
            TokenType::Uuid => Some("uuid"),
            TokenType::Ulid => Some("ulid"),
            TokenType::Timestamp => Some("timestamp"),
            TokenType::Duration => Some("duration"),
            TokenType::Size => Some("size"),
            TokenType::Any => Some("any"),
            TokenType::Void => Some("void"),
            TokenType::None => Some("None"),
            _ => None,
        };

        if let Some(name) = primitive {
            self.advance();
            return Ok(TypeExpr::Primitive {
                location: loc,
                name: name.to_string(),
            });
        }

        // decimal(precision, scale)
        if self.current_type() == TokenType::DecimalType {
            self.advance();
            if self.current_type() == TokenType::LParen {
                self.advance();
                let precision = self.expect_integer()? as u32;
                self.expect(TokenType::Comma)?;
                let scale = self.expect_integer()? as u32;
                self.expect(TokenType::RParen)?;
                return Ok(TypeExpr::DecimalPrecision {
                    location: loc,
                    precision,
                    scale,
                });
            }
            return Ok(TypeExpr::Primitive {
                location: loc,
                name: "decimal".to_string(),
            });
        }

        // Modifier types: list<T>, set<T>, map<K,V>, optional<T>
        let modifier = match self.current_type() {
            TokenType::List => Some("list"),
            TokenType::Set => Some("set"),
            TokenType::Map => Some("map"),
            TokenType::Optional => Some("optional"),
            TokenType::Ptr => Some("ptr"),
            _ => None,
        };

        if let Some(mod_name) = modifier {
            self.advance();
            self.expect(TokenType::LAngle)?;
            let mut args = vec![self.parse_type_expr()?];
            while self.current_type() == TokenType::Comma {
                self.advance();
                args.push(self.parse_type_expr()?);
            }
            self.expect(TokenType::RAngle)?;
            return Ok(TypeExpr::Modifier {
                location: loc,
                modifier: mod_name.to_string(),
                args,
            });
        }

        // oneof<...> - can be type union or literal union
        if self.current_type() == TokenType::Oneof {
            self.advance();
            self.expect(TokenType::LAngle)?;

            // Check first element to determine type
            if self.current_type() == TokenType::String {
                // Literal union
                let mut values = vec![self.expect_string()?];
                while self.current_type() == TokenType::Comma {
                    self.advance();
                    values.push(self.expect_string()?);
                }
                self.expect(TokenType::RAngle)?;
                return Ok(TypeExpr::LiteralUnion {
                    location: loc,
                    values,
                });
            } else {
                // Type union
                let mut types = vec![self.parse_type_expr()?];
                while self.current_type() == TokenType::Comma {
                    self.advance();
                    types.push(self.parse_type_expr()?);
                }
                self.expect(TokenType::RAngle)?;
                return Ok(TypeExpr::TypeUnion {
                    location: loc,
                    types,
                });
            }
        }

        // ref<Symbol>
        if self.current_type() == TokenType::Ref {
            self.advance();
            self.expect(TokenType::LAngle)?;
            let target = self.parse_dotted_name()?;
            self.expect(TokenType::RAngle)?;
            return Ok(TypeExpr::Ref {
                location: loc,
                target,
            });
        }

        // Inline struct: { field: type, ... }
        if self.current_type() == TokenType::LBrace {
            self.advance();
            self.skip_newlines();

            let mut fields = Vec::new();
            while self.current_type() != TokenType::RBrace {
                let field_loc = self.location();
                let field_name = self.expect_name()?;
                self.expect(TokenType::Colon)?;
                let field_type = self.parse_type_expr()?;
                fields.push(FieldDef {
                    location: field_loc,
                    name: field_name,
                    field_type,
                    default: None,
                    constraints: Vec::new(),
                });
                if self.current_type() == TokenType::Comma {
                    self.advance();
                }
                self.skip_newlines();
            }
            self.expect(TokenType::RBrace)?;
            return Ok(TypeExpr::InlineStruct {
                location: loc,
                fields,
            });
        }

        // Identifier as type reference (custom type or model name)
        if self.current_type() == TokenType::Identifier {
            let name = self.parse_dotted_name()?;
            return Ok(TypeExpr::Primitive {
                location: loc,
                name,
            });
        }

        if self.is_keyword_that_can_be_name() {
            let name = self.parse_dotted_identifier()?;
            return Ok(TypeExpr::Primitive { location: loc, name });
        }

        Err(ParseError::unexpected(
            self.location(),
            "type expression",
            self.current_type(),
        ))
    }

    // =========================================================================
    // HELPERS
    // =========================================================================

    /// Parse a literal value.
    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        match self.current_type() {
            TokenType::String => Ok(Literal::String(self.expect_string()?)),
            TokenType::Integer => Ok(Literal::Integer(self.expect_integer()?)),
            TokenType::Decimal => {
                if let TokenValue::Decimal(d) = self.current().value {
                    self.advance();
                    Ok(Literal::Decimal(d))
                } else {
                    Err(ParseError::at(self.location(), "Expected decimal"))
                }
            }
            TokenType::True => {
                self.advance();
                Ok(Literal::Boolean(true))
            }
            TokenType::False => {
                self.advance();
                Ok(Literal::Boolean(false))
            }
            TokenType::Now => {
                self.advance();
                Ok(Literal::Now)
            }
            TokenType::None => {
                self.advance();
                Ok(Literal::None)
            }
            // Identifier literal - could be plain identifier, keyword, or EnumName.Variant
            TokenType::Identifier => {
                let first = self.expect_name()?;
                if self.current_type() == TokenType::Dot {
                    // EnumName.Variant syntax
                    self.advance();
                    let variant = self.expect_name()?;
                    Ok(Literal::EnumVariant {
                        enum_name: first,
                        variant,
                    })
                } else {
                    Ok(Literal::Identifier(first))
                }
            }
            // Keywords can be used as identifiers in literals (e.g., NAMESPACE, IMPORT)
            _ if self.is_keyword_that_can_be_name() => {
                let first = self.expect_name()?;
                if self.current_type() == TokenType::Dot {
                    // EnumName.Variant syntax where EnumName is a keyword
                    self.advance();
                    let variant = self.expect_name()?;
                    Ok(Literal::EnumVariant {
                        enum_name: first,
                        variant,
                    })
                } else {
                    Ok(Literal::Identifier(first))
                }
            }
            TokenType::LBrace => {
                // Could be map or set literal
                // Map: { "key": value, ... } (has colon after first string)
                // Set: { value, value, ... } (comma-separated values)
                self.advance();
                self.skip_newlines();

                if self.current_type() == TokenType::RBrace {
                    // Empty set
                    self.advance();
                    return Ok(Literal::Set(Vec::new()));
                }

                // Parse first element to determine if map or set
                let first_val = self.parse_literal()?;

                if self.current_type() == TokenType::Colon {
                    // It's a map - first_val is the key
                    self.advance();
                    let key = match first_val {
                        Literal::String(s) => s,
                        _ => return Err(ParseError::at(self.location(), "Map keys must be strings")),
                    };
                    let value = self.parse_literal()?;
                    let mut pairs = vec![(key, value)];
                    while self.current_type() == TokenType::Comma {
                        self.advance();
                        self.skip_newlines();
                        if self.current_type() == TokenType::RBrace {
                            break;
                        }
                        let k = self.expect_string()?;
                        self.expect(TokenType::Colon)?;
                        let v = self.parse_literal()?;
                        pairs.push((k, v));
                        self.skip_newlines();
                    }
                    self.expect(TokenType::RBrace)?;
                    Ok(Literal::Map(pairs))
                } else {
                    // It's a set
                    let mut values = vec![first_val];
                    while self.current_type() == TokenType::Comma {
                        self.advance();
                        self.skip_newlines();
                        if self.current_type() == TokenType::RBrace {
                            break;
                        }
                        values.push(self.parse_literal()?);
                        self.skip_newlines();
                    }
                    self.expect(TokenType::RBrace)?;
                    Ok(Literal::Set(values))
                }
            }
            _ => Err(ParseError::unexpected(
                self.location(),
                "literal value",
                self.current_type(),
            )),
        }
    }
}

/// Parse Armature source code into an AST.
pub fn parse(source: &str, _filename: &str) -> Result<File, ParseError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse_file()
}

/// Detect file type based on extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Standard .arm specification file
    Arm,
    /// Project configuration .ac file
    Ac,
}

impl FileKind {
    /// Detect file kind from filename extension.
    pub fn from_filename(filename: &str) -> Self {
        if filename.ends_with(".ac") {
            FileKind::Ac
        } else {
            FileKind::Arm
        }
    }
}

/// Parse any Armature file (.arm or .ac) and return the appropriate AST.
pub fn parse_file(source: &str, filename: &str) -> Result<ParsedFile, ParseError> {
    let kind = FileKind::from_filename(filename);
    match kind {
        FileKind::Arm => {
            let file = parse(source, filename)?;
            Ok(ParsedFile::Arm(file))
        }
        FileKind::Ac => {
            let file = parse_ac(source, filename)?;
            Ok(ParsedFile::Ac(file))
        }
    }
}

/// Parse .ac project configuration file.
pub fn parse_ac(source: &str, _filename: &str) -> Result<AcFile, ParseError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse_ac_file()
}

impl Parser {
    // =========================================================================
    // .AC FILE PARSING
    // =========================================================================

    /// Parse .ac project configuration file.
    pub fn parse_ac_file(&mut self) -> Result<AcFile, ParseError> {
        let loc = self.location();
        self.skip_newlines();

        let mut project = None;
        let mut platforms = Vec::new();
        let mut aliases = Vec::new();
        let mut imports = Vec::new();
        let mut build = None;

        while self.current_type() != TokenType::Eof {
            self.skip_newlines();
            if self.current_type() == TokenType::Eof {
                break;
            }

            match self.current_type() {
                TokenType::Project => {
                    if project.is_some() {
                        return Err(ParseError::at(
                            self.location(),
                            "Duplicate project block - only one project block allowed per .ac file",
                        ));
                    }
                    project = Some(self.parse_project_block()?);
                }
                TokenType::Platform => {
                    platforms.push(self.parse_platform_block()?);
                }
                TokenType::Alias => {
                    aliases.push(self.parse_alias_decl()?);
                }
                TokenType::Import => {
                    imports.push(self.parse_import()?);
                }
                TokenType::Build => {
                    if build.is_some() {
                        return Err(ParseError::at(
                            self.location(),
                            "Duplicate build block - only one build block allowed per .ac file",
                        ));
                    }
                    build = Some(self.parse_build_block()?);
                }
                _ => {
                    return Err(ParseError::at(
                        self.location(),
                        format!(
                            "Unexpected token in .ac file: {:?}. Expected project, platform, alias, import, or build",
                            self.current_type()
                        ),
                    ));
                }
            }
            self.skip_newlines();
        }

        Ok(AcFile {
            location: loc,
            project,
            platforms,
            aliases,
            imports,
            build,
        })
    }

    /// Parse project block: `project "Name" { description "..." version "1.0.0" }`
    fn parse_project_block(&mut self) -> Result<ProjectBlock, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Project)?;
        let name = self.expect_string()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();

        while self.should_continue_block() {

            let field = self.parse_project_field()?;
            fields.push(field);
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(ProjectBlock {
            location: loc,
            name,
            fields,
        })
    }

    /// Parse project field: `description "text"` or `architecture { ... }`
    fn parse_project_field(&mut self) -> Result<ProjectField, ParseError> {
        let loc = self.location();
        let name = self.expect_name()?;

        let value = if self.current_type() == TokenType::LBrace {
            // Nested block
            self.advance();
            self.skip_newlines();
            let mut nested_fields = Vec::new();
            while self.should_continue_block() {
                nested_fields.push(self.parse_project_field()?);
                self.skip_newlines();
            }
            self.expect(TokenType::RBrace)?;
            ProjectValue::Block(nested_fields)
        } else {
            // String value
            ProjectValue::String(self.expect_string()?)
        };

        Ok(ProjectField {
            location: loc,
            name,
            value,
        })
    }

    /// Parse platform block: `platform rust { naming: snake_case optional: Option<T> }`
    fn parse_platform_block(&mut self) -> Result<PlatformBlock, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Platform)?;
        let name = self.expect_name()?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();

        while self.should_continue_block() {

            let field = self.parse_platform_field()?;
            fields.push(field);
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(PlatformBlock {
            location: loc,
            name,
            fields,
        })
    }

    /// Parse platform field: `naming: camelCase` or `optional: T?` or `structure { ... }`
    fn parse_platform_field(&mut self) -> Result<PlatformField, ParseError> {
        let loc = self.location();
        let name = self.expect_name()?;
        self.expect(TokenType::Colon)?;

        let value = if self.current_type() == TokenType::LBrace {
            // Nested block
            self.advance();
            self.skip_newlines();
            let mut nested_fields = Vec::new();
            while self.should_continue_block() {
                nested_fields.push(self.parse_platform_field()?);
                self.skip_newlines();
            }
            self.expect(TokenType::RBrace)?;
            PlatformValue::Block(nested_fields)
        } else if self.current_type() == TokenType::String {
            // String value
            PlatformValue::String(self.expect_string()?)
        } else if self.is_type_start() {
            // Type expression (for optional: T?, result: Result<T>, etc.)
            PlatformValue::TypeExpr(self.parse_type_expr()?)
        } else {
            // Identifier value
            PlatformValue::Identifier(self.expect_name()?)
        };

        Ok(PlatformField {
            location: loc,
            name,
            value,
        })
    }

    /// Check if current token can start a type expression.
    fn is_type_start(&self) -> bool {
        matches!(
            self.current_type(),
            TokenType::StringType
                | TokenType::Bytes
                | TokenType::Char
                | TokenType::Bool
                | TokenType::U8
                | TokenType::U16
                | TokenType::U32
                | TokenType::U64
                | TokenType::U128
                | TokenType::I8
                | TokenType::I16
                | TokenType::I32
                | TokenType::I64
                | TokenType::I128
                | TokenType::F32
                | TokenType::F64
                | TokenType::Uuid
                | TokenType::Ulid
                | TokenType::Timestamp
                | TokenType::Duration
                | TokenType::Size
                | TokenType::Any
                | TokenType::Void
                | TokenType::None
                | TokenType::DecimalType
                | TokenType::List
                | TokenType::Set
                | TokenType::Map
                | TokenType::Optional
                | TokenType::Ptr
                | TokenType::Oneof
                | TokenType::Ref
                | TokenType::LBrace
        )
    }

    /// Parse alias declaration: `alias UserId = uuid` or `alias Email = string [format: email]`
    fn parse_alias_decl(&mut self) -> Result<AliasDecl, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Alias)?;
        let name = self.expect_name()?;
        self.expect(TokenType::Equals)?;
        let target_type = self.parse_type_expr()?;

        // Optional constraints
        let constraints = if self.current_type() == TokenType::LBracket {
            self.parse_constraints()?
        } else {
            Vec::new()
        };

        Ok(AliasDecl {
            location: loc,
            name,
            target_type,
            constraints,
        })
    }

    /// Parse build block: `build { output: "dist/" mcp_db: ".armature/spec.db" }`
    fn parse_build_block(&mut self) -> Result<BuildBlock, ParseError> {
        let loc = self.location();
        self.expect(TokenType::Build)?;
        self.expect(TokenType::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();

        while self.should_continue_block() {

            let field_loc = self.location();
            let name = self.expect_name()?;
            self.expect(TokenType::Colon)?;
            let value = self.expect_string()?;

            fields.push(BuildField {
                location: field_loc,
                name,
                value,
            });
            self.skip_newlines();
        }

        self.expect(TokenType::RBrace)?;

        Ok(BuildBlock {
            location: loc,
            fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_file() {
        let result = parse("", "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.namespace.is_none());
        assert!(file.imports.is_empty());
        assert!(file.symbols.is_empty());
    }

    #[test]
    fn test_namespace() {
        let result = parse("namespace app.auth\n", "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.namespace.unwrap().name, "app.auth");
    }

    #[test]
    fn test_import() {
        let result = parse("import \"./models.arm\"\n", "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].path, "./models.arm");
    }

    #[test]
    fn test_simple_model() {
        let source = r#"
model User {
    context "A user in the system"
    field id: uuid
    field email: string [unique]
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.symbols.len(), 1);
        if let SymbolDef::Model(m) = &file.symbols[0] {
            assert_eq!(m.name, "User");
            assert_eq!(m.context.as_ref().unwrap(), "A user in the system");
            assert_eq!(m.fields.len(), 2);
        } else {
            panic!("Expected model");
        }
    }

    #[test]
    fn test_simple_enum() {
        let source = r#"
enum TokenType {
    context "All token types"
    variant EOF
    variant STRING
    variant INTEGER
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.symbols.len(), 1);
        if let SymbolDef::Enum(e) = &file.symbols[0] {
            assert_eq!(e.name, "TokenType");
            assert_eq!(e.variants.len(), 3);
            // Simple enum variants have no data type
            assert!(e.variants[0].data_type.is_none());
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_sum_type_enum() {
        let source = r#"
enum SymbolSpec {
    context "Tagged union for different symbol kinds"
    variant Model(ModelSpec)
    variant Operation(OperationSpec)
    variant Simple
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.symbols.len(), 1);
        if let SymbolDef::Enum(e) = &file.symbols[0] {
            assert_eq!(e.name, "SymbolSpec");
            assert_eq!(e.variants.len(), 3);

            // First variant: Model(ModelSpec)
            assert_eq!(e.variants[0].name, "Model");
            assert!(e.variants[0].data_type.is_some());
            if let Some(TypeExpr::Primitive { name, .. }) = &e.variants[0].data_type {
                assert_eq!(name, "ModelSpec");
            } else {
                panic!("Expected primitive type for Model variant");
            }

            // Second variant: Operation(OperationSpec)
            assert_eq!(e.variants[1].name, "Operation");
            assert!(e.variants[1].data_type.is_some());

            // Third variant: Simple (no data)
            assert_eq!(e.variants[2].name, "Simple");
            assert!(e.variants[2].data_type.is_none());
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_sum_type_with_collection() {
        let source = r#"
enum Container {
    variant Single(User)
    variant Multiple(list<User>)
    variant Mapped(map<string, User>)
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        if let SymbolDef::Enum(e) = &file.symbols[0] {
            // Multiple(list<User>)
            assert!(e.variants[1].data_type.is_some());
            if let Some(TypeExpr::Modifier { modifier, args, .. }) = &e.variants[1].data_type {
                assert_eq!(modifier, "list");
                assert_eq!(args.len(), 1);
            } else {
                panic!("Expected modifier type for Multiple variant");
            }

            // Mapped(map<string, User>)
            if let Some(TypeExpr::Modifier { modifier, args, .. }) = &e.variants[2].data_type {
                assert_eq!(modifier, "map");
                assert_eq!(args.len(), 2);
            } else {
                panic!("Expected modifier type for Mapped variant");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_interface_shorthand() {
        let source = r#"
interface UserStore {
    context "Storage for users"
    method findById(id: uuid): User?
    method insert(user: User): uuid
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        if let SymbolDef::Interface(i) = &file.symbols[0] {
            assert_eq!(i.name, "UserStore");
            assert_eq!(i.methods.len(), 2);
            assert_eq!(i.methods[0].name, "findById");
            assert_eq!(i.methods[0].params.len(), 1);
        } else {
            panic!("Expected interface");
        }
    }

    #[test]
    fn test_operation() {
        let source = r#"
operation CreateUser {
    context "Create a new user"
    input email: string
    input password: string
    output: User
    error email_taken "Email already exists"
    requires UserStore
    effect creates User
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        if let SymbolDef::Operation(o) = &file.symbols[0] {
            assert_eq!(o.name, "CreateUser");
            assert_eq!(o.inputs.len(), 2);
            assert!(o.output.is_some());
            assert_eq!(o.errors.len(), 1);
            assert_eq!(o.requires.len(), 1);
            assert_eq!(o.effects.len(), 1);
        } else {
            panic!("Expected operation");
        }
    }

    #[test]
    fn test_type_expr_optional_sugar() {
        let source = r#"
model Test {
    field maybe: User?
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        if let SymbolDef::Model(m) = &file.symbols[0] {
            if let TypeExpr::Optional { inner, .. } = &m.fields[0].field_type {
                if let TypeExpr::Primitive { name, .. } = inner.as_ref() {
                    assert_eq!(name, "User");
                } else {
                    panic!("Expected primitive inside optional");
                }
            } else {
                panic!("Expected optional type");
            }
        } else {
            panic!("Expected model");
        }
    }

    #[test]
    fn test_oneof_literal() {
        let source = r#"
model Doc {
    field status: oneof<"draft", "published", "archived">
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        if let SymbolDef::Model(m) = &file.symbols[0] {
            if let TypeExpr::LiteralUnion { values, .. } = &m.fields[0].field_type {
                assert_eq!(values, &vec!["draft", "published", "archived"]);
            } else {
                panic!("Expected literal union");
            }
        } else {
            panic!("Expected model");
        }
    }

    #[test]
    fn test_cli_surface() {
        let source = r#"
surface ListCommand {
    context "List available items"
    command "list"
    argument item: string?
    flag verbose: bool
    flag help: bool
    output format: table
    help "List all available items"
    accessibility {
        label "List command"
        hint "Lists items in the system"
        role: main
    }
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok());
        let file = result.unwrap();
        if let SymbolDef::Surface(s) = &file.symbols[0] {
            assert_eq!(s.name, "ListCommand");
            assert_eq!(s.arguments.len(), 1);
            assert_eq!(s.arguments[0].name, "item");
            assert_eq!(s.flags.len(), 2);
            assert_eq!(s.flags[0].name, "verbose");
            assert_eq!(s.flags[1].name, "help");
            assert_eq!(s.outputs.len(), 1);
            assert_eq!(s.outputs[0].name, "format");
            assert_eq!(s.outputs[0].value, "table");
            assert_eq!(s.help.as_ref().unwrap(), "List all available items");
            assert!(s.accessibility.is_some());
            let a = s.accessibility.as_ref().unwrap();
            assert_eq!(a.label.as_ref().unwrap(), "List command");
            assert_eq!(a.role.as_ref().unwrap(), "main");
        } else {
            panic!("Expected surface");
        }
    }

    #[test]
    fn test_embedded_surface_mapping() {
        let source = r#"
surface StatusLED {
    context "LED status indicator for embedded device"
    state system_state: string
    output pin: gpio_2
    output pattern: led_pattern
    mapping {
        idle: off
        running: blink_slow
        error: blink_fast
        complete: solid
    }
}
"#;
        let result = parse(source, "<test>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();
        if let SymbolDef::Surface(s) = &file.symbols[0] {
            assert_eq!(s.name, "StatusLED");
            assert!(s.mapping.is_some(), "Expected mapping block");
            let m = s.mapping.as_ref().unwrap();
            assert_eq!(m.entries.len(), 4);
            assert_eq!(m.entries[0].key, "idle");
            assert_eq!(m.entries[0].value, "off");
            assert_eq!(m.entries[1].key, "running");
            assert_eq!(m.entries[1].value, "blink_slow");
            assert_eq!(m.entries[2].key, "error");
            assert_eq!(m.entries[2].value, "blink_fast");
            assert_eq!(m.entries[3].key, "complete");
            assert_eq!(m.entries[3].value, "solid");
        } else {
            panic!("Expected surface");
        }
    }

    // =========================================================================
    // .AC FILE PARSING TESTS
    // =========================================================================

    #[test]
    fn test_empty_ac_file() {
        let result = parse_ac("", "<test.ac>");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.project.is_none());
        assert!(file.platforms.is_empty());
        assert!(file.aliases.is_empty());
        assert!(file.imports.is_empty());
        assert!(file.build.is_none());
    }

    #[test]
    fn test_ac_project_block() {
        let source = r#"
project "MyProject" {
    description "A test project"
    version "1.0.0"
    context "Project context description"
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        let project = file.project.expect("Expected project block");
        assert_eq!(project.name, "MyProject");
        assert_eq!(project.get_string("description"), Some("A test project"));
        assert_eq!(project.get_string("version"), Some("1.0.0"));
        assert_eq!(project.get_string("context"), Some("Project context description"));
    }

    #[test]
    fn test_ac_project_with_architecture() {
        let source = r#"
project "ArchProject" {
    description "Project with architecture block"
    architecture {
        pattern "MVVM"
        layers "presentation, domain, data"
    }
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        let project = file.project.expect("Expected project block");
        let arch = project.get_field("architecture").expect("Expected architecture field");
        if let ProjectValue::Block(fields) = arch {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "pattern");
            if let ProjectValue::String(s) = &fields[0].value {
                assert_eq!(s, "MVVM");
            } else {
                panic!("Expected string value for pattern");
            }
        } else {
            panic!("Expected block value for architecture");
        }
    }

    #[test]
    fn test_ac_platform_block() {
        let source = r#"
platform rust {
    naming: snake_case
    optional: optional<T>
    models: "src/models/"
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        assert_eq!(file.platforms.len(), 1);
        let platform = &file.platforms[0];
        assert_eq!(platform.name, "rust");
        assert_eq!(platform.fields.len(), 3);

        // Check identifier value
        if let Some(PlatformValue::Identifier(id)) = platform.get_field("naming") {
            assert_eq!(id, "snake_case");
        } else {
            panic!("Expected identifier for naming");
        }

        // Check string value
        if let Some(PlatformValue::String(s)) = platform.get_field("models") {
            assert_eq!(s, "src/models/");
        } else {
            panic!("Expected string for models");
        }
    }

    #[test]
    fn test_ac_multiple_platforms() {
        let source = r#"
platform rust {
    naming: snake_case
}

platform typescript {
    naming: camelCase
}

platform kotlin {
    naming: camelCase
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        assert_eq!(file.platforms.len(), 3);
        assert_eq!(file.platforms[0].name, "rust");
        assert_eq!(file.platforms[1].name, "typescript");
        assert_eq!(file.platforms[2].name, "kotlin");
    }

    #[test]
    fn test_ac_alias_simple() {
        let source = r#"
alias UserId = uuid
alias Email = string
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        assert_eq!(file.aliases.len(), 2);
        assert_eq!(file.aliases[0].name, "UserId");
        if let TypeExpr::Primitive { name, .. } = &file.aliases[0].target_type {
            assert_eq!(name, "uuid");
        } else {
            panic!("Expected primitive type for UserId");
        }
    }

    #[test]
    fn test_ac_alias_with_constraints() {
        let source = r#"
alias Email = string [format: email, max: 255]
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        assert_eq!(file.aliases.len(), 1);
        assert_eq!(file.aliases[0].name, "Email");
        assert_eq!(file.aliases[0].constraints.len(), 2);
        assert_eq!(file.aliases[0].constraints[0].name, "format");
        assert_eq!(file.aliases[0].constraints[1].name, "max");
    }

    #[test]
    fn test_ac_build_block() {
        let source = r#"
build {
    output: "dist/"
    mcp_db: ".armature/spec.db"
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        let build = file.build.expect("Expected build block");
        assert_eq!(build.get_field("output"), Some("dist/"));
        assert_eq!(build.get_field("mcp_db"), Some(".armature/spec.db"));
    }

    #[test]
    fn test_ac_imports() {
        let source = r#"
import "./models.arm"
import "./operations.arm"
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        assert_eq!(file.imports.len(), 2);
        assert_eq!(file.imports[0].path, "./models.arm");
        assert_eq!(file.imports[1].path, "./operations.arm");
    }

    #[test]
    fn test_ac_full_file() {
        let source = r#"
project "TaskManager" {
    description "A task management application"
    version "2.0.0"
    context "Helps users organize and track tasks"
}

platform rust {
    naming: snake_case
    optional: optional<T>
}

platform typescript {
    naming: camelCase
    optional: oneof<T, null>
}

alias TaskId = uuid
alias Priority = i32 [min: 1, max: 5]

import "./models.arm"
import "./operations.arm"
import "./events.arm"

build {
    output: "dist/"
    mcp_db: ".armature/spec.db"
}
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let file = result.unwrap();

        // Project
        assert!(file.project.is_some());
        let project = file.project.as_ref().unwrap();
        assert_eq!(project.name, "TaskManager");

        // Platforms
        assert_eq!(file.platforms.len(), 2);
        assert_eq!(file.platforms[0].name, "rust");
        assert_eq!(file.platforms[1].name, "typescript");

        // Aliases
        assert_eq!(file.aliases.len(), 2);
        assert_eq!(file.aliases[0].name, "TaskId");
        assert_eq!(file.aliases[1].name, "Priority");

        // Imports
        assert_eq!(file.imports.len(), 3);

        // Build
        assert!(file.build.is_some());
    }

    #[test]
    fn test_ac_file_kind_detection() {
        assert_eq!(FileKind::from_filename("project.ac"), FileKind::Ac);
        assert_eq!(FileKind::from_filename("models.arm"), FileKind::Arm);
        assert_eq!(FileKind::from_filename("test.ARM"), FileKind::Arm);
        assert_eq!(FileKind::from_filename("path/to/spec.ac"), FileKind::Ac);
    }

    #[test]
    fn test_parse_file_dispatch() {
        // Test .arm dispatch
        let arm_result = parse_file("model User { field id: uuid }", "test.arm");
        assert!(arm_result.is_ok());
        assert!(arm_result.unwrap().is_arm());

        // Test .ac dispatch
        let ac_result = parse_file("project \"Test\" { description \"test\" }", "test.ac");
        assert!(ac_result.is_ok());
        assert!(ac_result.unwrap().is_ac());
    }

    #[test]
    fn test_ac_duplicate_project_error() {
        let source = r#"
project "First" { }
project "Second" { }
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Duplicate project block"));
    }

    #[test]
    fn test_ac_duplicate_build_error() {
        let source = r#"
build { output: "a/" }
build { output: "b/" }
"#;
        let result = parse_ac(source, "<test.ac>");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Duplicate build block"));
    }
}
