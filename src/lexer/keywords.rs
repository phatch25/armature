//! Keyword lookup table for the Armature lexer.

use phf::phf_map;

use super::token::TokenType;

/// Compile-time perfect hash map for keyword lookup.
pub static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    // Symbol declarations
    "model" => TokenType::Model,
    "enum" => TokenType::Enum,
    "interface" => TokenType::Interface,
    "operation" => TokenType::Operation,
    "event" => TokenType::Event,
    "config" => TokenType::Config,
    "portal" => TokenType::Portal,
    "surface" => TokenType::Surface,

    // Declaration keywords
    "namespace" => TokenType::Namespace,
    "import" => TokenType::Import,
    "project" => TokenType::Project,
    "platform" => TokenType::Platform,
    "build" => TokenType::Build,
    "clone" => TokenType::Clone,
    "alias" => TokenType::Alias,

    // Symbol member keywords
    "field" => TokenType::Field,
    "variant" => TokenType::Variant,
    "relation" => TokenType::Relation,
    "invariant" => TokenType::Invariant,
    "method" => TokenType::Method,
    "input" => TokenType::Input,
    "output" => TokenType::Output,
    "error" => TokenType::Error,
    "requires" => TokenType::Requires,
    "effect" => TokenType::Effect,
    "precondition" => TokenType::Precondition,
    "postcondition" => TokenType::Postcondition,

    // Portal keywords
    "direction" => TokenType::Direction,
    "transport" => TokenType::Transport,
    "format" => TokenType::Format,
    "handler" => TokenType::Handler,
    "data_source" => TokenType::DataSource,

    // Surface keywords
    "display" => TokenType::Display,
    "interaction" => TokenType::Interaction,
    "command" => TokenType::Command,
    "argument" => TokenType::Argument,
    "flag" => TokenType::Flag,
    "property" => TokenType::Property,
    "accessibility" => TokenType::Accessibility,
    "help" => TokenType::Help,
    "state" => TokenType::State,
    "on" => TokenType::On,
    "mapping" => TokenType::Mapping,
    "parent" => TokenType::Parent,

    // Common keywords
    "context" => TokenType::Context,
    "producer" => TokenType::Producer,
    "consumer" => TokenType::Consumer,

    // Primitive types
    "string" => TokenType::StringType,
    "bytes" => TokenType::Bytes,
    "char" => TokenType::Char,
    "bool" => TokenType::Bool,

    // Integer types
    "u8" => TokenType::U8,
    "u16" => TokenType::U16,
    "u32" => TokenType::U32,
    "u64" => TokenType::U64,
    "u128" => TokenType::U128,
    "i8" => TokenType::I8,
    "i16" => TokenType::I16,
    "i32" => TokenType::I32,
    "i64" => TokenType::I64,
    "i128" => TokenType::I128,

    // Float types
    "f32" => TokenType::F32,
    "f64" => TokenType::F64,

    // Special types
    "uuid" => TokenType::Uuid,
    "ulid" => TokenType::Ulid,
    "timestamp" => TokenType::Timestamp,
    "duration" => TokenType::Duration,
    "decimal" => TokenType::DecimalType,
    "size" => TokenType::Size,
    "ptr" => TokenType::Ptr,
    "any" => TokenType::Any,
    "void" => TokenType::Void,

    // Type modifiers
    "list" => TokenType::List,
    "set" => TokenType::Set,
    "map" => TokenType::Map,
    "optional" => TokenType::Optional,
    "oneof" => TokenType::Oneof,
    "ref" => TokenType::Ref,

    // Constraint keywords
    "unique" => TokenType::Unique,
    "pattern" => TokenType::Pattern,
    "values" => TokenType::Values,
    "references" => TokenType::References,
    "min" => TokenType::Min,
    "max" => TokenType::Max,
    "length" => TokenType::Length,
    "computed_by" => TokenType::ComputedBy,
    "custom" => TokenType::Custom,
    "sensitive" => TokenType::Sensitive,
    "immutable" => TokenType::Immutable,
    "readonly" => TokenType::Readonly,
    "derived" => TokenType::Derived,
    "indexed" => TokenType::Indexed,
    "default" => TokenType::Default,
    "primary" => TokenType::Primary,

    // HTTP methods (uppercase)
    "GET" => TokenType::Get,
    "POST" => TokenType::Post,
    "PUT" => TokenType::Put,
    "PATCH" => TokenType::Patch,
    "DELETE" => TokenType::Delete,

    // Relation types
    "has_one" => TokenType::HasOne,
    "has_many" => TokenType::HasMany,
    "belongs_to" => TokenType::BelongsTo,
    "many_to_many" => TokenType::ManyToMany,

    // Effect types
    "creates" => TokenType::Creates,
    "updates" => TokenType::Updates,
    "deletes" => TokenType::Deletes,
    "emits" => TokenType::Emits,
    "calls" => TokenType::Calls,

    // Boolean literals
    "true" => TokenType::True,
    "false" => TokenType::False,

    // Special literals
    "now" => TokenType::Now,
    "none" => TokenType::None,

    // Architecture keywords
    "architecture" => TokenType::Architecture,
    "naming" => TokenType::Naming,
    "structure" => TokenType::Structure,
};

/// Look up a keyword by its string representation.
pub fn lookup(word: &str) -> Option<TokenType> {
    KEYWORDS.get(word).copied()
}
