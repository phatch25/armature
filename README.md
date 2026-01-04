# Armature

A schema language for software architecture that compiles to a queryable dependency graph. Specs capture intent. Implementation context propagates up as work completes.

## The Problem

AI agents are stateless and context-limited. Every session starts from zero. Every file is written in isolation. There's no shared memory of decisions, no coordination, no source of truth.

## The Solution

Armature is a bidirectional contract between specification and implementation:

1. **Spec flows down** - Intent, types, constraints, dependencies are frozen before implementation begins
2. **Context flows up** - As leaf nodes are implemented, actual signatures and file paths become queryable context for parent nodes

```
SPEC (frozen)                    IMPLEMENTATION (accumulates)
     │                                    ▲
     ▼                                    │
┌─────────┐                          ┌─────────┐
│ Parent  │◄─── queries children ────│ Parent  │
└────┬────┘                          └────┬────┘
     │                                    │
┌────┴────┐                          ┌────┴────┐
▼         ▼                          ▼         ▼
Leaf     Leaf    ─── implements ───► impl     impl
                                    context   context
```

## Architecture

```
.arm files ──► Compiler ──► Database (SQLite/Postgres)
                                │
                                ▼
                            CLI Tool
                          armature next     ← "what's ready?"
                          armature done     ← "here's what I built"
                                │
                                ▼
                           MCP Server
                         (agent tool calls)
```

**Compiler** (this repo): Parse → analyze → emit to database target

**CLI**: Query for unblocked work, report implementation details, propagate context

**MCP Server**: Wrap CLI for tool-using agents

## Quick Start

```bash
cargo build --release

# Validate a spec
./target/release/armature check myspec.arm

# Compile to SQLite
./target/release/armature compile myspec.arm -o spec.db

# List symbols
./target/release/armature ls myspec.arm --kind operation --verbose
```

## Language Overview

### Symbol Types

| Symbol | Purpose |
|--------|---------|
| `model` | Data structures with fields, relations, constraints |
| `enum` | Enumerated types, including sum types with payloads |
| `interface` | Method contracts |
| `operation` | Business logic with inputs, outputs, effects, pre/postconditions |
| `event` | Domain events with producers |
| `config` | Configuration schemas with defaults |
| `portal` | I/O boundaries (HTTP, UART, CAN, MQTT) |
| `surface` | UI definitions |

### Example

```armature
namespace myapp.orders

enum OrderStatus {
    variant Pending
    variant Confirmed
    variant Shipped(TrackingInfo)
    variant Delivered
    variant Cancelled(string)
}

model Order {
    context "Customer order with line items"

    field id: uuid [primary]
    field customer: ref<Customer>
    field status: OrderStatus = OrderStatus.Pending
    field items: list<LineItem>
    field total: decimal(10,2) [derived]
    field created_at: timestamp [default: now]

    relation customer: belongs_to<ref<Customer>>

    invariant positive_total "Total must be >= 0"
}

operation PlaceOrder {
    context "Create order from cart contents"

    input customer_id: uuid
    input items: list<LineItem>

    output: Order

    effect creates Order
    effect emits OrderPlaced

    precondition valid_items "At least one item required"
    precondition customer_exists "Customer must exist"
    postcondition order_persisted "Order saved to database"
}

event OrderPlaced {
    context "Emitted when order is successfully created"

    field order_id: uuid
    field customer_id: uuid
    field total: decimal(10,2)

    producer PlaceOrder
}

interface OrderService {
    method place(customer_id: uuid, items: list<LineItem>): Order
    method cancel {
        input order_id: uuid
        output: Order
        error not_found "Order does not exist"
        error already_shipped "Cannot cancel shipped order"
    }
}
```

### Type System

**Primitives:**
```
uuid ulid string bytes char bool
u8 u16 u32 u64 u128
i8 i16 i32 i64 i128
f32 f64 decimal(p,s)
timestamp duration size any void
```

**Modifiers:**
```
optional<T>  T?           # Nullable
list<T>                   # Ordered collection
set<T>                    # Unique collection
map<K,V>                  # Key-value pairs
ref<Symbol>               # Reference to another symbol
oneof<A,B,C>              # Union types
```

### Constraints

```
[primary]             # Primary key
[unique]              # No duplicates
[format: email]       # Format validation
[min: 0] [max: 100]   # Value range
[length: 1..255]      # String length
[immutable]           # Set once
[derived]             # Computed
[sensitive]           # PII/secrets
[indexed]             # Query hint
[default: now]        # Default value
```

## Database Schema

The compiled database contains:

- `symbol` - All definitions with full JSON and status tracking
- `field`, `variant`, `method` - Normalized symbol members
- `effect`, `reference` - Dependency graph edges
- `implementation_info` - Actual file paths and signatures (filled during implementation)
- `symbol_fts` - Full-text search index

Key queries:
- Find leaf nodes (no unimplemented dependencies)
- Get full context for a symbol (spec + all child implementations)
- Track progress through the graph

## Project Files (.ac)

Multi-file projects use `.ac` configuration:

```armature
project "MyApp" {
    version "1.0.0"
}

platform rust {
    naming: snake_case
}

platform typescript {
    naming: camelCase
}

alias UserId = uuid
alias Email = string [format: email]

import "./models.arm"
import "./operations.arm"
```

## Development

```bash
cargo test
cargo build --release
```

## Documentation

- [Language Syntax](docs/syntax.md) - Full grammar specification

## License

MIT
