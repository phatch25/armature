//! AST node definitions for Armature v2.

use crate::lexer::SourceLocation;

// =============================================================================
// TOP LEVEL
// =============================================================================

/// Root node representing a parsed .arm or .ac file.
#[derive(Debug, Clone)]
pub struct File {
    pub location: SourceLocation,
    pub namespace: Option<NamespaceDecl>,
    pub imports: Vec<ImportDecl>,
    pub symbols: Vec<SymbolDef>,
}

/// Namespace declaration: `namespace app.auth`
#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub location: SourceLocation,
    pub name: String, // dotted name like "app.auth"
}

/// Import declaration: `import ./path.arm` or `import @pkg/module`
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub location: SourceLocation,
    pub path: String,
}

// =============================================================================
// SYMBOL DEFINITIONS
// =============================================================================

/// All symbol definition types in Armature v2.
#[derive(Debug, Clone)]
pub enum SymbolDef {
    Model(ModelDef),
    Enum(EnumDef),
    Interface(InterfaceDef),
    Operation(OperationDef),
    Event(EventDef),
    Config(ConfigDef),
    Portal(PortalDef),
    Surface(SurfaceDef),
}

impl SymbolDef {
    pub fn name(&self) -> &str {
        match self {
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

    pub fn location(&self) -> SourceLocation {
        match self {
            SymbolDef::Model(m) => m.location,
            SymbolDef::Enum(e) => e.location,
            SymbolDef::Interface(i) => i.location,
            SymbolDef::Operation(o) => o.location,
            SymbolDef::Event(e) => e.location,
            SymbolDef::Config(c) => c.location,
            SymbolDef::Portal(p) => p.location,
            SymbolDef::Surface(s) => s.location,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
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
}

// =============================================================================
// TYPE EXPRESSIONS
// =============================================================================

/// Type expression in Armature v2.
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Primitive type: string, u32, f64, uuid, timestamp, etc.
    Primitive {
        location: SourceLocation,
        name: String,
    },

    /// Modifier type: list<T>, set<T>, map<K,V>, optional<T>
    Modifier {
        location: SourceLocation,
        modifier: String,
        args: Vec<TypeExpr>,
    },

    /// Reference type: ref<User>
    Ref {
        location: SourceLocation,
        target: String,
    },

    /// Optional sugar: T? (equivalent to optional<T>)
    Optional {
        location: SourceLocation,
        inner: Box<TypeExpr>,
    },

    /// Type union: oneof<User, Error, None>
    TypeUnion {
        location: SourceLocation,
        types: Vec<TypeExpr>,
    },

    /// Literal union: oneof<"draft", "published", "archived">
    LiteralUnion {
        location: SourceLocation,
        values: Vec<String>,
    },

    /// Inline struct: { field: type, ... }
    InlineStruct {
        location: SourceLocation,
        fields: Vec<FieldDef>,
    },

    /// Decimal with precision: decimal(10, 2)
    DecimalPrecision {
        location: SourceLocation,
        precision: u32,
        scale: u32,
    },
}

// =============================================================================
// COMMON STRUCTURES
// =============================================================================

/// Field definition: `field name: type = default [constraints]`
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub location: SourceLocation,
    pub name: String,
    pub field_type: TypeExpr,
    pub default: Option<Literal>,
    pub constraints: Vec<Constraint>,
}

/// Constraint on a field: `[unique]` or `[min: 0]` or `[length: 1..100]`
#[derive(Debug, Clone)]
pub struct Constraint {
    pub location: SourceLocation,
    pub name: String,
    pub value: Option<ConstraintValue>,
}

/// Possible values for constraints.
#[derive(Debug, Clone)]
pub enum ConstraintValue {
    String(String),
    Integer(i64),
    Decimal(f64),
    Identifier(String),
    Range { start: i64, end: i64 },
    List(Vec<String>),
}

/// Literal values for defaults.
#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Identifier(String), // Reference to enum variant or constant (e.g., NAMESPACE, IMPORT)
    /// Qualified enum variant: EnumName.Variant (e.g., ThemeName.Default)
    EnumVariant { enum_name: String, variant: String },
    Now,
    None,
    Map(Vec<(String, Literal)>),
    Set(Vec<Literal>),
}

/// Named string: `error_name "description"` or `invariant_name "condition"`
#[derive(Debug, Clone)]
pub struct NamedString {
    pub location: SourceLocation,
    pub name: String,
    pub value: String,
}

// =============================================================================
// MODEL
// =============================================================================

/// Model definition.
#[derive(Debug, Clone)]
pub struct ModelDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub base: Option<String>, // For clone: `model X clone Base { ... }`
    pub fields: Vec<FieldDef>,
    pub relations: Vec<RelationDef>,
    pub invariants: Vec<NamedString>,
}

