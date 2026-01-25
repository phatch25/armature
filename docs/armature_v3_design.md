# Armature 2.0 - Complete Design Specification

**Version:** 3.0-alpha  
**Date:** 2026-01-04  
**Status:** Design Phase - Pre-Implementation

---

## Summary

Armature is a specification language designed to solve coordination problems in AI-assisted software development. It compiles to a queryable SQLite dependency graph that enables AI agents to work in parallel on unblocked tasks while maintaining consistency.

**Core Innovation:** Armature treats specifications as **compiled prompts** - structured constraints that are both human-expressive and machine-deterministic, preventing AI agents from autopiloting to incorrect patterns while enabling sophisticated coordination.

---

## The Problem

Current AI-assisted development suffers from fundamental coordination failures:

1. **Hidden Inconsistencies**: AI agents produce code with hidden stubs, naming conflicts, and incomplete implementations
2. **Context Loss**: Design intent is lost between sessions and across multiple agents
3. **Conflated Concerns**: Design specifications mix with implementation details (naming conventions, language idioms)
4. **No Dependency Tracking**: Agents don't know what's buildable vs what's blocked
5. **Autopilot Errors**: Overloaded terms (model, operation) trigger pattern-matching to wrong paradigms

### What Doesn't Work

**Option A: Natural Language Specs**
- Ambiguous, interpretation varies
- No queryable structure
- Agents can't reliably extract dependencies

**Option B: Traditional Programming DSLs**
- Too coupled to specific languages/platforms
- Edge cases explode
- Becomes a meta-programming language

**Option C: Central Config Files**
- Doesn't scale to real applications
- Mixed concerns (what vs where vs how)
- Filesystem structure has no semantic meaning

### The Armature Solution

A specification language that is:
- **Platform-agnostic** at the spec level (pure intent)
- **Context-aware** at compilation (hierarchical .ac files)
- **Structurally queryable** (SQLite dependency graph)
- **Semantically unique** (vocabulary prevents autopilot)
- **Permissively extensible** (evolves organically)

---

## Architectural Foundations

### The Three-Layer Model

```
┌─────────────────────────────────────┐
│  .arm files (Logical Specs)         │  Platform-agnostic intent
│  - Runes, rituals, contours         │  Braced identifiers: {create user}
│  - Pure business logic              │
└─────────────────────────────────────┘
            ↓ compiled with
┌─────────────────────────────────────┐
│  .ac files (Hierarchical Context)   │  Transformation rules
│  - language, platform, conventions  │  Inherited down directory tree
│  - Identifier mapping rules         │
└─────────────────────────────────────┘
            ↓ produces
┌─────────────────────────────────────┐
│  SQLite IR (Queryable Graph)        │  AI coordination layer
│  - Symbols, dependencies, paths     │  "What can I build next?"
│  - Logical → Physical mappings      │
└─────────────────────────────────────┘
```

### From Central Manifest to Distributed Templates

**Old Model (v1):**
```
project.arm          # All specs in one place
project.ac           # Maps specs → file locations
generated/           # Output directory
```
**Problems:** Doesn't scale, mixed concerns, filesystem is meaningless

**New Model (v2):**
```
project/
  .ac                         # Root context
  backend/
    .ac                       # language: rust
    models/
      user.arm                # Spec lives where impl lives
    handlers/
      create_user.arm
  frontend/
    .ac                       # language: typescript
    components/
      dashboard.arm
```
**Benefits:** Filesystem IS scaffold, context inherits, specs co-located

---

## Vocabulary: The Pattern Interrupt

Overloaded terms cause AI autopilot. Unique terms force understanding.

| Old Term | New Term | Rationale |
|----------|----------|-----------|
| model | **rune** | Symbolic structure, mystical aesthetic, zero overload |
| operation | **ritual** | Prescribed action with intent and effects |
| interface | **contour** | External boundary/surface of interaction |
| enum | **enum** | Universal, kept as-is (anchor point) |
| event | *(removed)* | Now first-class clauses (`emits`, `on`) |
| config | **const rune** | Compile-time constant data |

**Additional Symbols:**
- **portal** - I/O boundaries (HTTP, MQTT, WebSocket, etc.)
- **surface** - UI definitions (components, screens, CLI)

### The Semantic Model

> "Rituals manipulate runes and emit runes through portals displayed on surfaces."

This sentence:
1. Cannot be pattern-matched to existing frameworks
2. Forces understanding of Armature's concepts
3. Creates coherent mental model of system flow

