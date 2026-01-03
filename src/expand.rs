//! Clone expansion module.
//!
//! This module handles the expansion of cloned symbols (e.g., `model X clone Base { ... }`).
//! Clone expansion merges fields, relations, and other properties from the base symbol
//! into the cloned symbol, creating a fully standalone definition.
//!
//! Design doc requirement: "Clone is template expansion, not OOP inheritance.
//! Compiler expands at parse time into standalone symbols."

use crate::parser::{File, SymbolDef, FieldDef, RelationDef, NamedString, PortalProperty};
use std::collections::{HashMap, HashSet};

/// Errors that can occur during clone expansion.
#[derive(Debug, Clone)]
pub enum ExpansionError {
    /// Base symbol not found.
    BaseNotFound { symbol: String, base: String },
    /// Chained clone detected (cloning a clone).
    ChainedClone { symbol: String, base: String },
    /// Type mismatch (model cloning portal, etc.).
    TypeMismatch { symbol: String, base: String, symbol_type: &'static str, base_type: &'static str },
}

impl std::fmt::Display for ExpansionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpansionError::BaseNotFound { symbol, base } =>
                write!(f, "Clone base '{}' not found for symbol '{}'", base, symbol),
            ExpansionError::ChainedClone { symbol, base } =>
                write!(f, "Cannot clone '{}' from '{}' - chained clones are not allowed", symbol, base),
            ExpansionError::TypeMismatch { symbol, base, symbol_type, base_type } =>
                write!(f, "Cannot clone '{}' ({}) from '{}' ({}) - type mismatch", symbol, symbol_type, base, base_type),
        }
    }
}

/// Data to expand a model clone.
struct ModelExpansion {
    index: usize,
    fields: Vec<FieldDef>,
    relations: Vec<RelationDef>,
    invariants: Vec<NamedString>,
    context: Option<String>,
}

/// Data to expand a portal clone.
struct PortalExpansion {
    index: usize,
    properties: Vec<PortalProperty>,
}