/// Relation definition: `relation sessions: has_many<Session>`
#[derive(Debug, Clone)]
pub struct RelationDef {
    pub location: SourceLocation,
    pub name: String,
    pub kind: RelationKind,
    pub target: TypeExpr, // Usually ref<T> wrapped in relation kind
}

/// Relation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,
}

impl RelationKind {
    /// Get the string representation of the relation kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationKind::HasOne => "has_one",
            RelationKind::HasMany => "has_many",
            RelationKind::BelongsTo => "belongs_to",
            RelationKind::ManyToMany => "many_to_many",
        }
    }
}

// =============================================================================
// ENUM
// =============================================================================

/// Enum definition with simple variants.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub variants: Vec<VariantDef>,
}

/// Enum variant: `variant EOF` or `variant Model(ModelSpec)` for sum types
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub location: SourceLocation,
    pub name: String,
    /// Optional associated data type for sum type variants (e.g., `variant Model(ModelSpec)`)
    pub data_type: Option<TypeExpr>,
}

// =============================================================================
// INTERFACE
// =============================================================================

/// Interface definition (strict contract).
#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub methods: Vec<MethodDef>,
}

/// Method definition in an interface.
/// Supports both shorthand: `method findById(id: uuid): User?`
/// and block form: `method findById { input id: uuid output: User? error not_found "..." }`
#[derive(Debug, Clone)]
pub struct MethodDef {
    pub location: SourceLocation,
    pub name: String,
    pub context: Option<String>,
    pub params: Vec<MethodParam>,
    pub return_type: Option<TypeExpr>,
    pub errors: Vec<NamedString>,
}

/// Method parameter: `id: uuid` or `user: User`
#[derive(Debug, Clone)]
pub struct MethodParam {
    pub location: SourceLocation,
    pub name: String,
    pub param_type: TypeExpr,
}

// =============================================================================
// OPERATION
// =============================================================================

/// Operation definition.
#[derive(Debug, Clone)]
pub struct OperationDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub inputs: Vec<FieldDef>,
    pub output: Option<TypeExpr>,
    pub errors: Vec<NamedString>,
    pub requires: Vec<String>, // Interface names
    pub effects: Vec<EffectDef>,
    pub preconditions: Vec<NamedString>,
    pub postconditions: Vec<NamedString>,
}

/// Effect definition: `effect creates User` or `effect emits UserCreated`
#[derive(Debug, Clone)]
pub struct EffectDef {
    pub location: SourceLocation,
    pub kind: EffectKind,
    pub target: String,
}

/// Effect kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Creates,
    Updates,
    Deletes,
    Emits,
    Calls,
}

impl EffectKind {
    /// Get the string representation of the effect kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectKind::Creates => "creates",
            EffectKind::Updates => "updates",
            EffectKind::Deletes => "deletes",
            EffectKind::Emits => "emits",
            EffectKind::Calls => "calls",
        }
    }
}

// =============================================================================
// EVENT
// =============================================================================

/// Event definition.
#[derive(Debug, Clone)]
pub struct EventDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub fields: Vec<FieldDef>,
    pub producer: Option<String>,
}

// =============================================================================
// CONFIG
// =============================================================================

/// Config definition.
#[derive(Debug, Clone)]
pub struct ConfigDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub fields: Vec<FieldDef>,
}

// =============================================================================
// PORTAL
// =============================================================================

/// Portal definition (system I/O boundary).
#[derive(Debug, Clone)]
pub struct PortalDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub base: Option<String>, // For clone
    pub properties: Vec<PortalProperty>,
}

