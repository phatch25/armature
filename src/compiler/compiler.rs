//! Compiler that converts Armature AST to SQLite database.

use crate::analyzer::SymbolTable;
use crate::parser::{
    File, SymbolDef, TypeExpr, FieldDef, Constraint, ConstraintValue, Literal,
    RelationDef, RelationKind, MethodDef, EffectDef, EffectKind,
    NamedString, PortalProperty, PortalValue, DisplayDef, SurfaceEvent,
    ArgumentDef, FlagDef,
    AcFile, ProjectValue, PlatformBlock, PlatformValue,
};
use rusqlite::{Connection, params};
use serde_json::{json, Value as JsonValue};
use std::path::Path;
use thiserror::Error;

use super::schema::{SCHEMA_SQL, SCHEMA_VERSION};

/// Compilation error.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Compiler for Armature AST.
pub struct Compiler<'a> {
    ast: &'a File,
    source_file: String,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler.
    pub fn new(ast: &'a File, _symbols: &SymbolTable, source_file: &str) -> Self {
        Self {
            ast,
            source_file: source_file.to_string(),
        }
    }

    /// Compile to a SQLite database file.
    pub fn compile(&self, output_path: &Path) -> Result<(), CompileError> {
        // Remove existing file if it exists
        if output_path.exists() {
            std::fs::remove_file(output_path)?;
        }

        // Create and connect to database
        let conn = Connection::open(output_path)?;

        // Create schema
        conn.execute_batch(SCHEMA_SQL)?;

        // Insert metadata
        self.insert_meta(&conn)?;

        // Insert project
        let project_id = self.insert_project(&conn)?;

        // Insert namespace
        let namespace_id = self.insert_namespace(&conn, project_id)?;

        // Insert all symbols
        for symbol in &self.ast.symbols {
            self.insert_symbol(&conn, namespace_id, symbol)?;
        }

        Ok(())
    }

    /// Insert metadata.
    fn insert_meta(&self, conn: &Connection) -> Result<(), CompileError> {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["schema_version", SCHEMA_VERSION.to_string()],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["compiler", "armature-rs"],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["compiler_version", env!("CARGO_PKG_VERSION")],
        )?;
        Ok(())
    }

    /// Insert project info.
    fn insert_project(&self, conn: &Connection) -> Result<i64, CompileError> {
        let namespace = self.ast.namespace.as_ref().map(|n| &n.name);
        let project_name = namespace
            .map(|n| n.split('.').last().unwrap_or(n))
            .unwrap_or("unnamed");

        conn.execute(
            "INSERT INTO project (name, namespace, source_file, compiled_at) VALUES (?, ?, ?, datetime('now'))",
            params![project_name, namespace, &self.source_file],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert namespace.
    fn insert_namespace(&self, conn: &Connection, project_id: i64) -> Result<i64, CompileError> {
        let namespace_name = self.ast.namespace.as_ref()
            .map(|n| n.name.as_str())
            .unwrap_or("default");

        conn.execute(
            "INSERT INTO namespace (project_id, name) VALUES (?, ?)",
            params![project_id, namespace_name],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert a symbol.
    fn insert_symbol(&self, conn: &Connection, namespace_id: i64, symbol: &SymbolDef) -> Result<(), CompileError> {
        let name = symbol.name();
        let kind = symbol.kind();
        let (context, doc_comment, definition) = self.serialize_symbol(symbol)?;

        conn.execute(
            "INSERT INTO symbol (namespace_id, name, kind, context, doc_comment, definition) VALUES (?, ?, ?, ?, ?, ?)",
            params![namespace_id, name, kind, context, doc_comment, definition],
        )?;

        let symbol_id = conn.last_insert_rowid();

        // Insert symbol-specific data
        match symbol {
            SymbolDef::Model(m) => {
                self.insert_fields(conn, symbol_id, &m.fields, "fields")?;
                self.insert_relations(conn, symbol_id, &m.relations)?;
                self.insert_named_strings(conn, symbol_id, &m.invariants, "invariants")?;
                if let Some(base) = &m.base {
                    self.insert_reference(conn, symbol_id, base, "clone_base", None)?;
                }
            }
            SymbolDef::Enum(e) => {
                for variant in &e.variants {
                    let data_type_json = variant.data_type.as_ref()
                        .map(|t| serde_json::to_string(&self.serialize_type_expr(t)))
                        .transpose()?;

                    conn.execute(
                        "INSERT INTO variant (symbol_id, name, data_type) VALUES (?, ?, ?)",
                        params![symbol_id, &variant.name, data_type_json],
                    )?;
                }
            }
            SymbolDef::Interface(i) => {
                self.insert_methods(conn, symbol_id, &i.methods)?;
            }
            SymbolDef::Operation(o) => {
                self.insert_fields(conn, symbol_id, &o.inputs, "inputs")?;
                self.insert_effects(conn, symbol_id, &o.effects)?;
                self.insert_named_strings(conn, symbol_id, &o.errors, "errors")?;
                self.insert_named_strings(conn, symbol_id, &o.preconditions, "preconditions")?;
                self.insert_named_strings(conn, symbol_id, &o.postconditions, "postconditions")?;
                for req in &o.requires {
                    self.insert_reference(conn, symbol_id, req, "requires", None)?;
                }
            }
            SymbolDef::Event(e) => {
                self.insert_fields(conn, symbol_id, &e.fields, "fields")?;
                if let Some(producer) = &e.producer {
                    self.insert_reference(conn, symbol_id, producer, "producer", None)?;
                }
            }
            SymbolDef::Config(c) => {
                self.insert_fields(conn, symbol_id, &c.fields, "fields")?;
            }
            SymbolDef::Portal(p) => {
                self.insert_portal_properties(conn, symbol_id, &p.properties)?;
                if let Some(base) = &p.base {
                    self.insert_reference(conn, symbol_id, base, "clone_base", None)?;
                }
            }
            SymbolDef::Surface(s) => {
                self.insert_displays(conn, symbol_id, &s.displays)?;
                self.insert_fields(conn, symbol_id, &s.states, "states")?;
                self.insert_named_strings(conn, symbol_id, &s.interactions, "interactions")?;
                self.insert_surface_events(conn, symbol_id, &s.events)?;
                self.insert_arguments(conn, symbol_id, &s.arguments)?;
                self.insert_flags(conn, symbol_id, &s.flags)?;
                if let Some(parent) = &s.parent {
                    self.insert_reference(conn, symbol_id, parent, "parent", None)?;
                }
            }
        }

        Ok(())
    }

    /// Serialize a symbol to (context, doc_comment, definition_json).
    fn serialize_symbol(&self, symbol: &SymbolDef) -> Result<(Option<String>, Option<String>, String), CompileError> {
        let (context, doc_comment, definition) = match symbol {
            SymbolDef::Model(m) => {
                let def = json!({
                    "kind": "model",
                    "name": m.name,
                    "context": m.context,
                    "base": m.base,
                    "fields": m.fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>(),
                    "relations": m.relations.iter().map(|r| self.serialize_relation(r)).collect::<Vec<_>>(),
                    "invariants": m.invariants.iter().map(|i| json!({"name": i.name, "value": i.value})).collect::<Vec<_>>(),
                });
                (m.context.clone(), m.doc_comment.clone(), def)
            }
            SymbolDef::Enum(e) => {
                let def = json!({
                    "kind": "enum",
                    "name": e.name,
                    "context": e.context,
                    "variants": e.variants.iter().map(|v| {
                        if let Some(data_type) = &v.data_type {
                            json!({
                                "name": v.name,
                                "data_type": self.serialize_type_expr(data_type)
                            })
                        } else {
                            json!({
                                "name": v.name
                            })
                        }
                    }).collect::<Vec<_>>(),
                });
                (e.context.clone(), e.doc_comment.clone(), def)
            }
            SymbolDef::Interface(i) => {
                let def = json!({
                    "kind": "interface",
                    "name": i.name,
                    "context": i.context,
                    "methods": i.methods.iter().map(|m| self.serialize_method(m)).collect::<Vec<_>>(),
                });
                (i.context.clone(), i.doc_comment.clone(), def)
            }
            SymbolDef::Operation(o) => {
                let def = json!({
                    "kind": "operation",
                    "name": o.name,
                    "context": o.context,
                    "inputs": o.inputs.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>(),
                    "output": o.output.as_ref().map(|t| self.serialize_type_expr(t)),
                    "errors": o.errors.iter().map(|e| json!({"name": e.name, "value": e.value})).collect::<Vec<_>>(),
                    "requires": &o.requires,
                    "effects": o.effects.iter().map(|e| self.serialize_effect(e)).collect::<Vec<_>>(),
                    "preconditions": o.preconditions.iter().map(|p| json!({"name": p.name, "value": p.value})).collect::<Vec<_>>(),
                    "postconditions": o.postconditions.iter().map(|p| json!({"name": p.name, "value": p.value})).collect::<Vec<_>>(),
                });
                (o.context.clone(), o.doc_comment.clone(), def)
            }
            SymbolDef::Event(e) => {
                let def = json!({
                    "kind": "event",
                    "name": e.name,
                    "context": e.context,
                    "fields": e.fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>(),
                    "producer": e.producer,
                });
                (e.context.clone(), e.doc_comment.clone(), def)
            }
            SymbolDef::Config(c) => {
                let def = json!({
                    "kind": "config",
                    "name": c.name,
                    "context": c.context,
                    "fields": c.fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>(),
                });
                (c.context.clone(), c.doc_comment.clone(), def)
            }
            SymbolDef::Portal(p) => {
                let def = json!({
                    "kind": "portal",
                    "name": p.name,
                    "context": p.context,
                    "base": p.base,
                    "properties": p.properties.iter().map(|prop| {
                        json!({
                            "name": prop.name,
                            "value": self.serialize_portal_value(&prop.value)
                        })
                    }).collect::<Vec<_>>(),
                });
                (p.context.clone(), p.doc_comment.clone(), def)
            }
            SymbolDef::Surface(s) => {
                let def = json!({
                    "kind": "surface",
                    "name": s.name,
                    "context": s.context,
                    "parent": s.parent,
                    "displays": s.displays.iter().map(|d| {
                        json!({
                            "name": d.name,
                            "type": self.serialize_type_expr(&d.display_type)
                        })
                    }).collect::<Vec<_>>(),
                    "interactions": s.interactions.iter().map(|i| json!({"name": i.name, "value": i.value})).collect::<Vec<_>>(),
                    "commands": s.commands.iter().map(|c| &c.name).collect::<Vec<_>>(),
                    "arguments": s.arguments.iter().map(|a| json!({
                        "name": a.name,
                        "type": self.serialize_type_expr(&a.arg_type)
                    })).collect::<Vec<_>>(),
                    "flags": s.flags.iter().map(|f| json!({
                        "name": f.name,
                        "type": self.serialize_type_expr(&f.flag_type)
                    })).collect::<Vec<_>>(),
                    "outputs": s.outputs.iter().map(|o| json!({
                        "name": o.name,
                        "value": o.value
                    })).collect::<Vec<_>>(),
                    "properties": s.properties.iter().map(|p| json!({"name": p.name, "value": p.value})).collect::<Vec<_>>(),
                    "states": s.states.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>(),
                    "events": s.events.iter().map(|e| {
                        json!({
                            "name": e.name,
                            "fields": e.fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>()
                        })
                    }).collect::<Vec<_>>(),
                    "help": s.help,
                    "accessibility": s.accessibility.as_ref().map(|a| json!({
                        "label": a.label,
                        "hint": a.hint,
                        "role": a.role
                    })),
                    "mapping": s.mapping.as_ref().map(|m| {
                        m.entries.iter().map(|e| json!({
                            "key": e.key,
                            "value": e.value
                        })).collect::<Vec<_>>()
                    }),
                });
                (s.context.clone(), s.doc_comment.clone(), def)
            }
        };

        Ok((context, doc_comment, serde_json::to_string(&definition)?))
    }

    /// Serialize a field to JSON.
    fn serialize_field(&self, field: &FieldDef) -> JsonValue {
        json!({
            "name": field.name,
            "type": self.serialize_type_expr(&field.field_type),
            "default": field.default.as_ref().map(|d| self.serialize_literal(d)),
            "constraints": field.constraints.iter().map(|c| self.serialize_constraint(c)).collect::<Vec<_>>(),
        })
    }

    /// Serialize a type expression to JSON.
    fn serialize_type_expr(&self, type_expr: &TypeExpr) -> JsonValue {
        match type_expr {
            TypeExpr::Primitive { name, .. } => json!({"kind": "primitive", "name": name}),
            TypeExpr::Modifier { modifier, args, .. } => json!({
                "kind": "modifier",
                "modifier": modifier,
                "args": args.iter().map(|a| self.serialize_type_expr(a)).collect::<Vec<_>>()
            }),
            TypeExpr::Ref { target, .. } => json!({"kind": "ref", "target": target}),
            TypeExpr::Optional { inner, .. } => json!({
                "kind": "optional",
                "inner": self.serialize_type_expr(inner)
            }),
            TypeExpr::TypeUnion { types, .. } => json!({
                "kind": "type_union",
                "types": types.iter().map(|t| self.serialize_type_expr(t)).collect::<Vec<_>>()
            }),
            TypeExpr::LiteralUnion { values, .. } => json!({
                "kind": "literal_union",
                "values": values
            }),
            TypeExpr::InlineStruct { fields, .. } => json!({
                "kind": "inline_struct",
                "fields": fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>()
            }),
            TypeExpr::DecimalPrecision { precision, scale, .. } => json!({
                "kind": "decimal",
                "precision": precision,
                "scale": scale
            }),
        }
    }

    /// Serialize a literal to JSON.
    fn serialize_literal(&self, literal: &Literal) -> JsonValue {
        match literal {
            Literal::String(s) => json!({"kind": "string", "value": s}),
            Literal::Integer(i) => json!({"kind": "integer", "value": i}),
            Literal::Decimal(d) => json!({"kind": "decimal", "value": d}),
            Literal::Boolean(b) => json!({"kind": "boolean", "value": b}),
            Literal::Identifier(i) => json!({"kind": "identifier", "value": i}),
            Literal::EnumVariant { enum_name, variant } => json!({
                "kind": "enum_variant",
                "enum_name": enum_name,
                "variant": variant
            }),
            Literal::Now => json!({"kind": "now"}),
            Literal::None => json!({"kind": "none"}),
            Literal::Map(pairs) => json!({
                "kind": "map",
                "pairs": pairs.iter().map(|(k, v)| json!({"key": k, "value": self.serialize_literal(v)})).collect::<Vec<_>>()
            }),
            Literal::Set(values) => json!({
                "kind": "set",
                "values": values.iter().map(|v| self.serialize_literal(v)).collect::<Vec<_>>()
            }),
        }
    }

    /// Serialize a constraint to JSON.
    fn serialize_constraint(&self, constraint: &Constraint) -> JsonValue {
        json!({
            "name": constraint.name,
            "value": constraint.value.as_ref().map(|v| self.serialize_constraint_value(v))
        })
    }

    /// Serialize a constraint value to JSON.
    fn serialize_constraint_value(&self, value: &ConstraintValue) -> JsonValue {
        match value {
            ConstraintValue::String(s) => json!({"kind": "string", "value": s}),
            ConstraintValue::Integer(i) => json!({"kind": "integer", "value": i}),
            ConstraintValue::Decimal(d) => json!({"kind": "decimal", "value": d}),
            ConstraintValue::Identifier(i) => json!({"kind": "identifier", "value": i}),
            ConstraintValue::Range { start, end } => json!({"kind": "range", "start": start, "end": end}),
            ConstraintValue::List(items) => json!({"kind": "list", "values": items}),
        }
    }

    /// Serialize a relation to JSON.
    fn serialize_relation(&self, relation: &RelationDef) -> JsonValue {
        json!({
            "name": relation.name,
            "kind": relation.kind.as_str(),
            "target": self.serialize_type_expr(&relation.target)
        })
    }

    /// Serialize a method to JSON.
    fn serialize_method(&self, method: &MethodDef) -> JsonValue {
        json!({
            "name": method.name,
            "context": method.context,
            "params": method.params.iter().map(|p| json!({"name": p.name, "type": self.serialize_type_expr(&p.param_type)})).collect::<Vec<_>>(),
            "return_type": method.return_type.as_ref().map(|t| self.serialize_type_expr(t)),
            "errors": method.errors.iter().map(|e| json!({"name": e.name, "value": e.value})).collect::<Vec<_>>()
        })
    }

    /// Serialize an effect to JSON.
    fn serialize_effect(&self, effect: &EffectDef) -> JsonValue {
        json!({
            "kind": effect.kind.as_str(),
            "target": effect.target
        })
    }

    /// Serialize a portal value to JSON.
    fn serialize_portal_value(&self, value: &PortalValue) -> JsonValue {
        match value {
            PortalValue::Identifier(s) => json!(s),
            PortalValue::String(s) => json!(s),
            PortalValue::Integer(i) => json!(i),
        }
    }

    /// Insert fields into the database.
    fn insert_fields(&self, conn: &Connection, symbol_id: i64, fields: &[FieldDef], field_context: &str) -> Result<(), CompileError> {
        for field in fields {
            let type_json = serde_json::to_string(&self.serialize_type_expr(&field.field_type))?;
            let default_json = field.default.as_ref()
                .map(|d| serde_json::to_string(&self.serialize_literal(d)))
                .transpose()?;
            let constraints_json = serde_json::to_string(
                &field.constraints.iter().map(|c| self.serialize_constraint(c)).collect::<Vec<_>>()
            )?;
            let optional = matches!(&field.field_type, TypeExpr::Optional { .. });

            conn.execute(
                "INSERT INTO field (symbol_id, name, type_expr, optional, default_value, constraints, field_context) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![symbol_id, &field.name, type_json, optional as i32, default_json, constraints_json, field_context],
            )?;
        }
        Ok(())
    }

    /// Insert relations into the database.
    fn insert_relations(&self, conn: &Connection, symbol_id: i64, relations: &[RelationDef]) -> Result<(), CompileError> {
        for relation in relations {
            let kind = match relation.kind {
                RelationKind::HasOne => "has_one",
                RelationKind::HasMany => "has_many",
                RelationKind::BelongsTo => "belongs_to",
                RelationKind::ManyToMany => "many_to_many",
            };
            // Extract target from the type expression
            let target = self.extract_ref_target(&relation.target);

            conn.execute(
                "INSERT INTO relation (symbol_id, name, kind, target_symbol) VALUES (?, ?, ?, ?)",
                params![symbol_id, &relation.name, kind, target],
            )?;
        }
        Ok(())
    }

    /// Extract the target name from a type expression (for relations).
    fn extract_ref_target(&self, type_expr: &TypeExpr) -> String {
        match type_expr {
            TypeExpr::Ref { target, .. } => target.clone(),
            TypeExpr::Primitive { name, .. } => name.clone(),
            TypeExpr::Modifier { args, .. } if !args.is_empty() => self.extract_ref_target(&args[0]),
            TypeExpr::Optional { inner, .. } => self.extract_ref_target(inner),
            _ => "unknown".to_string(),
        }
    }

    /// Insert methods into the database.
    fn insert_methods(&self, conn: &Connection, symbol_id: i64, methods: &[MethodDef]) -> Result<(), CompileError> {
        for method in methods {
            let params_json = serde_json::to_string(
                &method.params.iter().map(|p| json!({"name": p.name, "type": self.serialize_type_expr(&p.param_type)})).collect::<Vec<_>>()
            )?;
            let return_json = method.return_type.as_ref()
                .map(|t| serde_json::to_string(&self.serialize_type_expr(t)))
                .transpose()?;

            conn.execute(
                "INSERT INTO method (symbol_id, name, params, return_type) VALUES (?, ?, ?, ?)",
                params![symbol_id, &method.name, params_json, return_json],
            )?;
        }
        Ok(())
    }

    /// Insert effects into the database.
    fn insert_effects(&self, conn: &Connection, symbol_id: i64, effects: &[EffectDef]) -> Result<(), CompileError> {
        for effect in effects {
            let kind = match effect.kind {
                EffectKind::Creates => "creates",
                EffectKind::Updates => "updates",
                EffectKind::Deletes => "deletes",
                EffectKind::Emits => "emits",
                EffectKind::Calls => "calls",
            };

            conn.execute(
                "INSERT INTO effect (symbol_id, kind, target) VALUES (?, ?, ?)",
                params![symbol_id, kind, &effect.target],
            )?;
        }
        Ok(())
    }

    /// Insert portal properties into the database.
    fn insert_portal_properties(&self, conn: &Connection, symbol_id: i64, properties: &[PortalProperty]) -> Result<(), CompileError> {
        for prop in properties {
            let value_json = serde_json::to_string(&self.serialize_portal_value(&prop.value))?;

            conn.execute(
                "INSERT INTO portal_property (symbol_id, name, value) VALUES (?, ?, ?)",
                params![symbol_id, &prop.name, value_json],
            )?;
        }
        Ok(())
    }

    /// Insert named strings into the database.
    fn insert_named_strings(&self, conn: &Connection, symbol_id: i64, strings: &[NamedString], string_context: &str) -> Result<(), CompileError> {
        for ns in strings {
            conn.execute(
                "INSERT INTO named_string (symbol_id, string_context, name, value) VALUES (?, ?, ?, ?)",
                params![symbol_id, string_context, &ns.name, &ns.value],
            )?;
        }
        Ok(())
    }

    /// Insert displays into the database.
    fn insert_displays(&self, conn: &Connection, symbol_id: i64, displays: &[DisplayDef]) -> Result<(), CompileError> {
        for display in displays {
            let type_json = serde_json::to_string(&self.serialize_type_expr(&display.display_type))?;

            conn.execute(
                "INSERT INTO field (symbol_id, name, type_expr, optional, field_context) VALUES (?, ?, ?, 0, 'displays')",
                params![symbol_id, &display.name, type_json],
            )?;
        }
        Ok(())
    }

    /// Insert surface events into the database.
    fn insert_surface_events(&self, conn: &Connection, symbol_id: i64, events: &[SurfaceEvent]) -> Result<(), CompileError> {
        for event in events {
            // Store event as a named string with fields in the value as JSON
            let fields_json = serde_json::to_string(
                &event.fields.iter().map(|f| self.serialize_field(f)).collect::<Vec<_>>()
            )?;

            conn.execute(
                "INSERT INTO named_string (symbol_id, string_context, name, value) VALUES (?, ?, ?, ?)",
                params![symbol_id, "events", &event.name, fields_json],
            )?;
        }
        Ok(())
    }

    /// Insert CLI arguments into the database (stored as fields with field_context='arguments').
    fn insert_arguments(&self, conn: &Connection, symbol_id: i64, arguments: &[ArgumentDef]) -> Result<(), CompileError> {
        for arg in arguments {
            let type_json = serde_json::to_string(&self.serialize_type_expr(&arg.arg_type))?;

            conn.execute(
                "INSERT INTO field (symbol_id, name, type_expr, optional, field_context) VALUES (?, ?, ?, 0, 'arguments')",
                params![symbol_id, &arg.name, type_json],
            )?;
        }
        Ok(())
    }

    /// Insert CLI flags into the database (stored as fields with field_context='flags').
    fn insert_flags(&self, conn: &Connection, symbol_id: i64, flags: &[FlagDef]) -> Result<(), CompileError> {
        for flag in flags {
            let type_json = serde_json::to_string(&self.serialize_type_expr(&flag.flag_type))?;

            conn.execute(
                "INSERT INTO field (symbol_id, name, type_expr, optional, field_context) VALUES (?, ?, ?, 0, 'flags')",
                params![symbol_id, &flag.name, type_json],
            )?;
        }
        Ok(())
    }

    /// Insert a reference into the database.
    fn insert_reference(&self, conn: &Connection, symbol_id: i64, target: &str, ref_kind: &str, ref_context: Option<&str>) -> Result<(), CompileError> {
        conn.execute(
            "INSERT INTO reference (from_symbol_id, to_symbol_name, ref_kind, ref_context) VALUES (?, ?, ?, ?)",
            params![symbol_id, target, ref_kind, ref_context],
        )?;
        Ok(())
    }
}

/// Compile an AST to a SQLite database.
pub fn compile(ast: &File, symbols: &SymbolTable, source_file: &str, output_path: &Path) -> Result<(), CompileError> {
    let compiler = Compiler::new(ast, symbols, source_file);
    compiler.compile(output_path)
}

// =============================================================================
// AC FILE COMPILER
// =============================================================================

/// Compiler for .ac project configuration files.
pub struct AcCompiler<'a> {
    ac_file: &'a AcFile,
    source_file: String,
}

impl<'a> AcCompiler<'a> {
    /// Create a new .ac file compiler.
    pub fn new(ac_file: &'a AcFile, source_file: &str) -> Self {
        Self {
            ac_file,
            source_file: source_file.to_string(),
        }
    }

    /// Compile to a SQLite database file.
    pub fn compile(&self, output_path: &Path) -> Result<(), CompileError> {
        if output_path.exists() {
            std::fs::remove_file(output_path)?;
        }

        let conn = Connection::open(output_path)?;
        conn.execute_batch(SCHEMA_SQL)?;

        self.insert_meta(&conn)?;
        let project_id = self.insert_project(&conn)?;
        self.insert_platforms(&conn, project_id)?;
        self.insert_aliases(&conn, project_id)?;
        self.insert_build_config(&conn, project_id)?;
        self.insert_source_file(&conn, project_id, &self.source_file, "ac")?;

        Ok(())
    }

    fn insert_meta(&self, conn: &Connection) -> Result<(), CompileError> {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["schema_version", SCHEMA_VERSION.to_string()],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["compiler", "armature-rs"],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            params!["compiler_version", env!("CARGO_PKG_VERSION")],
        )?;
        Ok(())
    }

    fn insert_project(&self, conn: &Connection) -> Result<i64, CompileError> {
        let (name, description, version, context, architecture) = if let Some(project) = &self.ac_file.project {
            let desc = project.get_string("description").map(|s| s.to_string());
            let ver = project.get_string("version").map(|s| s.to_string());
            let ctx = project.get_string("context").map(|s| s.to_string());
            let arch = project.get_field("architecture")
                .map(|v| self.serialize_project_value(v))
                .map(|v| serde_json::to_string(&v))
                .transpose()?;
            (project.name.clone(), desc, ver, ctx, arch)
        } else {
            ("unnamed".to_string(), None, None, None, None)
        };

        conn.execute(
            "INSERT INTO project (name, description, version, context, architecture, source_file, compiled_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
            params![name, description, version, context, architecture, &self.source_file],
        )?;

        Ok(conn.last_insert_rowid())
    }

    fn insert_platforms(&self, conn: &Connection, project_id: i64) -> Result<(), CompileError> {
        for platform in &self.ac_file.platforms {
            let definition = self.serialize_platform_block(platform);
            let definition_json = serde_json::to_string(&definition)?;

            conn.execute(
                "INSERT INTO platform (project_id, name, definition) VALUES (?, ?, ?)",
                params![project_id, &platform.name, definition_json],
            )?;
        }
        Ok(())
    }

    fn insert_aliases(&self, conn: &Connection, project_id: i64) -> Result<(), CompileError> {
        for alias in &self.ac_file.aliases {
            let target_type = self.serialize_type_expr(&alias.target_type);
            let target_type_json = serde_json::to_string(&target_type)?;

            let constraints_json = if alias.constraints.is_empty() {
                None
            } else {
                let constraints: Vec<_> = alias.constraints.iter()
                    .map(|c| self.serialize_constraint(c))
                    .collect();
                Some(serde_json::to_string(&constraints)?)
            };

            conn.execute(
                "INSERT INTO alias (project_id, name, target_type, constraints) VALUES (?, ?, ?, ?)",
                params![project_id, &alias.name, target_type_json, constraints_json],
            )?;
        }
        Ok(())
    }

    fn insert_build_config(&self, conn: &Connection, project_id: i64) -> Result<(), CompileError> {
        if let Some(build) = &self.ac_file.build {
            for field in &build.fields {
                conn.execute(
                    "INSERT INTO build_config (project_id, key, value) VALUES (?, ?, ?)",
                    params![project_id, &field.name, &field.value],
                )?;
            }
        }
        Ok(())
    }

    fn insert_source_file(&self, conn: &Connection, project_id: i64, path: &str, kind: &str) -> Result<(), CompileError> {
        conn.execute(
            "INSERT INTO source_file (project_id, path, kind) VALUES (?, ?, ?)",
            params![project_id, path, kind],
        )?;
        Ok(())
    }

    fn serialize_project_value(&self, value: &ProjectValue) -> JsonValue {
        match value {
            ProjectValue::String(s) => json!(s),
            ProjectValue::Block(fields) => {
                let obj: serde_json::Map<String, JsonValue> = fields.iter()
                    .map(|f| (f.name.clone(), self.serialize_project_value(&f.value)))
                    .collect();
                JsonValue::Object(obj)
            }
        }
    }

    fn serialize_platform_block(&self, platform: &PlatformBlock) -> JsonValue {
        let obj: serde_json::Map<String, JsonValue> = platform.fields.iter()
            .map(|f| (f.name.clone(), self.serialize_platform_value(&f.value)))
            .collect();
        JsonValue::Object(obj)
    }

    fn serialize_platform_value(&self, value: &PlatformValue) -> JsonValue {
        match value {
            PlatformValue::Identifier(s) => json!({"kind": "identifier", "value": s}),
            PlatformValue::String(s) => json!({"kind": "string", "value": s}),
            PlatformValue::TypeExpr(t) => json!({"kind": "type_expr", "value": self.serialize_type_expr(t)}),
            PlatformValue::Block(fields) => {
                let obj: serde_json::Map<String, JsonValue> = fields.iter()
                    .map(|f| (f.name.clone(), self.serialize_platform_value(&f.value)))
                    .collect();
                json!({"kind": "block", "value": JsonValue::Object(obj)})
            }
        }
    }

    fn serialize_type_expr(&self, type_expr: &TypeExpr) -> JsonValue {
        match type_expr {
            TypeExpr::Primitive { name, .. } => json!({"kind": "primitive", "name": name}),
            TypeExpr::Modifier { modifier, args, .. } => json!({
                "kind": "modifier",
                "modifier": modifier,
                "args": args.iter().map(|a| self.serialize_type_expr(a)).collect::<Vec<_>>()
            }),
            TypeExpr::Ref { target, .. } => json!({"kind": "ref", "target": target}),
            TypeExpr::Optional { inner, .. } => json!({
                "kind": "optional",
                "inner": self.serialize_type_expr(inner)
            }),
            TypeExpr::TypeUnion { types, .. } => json!({
                "kind": "type_union",
                "types": types.iter().map(|t| self.serialize_type_expr(t)).collect::<Vec<_>>()
            }),
            TypeExpr::LiteralUnion { values, .. } => json!({"kind": "literal_union", "values": values}),
            TypeExpr::InlineStruct { fields, .. } => json!({
                "kind": "inline_struct",
                "fields": fields.iter().map(|f| json!({
                    "name": f.name,
                    "type": self.serialize_type_expr(&f.field_type)
                })).collect::<Vec<_>>()
            }),
            TypeExpr::DecimalPrecision { precision, scale, .. } => json!({
                "kind": "decimal",
                "precision": precision,
                "scale": scale
            }),
        }
    }

    fn serialize_constraint(&self, constraint: &Constraint) -> JsonValue {
        json!({
            "name": constraint.name,
            "value": constraint.value.as_ref().map(|v| self.serialize_constraint_value(v))
        })
    }

    fn serialize_constraint_value(&self, value: &ConstraintValue) -> JsonValue {
        match value {
            ConstraintValue::String(s) => json!({"kind": "string", "value": s}),
            ConstraintValue::Integer(i) => json!({"kind": "integer", "value": i}),
            ConstraintValue::Decimal(d) => json!({"kind": "decimal", "value": d}),
            ConstraintValue::Identifier(i) => json!({"kind": "identifier", "value": i}),
            ConstraintValue::Range { start, end } => json!({"kind": "range", "start": start, "end": end}),
            ConstraintValue::List(items) => json!({"kind": "list", "values": items}),
        }
    }
}

/// Compile an .ac file to a SQLite database.
pub fn compile_ac(ac_file: &AcFile, source_file: &str, output_path: &Path) -> Result<(), CompileError> {
    let compiler = AcCompiler::new(ac_file, source_file);
    compiler.compile(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::analyzer::analyze;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compile_simple() {
        let source = r#"
namespace test.example

model User {
    context "A user in the system"
    field id: uuid [primary]
    field name: string [length: 1..100]
    field email: string [unique, format: email]
}

enum Status {
    context "Status values"
    variant Active
    variant Inactive
    variant Pending
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        // Verify database contents
        let conn = Connection::open(temp_file.path()).unwrap();

        // Check meta
        let version: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, SCHEMA_VERSION.to_string());

        // Check symbols
        let symbol_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM symbol",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(symbol_count, 2); // User and Status

        // Check variants
        let variant_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM variant",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(variant_count, 3); // Active, Inactive, Pending

        // Check fields
        let field_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM field",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(field_count, 3); // id, name, email
    }

    #[test]
    fn test_compile_sum_type() {
        let source = r#"
namespace test.sumtypes

enum SymbolSpec {
    context "Tagged union for symbol specifications"
    variant Model(ModelDef)
    variant Operation(OperationDef)
    variant Simple
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        // Check variants
        let variant_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM variant",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(variant_count, 3);

        // Check that Model variant has a data_type
        let model_data_type: Option<String> = conn.query_row(
            "SELECT data_type FROM variant WHERE name = 'Model'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(model_data_type.is_some(), "Model variant should have data_type");
        let data_type_json = model_data_type.unwrap();
        assert!(data_type_json.contains("ModelDef"), "data_type should contain ModelDef");

        // Check that Simple variant has no data_type
        let simple_data_type: Option<String> = conn.query_row(
            "SELECT data_type FROM variant WHERE name = 'Simple'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(simple_data_type.is_none(), "Simple variant should not have data_type");

        // Check that the definition JSON contains the correct structure
        let definition: String = conn.query_row(
            "SELECT definition FROM symbol WHERE name = 'SymbolSpec'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(definition.contains("\"name\":\"Model\""), "Definition should contain Model variant");
        assert!(definition.contains("data_type"), "Definition should contain data_type for sum type variants");
    }

    #[test]
    fn test_compile_interface() {
        let source = r#"
namespace test

interface UserRepository {
    context "Repository for user data"
    method findById(id: uuid): User?
    method findByEmail(email: string): User?
    method save(user: User): User
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        let method_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM method",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(method_count, 3);
    }

    #[test]
    fn test_compile_operation() {
        let source = r#"
namespace test

operation CreateUser {
    context "Create a new user"
    input name: string
    input email: string
    output: User
    error invalid_email "Email format is invalid"
    effect creates User
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        let effect_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM effect",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(effect_count, 1);

        let error_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM named_string WHERE string_context = 'errors'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(error_count, 1);
    }

    #[test]
    fn test_compile_clone_expansion() {
        let source = r#"
namespace test

model BaseEntity {
    context "Base entity with common fields"
    field id: uuid [primary]
    field created_at: timestamp
    field updated_at: timestamp
}

model User clone BaseEntity {
    context "User account"
    field email: string [unique, format: email]
    field name: string
}
"#;
        let ast = parse(source, "<test>").unwrap();

        // Expand clones before analysis
        let (ast, expansion_errors) = crate::expand::expand_clones(ast);
        assert!(expansion_errors.is_empty(), "Clone expansion failed: {:?}", expansion_errors);

        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        // User should have 5 fields total: id, created_at, updated_at (inherited) + email, name (own)
        let user_field_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM field f JOIN symbol s ON f.symbol_id = s.id WHERE s.name = 'User'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(user_field_count, 5);
    }

    #[test]
    fn test_compile_surface_with_mapping() {
        let source = r#"
namespace test

surface StatusLED {
    context "LED status indicator"
    state system_state: string
    output pin: gpio_2
    output pattern: led_pattern
    mapping {
        idle: off
        running: blink_slow
        error: blink_fast
    }
}
"#;
        let ast = parse(source, "<test>").unwrap();
        let (symbols, _) = analyze(&ast);

        let temp_file = NamedTempFile::new().unwrap();
        compile(&ast, &symbols, "<test>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        // Check that the surface was compiled
        let symbol_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM symbol WHERE kind = 'surface'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(symbol_count, 1);

        // Check that the definition JSON contains the mapping
        let definition: String = conn.query_row(
            "SELECT definition FROM symbol WHERE name = 'StatusLED'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(definition.contains("mapping"), "Definition should contain mapping");
        assert!(definition.contains("idle"), "Definition should contain mapping key 'idle'");
        assert!(definition.contains("off"), "Definition should contain mapping value 'off'");
    }

    #[test]
    fn test_compile_ac_file() {
        use crate::parser::parse_ac;

        let source = r#"
project "TestProject" {
    description "A test project for compilation"
    version "1.0.0"
    context "Testing .ac compilation"
    architecture {
        pattern "MVVM"
        layers "presentation, domain, data"
    }
}

platform rust {
    naming: snake_case
    optional: optional<T>
}

platform typescript {
    naming: camelCase
    optional: oneof<T, none>
}

alias UserId = uuid
alias Email = string [format: email, max: 255]

build {
    output: "dist/"
    mcp_db: ".armature/spec.db"
}
"#;
        let ac_file = parse_ac(source, "<test.ac>").unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        compile_ac(&ac_file, "<test.ac>", temp_file.path()).unwrap();

        let conn = Connection::open(temp_file.path()).unwrap();

        // Check project
        let project_name: String = conn.query_row(
            "SELECT name FROM project",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(project_name, "TestProject");

        let project_version: String = conn.query_row(
            "SELECT version FROM project",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(project_version, "1.0.0");

        // Check platforms
        let platform_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM platform",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(platform_count, 2);

        // Check aliases
        let alias_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM alias",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(alias_count, 2);

        // Check build config
        let build_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM build_config",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(build_count, 2);

        // Check source file tracking
        let source_file_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM source_file WHERE kind = 'ac'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(source_file_count, 1);
    }
}