**Example:**
```armature
rune user {
  field email: string
  field created_at: timestamp
}

ritual create_user {
  given email: string
  returns user
  creates user
  emits user_created
}

portal user_api {
  direction: input
  transport: http
  on request {
    invoke create_user
  }
}

surface dashboard {
  display users: list<user>
  on user_created {
    refresh users
  }
}
```

---

## Unified Type System

### Everything Is A Rune

There is no distinction between "primitives" and "types" - everything is a rune.

**System Runes** (built-in):
```
string, bytes, char
bool
u8, u16, u32, u64, u128
i8, i16, i32, i64, i128
f32, f64
decimal(precision, scale)
uuid, ulid
timestamp, duration
```

**User Runes** (defined in specs):
```armature
rune user {
  field id: uuid
  field email: string
}

rune address {
  field street: string
  field city: string
}
```

**Collections** (generic over runes):
```
list<rune>
set<rune>
map<rune, rune>
optional<rune>
```

### No Anonymous Types

Every data shape must be a named rune:

❌ **Forbidden:**
```armature
ritual create_user {
  emits user_created with { user_id: uuid, timestamp: timestamp }
}
```

✅ **Required:**
```armature
rune user_created_event {
  field user_id: uuid
  field timestamp: timestamp
}

ritual create_user {
  emits user_created_event
}
```

**Why:** Single source of truth, explicit dependencies, queryable, prevents hallucination.

---

## Path Navigation: The `::` Operator

Single operator for all hierarchical access:

**Namespace Qualification:**
```
backend::auth::user
frontend::components::dashboard
```

**Member Access:**
```
user::email
user::profile::bio
```

**Cross-File References:**
```
auth::models::user::created_at
```

**In Clauses:**
```armature
ritual validate_order {
  requires order::total == sum(order::items::price)
  requires order::shipping::country in allowed_countries
}
```

### Namespace Sources

Namespaces are derived from:
1. **Directory structure** (implicit)
2. **Explicit declarations** in .arm files
3. **Import statements**

Example:
```
backend/
  auth/
    user.arm
    
# File: backend/auth/user.arm
namespace backend::auth

rune user {
  field email: string
}

# Referenced elsewhere as: backend::auth::user
# Member access: backend::auth::user::email
```

---

## Logical Identifiers: Breaking Language Bias

Specs use **braced lowercase space-separated words**:

```armature
ritual {load config} {
  given {file path}: string
  returns {app config}
}

rune {user profile} {
  field {email address}: string
  field {created at}: timestamp
}
```

### Why Braces?

No programming language uses `{lowercase space separated}` syntax, so:
- Cannot be confused with implementation code
- Forces platform-agnostic thinking
- Reads like natural language intent

### Transformation Rules

The compiler transforms logical → physical via .ac context:

```
# .ac context
identifier_convention: snake_case

# Transformation
{load config} → load_config
{email address} → email_address
```

```
# .ac context  
identifier_convention: camelCase

# Transformation
{load config} → loadConfig
{email address} → emailAddress
```

**Deterministic template expansion at compile time.**

---

## Hierarchical Context: The .ac File System

### Context Inheritance

.ac files provide transformation rules that cascade down the directory tree:

```
project/
  .ac                    # Root
    project: "MyApp"
    version: "1.0.0"
  
  backend/
    .ac                  # Inherits root + adds
      language: rust
      identifier_convention: snake_case
      platform: server
    
    api/
      handlers/
        user.arm         # Inherits: MyApp, 1.0.0, rust, snake_case, server
    
    models/
      user.arm           # Inherits: MyApp, 1.0.0, rust, snake_case, server
      
  frontend/
    .ac                  # Inherits root + adds different context
      language: typescript
      identifier_convention: camelCase
      platform: web
      
    components/
      .ac                # Inherits all ancestors + adds
        framework: react
      
      dashboard/
        user.arm         # Inherits: MyApp, 1.0.0, typescript, camelCase, web, react
```

### Context Accumulation

Each .arm file sees the **merged context of all ancestors**:

```
Path: frontend/components/dashboard/user.arm

Accumulated context:
  project: "MyApp"             # from root/.ac
  version: "1.0.0"             # from root/.ac
  language: "typescript"       # from frontend/.ac
  identifier_convention: camelCase  # from frontend/.ac
  platform: "web"              # from frontend/.ac
  framework: "react"           # from frontend/components/.ac
```

### Platform-Specific Mappings

The same logical spec appears in multiple contexts:

```armature
# backend/models/user.arm (rust context)
rune user {
  field email: string
}
# Generates: struct User { email: String }

# frontend/types/user.arm (typescript context)  
rune user {
  field email: string
}
# Generates: interface User { email: string }
```