/// Expand all clones in an AST file, merging base symbol properties into cloned symbols.
/// Returns the expanded file and any errors encountered.
pub fn expand_clones(mut file: File) -> (File, Vec<ExpansionError>) {
    let mut errors = Vec::new();
    let mut model_expansions = Vec::new();
    let mut portal_expansions = Vec::new();

    // First pass: collect expansion data
    // Build maps from the current state of symbols
    let symbol_names: HashMap<String, usize> = file.symbols.iter()
        .enumerate()
        .map(|(i, s)| (get_symbol_name(s).to_string(), i))
        .collect();

    // Track which symbols are clones (to detect chained clones)
    let clone_bases: HashSet<String> = file.symbols.iter()
        .filter_map(|s| {
            match s {
                SymbolDef::Model(m) => m.base.as_ref().map(|_| m.name.clone()),
                SymbolDef::Portal(p) => p.base.as_ref().map(|_| p.name.clone()),
                _ => None,
            }
        })
        .collect();

    // Collect expansions
    for (index, symbol) in file.symbols.iter().enumerate() {
        match symbol {
            SymbolDef::Model(m) => {
                if let Some(base_name) = &m.base {
                    // Check for chained clone
                    if clone_bases.contains(base_name) {
                        errors.push(ExpansionError::ChainedClone {
                            symbol: m.name.clone(),
                            base: base_name.clone(),
                        });
                        continue;
                    }

                    // Find base symbol
                    let base_index = match symbol_names.get(base_name) {
                        Some(&i) => i,
                        None => {
                            errors.push(ExpansionError::BaseNotFound {
                                symbol: m.name.clone(),
                                base: base_name.clone(),
                            });
                            continue;
                        }
                    };

                    // Get base model
                    let base_model = match &file.symbols[base_index] {
                        SymbolDef::Model(bm) => bm,
                        other => {
                            errors.push(ExpansionError::TypeMismatch {
                                symbol: m.name.clone(),
                                base: base_name.clone(),
                                symbol_type: "model",
                                base_type: symbol_kind_name(other),
                            });
                            continue;
                        }
                    };

                    // Collect fields/relations/invariants to inherit
                    let clone_field_names: HashSet<_> = m.fields.iter()
                        .map(|f| f.name.as_str())
                        .collect();
                    let clone_relation_names: HashSet<_> = m.relations.iter()
                        .map(|r| r.name.as_str())
                        .collect();
                    let clone_invariant_names: HashSet<_> = m.invariants.iter()
                        .map(|i| i.name.as_str())
                        .collect();

                    let inherited_fields: Vec<_> = base_model.fields.iter()
                        .filter(|f| !clone_field_names.contains(f.name.as_str()))
                        .cloned()
                        .collect();
                    let inherited_relations: Vec<_> = base_model.relations.iter()
                        .filter(|r| !clone_relation_names.contains(r.name.as_str()))
                        .cloned()
                        .collect();
                    let inherited_invariants: Vec<_> = base_model.invariants.iter()
                        .filter(|i| !clone_invariant_names.contains(i.name.as_str()))
                        .cloned()
                        .collect();

                    model_expansions.push(ModelExpansion {
                        index,
                        fields: inherited_fields,
                        relations: inherited_relations,
                        invariants: inherited_invariants,
                        context: if m.context.is_none() { base_model.context.clone() } else { None },
                    });
                }
            }
            SymbolDef::Portal(p) => {
                if let Some(base_name) = &p.base {
                    // Check for chained clone
                    if clone_bases.contains(base_name) {
                        errors.push(ExpansionError::ChainedClone {
                            symbol: p.name.clone(),
                            base: base_name.clone(),
                        });
                        continue;
                    }

                    // Find base symbol
                    let base_index = match symbol_names.get(base_name) {
                        Some(&i) => i,
                        None => {
                            errors.push(ExpansionError::BaseNotFound {
                                symbol: p.name.clone(),
                                base: base_name.clone(),
                            });
                            continue;
                        }
                    };

                    // Get base portal
                    let base_portal = match &file.symbols[base_index] {
                        SymbolDef::Portal(bp) => bp,
                        other => {
                            errors.push(ExpansionError::TypeMismatch {
                                symbol: p.name.clone(),
                                base: base_name.clone(),
                                symbol_type: "portal",
                                base_type: symbol_kind_name(other),
                            });
                            continue;
                        }
                    };

                    // Collect properties to inherit
                    let clone_prop_names: HashSet<_> = p.properties.iter()
                        .map(|prop| prop.name.as_str())
                        .collect();

                    let inherited_props: Vec<_> = base_portal.properties.iter()
                        .filter(|prop| !clone_prop_names.contains(prop.name.as_str()))
                        .cloned()
                        .collect();

                    portal_expansions.push(PortalExpansion {
                        index,
                        properties: inherited_props,
                    });
                }
            }
            _ => {}
        }
    }

    // Second pass: apply expansions
    for exp in model_expansions {
        if let SymbolDef::Model(m) = &mut file.symbols[exp.index] {
            // Prepend inherited fields (base fields come first)
            let mut merged_fields = exp.fields;
            merged_fields.extend(m.fields.drain(..));
            m.fields = merged_fields;

            // Prepend inherited relations
            let mut merged_relations = exp.relations;
            merged_relations.extend(m.relations.drain(..));
            m.relations = merged_relations;

            // Prepend inherited invariants
            let mut merged_invariants = exp.invariants;
            merged_invariants.extend(m.invariants.drain(..));
            m.invariants = merged_invariants;

            // Inherit context if not provided
            if m.context.is_none() {
                m.context = exp.context;
            }
        }
    }

    for exp in portal_expansions {
        if let SymbolDef::Portal(p) = &mut file.symbols[exp.index] {
            // Prepend inherited properties
            let mut merged_props = exp.properties;
            merged_props.extend(p.properties.drain(..));
            p.properties = merged_props;
        }
    }

    (file, errors)
}

/// Get the name of a symbol.
fn get_symbol_name(symbol: &SymbolDef) -> &str {
    match symbol {
        SymbolDef::Model(m) => &m.name,
        SymbolDef::Enum(e) => &e.name,
        SymbolDef::Interface(i) => &i.name,
        SymbolDef::Operation(o) => &o.name,
        SymbolDef::Event(e) => &e.name,
        SymbolDef::Config(c) => &c.name,
        SymbolDef::Portal(p) => &p.name,
        SymbolDef::Surface(s) => &s.name,
    }
}

