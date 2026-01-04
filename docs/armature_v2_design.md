# Armature v2 Design Specification

## Core Philosophy

**Armature is a focus mechanism for AI agents, not a programming language.**

The problem: Billion-dollar models can do anything, but inconsistently and unfocused.
The solution: Structured intent that agents query. Spec defines shape, agent implements idiomatically.

**Principles:**
- Specify *what* to build, not *how* to build it
- Reduce the search space from "anything" to "this shape"
- Explicit types required (no ambiguous `identifier`)
- Concrete semantics over abstract types
- Language-agnostic via platform translation rules
- Compiler enforces precision, AI implements idiomatically
- Context fields are load-bearing - they provide direction, not documentation

---

## File Structure

### .ac Files (Project Config)
Entry point for compilation. Defines platform, imports, aliases, and project context.

```armature
project "TidyBot" {
  description "Smart house cleaning manager"
  version "1.0.0"
  
  context "Mobile Android app for managing household cleaning tasks..."
  
  architecture {
    pattern: "MVVM"
    database: "Room (SQLite)"
    ui: "Jetpack Compose"
  }
}

platform android_kotlin {
  naming: camelCase
  optional: T?
  result: Result<T>
  
  structure {
    models: "src/main/java/models/"
    operations: "src/main/java/usecases/"
  }
}

# Type aliases
alias UserId = uuid
alias Email = string [format: email, max_length: 255]

# Project imports
import ./src/models.arm
import ./src/operations.arm
import @armature/auth

build {
  output: "app/build/"
  mcp_db: ".armature/project.db"
}
```

**Compilation:** `armature compile project.ac -o .armature/project.db`

### .arm Files (Pure Specs)
Platform-agnostic symbol definitions. Reusable across projects.

```armature
namespace app.auth
import ./common/types.arm

model User {
  field id: uuid
  field email: string [unique, format: email]
  field password_hash: bytes [sensitive]
}

interface UserStore {
  method findById(id: uuid): User?
  method insert(user: User): uuid
}
```

**Import behavior:**
- Entire file imported, compiler culls unused symbols
- Imported files must be "safe" (self-contained, compile independently)
- Import-once guaranteed (no duplicates, even in diamond dependencies)

---

## Type System

### Concrete Types (Required)
```armature
# Sized integers
u8, u16, u32, u64, u128
i8, i16, i32, i64, i128

# Floats
f32, f64

# Fixed precision
decimal(precision, scale)  # e.g., decimal(10, 2) for money

# Text
string        # UTF-8 string
bytes         # raw binary
char          # single Unicode codepoint

# Temporal
timestamp     # Point in time (platform determines representation)

# Collections
list<T>
set<T>
map<K,V>

# Optionality
T?                    # Optional type (sugar for optional<T>)
optional<T>           # Explicit optional (equivalent to T?)

# References (for relationships only)
ref<Symbol>           # Relationship to another symbol (foreign keys, relations)

# Union types
oneof<T1, T2, T3>     # Type union: value is one of these types
oneof<"a", "b", "c">  # Literal union: value is one of these strings

# Platform-specific (when needed)
size          # usize/size_t
ptr<T>        # raw pointer (unsafe)
any           # escape hatch (explicit opt-in)
```

### Platform Translation
Spec defines canonical form. Compiler translates to platform idioms.

**Example:**
```armature
method findById(id: uuid): User?
```

Translates to:
- **Rust:** `fn find_by_id(&self, id: Uuid) -> Option<User>`
- **Python:** `def find_by_id(self, id: str) -> User | None`
- **TypeScript:** `findById(id: string): User | null`

Platform conventions declared in `.ac` file guide translation.

---

## Union Types

### Type Unions
Value can be one of several types:

```armature
field value: oneof<string, i64, f64, None>
field result: oneof<User, Error>
output: oneof<Success, Failure, Pending>
```

**Compiler validation:**
- All members must be type identifiers (not string literals)
- No duplicate types
- **Must have at least two members** - single-element unions are pointless
- Platform translates to appropriate union representation

### Literal Unions
Value must be one of specific literal strings:

```armature
field status: oneof<"draft", "published", "archived">
field direction: oneof<"input", "output">
field method: oneof<"GET", "POST", "PUT", "DELETE">
```

**Compiler validation:**
- All members must be string literals (not type identifiers)
- **Cannot mix literals and types:** `oneof<"draft", User>` is an error
- **Must have at least two members** - single-element unions are pointless
- Numeric literal unions not supported - use enums instead