Same logical model, different physical implementations.

---

## Symbol Kinds: The Core Vocabulary

### 1. Rune / Const Rune

**Purpose:** Data structures

```armature
rune user {
  context "Primary user entity"
  
  field id: uuid [primary]
  field email: string [unique, format: email]
  field created_at: timestamp [immutable]
}

const rune app_config {
  context "Application configuration constants"
  
  field port: i32 = 8080
  field max_connections: i32 = 100
  field database_url: string = "postgres://localhost"
}
```

**Clauses:**
- `context` - documentation (0-1)
- `field` - data members (1+)
- `relation` - has_one, has_many, belongs_to, many_to_many (0+)
- `invariant` - validation rules (0+)

**Difference: const rune**
- All fields MUST have default values
- Values fixed at compile/deploy time

### 2. Enum

**Purpose:** Simple enumerations

```armature
enum status {
  context "User account status"
  
  variant pending
  variant active
  variant suspended
  variant deleted
}

enum http_method {
  variant GET
  variant POST
  variant PUT
  variant DELETE
}
```

**Clauses:**
- `context` - documentation (0-1)
- `variant` - enumeration values (1+)

**Note:** No associated data (no sum types). Keep it simple.

### 3. Ritual / Const Ritual

**Purpose:** Business logic transformations

```armature
ritual create_user {
  context "Primary user registration flow"
  
  given email: string
  given password: string
  
  validate "check email format, disposable domains, MX records"
  validate "password strength: min 12 chars, mixed case, numbers"
  
  returns user
  creates user
  emits user_created
  
  on duplicate_email {
    emit validation_error
    respond "suggest password reset if account exists"
  }
}

const ritual get_user {
  context "Retrieve user by ID - read-only query"
  
  given user_id: uuid
  returns user
  
  # No effects allowed in const ritual
}
```

**Clauses:**
- `context` - documentation (0-1)
- `given` - inputs (0+)
- `returns` - output (0-1)
- `creates` / `updates` / `deletes` - rune effects (0+)
- `emits` - event emissions (0+)
- `calls` - invoke other rituals/contours (0+)
- `requires` - preconditions (0+)
- `validate` - validation guidance (0+)
- `on` - event handlers (0+)

**Difference: const ritual**
- CANNOT have: creates, updates, deletes, emits
- Pure query/computation only

### 4. Contour

**Purpose:** Interface boundaries (collections of ritual signatures)

```armature
contour user_repository {
  context "User data access interface"
  
  ritual find_by_id(id: uuid): user?
  ritual find_by_email(email: string): user?
  ritual save(user: user): void
  ritual delete(id: uuid): void
}

contour payment_processor {
  ritual charge(amount: decimal, source: payment_method): transaction
  ritual refund(transaction_id: uuid): refund_result
}
```

**Clauses:**
- `context` - documentation (0-1)
- `ritual` - method declarations (1+)

**Two forms:**
```armature
# Inline form (simple signatures)
ritual find_by_id(id: uuid): user?

# Block form (complex signatures with errors)
ritual process_payment {
  given order_id: uuid
  given payment_method: payment_method
  returns transaction
  emits payment_failed
  emits payment_successful
}
```

### 5. Portal

**Purpose:** I/O boundaries (HTTP, WebSocket, MQTT, gRPC, CLI, etc.)

```armature
portal user_api {
  context "REST API for user management"
  
  direction: input
  transport: http
  method: POST
  path: "/users"
  format: json
  
  on request {
    invoke create_user
    emit user_created
  }
  
  on error {
    respond "return 400 with error details"
  }
}

portal user_events {
  context "Real-time user event stream"
  
  direction: output
  transport: websocket
  topic: "users"
  
  on user_created {
    broadcast: true
  }
}

portal cleanup_job {
  context "Scheduled maintenance task"
  
  direction: internal
  transport: cron
  schedule: "0 0 * * *"
  
  on timer {
    invoke cleanup_expired_sessions
  }
}
```

**Required Properties:**
- `direction`: input | output | bidirectional
- `transport`: http | websocket | mqtt | grpc | uart | spi | i2c | cli | cron | mcp

**Optional Properties** (transport-specific):
- `method`, `path`, `format` (HTTP)
- `topic`, `qos` (MQTT)
- `baud`, `parity` (UART)
- `schedule` (cron)
- `tool_name` (MCP)

### 6. Surface

**Purpose:** UI definitions (web, mobile, CLI, embedded displays)

