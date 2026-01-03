//! Shared type definitions for Armature.
//!
//! This module contains constants and utilities shared between the parser,
//! analyzer, and compiler.

/// Primitive type names that are built into Armature.
/// These types don't need to be resolved as symbols - they're always valid.
pub const PRIMITIVE_TYPES: &[&str] = &[
    // Text types
    "string", "bytes", "char",
    // Boolean
    "bool",
    // Unsigned integers
    "u8", "u16", "u32", "u64", "u128",
    // Signed integers
    "i8", "i16", "i32", "i64", "i128",
    // Floats
    "f32", "f64",
    // Identifiers
    "uuid", "ulid",
    // Temporal
    "timestamp", "duration",
    // Special types
    "decimal",  // decimal(precision, scale)
    "size",     // Platform-specific size type
    "ptr",      // Raw pointer (unsafe)
    "any",      // Escape hatch
    "void",     // No return value
    "none",     // Null/absent value (used in oneofs)
];

/// Check if a type name is a primitive type.
pub fn is_primitive(name: &str) -> bool {
    PRIMITIVE_TYPES.iter().any(|&p| p.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives_are_recognized() {
        assert!(is_primitive("string"));
        assert!(is_primitive("String")); // Case-insensitive
        assert!(is_primitive("uuid"));
        assert!(is_primitive("u32"));
        assert!(is_primitive("timestamp"));
    }

    #[test]
    fn test_non_primitives() {
        assert!(!is_primitive("User"));
        assert!(!is_primitive("MyType"));
        assert!(!is_primitive("list"));  // Modifier, not primitive
        assert!(!is_primitive("ref"));   // Modifier, not primitive
    }
}
