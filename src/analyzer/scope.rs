//! Symbol table and scope management for semantic analysis.

use crate::lexer::SourceLocation;
use crate::parser::{SymbolDef, File};
use std::collections::HashMap;

/// Information about a symbol in the symbol table.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub location: SourceLocation,
    pub context: Option<String>,
    pub is_clone: bool,
    pub clone_base: Option<String>,
    /// Symbols this symbol references (for dependency tracking)
    pub references: Vec<String>,
    /// Whether this symbol has been referenced by another symbol
    pub is_referenced: bool,
}

/// The kind of symbol (matches SymbolDef variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Model,
    Enum,
    Interface,
    Operation,
    Event,
    Config,
    Portal,
    Surface,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Model => "model",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "interface",
            SymbolKind::Operation => "operation",
            SymbolKind::Event => "event",
            SymbolKind::Config => "config",
            SymbolKind::Portal => "portal",
            SymbolKind::Surface => "surface",
        }
    }
}

/// Symbol table holding all symbols in a file.
#[derive(Debug)]
pub struct SymbolTable {
    symbols: HashMap<String, SymbolInfo>,
    namespace: Option<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            namespace: None,
        }
    }

    /// Build a symbol table from a parsed AST.
    pub fn from_ast(ast: &File) -> Self {
        let mut table = Self::new();

        table.namespace = ast.namespace.as_ref().map(|n| n.name.clone());

        for symbol in &ast.symbols {
            let info = Self::extract_symbol_info(symbol);
            table.symbols.insert(info.name.clone(), info);
        }

        table
    }

    /// Extract symbol info from a SymbolDef.
    fn extract_symbol_info(symbol: &SymbolDef) -> SymbolInfo {
        match symbol {
            SymbolDef::Model(m) => SymbolInfo {
                name: m.name.clone(),
                kind: SymbolKind::Model,
                location: m.location,
                context: m.context.clone(),
                is_clone: m.base.is_some(),
                clone_base: m.base.clone(),
                references: Self::collect_model_refs(m),
                is_referenced: false,
            },
            SymbolDef::Enum(e) => SymbolInfo {
                name: e.name.clone(),
                kind: SymbolKind::Enum,
                location: e.location,
                context: e.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Vec::new(),
                is_referenced: false,
            },
            SymbolDef::Interface(i) => SymbolInfo {
                name: i.name.clone(),
                kind: SymbolKind::Interface,
                location: i.location,
                context: i.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Self::collect_interface_refs(i),
                is_referenced: false,
            },
            SymbolDef::Operation(o) => SymbolInfo {
                name: o.name.clone(),
                kind: SymbolKind::Operation,
                location: o.location,
                context: o.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Self::collect_operation_refs(o),
                is_referenced: false,
            },
            SymbolDef::Event(e) => SymbolInfo {
                name: e.name.clone(),
                kind: SymbolKind::Event,
                location: e.location,
                context: e.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Self::collect_event_refs(e),
                is_referenced: false,
            },
            SymbolDef::Config(c) => SymbolInfo {
                name: c.name.clone(),
                kind: SymbolKind::Config,
                location: c.location,
                context: c.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Self::collect_type_refs_from_fields(&c.fields),
                is_referenced: false,
            },
            SymbolDef::Portal(p) => SymbolInfo {
                name: p.name.clone(),
                kind: SymbolKind::Portal,
                location: p.location,
                context: None, // Portals don't have context in v2
                is_clone: p.base.is_some(),
                clone_base: p.base.clone(),
                references: Self::collect_portal_refs(p),
                is_referenced: false,
            },
            SymbolDef::Surface(s) => SymbolInfo {
                name: s.name.clone(),
                kind: SymbolKind::Surface,
                location: s.location,
                context: s.context.clone(),
                is_clone: false,
                clone_base: None,
                references: Self::collect_surface_refs(s),
                is_referenced: false,
            },
        }
    }

    /// Collect type references from a model.
    fn collect_model_refs(m: &crate::parser::ModelDef) -> Vec<String> {
        let mut refs = Vec::new();

        // Clone base
        if let Some(base) = &m.base {
            refs.push(base.clone());
        }

        // Field types
        refs.extend(Self::collect_type_refs_from_fields(&m.fields));

        // Relation targets
        for rel in &m.relations {
            refs.extend(Self::collect_type_refs(&rel.target));
        }

        refs
    }

    /// Collect type references from an interface.
    fn collect_interface_refs(i: &crate::parser::InterfaceDef) -> Vec<String> {
        let mut refs = Vec::new();
        for method in &i.methods {
            for param in &method.params {
                refs.extend(Self::collect_type_refs(&param.param_type));
            }
            if let Some(ret) = &method.return_type {
                refs.extend(Self::collect_type_refs(ret));
            }
        }
        refs
    }

    /// Collect type references from an operation.
    fn collect_operation_refs(o: &crate::parser::OperationDef) -> Vec<String> {
        let mut refs = Vec::new();

        // Input types
        refs.extend(Self::collect_type_refs_from_fields(&o.inputs));

        // Output type
        if let Some(out) = &o.output {
            refs.extend(Self::collect_type_refs(out));
        }

        // Required interfaces
        refs.extend(o.requires.iter().cloned());

        // Effect targets
        for effect in &o.effects {
            refs.push(effect.target.clone());
        }

        refs
    }

    /// Collect type references from an event.
    fn collect_event_refs(e: &crate::parser::EventDef) -> Vec<String> {
        let mut refs = Self::collect_type_refs_from_fields(&e.fields);

        // Producer
        if let Some(producer) = &e.producer {
            refs.push(producer.clone());
        }

        refs
    }

    /// Collect references from a portal.
    fn collect_portal_refs(p: &crate::parser::PortalDef) -> Vec<String> {
        let mut refs = Vec::new();

        // Clone base
        if let Some(base) = &p.base {
            refs.push(base.clone());
        }

        // Handler property
        for prop in &p.properties {
            if prop.name == "handler" {
                if let crate::parser::PortalValue::Identifier(h) = &prop.value {
                    refs.push(h.clone());
                }
            }
        }

        refs
    }

    /// Collect references from a surface.
    fn collect_surface_refs(s: &crate::parser::SurfaceDef) -> Vec<String> {
        let mut refs = Vec::new();

        // Parent surface
        if let Some(parent) = &s.parent {
            refs.push(parent.clone());
        }

        // Display types
        for display in &s.displays {
            refs.extend(Self::collect_type_refs(&display.display_type));
        }

        // State types
        refs.extend(Self::collect_type_refs_from_fields(&s.states));

        // Event field types
        for event in &s.events {
            refs.extend(Self::collect_type_refs_from_fields(&event.fields));
        }

        refs
    }

    /// Collect type references from a list of fields.
    fn collect_type_refs_from_fields(fields: &[crate::parser::FieldDef]) -> Vec<String> {
        let mut refs = Vec::new();
        for field in fields {
            refs.extend(Self::collect_type_refs(&field.field_type));
        }
        refs
    }

    /// Collect type references from a type expression.
    fn collect_type_refs(type_expr: &crate::parser::TypeExpr) -> Vec<String> {
        use crate::parser::TypeExpr;

        match type_expr {
            TypeExpr::Primitive { .. } => Vec::new(),
            TypeExpr::Modifier { args, .. } => {
                args.iter().flat_map(Self::collect_type_refs).collect()
            }
            TypeExpr::Ref { target, .. } => vec![target.clone()],
            TypeExpr::Optional { inner, .. } => Self::collect_type_refs(inner),
            TypeExpr::TypeUnion { types, .. } => {
                types.iter().flat_map(Self::collect_type_refs).collect()
            }
            TypeExpr::LiteralUnion { .. } => Vec::new(),
            TypeExpr::InlineStruct { fields, .. } => Self::collect_type_refs_from_fields(fields),
            TypeExpr::DecimalPrecision { .. } => Vec::new(),
        }
    }

    /// Get a symbol by name.
    pub fn get(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols.get(name)
    }

    /// Get a mutable symbol by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SymbolInfo> {
        self.symbols.get_mut(name)
    }

    /// Check if a symbol exists.
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Get the namespace.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Iterate over all symbols.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &SymbolInfo)> {
        self.symbols.iter()
    }

    /// Iterate mutably over all symbols.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut SymbolInfo)> {
        self.symbols.iter_mut()
    }

    /// Get symbol count.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