```armature
surface user_dashboard {
  context "Main user management interface"
  
  display users: list<user>
  display selected_user: user?
  
  state filter: string = ""
  state page: i32 = 1
  
  interaction search "filter users by email or name"
  interaction select_user "highlight selected row"
  
  on load {
    invoke fetch_users
  }
  
  on search_change {
    update filter
    invoke fetch_users
  }
  
  on user_created {
    refresh users
    display "show success notification, clear form"
  }
}

surface registration_form {
  state form_data: registration_input
  
  on submit {
    validate "all required fields, terms accepted"
    invoke create_user
  }
  
  on validation_error {
    display "inline errors next to invalid fields, keep valid data"
  }
}
```

**Clauses:**
- `context` - documentation (0-1)
- `parent` - hierarchy (0-1)
- `display` - data binding (0+)
- `state` - local state (0+)
- `interaction` - user actions (0+)
- `on` - event handlers (0+)

---

## Universal Clauses: Consistency Across Symbols

### The `context` Clause

**Appears:** 0-1 times in any symbol
**Position:** Anywhere in symbol body (recommended first)
**Purpose:** Human-readable documentation

```armature
rune user {
  context "Primary user entity with authentication"
  # ...
}

ritual create_user {
  context "Registration flow with email verification"
  # ...
}
```

### The `on` Clause

**Appears:** 0+ times in any symbol
**Purpose:** Event handling (same syntax everywhere)

```armature
ritual create_user {
  on duplicate_email {
    emit validation_error
  }
}

surface dashboard {
  on user_created {
    refresh users
  }
}

portal api {
  on request {
    invoke handler
  }
}
```

**Action verbs** (extensible):
- `emit` - send event rune
- `invoke` - call ritual
- `summon` - call portal
- `update` - modify state/data
- `refresh` - reload/re-query
- `redirect` - navigate
- `respond` - reply to caller
- `validate` - check conditions
- `display` - show to user
- `broadcast` - send to all
- Any identifier (custom verbs)

### Field Definition Pattern

**Syntax:** `identifier: type_expr constraints? default_value?`

**Appears in:**
- `field` (runes)
- `given` (rituals)
- `display` / `state` (surfaces)

```armature
field email: string [unique, format: email]
given user_id: uuid
state counter: i32 = 0
display users: list<user>
```

### Reference Pattern

**Syntax:** `keyword identifier`

**Appears in:**
- `creates` / `updates` / `deletes` (rituals)
- `emits` (rituals, portals)
- `calls` / `requires` (rituals)
- `parent` (surfaces)

```armature
creates user
emits user_created
calls send_email
requires authenticated
parent dashboard_layout
```

---

## Events: First-Class Data Flow

### Events Are Runes

Events are not a separate symbol kind - they're runes that flow through the system:

```armature
rune user_created {
  field user_id: uuid
  field email: string
  field timestamp: timestamp
}
```

### Event Emission & Handling

**Emit from rituals:**
```armature
ritual create_user {
  given email: string
  returns user
  creates user
  emits user_created    # References the rune
}
```

**Handle in rituals:**
```armature
ritual send_welcome_email {
  on user_created {
    invoke email_service
  }
}
```

**Handle in surfaces:**
```armature
surface user_list {
  display users: list<user>
  
  on user_created {
    refresh users
    display "show success notification"
  }
}
```

**Handle in portals:**
```armature
portal user_events {
  direction: output
  transport: websocket
  
  on user_created {
    broadcast: true
  }
}
```

### Event Flow Example

```
ritual create_user
  ↓ emits user_created
  ├→ ritual send_welcome_email (on user_created)
  ├→ ritual update_analytics (on user_created)
  ├→ surface user_list (on user_created)
  └→ portal user_events (on user_created)
```

---

## Concurrency & Synchronization

### Async Rituals

**Modifier:** `async` on ritual

```armature
async ritual fetch_user_data {
  given user_id: uuid
  returns user_data
  calls database::query
}
```

Implementation depends on .ac context:
- Rust → `async fn`
- TypeScript → `async function`
- Python → `async def`
- Go → goroutine
- Java → CompletableFuture

### Parallel Execution

**Block:** `parallel { }`

```armature
async ritual load_dashboard {
  given user_id: uuid
  
  parallel {
    await fetch_user
    await fetch_posts
    await fetch_notifications
  }
  
  returns dashboard_data
}
```

### Sequential Execution

**Block:** `sequential { }` (or implicit default)

```armature
ritual process_payment {
  sequential {
    invoke validate_card
    invoke charge_card
    invoke update_order
  }
}
```

