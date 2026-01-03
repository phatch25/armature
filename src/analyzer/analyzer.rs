//! Semantic analyzer for Armature AST.
//!
//! The analyzer performs semantic validation on a parsed AST, including:
//! - Duplicate symbol detection
//! - Reference resolution (checking that referenced types exist)
//! - Clone validation (no chained clones, base must exist)
//! - Type modifier arity checking (e.g., list<T> needs 1 arg, map<K,V> needs 2)
//! - Union validation (minimum 2 members)
//! - Signature validation (ref<T> not allowed in method/operation signatures)
//! - Context warnings (missing or too long)

use crate::parser::{File, SymbolDef, TypeExpr};
use crate::types::is_primitive;
use super::errors::{AnalysisError, AnalysisWarning, AnalysisResult};
use super::scope::{SymbolTable, SymbolKind};

/// Maximum recommended context length.
/// Context strings longer than this trigger a warning (soft limit, not error).
const MAX_CONTEXT_LENGTH: usize = 250;

/// Semantic analyzer.
pub struct Analyzer<'a> {
    ast: &'a File,
    symbols: SymbolTable,
    result: AnalysisResult,
}

impl<'a> Analyzer<'a> {
    /// Create a new analyzer for the given AST.
    pub fn new(ast: &'a File) -> Self {
        Self {
            ast,
            symbols: SymbolTable::from_ast(ast),
            result: AnalysisResult::new(),
        }
    }

    /// Run all analysis passes and return the result.
    pub fn analyze(mut self) -> (SymbolTable, AnalysisResult) {
        self.check_duplicate_symbols();
        self.resolve_references();
        self.check_clones();
        self.check_type_modifiers();
        self.check_union_members();
        self.check_ref_in_signatures();
        self.check_context_warnings();
        self.check_unused_symbols();

        (self.symbols, self.result)
    }

    /// Check for duplicate symbol names.
    fn check_duplicate_symbols(&mut self) {
        use std::collections::HashMap;

        let mut seen: HashMap<&str, crate::lexer::SourceLocation> = HashMap::new();

        for symbol in &self.ast.symbols {
            let name = symbol.name();
            let location = symbol.location();

            if let Some(&prev_loc) = seen.get(name) {
                self.result.errors.push(AnalysisError::DuplicateSymbol {
                    location,
                    name: name.to_string(),
                });
                // Also report first occurrence for context
                let _ = prev_loc; // Could add a "first defined here" note
            } else {
                seen.insert(name, location);
            }
        }
    }

    /// Resolve all type references and mark symbols as referenced.
    fn resolve_references(&mut self) {
        for symbol in &self.ast.symbols {
            match symbol {
                SymbolDef::Model(m) => {
                    // Check field types
                    for field in &m.fields {
                        self.resolve_type_expr(&field.field_type);
                    }
                    // Check relation targets
                    for rel in &m.relations {
                        self.resolve_type_expr(&rel.target);
                    }
                }
                SymbolDef::Interface(i) => {
                    for method in &i.methods {
                        for param in &method.params {
                            self.resolve_type_expr(&param.param_type);
                        }
                        if let Some(ret) = &method.return_type {
                            self.resolve_type_expr(ret);
                        }
                    }
                }
                SymbolDef::Operation(o) => {
                    // Input types
                    for input in &o.inputs {
                        self.resolve_type_expr(&input.field_type);
                    }
                    // Output type
                    if let Some(out) = &o.output {
                        self.resolve_type_expr(out);
                    }
                    // Required interfaces
                    for req in &o.requires {
                        if !self.symbols.contains(req) {
                            self.result.errors.push(AnalysisError::RequiredInterfaceNotFound {
                                location: o.location,
                                interface: req.clone(),
                            });
                        } else {
                            if let Some(info) = self.symbols.get_mut(req) {
                                info.is_referenced = true;
                            }
                        }
                    }
                    // Effect targets
                    for effect in &o.effects {
                        if !self.symbols.contains(&effect.target) && !is_primitive(&effect.target) {
                            self.result.errors.push(AnalysisError::EffectTargetNotFound {
                                location: effect.location,
                                target: effect.target.clone(),
                            });
                        } else {
                            if let Some(info) = self.symbols.get_mut(&effect.target) {
                                info.is_referenced = true;
                            }
                        }
                    }
                }
                SymbolDef::Event(e) => {
                    for field in &e.fields {
                        self.resolve_type_expr(&field.field_type);
                    }
                    // Producer
                    if let Some(producer) = &e.producer {
                        if !self.symbols.contains(producer) {
                            self.result.errors.push(AnalysisError::ProducerNotFound {
                                location: e.location,
                                producer: producer.clone(),
                            });
                        } else {
                            if let Some(info) = self.symbols.get_mut(producer) {
                                info.is_referenced = true;
                            }
                        }
                    }
                }
                SymbolDef::Config(c) => {
                    for field in &c.fields {
                        self.resolve_type_expr(&field.field_type);
                    }
                }
                SymbolDef::Portal(p) => {
                    // Handler property
                    for prop in &p.properties {
                        if prop.name == "handler" {
                            if let crate::parser::PortalValue::Identifier(h) = &prop.value {
                                if !self.symbols.contains(h) {
                                    self.result.errors.push(AnalysisError::HandlerNotFound {
                                        location: prop.location,
                                        handler: h.clone(),
                                    });
                                } else {
                                    if let Some(info) = self.symbols.get_mut(h) {
                                        info.is_referenced = true;
                                    }
                                }
                            }
                        }
                    }
                }
                SymbolDef::Surface(s) => {
                    // Parent surface
                    if let Some(parent) = &s.parent {
                        if !self.symbols.contains(parent) {
                            self.result.errors.push(AnalysisError::UndefinedReference {
                                location: s.location,
                                name: parent.clone(),
                            });
                        } else {
                            if let Some(info) = self.symbols.get_mut(parent) {
                                info.is_referenced = true;
                            }
                        }
                    }
                    // Display types
                    for display in &s.displays {
                        self.resolve_type_expr(&display.display_type);
                    }
                    // State types
                    for state in &s.states {
                        self.resolve_type_expr(&state.field_type);
                    }
                    // Event field types
                    for event in &s.events {
                        for field in &event.fields {
                            self.resolve_type_expr(&field.field_type);
                        }
                    }
                }
                SymbolDef::Enum(_) => {
                    // Enums have no references to check
                }
            }
        }
    }

