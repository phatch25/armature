# Armature Syntax Specification

**Version:** 2.0-draft
**Status:** Normative

This document defines the syntax of the Armature language. Parsers MUST accept all constructs defined here and MUST reject constructs not defined here.

---

## Notation

This spec uses a modified BNF:

```
rule      := production              # definition
'literal' := literal token           # quoted literals
UPPER     := terminal (token class)  # uppercase = token
lower     := non-terminal            # lowercase = rule
a b       := sequence                # a followed by b
a | b     := alternation             # a or b
a?        := optional                # zero or one
a*        := repetition              # zero or more
a+        := repetition              # one or more
(a b)     := grouping                # group for precedence
```

---

## 1. Lexical Structure

### 1.1 Character Set

Source files are UTF-8 encoded.

### 1.2 Whitespace

```
WHITESPACE := ' ' | '\t' | '\r' | '\n'
```

Whitespace separates tokens. It is not significant except within strings.

### 1.3 Comments

```
LINE_COMMENT  := '#' (!('*' | '\n'))? (!'\n')* '\n'
BLOCK_COMMENT := '#*' .* '*#'
```

Line comments extend from `#` to end of line.
Block comments use `#* ... *#` and can span multiple lines. Block comments are captured as doc comments when placed before symbol definitions.

### 1.4 Identifiers

```
IDENT         := IDENT_START IDENT_CONT*
IDENT_START   := 'a'..'z' | 'A'..'Z' | '_'
IDENT_CONT    := IDENT_START | '0'..'9'
```

Identifiers are case-sensitive. Convention:
- `PascalCase` for symbol names (types, models, operations)
- `snake_case` for field names, method names, variant names

### 1.5 Qualified Names

```
QUALIFIED_NAME := IDENT ('.' IDENT)*
```

Used for namespace references and symbol references (e.g., `app.auth.User`).

### 1.6 Keywords and Names

Armature has **contextual keywords**. A word is only a keyword when it appears in a keyword position. In name positions, any word (including keywords) can be used as a name.

**Keyword positions:** The start of a construct where the parser expects a specific keyword.

**Name positions:** After a keyword, where the parser expects a name (symbol name, field name, method name, etc.).

```armature
model field {           # "model" = keyword, "field" = name
  field field: string   # "field" = keyword, "field" = name, "string" = type
  field model: config   # "field" = keyword, "model" = name, "config" = type
}
```

**Keywords by category:**

```
# Symbol kinds (introduce top-level definitions)
model  enum  interface  operation  event  config  portal  surface

# Member keywords (introduce members within symbols)
namespace  context  field  relation  variant  method
input  output  error  requires  effect  precondition  postcondition
invariant  producer  clone  display  state  interaction  on
command  argument  flag  help  mapping  parent  property  accessibility

# Type constructors
ref  list  set  map  oneof  optional  ptr

# Relation kinds
has_one  has_many  belongs_to  many_to_many

# Effect kinds
creates  updates  deletes  emits  calls

# Literals
true  false  now  none

# Portal/surface
direction  transport  handler  data_source

# Project file (.ac)
project  platform  alias  import  build
```

**Style convention (not enforced):**
- `PascalCase` for symbol names: `User`, `CreateUser`, `UserStore`
- `snake_case` for field/method/input names: `user_id`, `find_by_email`

### 1.7 Literals

#### String Literals

```
STRING        := SINGLE_STRING | TRIPLE_STRING
SINGLE_STRING := '"' STRING_CHAR* '"'
TRIPLE_STRING := '"""' .* '"""'
STRING_CHAR   := !('"' | '\n') | ESCAPE_SEQ
ESCAPE_SEQ    := '\\' ('n' | 't' | 'r' | '"' | '\\')
```

Single-quoted strings cannot contain unescaped newlines.
Triple-quoted strings (`"""..."""`) can span multiple lines.

Escape sequences:
- `\n` - newline
- `\t` - tab
- `\r` - carriage return
- `\"` - literal quote
- `\\` - literal backslash

#### Integer Literals

```
INTEGER := DECIMAL_INT | HEX_INT
DECIMAL_INT := '0' | ('1'..'9' DIGIT*)
HEX_INT := '0x' HEX_DIGIT+
DIGIT := '0'..'9'
HEX_DIGIT := DIGIT | 'a'..'f' | 'A'..'F'
```

#### Float Literals

```
FLOAT     := DIGIT+ '.' DIGIT+ EXPONENT?
          |  DIGIT+ EXPONENT
EXPONENT  := ('e' | 'E') ('+' | '-')? DIGIT+
```