/// Portal property: `direction: input` or `transport: http` etc.
#[derive(Debug, Clone)]
pub struct PortalProperty {
    pub location: SourceLocation,
    pub name: String,
    pub value: PortalValue,
}

/// Portal property value.
#[derive(Debug, Clone)]
pub enum PortalValue {
    Identifier(String),
    String(String),
    Integer(i64),
}

// =============================================================================
// SURFACE
// =============================================================================

/// Surface definition (UI/UX).
/// Supports web, mobile, CLI, and embedded displays.
#[derive(Debug, Clone)]
pub struct SurfaceDef {
    pub location: SourceLocation,
    pub name: String,
    pub doc_comment: Option<String>,
    pub context: Option<String>,
    pub parent: Option<String>,
    pub displays: Vec<DisplayDef>,
    pub interactions: Vec<NamedString>,
    pub commands: Vec<CommandDef>,
    pub arguments: Vec<ArgumentDef>,
    pub flags: Vec<FlagDef>,
    pub outputs: Vec<OutputDef>,
    pub properties: Vec<SurfaceProperty>,
    pub states: Vec<FieldDef>,
    pub events: Vec<SurfaceEvent>,
    pub help: Option<String>,
    pub accessibility: Option<AccessibilityDef>,
    pub mapping: Option<MappingDef>,
}

/// Display definition: `display tasks: list<Task>`
#[derive(Debug, Clone)]
pub struct DisplayDef {
    pub location: SourceLocation,
    pub name: String,
    pub display_type: TypeExpr,
}

/// Command definition for CLI surfaces.
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub location: SourceLocation,
    pub name: String,
}

/// Surface property: `property layout: vertical`
#[derive(Debug, Clone)]
pub struct SurfaceProperty {
    pub location: SourceLocation,
    pub name: String,
    pub value: String,
}

/// Surface event: `event onComplete { task_id: uuid }`
#[derive(Debug, Clone)]
pub struct SurfaceEvent {
    pub location: SourceLocation,
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// CLI argument definition: `argument attractor: string?`
#[derive(Debug, Clone)]
pub struct ArgumentDef {
    pub location: SourceLocation,
    pub name: String,
    pub arg_type: TypeExpr,
}

/// CLI flag definition: `flag verbose: bool`
#[derive(Debug, Clone)]
pub struct FlagDef {
    pub location: SourceLocation,
    pub name: String,
    pub flag_type: TypeExpr,
}

/// Output definition: `output format: table` or `output pin: gpio_2`
#[derive(Debug, Clone)]
pub struct OutputDef {
    pub location: SourceLocation,
    pub name: String,
    pub value: String,
}

/// Accessibility block for a11y metadata.
#[derive(Debug, Clone)]
pub struct AccessibilityDef {
    pub location: SourceLocation,
    pub label: Option<String>,
    pub hint: Option<String>,
    pub role: Option<String>,
}

/// Mapping block: maps state values to output values.
/// Example: `mapping { idle: off, running: blink_slow }`
#[derive(Debug, Clone)]
pub struct MappingDef {
    pub location: SourceLocation,
    pub entries: Vec<MappingEntry>,
}

/// Single mapping entry: `idle: off`
#[derive(Debug, Clone)]
pub struct MappingEntry {
    pub location: SourceLocation,
    pub key: String,
    pub value: String,
}

// =============================================================================
// PROJECT CONFIG (.ac files)
// =============================================================================

/// Result of parsing either a .arm or .ac file.
#[derive(Debug, Clone)]
pub enum ParsedFile {
    /// Standard .arm specification file
    Arm(File),
    /// Project configuration .ac file
    Ac(AcFile),
}

impl ParsedFile {
    /// Returns true if this is a .arm file.
    pub fn is_arm(&self) -> bool {
        matches!(self, ParsedFile::Arm(_))
    }

    /// Returns true if this is a .ac file.
    pub fn is_ac(&self) -> bool {
        matches!(self, ParsedFile::Ac(_))
    }

    /// Returns the .arm file if this is one, None otherwise.
    pub fn as_arm(&self) -> Option<&File> {
        match self {
            ParsedFile::Arm(f) => Some(f),
            ParsedFile::Ac(_) => None,
        }
    }