**When to use:**
- **Type unions:** Return values that could be different types, error handling
- **Literal unions:** String constants, status values, small fixed sets
- **Enums:** Named variants, larger sets, when you want explicit type safety

---

## Reference Semantics

**`ref<T>` is for relationships only** - entities with independent identity that this model points to.

```armature
# Relationships - ref<> means "points to separate entity"
model Task {
  field owner_id: uuid [references: User.id]  # Explicit FK
  field owner: ref<User>                       # Relationship to User (has own identity)
  relation zone: belongs_to<ref<Zone>>         # Relation type
}

# Embedded - bare type means "contains, no separate identity"
model Token {
  field token_type: TokenType
  field location: SourceLocation  # Embedded struct, not a relationship
}

# Signatures - always bare types
operation GetUser {
  input id: uuid
  output: User?        # Not ref<User>?
}

interface UserStore {
  method findById(id: uuid): User?  # Not ref<User>?
}
```

**The rule:**
- **`ref<Symbol>`** = "points to a separate entity with its own identity" (foreign key, relationship)
- **`Symbol`** (bare) = "contains/embeds this type" (value semantics, inline)
- **Signatures** = always bare types, never `ref<>`

**When to use each:**
| Use | Semantics | Example |
|-----|-----------|---------|
| `ref<User>` | User exists independently, this is a pointer/FK | `field author: ref<User>` |
| `User` | User is embedded/inline, no separate identity | `field location: SourceLocation` |

**Mutation:** Use `effect` blocks to document what an operation modifies. The agent infers mutability from effects, not from parameter annotations.

```armature
operation UpdateUser {
  input user: User
  output: User
  effect updates User  # Agent knows this mutates User
}
```

---

## Method Syntax

Interfaces support both **shorthand** and **flattened** syntax:

### Shorthand (Recommended)
```armature
interface UserStore {
  method findById(id: uuid): User?
  method findByEmail(email: string): User?
  method insert(user: User): uuid
  method update(user: User): void
  method delete(id: uuid): bool
}
```

### Flattened (Consistent with operations)
```armature
interface UserStore {
  method findById {
    input id: uuid
    output: User?
  }
  
  method insert {
    input user: User
    output: uuid
  }
}
```

Both are valid. Use shorthand for simple signatures, flattened for complex ones with errors/constraints.

---

## Constraint Syntax

Fields can have validation constraints, metadata markers, and behavioral hints.

### Available Constraints

```armature
# Validation
unique                           # No duplicates
format: email|url|uuid|e164      # Named formats
pattern: "description"           # Described pattern (not regex)
values: ["a", "b", "c"]          # Enumerated allowed values
min: N / max: N                  # Value bounds
length: A..B                     # String length range
size: N                          # Bytes size
references: Model.field          # Foreign key validation
custom: "description"            # Escape hatch for special validation

# Metadata
sensitive                        # PII/secrets
immutable                        # Set once, never changes
readonly                         # Never writable
derived                          # Computed, not stored
computed_by: Operation           # Explicit computation source
indexed                          # Query optimization hint
default: value                   # Default value
```

### Examples

```armature
field email: string [unique, format: email]
field age: u8 [min: 0, max: 150]
field name: string [length: 1..100]
field slug: string [pattern: "lowercase-with-hyphens"]
field password_hash: bytes [sensitive, immutable]
field created_at: timestamp [readonly, default: now]
field score: decimal [derived, computed_by: CalculateScore]
field user_id: uuid [references: User.id, indexed]
field status: string [values: ["draft", "published", "archived"]]
field bitcoin_addr: string [custom: "valid Bitcoin address format"]
```

### Constraint Philosophy

- **Semantic over syntax**: Use `format: email` not regex patterns
- **Descriptive patterns**: `pattern: "lowercase-with-hyphens"` describes intent
- **Type-aware**: Don't specify redundant constraints (e.g., `u8 [min: 0]`)
- **Custom escape hatch**: When standard constraints don't fit, describe what you need

---

## Symbol Definitions

### Flattened, Keyword-Prefixed Syntax
Order within blocks is meaningless. Keywords make structure explicit.

#### Model
```armature
model User {
  context "A registered user. Email is unique, passwords hashed with Argon2id."
  
  field id: uuid [primary]
  field email: string [unique, format: email]
  field password_hash: bytes [sensitive]
  field created_at: timestamp = now
  field active: bool = true
  
  relation sessions: has_many<Session>
  relation profile: has_one<UserProfile>
  
  invariant email_lowercase "email is always stored lowercase"
  invariant password_hashed "password_hash is never plaintext"
}
```