Scientific notation is supported (e.g., `1e10`, `6.02e-23`, `1.5E+3`).

#### Range Literals

```
RANGE := INTEGER '..' INTEGER
```

Used in constraints (e.g., `length: 1..100`).

---

## 2. File Structure

### 2.1 Specification Files (.arm)

```
arm_file := namespace_decl? import_decl* symbol_def*

namespace_decl := 'namespace' QUALIFIED_NAME

import_decl := 'import' STRING
```

A `.arm` file contains an optional namespace declaration, optional imports, and symbol definitions.

### 2.2 Project Files (.ac)

```
ac_file := project_block? platform_block* alias_decl* import_decl* build_block?

project_block := 'project' STRING '{' project_field* '}'
project_field := IDENT STRING

platform_block := 'platform' IDENT '{' platform_field* '}'
platform_field := IDENT ':' (IDENT | type_expr)

alias_decl := 'alias' IDENT '=' type_expr

import_decl := 'import' STRING

build_block := 'build' '{' build_field* '}'
build_field := IDENT ':' STRING
```

---

## 3. Symbol Definitions

### 3.1 General Form

```
symbol_def := symbol_kind IDENT clone_clause? '{' symbol_body '}'

symbol_kind := 'model' | 'enum' | 'interface' | 'operation'
             | 'event' | 'config' | 'portal' | 'surface'

clone_clause := 'clone' IDENT
```

### 3.2 Context Clause

Valid in all symbol kinds:

```
context_clause := 'context' STRING
```

### 3.3 Model

```
model_body := context_clause? (field_def | relation_def | invariant_def)*

field_def := 'field' IDENT ':' type_expr constraints? default_value?
constraints := '[' constraint (',' constraint)* ']'
constraint := IDENT (':' constraint_value)?
constraint_value := IDENT | STRING | INTEGER | RANGE | array_literal | QUALIFIED_NAME
array_literal := '[' (STRING (',' STRING)*)? ']'
default_value := '=' literal_value

relation_def := 'relation' IDENT ':' relation_type
relation_type := relation_kind '<' 'ref' '<' IDENT '>' '>'
relation_kind := 'has_one' | 'has_many' | 'belongs_to' | 'many_to_many'

invariant_def := 'invariant' IDENT STRING
```

### 3.4 Enum

```
enum_body := context_clause? variant_def+

variant_def := 'variant' IDENT data_type_clause?
data_type_clause := '(' type_expr ')'
```

Enums MUST have at least one variant.

**Simple enums** have variants with no associated data:

```armature
enum Status {
    variant Pending
    variant Active
    variant Suspended
}
```

**Sum types** (tagged unions) have variants with associated data types:

```armature
enum SymbolSpec {
    variant Model(ModelSpec)
    variant Operation(OperationSpec)
    variant Interface(InterfaceSpec)
    variant Simple              # Mixed: some with data, some without
}
```

Sum type variants can carry any valid type expression:

```armature
enum Result {
    variant Ok(User)           # Reference type
    variant Error(string)      # Primitive type
    variant Multiple(list<User>)  # Collection type
    variant Complex({ code: i32, message: string })  # Inline struct
}
```

### 3.5 Interface

```
interface_body := context_clause? method_def+

method_def := method_inline | method_block

method_inline := 'method' IDENT '(' param_list? ')' ':' type_expr
param_list := param (',' param)*
param := IDENT ':' type_expr

method_block := 'method' IDENT '{' method_body '}'
method_body := context_clause? (input_def | output_def | error_def)*
input_def := 'input' IDENT ':' type_expr default_value?
output_def := 'output' ':' type_expr
error_def := 'error' IDENT STRING
```

Interfaces MUST have at least one method.

**Method block form** supports `context` for per-method documentation:

```armature
interface SpecTools {
    context "MCP tool interface"

    method task {
        context "Get implementation task with resolved dependencies"
        input name: string
        output: ImplementationTask
        error not_found "Symbol does not exist"
    }
}
```

### 3.6 Operation

```
operation_body := context_clause?
                  input_def*
                  output_def?
                  error_def*
                  requires_def*
                  effect_def*
                  precondition_def*
                  postcondition_def*

requires_def := 'requires' IDENT
effect_def := 'effect' effect_kind IDENT
effect_kind := 'creates' | 'updates' | 'deletes' | 'emits' | 'calls'
precondition_def := 'precondition' IDENT STRING
postcondition_def := 'postcondition' IDENT STRING
```