    /// Resolve a type expression and report errors for undefined references.
    fn resolve_type_expr(&mut self, type_expr: &TypeExpr) {
        match type_expr {
            TypeExpr::Primitive { name, location } => {
                // Primitives are always valid, but check for user-defined types
                // that look like primitives but aren't recognized
                if !is_primitive(name) && !self.symbols.contains(name) {
                    self.result.errors.push(AnalysisError::UndefinedReference {
                        location: *location,
                        name: name.clone(),
                    });
                } else if self.symbols.contains(name) {
                    if let Some(info) = self.symbols.get_mut(name) {
                        info.is_referenced = true;
                    }
                }
            }
            TypeExpr::Modifier { args, .. } => {
                for arg in args {
                    self.resolve_type_expr(arg);
                }
            }
            TypeExpr::Ref { target, location } => {
                // For qualified names like "namespace.Symbol", extract the symbol name
                let symbol_name = target.rsplit('.').next().unwrap_or(target);
                if !self.symbols.contains(symbol_name) && !self.symbols.contains(target) {
                    self.result.errors.push(AnalysisError::UndefinedReference {
                        location: *location,
                        name: target.clone(),
                    });
                } else {
                    // Mark the symbol as referenced
                    if let Some(info) = self.symbols.get_mut(symbol_name) {
                        info.is_referenced = true;
                    } else if let Some(info) = self.symbols.get_mut(target) {
                        info.is_referenced = true;
                    }
                }
            }
            TypeExpr::Optional { inner, .. } => {
                self.resolve_type_expr(inner);
            }
            TypeExpr::TypeUnion { types, .. } => {
                for t in types {
                    self.resolve_type_expr(t);
                }
            }
            TypeExpr::LiteralUnion { .. } => {
                // No references
            }
            TypeExpr::InlineStruct { fields, .. } => {
                for field in fields {
                    self.resolve_type_expr(&field.field_type);
                }
            }
            TypeExpr::DecimalPrecision { .. } => {
                // No references
            }
        }
    }