    /// Returns the .ac file if this is one, None otherwise.
    pub fn as_ac(&self) -> Option<&AcFile> {
        match self {
            ParsedFile::Arm(_) => None,
            ParsedFile::Ac(f) => Some(f),
        }
    }

    /// Consumes self and returns the .arm file if this is one.
    pub fn into_arm(self) -> Option<File> {
        match self {
            ParsedFile::Arm(f) => Some(f),
            ParsedFile::Ac(_) => None,
        }
    }

    /// Consumes self and returns the .ac file if this is one.
    pub fn into_ac(self) -> Option<AcFile> {
        match self {
            ParsedFile::Arm(_) => None,
            ParsedFile::Ac(f) => Some(f),
        }
    }
}

/// Root node for .ac project configuration files.
///
/// Project files define project metadata, platform translation rules,
/// type aliases, imports, and build configuration.
#[derive(Debug, Clone)]
pub struct AcFile {
    pub location: SourceLocation,
    pub project: Option<ProjectBlock>,
    pub platforms: Vec<PlatformBlock>,
    pub aliases: Vec<AliasDecl>,
    pub imports: Vec<ImportDecl>,
    pub build: Option<BuildBlock>,
}

/// Project block: `project "Name" { description "..." version "1.0.0" }`
#[derive(Debug, Clone)]
pub struct ProjectBlock {
    pub location: SourceLocation,
    pub name: String,
    pub fields: Vec<ProjectField>,
}

impl ProjectBlock {
    /// Get a field value by name.
    pub fn get_field(&self, name: &str) -> Option<&ProjectValue> {
        self.fields.iter().find(|f| f.name == name).map(|f| &f.value)
    }

    /// Get a string field value by name.
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.get_field(name).and_then(|v| match v {
            ProjectValue::String(s) => Some(s.as_str()),
            _ => None,
        })
    }
}

/// Project field: key-value pair within a project block.
#[derive(Debug, Clone)]
pub struct ProjectField {
    pub location: SourceLocation,
    pub name: String,
    pub value: ProjectValue,
}

/// Value types allowed in project blocks.
#[derive(Debug, Clone)]
pub enum ProjectValue {
    /// Simple string value: `description "Some text"`
    String(String),
    /// Nested block for structured data: `architecture { pattern: "MVVM" }`
    Block(Vec<ProjectField>),
}

/// Platform block: `platform rust { naming: snake_case optional: Option<T> }`
#[derive(Debug, Clone)]
pub struct PlatformBlock {
    pub location: SourceLocation,
    pub name: String,
    pub fields: Vec<PlatformField>,
}

impl PlatformBlock {
    /// Get a field value by name.
    pub fn get_field(&self, name: &str) -> Option<&PlatformValue> {
        self.fields.iter().find(|f| f.name == name).map(|f| &f.value)
    }
}

/// Platform field: key-value pair within a platform block.
#[derive(Debug, Clone)]
pub struct PlatformField {
    pub location: SourceLocation,
    pub name: String,
    pub value: PlatformValue,
}

/// Value types allowed in platform blocks.
#[derive(Debug, Clone)]
pub enum PlatformValue {
    /// Identifier value: `naming: camelCase`
    Identifier(String),
    /// Type expression: `optional: T?` or `result: Result<T>`
    TypeExpr(TypeExpr),
    /// String value: `models: "src/main/java/models/"`
    String(String),
    /// Nested block: `structure { models: "src/..." }`
    Block(Vec<PlatformField>),
}

/// Type alias declaration: `alias UserId = uuid` or `alias Email = string [format: email]`
#[derive(Debug, Clone)]
pub struct AliasDecl {
    pub location: SourceLocation,
    pub name: String,
    pub target_type: TypeExpr,
    pub constraints: Vec<Constraint>,
}

/// Build configuration block: `build { output: "dist/" mcp_db: ".armature/spec.db" }`
#[derive(Debug, Clone)]
pub struct BuildBlock {
    pub location: SourceLocation,
    pub fields: Vec<BuildField>,
}

impl BuildBlock {
    /// Get a field value by name.
    pub fn get_field(&self, name: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.value.as_str())
    }
}

/// Build field: key-value pair within a build block.
#[derive(Debug, Clone)]
pub struct BuildField {
    pub location: SourceLocation,
    pub name: String,
    pub value: String,
}