### 3.7 Event

```
event_body := context_clause? field_def* producer_def?

producer_def := 'producer' IDENT
```

### 3.8 Config

```
config_body := context_clause? field_def*
```

Config fields SHOULD have default values.

### 3.9 Portal

```
portal_body := context_clause? portal_property+

portal_property := IDENT ':' portal_value
portal_value := IDENT | STRING | INTEGER | HEX_INT
```

Required properties:
- `direction`: `input` | `output`
- `transport`: identifier (e.g., `http`, `mqtt`, `uart`)

Common properties by transport:
- **http**: `method`, `path`, `format`, `handler`
- **mcp**: `tool_name`, `handler`
- **uart**: `baud`, `format`, `handler`
- **mqtt**: `topic`, `qos`, `handler`

### 3.10 Surface

```
surface_body := context_clause?
                parent_def?
                (display_def | state_def | interaction_def |
                 on_event_def | command_def | argument_def |
                 flag_def | help_def | output_def | property_def |
                 mapping_def | accessibility_def)*

parent_def := 'parent' IDENT
display_def := 'display' IDENT ':' type_expr
state_def := 'state' IDENT ':' type_expr default_value?
interaction_def := 'interaction' IDENT STRING
on_event_def := 'on' IDENT '{' (IDENT ':' type_expr (',' IDENT ':' type_expr)*)? '}'
command_def := 'command' STRING
argument_def := 'argument' IDENT ':' type_expr
flag_def := 'flag' IDENT ':' type_expr
help_def := 'help' STRING
output_def := 'output' IDENT ':' IDENT
property_def := 'property' IDENT ':' IDENT
mapping_def := 'mapping' '{' mapping_entry* '}'
mapping_entry := IDENT ':' IDENT
accessibility_def := 'accessibility' '{' accessibility_entry* '}'
accessibility_entry := IDENT (':' IDENT | STRING)
```

---

## 4. Type Expressions

```
type_expr := base_type optional_marker?

base_type := primitive_type
           | ref_type
           | collection_type
           | oneof_type
           | inline_struct
           | IDENT                    # User-defined type or alias

inline_struct := '{' (IDENT ':' type_expr (',' IDENT ':' type_expr)*)? '}'

optional_marker := '?'

primitive_type := 'uuid' | 'ulid'
                | 'string' | 'bytes' | 'char'
                | 'bool'
                | 'u8' | 'u16' | 'u32' | 'u64' | 'u128'
                | 'i8' | 'i16' | 'i32' | 'i64' | 'i128'
                | 'f32' | 'f64'
                | decimal_type
                | 'timestamp' | 'duration'
                | 'size' | 'any' | 'void' | 'none'

decimal_type := 'decimal' '(' INTEGER ',' INTEGER ')'

ref_type := 'ref' '<' IDENT '>'

collection_type := list_type | set_type | map_type | ptr_type
list_type := 'list' '<' type_expr '>'
set_type := 'set' '<' type_expr '>'
map_type := 'map' '<' type_expr ',' type_expr '>'
ptr_type := 'ptr' '<' type_expr '>'

oneof_type := 'oneof' '<' oneof_members '>'
oneof_members := type_member (',' type_member)+    # min 2
type_member := IDENT | STRING
```

**Rules:**
- `oneof` MUST have at least 2 members
- `oneof` members MUST be all types OR all string literals, not mixed
- `ref<T>` is for relationships; bare `T` is for embedded values
- `optional_marker` (`?`) is equivalent to wrapping in `optional<T>`

---

## 5. Literal Values

```
literal_value := STRING
              | INTEGER
              | FLOAT
              | 'true' | 'false'
              | QUALIFIED_NAME          # Enum variant: EnumName.Variant
              | IDENT                   # Bare identifier (e.g., 'now')
```

---

## 6. Constraints

### 6.1 Validation Constraints

| Constraint | Value | Meaning |
|------------|-------|---------|
| `unique` | none | No duplicate values |
| `format` | identifier | Named format: `email`, `url`, `uuid`, `e164` |
| `pattern` | string | Described regex pattern |
| `values` | array | Enumerated allowed values |
| `min` | integer | Minimum value (inclusive) |
| `max` | integer | Maximum value (inclusive) |
| `length` | range | String length bounds |
| `references` | qualified_name | Foreign key to Model.field |

### 6.2 Metadata Constraints