    /// Check clone base validity.
    fn check_clones(&mut self) {
        for symbol in &self.ast.symbols {
            let (name, base, location) = match symbol {
                SymbolDef::Model(m) => {
                    if let Some(base) = &m.base {
                        (m.name.clone(), base.clone(), m.location)
                    } else {
                        continue;
                    }
                }
                SymbolDef::Portal(p) => {
                    if let Some(base) = &p.base {
                        (p.name.clone(), base.clone(), p.location)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            // Check if base exists
            if let Some(base_info) = self.symbols.get(&base) {
                // Mark base as referenced
                let kind = base_info.kind;
                let is_base_clone = base_info.is_clone;

                if let Some(info) = self.symbols.get_mut(&base) {
                    info.is_referenced = true;
                }

                // Check for chained clones
                if is_base_clone {
                    self.result.errors.push(AnalysisError::ChainedClone {
                        location,
                        base: base.clone(),
                    });
                }

                // Check kind matches (model clones model, portal clones portal)
                let expected_kind = match symbol {
                    SymbolDef::Model(_) => SymbolKind::Model,
                    SymbolDef::Portal(_) => SymbolKind::Portal,
                    _ => unreachable!(),
                };

                if kind != expected_kind {
                    self.result.errors.push(AnalysisError::CloneBaseNotFound {
                        location,
                        base: base.clone(),
                    });
                }
            } else {
                self.result.errors.push(AnalysisError::CloneBaseNotFound {
                    location,
                    base,
                });
            }

            // Update the symbol's is_referenced for the base
            let _ = name; // Used for potential "clone of X" tracking
        }
    }

    /// Check type modifier arity.
    fn check_type_modifiers(&mut self) {
        for symbol in &self.ast.symbols {
            match symbol {
                SymbolDef::Model(m) => {
                    for field in &m.fields {
                        self.check_type_expr_modifiers(&field.field_type);
                    }
                    for rel in &m.relations {
                        self.check_type_expr_modifiers(&rel.target);
                    }
                }
                SymbolDef::Interface(i) => {
                    for method in &i.methods {
                        for param in &method.params {
                            self.check_type_expr_modifiers(&param.param_type);
                        }
                        if let Some(ret) = &method.return_type {
                            self.check_type_expr_modifiers(ret);
                        }
                    }
                }
                SymbolDef::Operation(o) => {
                    for input in &o.inputs {
                        self.check_type_expr_modifiers(&input.field_type);
                    }
                    if let Some(out) = &o.output {
                        self.check_type_expr_modifiers(out);
                    }
                }
                SymbolDef::Event(e) => {
                    for field in &e.fields {
                        self.check_type_expr_modifiers(&field.field_type);
                    }
                }
                SymbolDef::Config(c) => {
                    for field in &c.fields {
                        self.check_type_expr_modifiers(&field.field_type);
                    }
                }
                SymbolDef::Surface(s) => {
                    for display in &s.displays {
                        self.check_type_expr_modifiers(&display.display_type);
                    }
                    for state in &s.states {
                        self.check_type_expr_modifiers(&state.field_type);
                    }
                    for event in &s.events {
                        for field in &event.fields {
                            self.check_type_expr_modifiers(&field.field_type);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check a type expression for valid modifier usage.
    fn check_type_expr_modifiers(&mut self, type_expr: &TypeExpr) {
        match type_expr {
            TypeExpr::Modifier { location, modifier, args } => {
                let (expected_min, expected_max) = match modifier.to_lowercase().as_str() {
                    "list" | "set" | "optional" => (1, 1),
                    "map" => (2, 2),
                    "oneof" => (2, usize::MAX),
                    _ => (1, usize::MAX), // Unknown modifiers - allow any arity
                };

                let got = args.len();
                if got < expected_min || got > expected_max {
                    let expected = if expected_min == expected_max {
                        expected_min
                    } else {
                        expected_min // Report minimum
                    };
                    self.result.errors.push(AnalysisError::InvalidModifierArity {
                        location: *location,
                        modifier: modifier.clone(),
                        expected,
                        got,
                    });
                }

                // Recursively check args
                for arg in args {
                    self.check_type_expr_modifiers(arg);
                }
            }
            TypeExpr::Optional { inner, .. } => {
                self.check_type_expr_modifiers(inner);
            }
            TypeExpr::TypeUnion { types, .. } => {
                for t in types {
                    self.check_type_expr_modifiers(t);
                }
            }
            TypeExpr::InlineStruct { fields, .. } => {
                for field in fields {
                    self.check_type_expr_modifiers(&field.field_type);
                }
            }
            _ => {}
        }
    }

    /// Check that union types have at least 2 members.
    /// Design doc: "Must have at least two members - single-element unions are pointless"
    fn check_union_members(&mut self) {
        for symbol in &self.ast.symbols {
            match symbol {
                SymbolDef::Model(m) => {
                    for field in &m.fields {
                        self.check_union_type_expr(&field.field_type);
                    }
                    for rel in &m.relations {
                        self.check_union_type_expr(&rel.target);
                    }
                }
                SymbolDef::Interface(i) => {
                    for method in &i.methods {
                        for param in &method.params {
                            self.check_union_type_expr(&param.param_type);
                        }
                        if let Some(ret) = &method.return_type {
                            self.check_union_type_expr(ret);
                        }
                    }
                }
                SymbolDef::Operation(o) => {
                    for input in &o.inputs {
                        self.check_union_type_expr(&input.field_type);
                    }
                    if let Some(out) = &o.output {
                        self.check_union_type_expr(out);
                    }
                }
                SymbolDef::Event(e) => {
                    for field in &e.fields {
                        self.check_union_type_expr(&field.field_type);
                    }
                }
                SymbolDef::Config(c) => {
                    for field in &c.fields {
                        self.check_union_type_expr(&field.field_type);
                    }
                }
                SymbolDef::Surface(s) => {
                    for display in &s.displays {
                        self.check_union_type_expr(&display.display_type);
                    }
                    for state in &s.states {
                        self.check_union_type_expr(&state.field_type);
                    }
                    for event in &s.events {
                        for field in &event.fields {
                            self.check_union_type_expr(&field.field_type);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check a type expression for valid union member counts.
    fn check_union_type_expr(&mut self, type_expr: &TypeExpr) {
        match type_expr {
            TypeExpr::TypeUnion { location, types } => {
                if types.len() < 2 {
                    self.result.errors.push(AnalysisError::UnionTooFewMembers {
                        location: *location,
                        count: types.len(),
                    });
                }
                // Recursively check nested types
                for t in types {
                    self.check_union_type_expr(t);
                }
            }
            TypeExpr::LiteralUnion { location, values } => {
                if values.len() < 2 {
                    self.result.errors.push(AnalysisError::UnionTooFewMembers {
                        location: *location,
                        count: values.len(),
                    });
                }
            }
            TypeExpr::Modifier { args, .. } => {
                for arg in args {
                    self.check_union_type_expr(arg);
                }
            }
            TypeExpr::Optional { inner, .. } => {
                self.check_union_type_expr(inner);
            }
            TypeExpr::InlineStruct { fields, .. } => {
                for field in fields {
                    self.check_union_type_expr(&field.field_type);
                }
            }
            _ => {}
        }
    }

    /// Check that ref<T> is not used in method/operation signatures.
    /// Design doc: "Signatures = always bare types, never ref<>"
    fn check_ref_in_signatures(&mut self) {
        for symbol in &self.ast.symbols {
            match symbol {
                SymbolDef::Interface(i) => {
                    for method in &i.methods {
                        for param in &method.params {
                            self.check_ref_in_type_expr(&param.param_type, true);
                        }
                        if let Some(ret) = &method.return_type {
                            self.check_ref_in_type_expr(ret, true);
                        }
                    }
                }
                SymbolDef::Operation(o) => {
                    for input in &o.inputs {
                        self.check_ref_in_type_expr(&input.field_type, true);
                    }
                    if let Some(out) = &o.output {
                        self.check_ref_in_type_expr(out, true);
                    }
                }
                _ => {}
            }
        }
    }

    /// Check for ref<T> in a type expression, optionally reporting an error.
    fn check_ref_in_type_expr(&mut self, type_expr: &TypeExpr, report_error: bool) {
        match type_expr {
            TypeExpr::Ref { location, .. } => {
                if report_error {
                    self.result.errors.push(AnalysisError::RefInSignature {
                        location: *location,
                    });
                }
            }
            TypeExpr::Modifier { args, .. } => {
                for arg in args {
                    self.check_ref_in_type_expr(arg, report_error);
                }
            }
            TypeExpr::Optional { inner, .. } => {
                self.check_ref_in_type_expr(inner, report_error);
            }
            TypeExpr::TypeUnion { types, .. } => {
                for t in types {
                    self.check_ref_in_type_expr(t, report_error);
                }
            }
            TypeExpr::InlineStruct { fields, .. } => {
                for field in fields {
                    self.check_ref_in_type_expr(&field.field_type, report_error);
                }
            }
            _ => {}
        }
    }

    /// Check for context warnings.
    fn check_context_warnings(&mut self) {
        for (name, info) in self.symbols.iter() {
            // Skip portals (they don't have context in v2)
            if info.kind == SymbolKind::Portal {
                continue;
            }

            match &info.context {
                None => {
                    self.result.warnings.push(AnalysisWarning::MissingContext {
                        location: info.location,
                        symbol_name: name.clone(),
                    });
                }
                Some(ctx) if ctx.len() > MAX_CONTEXT_LENGTH => {
                    self.result.warnings.push(AnalysisWarning::ContextTooLong {
                        location: info.location,
                        symbol_name: name.clone(),
                        length: ctx.len(),
                        max_length: MAX_CONTEXT_LENGTH,
                    });
                }
                _ => {}
            }
        }
    }

    /// Check for unused symbols.
    fn check_unused_symbols(&mut self) {
        for (name, info) in self.symbols.iter() {
            if !info.is_referenced {
                self.result.warnings.push(AnalysisWarning::UnusedSymbol {
                    location: info.location,
                    symbol_name: name.clone(),
                });
            }
        }
    }
}

/// Analyze a parsed AST and return the result.
pub fn analyze(ast: &File) -> (SymbolTable, AnalysisResult) {
    Analyzer::new(ast).analyze()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_duplicate_symbol() {
        let source = r#"
namespace test

model User {
    field id: uuid
}

model User {
    field name: string
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| matches!(e, AnalysisError::DuplicateSymbol { name, .. } if name == "User")));
    }

    #[test]
    fn test_undefined_reference() {
        let source = r#"
namespace test

model User {
    field profile: ref<Profile>
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| matches!(e, AnalysisError::UndefinedReference { name, .. } if name == "Profile")));
    }

    #[test]
    fn test_valid_reference() {
        let source = r#"
namespace test

model Profile {
    field bio: string
}

model User {
    field profile: ref<Profile>
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, result) = analyze(&ast);
        assert!(result.is_ok());
        // Profile should be marked as referenced
        assert!(symbols.get("Profile").unwrap().is_referenced);
    }

    #[test]
    fn test_missing_context_warning() {
        let source = r#"
namespace test

model User {
    field id: uuid
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        assert!(result.warnings.iter().any(|w| matches!(w, AnalysisWarning::MissingContext { symbol_name, .. } if symbol_name == "User")));
    }

    #[test]
    fn test_context_ok() {
        let source = r#"
namespace test

model User {
    context "Represents a user in the system"
    field id: uuid
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        assert!(!result.warnings.iter().any(|w| matches!(w, AnalysisWarning::MissingContext { symbol_name, .. } if symbol_name == "User")));
    }

    #[test]
    fn test_primitive_types() {
        let source = r#"
namespace test

model User {
    context "Test"
    field id: uuid
    field name: string
    field age: u32
    field balance: f64
    field created: timestamp
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should not have undefined reference errors for primitives
        assert!(!result.errors.iter().any(|e| matches!(e, AnalysisError::UndefinedReference { .. })));
    }

    #[test]
    fn test_modifier_arity() {
        let source = r#"
namespace test

model User {
    context "Test"
    field tags: list<string>
    field metadata: map<string, i32>
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should not have arity errors for valid modifiers
        assert!(!result.errors.iter().any(|e| matches!(e, AnalysisError::InvalidModifierArity { .. })));
    }

    #[test]
    fn test_union_minimum_members() {
        let source = r#"
namespace test

model Doc {
    context "Test"
    field status: oneof<"draft", "published">
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should not have union errors for valid unions
        assert!(!result.errors.iter().any(|e| matches!(e, AnalysisError::UnionTooFewMembers { .. })));
    }

    #[test]
    fn test_ref_in_signature_error() {
        let source = r#"
namespace test

model User {
    context "Test user model"
    field id: uuid
}

interface UserStore {
    context "Storage"
    method findById(id: uuid): ref<User>
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should have ref in signature error
        assert!(result.errors.iter().any(|e| matches!(e, AnalysisError::RefInSignature { .. })));
    }

    #[test]
    fn test_operation_ref_in_output_error() {
        let source = r#"
namespace test

model User {
    context "Test user model"
    field id: uuid
}

operation GetUser {
    context "Get user by id"
    input id: uuid
    output: ref<User>
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should have ref in signature error for operation output
        assert!(result.errors.iter().any(|e| matches!(e, AnalysisError::RefInSignature { .. })));
    }

    #[test]
    fn test_valid_bare_types_in_signatures() {
        let source = r#"
namespace test

model User {
    context "Test user model"
    field id: uuid
}

interface UserStore {
    context "Storage"
    method findById(id: uuid): User?
    method save(user: User): User
}

operation GetUser {
    context "Get user"
    input id: uuid
    output: User?
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (_, result) = analyze(&ast);
        // Should not have any ref in signature errors for bare types
        assert!(!result.errors.iter().any(|e| matches!(e, AnalysisError::RefInSignature { .. })));
    }
}
