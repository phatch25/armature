//! Database schema for compiled Armature specs.

/// Schema version for compatibility checking.
pub const SCHEMA_VERSION: u32 = 4;

/// SQLite schema for Armature v2 compiled specs.
pub const SCHEMA_SQL: &str = r#"
-- Armature compiled spec schema v2

-- Metadata
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Project info (enhanced for .ac file support)
CREATE TABLE IF NOT EXISTS project (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    namespace TEXT,
    description TEXT,
    version TEXT,
    context TEXT,
    architecture TEXT,    -- JSON of architecture block
    source_file TEXT,
    compiled_at TEXT NOT NULL,
    frozen INTEGER DEFAULT 0
);

-- Platform configurations (from .ac files)
CREATE TABLE IF NOT EXISTS platform (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id),
    name TEXT NOT NULL,
    definition TEXT NOT NULL,  -- JSON of all platform fields
    UNIQUE(project_id, name)
);

-- Type aliases (from .ac files)
CREATE TABLE IF NOT EXISTS alias (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id),
    name TEXT NOT NULL,
    target_type TEXT NOT NULL,  -- JSON of type expression
    constraints TEXT,           -- JSON of constraints (nullable)
    UNIQUE(project_id, name)
);

-- Build configuration (from .ac files)
CREATE TABLE IF NOT EXISTS build_config (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    UNIQUE(project_id, key)
);

-- Source file tracking for multi-file projects
CREATE TABLE IF NOT EXISTS source_file (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id),
    path TEXT NOT NULL,
    kind TEXT NOT NULL,  -- 'ac' or 'arm'
    hash TEXT,           -- Content hash for incremental compilation
    UNIQUE(project_id, path)
);

-- Namespaces
CREATE TABLE IF NOT EXISTS namespace (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id),
    name TEXT NOT NULL,
    UNIQUE(project_id, name)
);

-- Symbols (all types)
CREATE TABLE IF NOT EXISTS symbol (
    id INTEGER PRIMARY KEY,
    namespace_id INTEGER NOT NULL REFERENCES namespace(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL,  -- model, enum, interface, operation, event, config, portal, surface
    context TEXT,        -- v2 context string (was 'purpose' in v1)
    doc_comment TEXT,    -- Associated block comment
    definition TEXT NOT NULL,  -- JSON of full definition
    status TEXT DEFAULT 'planned',  -- planned, scaffolded, implemented
    UNIQUE(namespace_id, name)
);

-- Fields (for models, operations, configs, events, surfaces)
CREATE TABLE IF NOT EXISTS field (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    name TEXT NOT NULL,
    type_expr TEXT NOT NULL,  -- JSON of type expression
    optional INTEGER DEFAULT 0,
    default_value TEXT,       -- JSON
    constraints TEXT,         -- JSON array of constraints
    field_context TEXT NOT NULL,  -- fields, inputs, states, displays, etc.
    UNIQUE(symbol_id, name, field_context)
);

-- Relations (for models)
CREATE TABLE IF NOT EXISTS relation (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL,         -- has_one, has_many, belongs_to, many_to_many
    target_symbol TEXT NOT NULL,  -- target type name
    UNIQUE(symbol_id, name)
);

-- References (dependencies between symbols)
CREATE TABLE IF NOT EXISTS reference (
    id INTEGER PRIMARY KEY,
    from_symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    to_symbol_name TEXT NOT NULL,  -- target symbol name
    ref_kind TEXT NOT NULL,        -- requires, effects, handler, producer, clone_base, etc.
    ref_context TEXT               -- additional context
);

-- Variants (for enums, including sum type variants with associated data)
CREATE TABLE IF NOT EXISTS variant (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    name TEXT NOT NULL,
    data_type TEXT,  -- JSON of type expression for sum type variants (null for simple enums)
    UNIQUE(symbol_id, name)
);

-- Methods (for interfaces)
CREATE TABLE IF NOT EXISTS method (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    name TEXT NOT NULL,
    params TEXT NOT NULL,       -- JSON array of {name, type}
    return_type TEXT,           -- JSON of type expression (nullable)
    UNIQUE(symbol_id, name)
);

-- Effects (for operations)
CREATE TABLE IF NOT EXISTS effect (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    kind TEXT NOT NULL,         -- creates, updates, deletes, emits, calls
    target TEXT NOT NULL,       -- target symbol name
    UNIQUE(symbol_id, kind, target)
);

-- Portal properties
CREATE TABLE IF NOT EXISTS portal_property (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    name TEXT NOT NULL,
    value TEXT NOT NULL,        -- JSON value
    UNIQUE(symbol_id, name)
);

-- Named strings (errors, invariants, preconditions, postconditions, interactions)
CREATE TABLE IF NOT EXISTS named_string (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id),
    string_context TEXT NOT NULL,  -- errors, invariants, preconditions, postconditions, interactions
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    UNIQUE(symbol_id, string_context, name)
);

-- Implementation metadata
CREATE TABLE IF NOT EXISTS implementation_info (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL REFERENCES symbol(id) UNIQUE,
    file_path TEXT,
    signatures TEXT,            -- JSON array of signature strings
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS symbol_fts USING fts5(
    name,
    context,
    kind,
    doc_comment,
    content='symbol',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS symbol_ai AFTER INSERT ON symbol BEGIN
    INSERT INTO symbol_fts(rowid, name, context, kind, doc_comment)
    VALUES (new.id, new.name, new.context, new.kind, new.doc_comment);
END;

CREATE TRIGGER IF NOT EXISTS symbol_ad AFTER DELETE ON symbol BEGIN
    INSERT INTO symbol_fts(symbol_fts, rowid, name, context, kind, doc_comment)
    VALUES ('delete', old.id, old.name, old.context, old.kind, old.doc_comment);
END;

CREATE TRIGGER IF NOT EXISTS symbol_au AFTER UPDATE ON symbol BEGIN
    INSERT INTO symbol_fts(symbol_fts, rowid, name, context, kind, doc_comment)
    VALUES ('delete', old.id, old.name, old.context, old.kind, old.doc_comment);
    INSERT INTO symbol_fts(rowid, name, context, kind, doc_comment)
    VALUES (new.id, new.name, new.context, new.kind, new.doc_comment);
END;

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_symbol_kind ON symbol(kind);
CREATE INDEX IF NOT EXISTS idx_symbol_status ON symbol(status);
CREATE INDEX IF NOT EXISTS idx_symbol_namespace ON symbol(namespace_id);
CREATE INDEX IF NOT EXISTS idx_reference_from ON reference(from_symbol_id);
CREATE INDEX IF NOT EXISTS idx_reference_to ON reference(to_symbol_name);
CREATE INDEX IF NOT EXISTS idx_field_symbol ON field(symbol_id);
CREATE INDEX IF NOT EXISTS idx_variant_symbol ON variant(symbol_id);
CREATE INDEX IF NOT EXISTS idx_method_symbol ON method(symbol_id);
CREATE INDEX IF NOT EXISTS idx_effect_symbol ON effect(symbol_id);
"#;