### Synchronization Primitives

**System runes:**
- `mutex` - mutual exclusion
- `semaphore` - counting semaphore
- `rwlock` - read-write lock
- `channel` - message passing

**Verbs:**
- `acquire` - take lock
- `release` - release lock
- `await` - wait for async result

```armature
rune shared_counter {
  field value: i32
  field lock: mutex
}

ritual increment {
  given counter: shared_counter
  
  acquire counter::lock
  updates counter::value
  release counter::lock
}

# Or use atomic constraint
ritual increment [atomic] {
  given counter: shared_counter
  updates counter::value
}
```

---

## Error Handling: Forced Modularity

### Philosophy

Each ritual is atomic - it either succeeds or emits specific errors. No hidden exceptions or try/catch blocks.

### Declaring Errors

Rituals declare what errors they can emit:

```armature
ritual validate_payment {
  given order: order
  given payment: payment_method
  
  returns validation_result
  
  emits insufficient_funds
  emits invalid_card
  emits network_timeout
}
```

### Handling Errors

Callers handle errors explicitly via `on`:

```armature
ritual process_order {
  given order: order
  
  calls validate_payment
  
  on insufficient_funds {
    emit payment_failed
    respond "notify user of insufficient funds"
  }
  
  on invalid_card {
    emit payment_failed
    respond "request new payment method"
  }
  
  on network_timeout {
    retry "with exponential backoff, max 3 attempts"
  }
}
```

### Benefits

- **Single Responsibility:** Each ritual has one job
- **Explicit Paths:** All error conditions are visible
- **Composable:** Errors propagate up through callers
- **Testable:** Small, focused units
- **Platform-Agnostic:** Maps to exceptions, Result types, or error returns

---

## Inline Guidance: Contextual Prompts

### Structured + Prose

Armature combines structured dependency tracking with natural language guidance:

```armature
ritual create_user {
  context "Primary registration flow with email verification"
  
  given email: string
  given password: string
  
  validate "email: RFC 5322 format, check MX records, block disposable domains"
  validate "password: min 12 chars, uppercase, lowercase, number, special char"
  
  creates user
  emits user_created
  
  on duplicate_email {
    emit validation_error
    respond "suggest password reset, include forgot password link"
  }
  
  on success {
    trigger "send verification email within 30 seconds, log signup source"
  }
}
```

### Why Both?

**Structured verbs** (emit, invoke, update):
- Queryable dependencies
- Build dependency graph
- Enable parallelization

**Prose strings**:
- Implementation guidance
- Business rules
- AI context for specifics

The AI agent gets:
1. **What to call:** `emit validation_error`
2. **How to implement:** `"suggest password reset, include forgot password link"`

---

## Permissive Extensibility

### Design Principle

> The compiler accepts any syntactically valid combination of modifiers and clauses. Undefined combinations generate warnings in normal mode, errors in strict mode.

### Examples of Undefined Behavior

```armature
const portal api_schema {
  # Maybe: static route definitions?
  # Maybe: immutable API contract?
  # Undefined but valid - tooling decides
}

rune user {
  emits user_changed
  # Maybe: database triggers?
  # Maybe: reactive updates?
  # Undefined but valid - implementation interprets
}

const surface header {
  # Maybe: template that never re-renders?
  # Maybe: static component?
  # Undefined but valid - framework decides
}
```

### Compiler Feedback

**Normal Mode:**
```
$ armature compile project.ac

Warning: const modifier on portal 'api_schema' has undefined behavior
  → const portal api_schema { ... }
  Location: backend/api/schema.arm:1

Warning: 'emits' clause in rune 'user' has undefined semantics
  → emits user_changed
  Location: backend/models/user.arm:5

Compiled successfully with 2 warnings.
Symbols: 47 | References: 132 | Warnings: 2
```

**Strict Mode:**
```
$ armature compile --strict project.ac

Error: const modifier on portal 'api_schema' is not defined
  → const portal api_schema { ... }
  Location: backend/api/schema.arm:1
  
Strict mode: undefined constructs not allowed.
Compilation failed.
```

### Benefits

- **Organic Evolution:** New patterns can emerge from use
- **Platform Flexibility:** Different targets can interpret differently
- **No Artificial Limits:** Don't constrain future discoveries
- **Guardrails Available:** Strict mode enforces rigor when needed

---

## Multi-Pass Compilation

### Pass 1: Discovery & Symbol Table

**Goal:** Build a map of what exists