| Constraint | Value | Meaning |
|------------|-------|---------|
| `primary` | none | Primary key field |
| `sensitive` | none | Contains PII or secrets |
| `immutable` | none | Can only be set once |
| `readonly` | none | Never writable by users |
| `derived` | none | Computed value |
| `indexed` | none | Should be indexed for queries |
| `default` | literal | Default value |
| `computed_by` | identifier | Symbol that computes this value |

---

## 7. Ordering and Optionality

### 7.1 Clause Ordering

Within a symbol body, clauses MAY appear in any order. However, the RECOMMENDED order is:

**model**: context, fields, relations, invariants
**enum**: context, variants
**interface**: context, methods
**operation**: context, inputs, output, errors, requires, effects, preconditions, postconditions
**event**: context, fields, producer
**config**: context, fields
**portal**: context, direction, transport, other properties, handler
**surface**: context, parent, displays, state, interactions, events

### 7.2 Required vs Optional

| Symbol | Required | Optional |
|--------|----------|----------|
| model | at least one field or relation | context, invariants |
| enum | at least one variant | context |
| interface | at least one method | context |
| operation | (none) | all clauses |
| event | (none) | context, fields, producer |
| config | (none) | context, fields |
| portal | direction, transport | context, all others |
| surface | (none) | all clauses |

---

## 8. Whitespace and Formatting

### 8.1 Not Significant

Whitespace (spaces, tabs, newlines) is not significant except:
- Within string literals
- To separate tokens

### 8.2 No Semicolons

Statements are NOT terminated by semicolons. Newlines are not significant.

### 8.3 No Trailing Commas Required

Commas separate items in lists. Trailing commas are OPTIONAL.

```armature
# Both valid:
field tags: list<string> [unique, indexed]
field tags: list<string> [unique, indexed,]
```

---

## 9. Ambiguity Resolutions

### 9.1 Method Syntax

Both inline and block forms are valid:

```armature
# Inline - for simple signatures
method findById(id: uuid): User?

# Block - for complex signatures with errors
method findById {
  input id: uuid
  output: User?
  error not_found "User not found"
}
```

The block form MUST be used when the method declares errors.

### 9.2 Default Values

Default values use `=` after the type (and constraints if present):

```armature
field active: bool = true
field role: string [default: "user"]     # INVALID - use = syntax
field role: string = "user"              # Valid
```

The `default` constraint is reserved for special values like `now`:

```armature
field created_at: timestamp [default: now]
```

### 9.3 Optional Types

Both forms are equivalent:

```armature
field name: string?
field name: optional<string>
```

The `?` suffix is PREFERRED for brevity.

### 9.4 Clone Override

When cloning, redeclaring a field overrides the parent's definition completely:

```armature
model Base {
  field id: uuid
  field name: string
}

model Child clone Base {
  field name: string [length: 1..50]  # Overrides Base.name entirely
}
```

---

## 10. Reserved for Future

The following are reserved and MUST NOT be used:

- C-style block comments (`/* */`)
- Semicolons as statement terminators
- `extends`, `implements` as keywords (use `clone`)
- `type` as a symbol kind
- `fn`, `func`, `def` as keywords
- `mixin`, `include`, `embed` (potential future composition features)

---

## Appendix A: Grammar Summary

```
# Files
arm_file := namespace_decl? import_decl* symbol_def*
ac_file := project_block? platform_block* alias_decl* import_decl* build_block?

# Symbols
symbol_def := symbol_kind IDENT clone_clause? '{' symbol_body '}'
symbol_kind := 'model' | 'enum' | 'interface' | 'operation' | 'event' | 'config' | 'portal' | 'surface'
clone_clause := 'clone' IDENT

# Enum variants (including sum types)
variant_def := 'variant' IDENT ('(' type_expr ')')?

# Types
type_expr := base_type '?'?
base_type := primitive_type | ref_type | collection_type | oneof_type | IDENT
primitive_type := 'uuid' | 'string' | 'bool' | 'u8' | ... | 'timestamp' | 'void' | 'none'
ref_type := 'ref' '<' IDENT '>'
collection_type := ('list' | 'set' | 'ptr') '<' type_expr '>' | 'map' '<' type_expr ',' type_expr '>'
oneof_type := 'oneof' '<' (IDENT | STRING) (',' (IDENT | STRING))+ '>'

# Common clauses
field_def := 'field' IDENT ':' type_expr constraints? ('=' literal)?
constraints := '[' constraint (',' constraint)* ']'
input_def := 'input' IDENT ':' type_expr ('=' literal)?
output_def := 'output' ':' type_expr
error_def := 'error' IDENT STRING
```