**Keywords:**
- `context` - optional, ~250 chars soft limit (compiler warns, doesn't reject). Brevity forces clarity. Describes purpose/usage for the implementing agent.
- `field` - data field with type and constraints
- `relation` - relationship to other models
- `invariant` - condition that must always hold (generated as assertions)

#### Interface (formerly Service)
```armature
interface UserStore {
  context "Persistent storage for user records"
  
  method findById(id: uuid): User?
  method findByEmail(email: string): User?
  method insert(user: User): uuid
  method update(user: User): void
  method delete(id: uuid): bool
}
```

Interfaces are **strict contracts**. Implementation must satisfy all method signatures.

#### Operation
```armature
operation CreateUser {
  context "Register a new user account with email/password authentication"
  
  input email: string
  input password: string
  input name: string
  
  output: User
  
  error email_taken "A user with this email already exists"
  error invalid_email "Email format is invalid"
  error weak_password "Password does not meet strength requirements"
  
  requires UserStore
  requires PasswordHasher
  
  effect creates User
  effect emits UserCreated
  
  precondition email_valid "email matches email format"
  precondition password_strong "password meets minimum strength requirements"
  
  postcondition user_exists "User with email can be retrieved"
  postcondition password_hashed "Stored password_hash is not plaintext"
}
```

**Keywords:**
- `input` - required parameter
- `output` - return type
- `error` - named error case with description
- `requires` - interface dependency
- `effect` - side effect (creates/updates/deletes/emits)
- `precondition` - must be true before execution
- `postcondition` - must be true after execution

#### Event
```armature
event UserCreated {
  context "Emitted when a new user successfully registers"
  
  field user_id: uuid
  field email: string
  field created_at: timestamp
  
  producer CreateUser
}
```

#### Enum
Simple enumerations with named variants. For data-carrying variants, use separate models with context explaining the association.

```armature
enum TokenType {
  context "All possible token types in the lexer"
  
  variant EOF
  variant NEWLINE
  variant DOC_COMMENT
  variant STRING
  variant INTEGER
  variant IDENTIFIER
}

enum Result {
  context "Operation result - either success or error"
  
  variant Ok
  variant Error
}

# Associated data via separate models
model OkValue {
  context "Successful result - associated with Result::Ok variant"
  field value: any
}

model ErrorValue {
  context "Error result - associated with Result::Error variant"
  field message: string
  field code: u32?
}
```

**Rules:**
- Must have at least one variant
- Variants are simple identifiers (no inline data)
- No duplicate variant names

**Usage:**
```armature
field status: TokenType
field result: Result
```

#### Config
```armature
config AppConfig {
  context "Application-wide settings"
  
  field default_time_budget_minutes: u32 = 30
  field max_tasks_per_day: u32 = 5
  field streak_reset_hours: u32 = 36
}
```

#### Portal (System Boundaries)
Defines where data enters or exits the system. Communication with external systems, protocols, and transports.

```armature
portal RegisterUser {
  direction: input
  transport: http
  method: POST
  path: "/auth/register"
  format: json
  handler: CreateUser
}

portal SensorReadings {
  direction: input
  transport: uart
  baud: 115200
  format: ascii
  handler: ProcessSensor
}

portal CANOutput {
  direction: output
  transport: can_j1939
  message_id: 0x18EEFF00
  format: binary
  data_source: FormatTrajectory
}

portal OrderEvents {
  direction: input
  transport: rabbitmq
  queue: "orders"
  format: json
  handler: ProcessOrder
}
```

**Keywords:**
- `direction` - `input` (consume) or `output` (produce)
- `transport` - protocol/physical layer (http, uart, can, mqtt, kafka, etc.)
- `format` - data encoding (json, binary, protobuf, ascii, etc.)
- `handler` - operation that processes input
- `data_source` - operation that produces output
- Protocol-specific fields (method, path, baud, message_id, etc.)

#### Surface (UI/UX)
Defines user interface and interaction. What humans/operators see and interact with. Cross-platform: web, mobile, CLI, embedded displays, etc.

```armature
surface DailyTaskList {
  parent MainScreen
  
  context "Main screen showing today's prioritized tasks"
  
  display tasks: list<Task>
  display progress: TaskProgress
  
  interaction swipe_complete "Swipe task right to complete"
  interaction tap_details "Tap to expand task details"
  interaction pull_refresh "Pull down to refresh list"
  
  property layout: vertical
  property spacing: 16
  
  accessibility {
    label "Daily task list"
    hint "Shows prioritized cleaning tasks for today"
    role: main
  }
  
  state loading: bool = false
  state selected: Task?
  
  event onComplete { task_id: uuid }
  event onRefresh { }
}

surface ListCommand {
  command "list"
  argument attractor: string?
  flag verbose: bool
  flag help: bool
  
  output format: table
  
  help "List all available strange attractors with parameters"
  
  event onExecute { attractor: string?, verbose: bool }
}

surface StatusLED {
  display system_state: oneof<idle, running, error, complete>
  
  output pin: gpio_2
  output pattern: oneof<off, solid, blink_slow, blink_fast>
  
  mapping {
    idle: off
    running: blink_slow
    error: blink_fast
    complete: solid
  }
}
```

**Keywords:**
- `parent` - hierarchical navigation (surfaces can nest)
- `display` - what data is shown to user
- `interaction` - user actions with descriptions
- `command` - CLI command name
- `argument` - positional CLI argument
- `flag` - CLI flag/option
- `property` - visual/layout hints (optional, platform can ignore)
- `accessibility` - a11y metadata
- `help` - documentation string
- `state` - local UI state
- `event` - user-triggered events
- `mapping` - state-to-output mappings

**Portal vs Surface:**
- **Portal** = System I/O boundary (data enters/exits)
- **Surface** = User interface (humans interact)

Example - web login has both:
```armature
portal LoginAPI {
  direction: input
  transport: http
  path: "/auth/login"
  handler: AuthenticateUser
}

surface LoginScreen {
  display email_field: string
  display password_field: string
  interaction submit "Log in with credentials"
  event onSubmit { email: string, password: string }
}
```

---

## Clone (Template Expansion)

Reuse symbol definitions with overrides. **Single-level only** - no inheritance chains.

### Syntax

```armature
symboltype Name clone BaseSymbol {
  # Only specify what changes
  # Everything else inherited
}
```

### Rules

1. **One level deep maximum** - cannot clone a clone
2. **Base symbol must be complete** - target cannot have `clone` in its definition
3. **Overrides only** - specify only what changes, rest is inherited
4. **Type-safe** - override fields must match base types or be new additions

### Examples

**REST API endpoints:**
```armature
portal GetUser {
  transport: http
  method: GET
  path: "/users/{id}"
  format: json
  handler: FindUser
}

portal GetProduct clone GetUser {
  path: "/products/{id}"
  handler: FindProduct
}
# Inherits: transport, method, format
# Overrides: path, handler
```

**Models with common fields:**
```armature
model BaseEntity {
  field id: uuid [primary]
  field created_at: timestamp [readonly, default: now]
  field updated_at: timestamp
}

model User clone BaseEntity {
  field email: string [unique, format: email]
  field password_hash: bytes [sensitive]
}

model Product clone BaseEntity {
  field name: string [length: 1..200]
  field price: decimal(10,2) [min: 0]
}
# Both inherit id, created_at, updated_at
```

**Surfaces with common properties:**
```armature
surface BaseScreen {
  property layout: vertical
  property spacing: 16
  
  accessibility {
    role: main
  }
  
  interaction back "Navigate back"
  
  state loading: bool = false
}

surface TaskListScreen clone BaseScreen {
  display tasks: list<Task>
  interaction pull_refresh "Refresh task list"
}

surface ProfileScreen clone BaseScreen {
  display user: User
  interaction edit "Edit profile"
}
# Both inherit layout, accessibility, back button, loading state
```

### What Clone Is NOT

```armature
model Base { field id: uuid }

model Child clone Base { field name: string }

model GrandChild clone Child { field age: u8 }
# ERROR: Cannot clone Child - it's already a clone of Base
```

Clone is **template expansion**, not OOP inheritance. Compiler expands at parse time into standalone symbols.

---

## Registry & Implementation Tracking

### Permissive Spec, Strict Implementation

**Spec (intent):**
```armature
operation getUserById {
  input id: uuid
  output: User?
}
```

**Implementation reports back:**
```python
update_status(
  symbol_id=42,
  status="implemented",
  signature="def get_user_by_id(id: str) -> User | None",
  location="src/services/user_service.py::get_user_by_id",
  line=45,
  length=13,
  platform_convention="python_snake_case"
)
```

**Query returns:**
```json
{
  "name": "getUserById",
  "status": "implemented",
  "signature": "def get_user_by_id(id: str) -> User | None",
  "location": "src/services/user_service.py::get_user_by_id",
  "line": 45,
  "length": 13,
  "spec": {
    "input": {"id": "uuid"},
    "output": "User?"
  }
}
```

### Semantic Map
Registry tracks:
- Symbol name → exact file path
- Symbol name → actual signature
- Symbol name → implementation status
- Line number and length (for incremental rebuilds)

**Benefits:**
- Multi-agent coordination (agents know where code lives)
- No duplicate implementations (location conflicts detected)
- Bidirectional traceability (spec ↔ implementation)
- Code archeology (which spec generated this function?)

---

## Build Order

Compiler enforces: **enum → model → event → interface → operation → portal → surface → config**

Rationale: Enums are often referenced by model fields, so they must exist first. Models define data, events signal changes, interfaces define contracts, operations compose behavior, portals/surfaces expose to the outside world, configs are just constants.

Symbols can be defined in any order within .arm files. Compiler sorts dependency graph automatically.

---

## Scientific Notation Support

Lexer supports scientific notation for numerical literals:
```armature
field divergence_threshold: f64 = 1e10
field planck_constant: f64 = 6.626e-34
```

---

## Key Changes from v1

1. **.ac file as entry point** - platform config separated from specs
2. **Flattened syntax** - keyword-prefixed, no nested blocks
3. **Concrete types** - `uuid`, `u32`, `f64`, `timestamp` instead of `identifier`, `integer`, `decimal`
4. **Interfaces not services** - strict contracts with method signatures
5. **Implementation registry** - exact locations and signatures tracked
6. **Platform translation** - canonical spec form, idiomatic implementation
7. **Import-once** - automatic deduplication, no #pragma needed
8. **Optional context** - only specify when it adds value (~250 char soft limit)
9. **Scientific notation** - first-class support for numerical domains
10. **Portal (new)** - system I/O boundaries replace endpoints (HTTP, UART, CAN, MQTT, etc.)
11. **Surface (new)** - UI/UX abstraction replaces components (web, mobile, CLI, LED, etc.)
12. **Clone (new)** - template expansion for DRY, single-level only (no inheritance chains)
13. **Enhanced constraints** - `values`, `references`, `computed_by`, `custom` escape hatch
14. **Enum (new)** - simple named variants for type safety
15. **Union types** - `oneof<>` for type unions and literal unions (no mixing, minimum 2 members)
16. **ref<T> scoped** - relationships only, not for signatures (bare types in operations/methods)
17. **T? sugar** - `User?` equivalent to `optional<User>`
18. **Method shorthand** - both `method name(args): return` and flattened syntax supported
19. **Effect-based mutation** - `effect updates X` instead of `mut` parameter annotations

---

## Design Principles

1. **Focus mechanism** - reduce agent search space from "anything" to "this shape"
2. **Specify what, not how** - structure and contracts, not algorithms
3. **Force precision** - explicit types, concrete semantics
4. **Default strict, opt-in flexible** - `any` is explicit escape hatch
5. **Language-agnostic via translation** - spec is canonical, platforms adapt
6. **Spec = intent, registry = reality** - bidirectional sync
7. **Token-optimized for AI** - designed for LLM consumption first
8. **No ceremony** - only specify what adds value
9. **Compiler does the work** - sort deps, validate contracts, cull unused symbols
10. **DRY via clone** - template expansion without inheritance complexity
11. **Effects over annotations** - `effect updates X` tells the agent what mutates

---

## Example: Complete Project

**project.ac:**
```armature
project "MyApp" {
  description "User authentication service"
  version "0.1.0"
}

platform rust {
  naming: snake_case
  optional: Option<T>
}

alias UserId = uuid

import ./src/auth.arm

build {
  mcp_db: ".armature/myapp.db"
}
```

**src/auth.arm:**
```armature
namespace myapp.auth

model User {
  field id: UserId
  field email: string [unique, format: email]
  field created_at: timestamp [readonly, default: now]
}

interface UserStore {
  method findById(id: UserId): User?
  method insert(user: User): UserId
}

operation GetUser {
  input id: UserId
  output: User?
  requires UserStore
}

operation CreateUser {
  input email: string
  output: User
  error email_taken "Email already registered"
  requires UserStore
  effect creates User
}
```

**Compilation:**
```bash
armature compile project.ac -o .armature/myapp.db
```

**Result:** SQLite database with queryable schema, ready for MCP consumption.

---

*Document updated after design review. Ready for Rust implementation.*