1. Walk directory tree recursively
2. Discover all .ac files, build context inheritance chains
3. Parse all .arm files (shallow - just symbol definitions)
4. Extract: symbol kind, name, location, namespace
5. Build symbol table: `namespace::name → { kind, file, context }`

**Output:** Complete catalog of all symbols with their paths

### Pass 2: Resolution & Validation

**Goal:** Resolve all references and validate semantics

1. Parse full symbol bodies (deep - all clauses)
2. Resolve all rune/ritual/contour references
3. Validate types (all referenced runes exist)
4. Build dependency graph edges (A calls B, C emits D)
5. Detect invalid circular dependencies
6. Check for undefined modifier/clause combinations (generate warnings)

**Output:** Validated dependency graph with all references resolved

### Pass 3: Transformation & Emission

**Goal:** Generate queryable SQLite IR

1. Transform logical identifiers → physical (via .ac context rules)
2. Generate SQLite schema tables:
   - `symbols` - all defined symbols
   - `references` - dependency edges
   - `contexts` - accumulated .ac contexts per symbol
   - `paths` - namespace hierarchies
3. Insert all data into SQLite database

**Output:** SQLite file ready for AI agent queries

### Handling Circular References

**Valid circles** (runes referencing each other):
```armature
rune user {
  field posts: list<post>
}

rune post {
  field author: user
}
```
✓ Allowed - the spec describes relationships, not implementation order

**Invalid circles** (rituals requiring each other):
```armature
ritual A {
  requires B
}

ritual B {
  requires A
}
```
✗ Error - logical impossibility, cannot satisfy both

---

## The SQLite Intermediate Representation

### Schema Tables

**symbols:**
```sql
CREATE TABLE symbols (
  id INTEGER PRIMARY KEY,
  namespace TEXT,
  name TEXT,
  logical_name TEXT,        -- {create user}
  physical_name TEXT,       -- create_user, createUser, etc.
  kind TEXT,                -- rune, ritual, contour, portal, surface, enum
  modifiers TEXT,           -- const, async (JSON array)
  file_path TEXT,
  line_number INTEGER,
  context_doc TEXT
);
```

**references:**
```sql
CREATE TABLE references (
  from_symbol_id INTEGER,
  to_symbol_id INTEGER,
  reference_type TEXT,      -- creates, emits, calls, returns, field_type, etc.
  clause TEXT,              -- which clause made this reference
  FOREIGN KEY (from_symbol_id) REFERENCES symbols(id),
  FOREIGN KEY (to_symbol_id) REFERENCES symbols(id)
);
```

**contexts:**
```sql
CREATE TABLE contexts (
  symbol_id INTEGER,
  key TEXT,                 -- language, platform, identifier_convention, etc.
  value TEXT,
  source_file TEXT,         -- which .ac file provided this
  FOREIGN KEY (symbol_id) REFERENCES symbols(id)
);
```

**clauses:**
```sql
CREATE TABLE clauses (
  symbol_id INTEGER,
  clause_type TEXT,         -- field, given, emits, on, etc.
  clause_data TEXT,         -- JSON representation of clause
  FOREIGN KEY (symbol_id) REFERENCES symbols(id)
);
```

### Query Examples

**"What can I build next?"**
```sql
SELECT s.name, s.kind
FROM symbols s
WHERE NOT EXISTS (
  SELECT 1 FROM references r
  JOIN symbols dep ON r.to_symbol_id = dep.id
  WHERE r.from_symbol_id = s.id
    AND dep.implementation_status = 'pending'
);
```

**"What does this ritual depend on?"**
```sql
SELECT dep.name, dep.kind, r.reference_type
FROM references r
JOIN symbols dep ON r.to_symbol_id = dep.id
WHERE r.from_symbol_id = (
  SELECT id FROM symbols WHERE name = 'create_user'
);
```

**"Show me all rituals that emit this event:"**
```sql
SELECT s.name
FROM symbols s
JOIN references r ON s.id = r.from_symbol_id
WHERE r.to_symbol_id = (
  SELECT id FROM symbols WHERE name = 'user_created'
)
AND r.reference_type = 'emits';
```

---

## Core Design Principles

### 1. Runes + Rituals = The Engine

Everything else is composition:
- **Events** = runes flowing through `emits`/`on`
- **Validation** = rituals returning success/failure
- **Pipelines** = rituals calling rituals
- **State machines** = enums + rituals
- **Async** = modifier on rituals
- **Locks** = system runes + acquire/release

Other symbols are interfaces to the world:
- **Contour** = how rituals are grouped
- **Portal** = how runes/rituals connect to I/O
- **Surface** = how runes/rituals present to users

