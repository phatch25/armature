# Armature

A declarative schema language for describing software architecture. Armature captures **what** to build (not how) and compiles to a SQLite database that AI agents query during implementation.

## Quick Start

```bash
cargo build --release

# Validate a spec file
./target/release/armature check myspec.arm

# Compile to SQLite
./target/release/armature compile myspec.arm -o spec.db

# List symbols
./target/release/armature ls myspec.arm
./target/release/armature ls myspec.arm --kind model --verbose
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `check <file>` | Validate syntax and semantics |
| `compile <file> -o <output>` | Compile to SQLite database |
| `ls <file>` | List symbols (filter with `--kind`, detail with `--verbose`) |
| `ast <file>` | Show parsed AST (debugging) |
| `tokens <file>` | Show lexer tokens (debugging) |

## File Types

- `.arm` - Specification files containing symbol definitions
- `.ac` - Project configuration files (platform rules, aliases, imports, build config)

## Language Overview

### Symbol Types

| Symbol | Purpose |
|--------|---------|
| `model` | Data structures with fields and relations |
| `enum` | Enumerated types with variants (supports sum types) |
| `interface` | Shared method contracts |
| `operation` | Business logic with inputs, outputs, effects |
| `event` | Domain events with payloads |
| `config` | Configuration schemas |
| `portal` | System I/O boundaries (HTTP, UART, CAN, MQTT) |
| `surface` | UI/UX definitions |

### Type System

**Primitives:**
```
uuid ulid
u8 u16 u32 u64 u128
i8 i16 i32 i64 i128
f32 f64 decimal(p,s)
string bytes char bool
timestamp duration size ptr any void
```

**Modifiers:**
```
optional<T>  T?           # Nullable
list<T>                   # Ordered collection
set<T>                    # Unique collection
map<K,V>                  # Key-value pairs
ref<Symbol>               # Reference to symbol
oneof<A,B,C>              # Union types
oneof<"a","b","c">        # Literal unions
```

### Example

```armature
namespace myapp.users

model User {
    context "Application user account"

    field id: uuid
    field email: string [unique, format: email]
    field name: string?
    field role: ref<Role>
    field created_at: timestamp

    relation posts: has_many<ref<Post>>

    invariant valid_email "Email must be valid format"
}

enum Role {
    context "User permission levels"

    variant Admin
    variant Member
    variant Guest
}

operation CreateUser {
    context "Register a new user account"

    input email: string
    input name: string?
    input role: ref<Role> = Role.Member

    output ref<User>

    effect creates ref<User>

    error EmailTaken "Email address already registered"
    error InvalidEmail "Email format is invalid"

    precondition valid_input "Email must be non-empty"
    postcondition user_exists "User record exists in database"
}
```

### Constraints

```
[unique]              # No duplicates
[format: email]       # Format validation
[pattern: "desc"]     # Pattern description
[min: 0] [max: 100]   # Value range
[length: 1..255]      # Length range
[size: 0..1000]       # Collection size
[immutable]           # Cannot change after creation
[readonly]            # Cannot be set directly
[derived]             # Computed value
[sensitive]           # PII/secrets
[indexed]             # Query optimization hint
[default: value]      # Default value
```

### Relations

```
has_one<ref<Model>>       # 1:1
has_many<ref<Model>>      # 1:N
belongs_to<ref<Model>>    # N:1 (foreign key side)
many_to_many<ref<Model>>  # N:N
```

## Project Files (.ac)

Project configuration files define multi-file projects:

```armature
project "MyProject" {
    description "Project description"
    version "1.0.0"
    context "What this project does"
}

platform rust {
    naming: snake_case
    optional: optional<T>
}

platform typescript {
    naming: camelCase
    optional: oneof<T, null>
}

alias UserId = uuid
alias Email = string [format: email, max: 255]

import "./models.arm"
import "./operations.arm"

build {
    output: "dist/"
    mcp_db: ".armature/spec.db"
}
```

## Design Principles

- **Design before execution** - Spec is frozen before implementation begins
- **Specification is permission** - Names, types, relationships are enforced; implementation details are trusted
- **No stubs** - "Done" means actually implemented, not TODO comments
- **Context is load-bearing** - Context fields provide direction for AI agents
- **Token-optimized** - Language designed for LLM efficiency

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release

# Run with debug output
RUST_BACKTRACE=1 ./target/release/armature check myspec.arm
```

## Documentation

- [Language Syntax](docs/syntax.md) - Full syntax specification

## License

MIT