/// Get a human-readable name for a symbol kind.
fn symbol_kind_name(symbol: &SymbolDef) -> &'static str {
    match symbol {
        SymbolDef::Model(_) => "model",
        SymbolDef::Enum(_) => "enum",
        SymbolDef::Interface(_) => "interface",
        SymbolDef::Operation(_) => "operation",
        SymbolDef::Event(_) => "event",
        SymbolDef::Config(_) => "config",
        SymbolDef::Portal(_) => "portal",
        SymbolDef::Surface(_) => "surface",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_model_clone_expansion() {
        let source = r#"
model BaseEntity {
    context "Base entity with common fields"
    field id: uuid [primary]
    field created_at: timestamp [readonly, default: now]
    field updated_at: timestamp
}

model User clone BaseEntity {
    context "User account"
    field email: string [unique, format: email]
    field password_hash: bytes [sensitive]
}
"#;
        let file = parse(source, "<test>").unwrap();
        let (expanded, errors) = expand_clones(file);

        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);

        // Find the User model
        let user = expanded.symbols.iter()
            .find_map(|s| match s {
                SymbolDef::Model(m) if m.name == "User" => Some(m),
                _ => None,
            })
            .expect("User model not found");

        // Should have inherited fields from BaseEntity + own fields
        assert_eq!(user.fields.len(), 5, "Expected 5 fields (3 inherited + 2 own)");

        // Check field names
        let field_names: Vec<_> = user.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"id"), "Missing inherited 'id' field");
        assert!(field_names.contains(&"created_at"), "Missing inherited 'created_at' field");
        assert!(field_names.contains(&"updated_at"), "Missing inherited 'updated_at' field");
        assert!(field_names.contains(&"email"), "Missing own 'email' field");
        assert!(field_names.contains(&"password_hash"), "Missing own 'password_hash' field");
    }

    #[test]
    fn test_chained_clone_error() {
        let source = r#"
model Base {
    context "Base"
    field id: uuid
}

model Child clone Base {
    field name: string
}

model GrandChild clone Child {
    field age: u8
}
"#;
        let file = parse(source, "<test>").unwrap();
        let (_, errors) = expand_clones(file);

        assert_eq!(errors.len(), 1, "Expected 1 error for chained clone");
        assert!(matches!(errors[0], ExpansionError::ChainedClone { .. }));
    }

    #[test]
    fn test_clone_field_override() {
        let source = r#"
model Base {
    context "Base"
    field id: uuid
    field name: string [length: 1..50]
}

model Extended clone Base {
    field name: string [length: 1..100]
    field extra: bool
}
"#;
        let file = parse(source, "<test>").unwrap();
        let (expanded, errors) = expand_clones(file);

        assert!(errors.is_empty());

        let extended = expanded.symbols.iter()
            .find_map(|s| match s {
                SymbolDef::Model(m) if m.name == "Extended" => Some(m),
                _ => None,
            })
            .expect("Extended model not found");

        // Should have 3 fields: id (inherited), name (overridden), extra (own)
        assert_eq!(extended.fields.len(), 3);

        // The 'name' field should be the overridden version (with length: 1..100)
        let name_field = extended.fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name_field.constraints.len(), 1);
    }

    #[test]
    fn test_base_not_found_error() {
        let source = r#"
model Child clone NonExistent {
    field name: string
}
"#;
        let file = parse(source, "<test>").unwrap();
        let (_, errors) = expand_clones(file);

        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ExpansionError::BaseNotFound { .. }));
    }

    #[test]
    fn test_type_mismatch_error() {
        let source = r#"
enum Status {
    context "Status enum"
    variant Active
    variant Inactive
}

model Child clone Status {
    field name: string
}
"#;
        let file = parse(source, "<test>").unwrap();
        let (_, errors) = expand_clones(file);

        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ExpansionError::TypeMismatch { .. }));
    }
}