### 2. Trust the Primitives

Don't add keywords for every edge case. The core is:
- **Runes hold data**
- **Rituals transform data**

Patterns emerge from these primitives without special syntax.

### 3. Platform-Agnostic Specs

Specs describe **what**, not **how**:
- ✓ "This ritual is async"
- ✗ "This ritual uses tokio runtime with 10 worker threads"

Implementation details belong in .ac context or are inferred by AI agents.

### 4. Forced Modularity

Each ritual is atomic:
- Either succeeds or emits specific errors
- No hidden exceptions
- Callers handle errors explicitly
- Composable, testable, clear

### 5. Queryable Dependency Graph

Every reference is explicit:
- AI agents query "what's buildable?"
- Parallel work on unblocked tasks
- No hidden dependencies
- No stub hell

### 6. Permissive Extension

Accept undefined constructs with warnings:
- Language evolves organically
- Platforms interpret differently
- Strict mode available for rigor
- No artificial limits

### 7. Separation of Concerns

Three distinct layers:
- **.arm files** = logical intent (platform-agnostic)
- **.ac files** = context & transformation (platform-specific)
- **SQLite IR** = queryable coordination (AI-friendly)

Each layer has a clear job. No mixing.

---

## Implementation Roadmap

### Phase 1: Foundation (Current)
- [x] Design vocabulary and symbol kinds
- [x] Design hierarchical .ac context system
- [x] Design logical identifier system
- [x] Design multi-pass compilation strategy
- [ ] Define Armature values (validation framework)
- [ ] Validate all design decisions against values

### Phase 2: Lexer & Parser
- [ ] Implement lexer for braced identifiers
- [ ] Implement whitespace-aware tokenizer
- [ ] Build recursive descent parser
- [ ] Validate against syntax spec
- [ ] Error reporting with file/line/column

### Phase 3: Context Resolution
- [ ] .ac file discovery and parsing
- [ ] Context inheritance accumulation
- [ ] Identifier transformation engine
- [ ] Platform-specific type mappings

### Phase 4: Compilation Pipeline
- [ ] Pass 1: Symbol discovery
- [ ] Pass 2: Reference resolution
- [ ] Circular dependency detection
- [ ] Undefined construct warnings
- [ ] Strict mode validation

### Phase 5: SQLite Emission
- [ ] Schema design and creation
- [ ] Symbol insertion
- [ ] Reference graph construction
- [ ] Context metadata storage
- [ ] Query optimization

### Phase 6: Tooling & CLI
- [ ] `armature compile` command
- [ ] `armature query` for graph inspection
- [ ] `armature validate` for strict checking
- [ ] `armature prompt` for "next task"
- [ ] VSCode extension (syntax highlighting)

### Phase 7: Dogfooding
- [ ] Rewrite Armature compiler in Armature
- [ ] Use SQLite IR to coordinate AI agents
- [ ] Build MCP server around Armature
- [ ] Bootstrap: AI agents work on Armature itself

---

## Open Questions & Future Work

### Language Features
1. Should `sequential { }` be explicit or just default behavior?
2. Do we need explicit lifetime/scope annotations?
3. Should relations (has_one, belongs_to) be expanded or simplified?
4. Transaction boundaries - implicit or explicit?

### Tooling
1. How should AI agents report implementation status back to IR?
2. Should there be a watch mode for incremental compilation?
3. What's the format for AI agent prompts generated from IR?
4. How do agents mark symbols as "in progress" vs "complete"?

### Platform Targets
1. What's the minimal .ac context for each platform?
2. How do we handle platform-specific features (iOS widgets, Android services)?
3. Should there be blessed .ac templates for common stacks?
4. How do embedded targets with resource constraints work?

### Ecosystem
1. Package management for shared runes/rituals?
2. Testing framework - how to spec tests in Armature?
3. Documentation generation from context clauses?
4. Migration tooling from v1 to v2?

---

## Appendix: Complete Grammar Reference

### Top-Level Constructs

```
file := namespace_decl? import_decl* symbol_def*

namespace_decl := 'namespace' path

import_decl := 'import' STRING

symbol_def := modifier* symbol_kind identifier clone_clause? '{' symbol_body '}'

modifier := 'const' | 'async'

symbol_kind := 'rune' | 'enum' | 'ritual' | 'contour' | 'portal' | 'surface'

clone_clause := 'clone' identifier
```

### Identifiers & Paths

```
identifier := '{' word+ '}'        # Logical: {create user}
            | bare_identifier      # References: user, string

word := [a-z]+

path := identifier ('::' identifier)*
```

### Type Expressions

```
type_expr := identifier
           | 'list' '<' type_expr '>'
           | 'set' '<' type_expr '>'
           | 'map' '<' type_expr ',' type_expr '>'
           | 'optional' '<' type_expr '>'
           | type_expr '?'              # Shorthand for optional
```

### Rune Clauses

```
rune_body := clause*

clause := context_clause
        | field_clause
        | relation_clause
        | invariant_clause
        | on_clause

field_clause := 'field' identifier ':' type_expr constraints? default_value?

relation_clause := 'relation' identifier ':' relation_kind '<' 'ref' '<' identifier '>' '>'

relation_kind := 'has_one' | 'has_many' | 'belongs_to' | 'many_to_many'

invariant_clause := 'invariant' identifier STRING
```

### Enum Clauses

```
enum_body := clause*

clause := context_clause
        | variant_clause

variant_clause := 'variant' identifier
```

### Ritual Clauses

```
ritual_body := clause*

clause := context_clause
        | given_clause
        | returns_clause
        | creates_clause
        | updates_clause
        | deletes_clause
        | emits_clause
        | calls_clause
        | requires_clause
        | validate_clause
        | on_clause
        | parallel_clause
        | sequential_clause

given_clause := 'given' identifier ':' type_expr default_value?

returns_clause := 'returns' ':' type_expr
                | 'returns' identifier

creates_clause := 'creates' identifier
updates_clause := 'updates' path
deletes_clause := 'deletes' identifier
emits_clause := 'emits' identifier
calls_clause := 'calls' identifier
requires_clause := 'requires' (identifier | STRING)
validate_clause := 'validate' STRING

parallel_clause := 'parallel' '{' action* '}'
sequential_clause := 'sequential' '{' action* '}'
```

### On Clause & Actions

```
on_clause := 'on' identifier '{' action* '}'

action := verb (identifier | STRING)
        | acquire_action
        | release_action
        | await_action

verb := 'emit' | 'invoke' | 'summon' | 'update' | 'refresh'
      | 'redirect' | 'trigger' | 'respond' | 'display'
      | 'validate' | 'broadcast' | identifier

acquire_action := 'acquire' path
release_action := 'release' path
await_action := 'await' identifier
```

### Contour Clauses

```
contour_body := clause*

clause := context_clause
        | ritual_decl

ritual_decl := 'ritual' identifier '(' param_list? ')' ':' type_expr
             | 'ritual' identifier '{' ritual_body '}'

param_list := param (',' param)*
param := identifier ':' type_expr
```

### Portal Clauses

```
portal_body := clause*

clause := context_clause
        | property_clause
        | on_clause

property_clause := identifier ':' value

value := identifier | STRING | INTEGER | HEX_INT | bool_literal
```

### Surface Clauses

```
surface_body := clause*

clause := context_clause
        | parent_clause
        | display_clause
        | state_clause
        | interaction_clause
        | on_clause

parent_clause := 'parent' identifier
display_clause := 'display' identifier ':' type_expr
state_clause := 'state' identifier ':' type_expr default_value?
interaction_clause := 'interaction' identifier STRING
```

### Common Constructs

```
context_clause := 'context' STRING

constraints := '[' constraint (',' constraint)* ']'
constraint := identifier (':' constraint_value)?
constraint_value := literal | identifier | range | array_literal

default_value := '=' literal

literal := STRING | INTEGER | FLOAT | bool_literal | path
bool_literal := 'true' | 'false'
range := INTEGER '..' INTEGER
array_literal := '[' (STRING (',' STRING)*)? ']'
```

---

## Conclusion

Armature v2 represents a fundamental rethinking of how AI agents coordinate on software development. By separating logical intent from implementation details, maintaining a queryable dependency graph, and using semantically unique vocabulary, it enables parallel AI work while preventing the consistency failures that plague current approaches.

The key innovations:
1. **Vocabulary that forces understanding** (runes, rituals, contours)
2. **Filesystem as scaffold** (specs co-located with implementation)
3. **Hierarchical context** (.ac files cascading down)
4. **Logical identifiers** (platform-agnostic via braces)
5. **Everything is a rune** (unified type system)
6. **Events as first-class** (data flow through the system)
7. **Permissive extensibility** (organic evolution)

The result is a specification language that is both **expressive enough for humans** and **deterministic enough for AI coordination** - the compiled prompt that enables sophisticated multi-agent software development.

---

**Document Version:** 2.0-alpha  
**Last Updated:** 2026-01-04  
**Status:** Ready for values validation
