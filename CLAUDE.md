# Working with Armature

## What This Is

Compiler for the Armature schema language. Parses `.arm` and `.ac` files, analyzes them, emits to SQLite.

## Structure

```
src/
  lexer/       # Tokenizer (lexer.rs, tokens.rs, keywords.rs)
  parser/      # Recursive descent parser (parser.rs, ast.rs)
  analyzer/    # Semantic analysis, reference checking
  expand/      # Clone expansion
  compiler/    # SQLite emission
  types.rs     # Primitive type handling
  main.rs      # CLI entry point
```

## Key Commands

```bash
cargo test                           # 77 tests
cargo run -- check file.arm          # Validate
cargo run -- compile file.arm -o x.db --allow-warnings
cargo run -- ls file.arm --kind model --verbose
```

## Parser Patterns

- `should_continue_block()` - Standard block loop helper
- `expect_*` functions return errors, advance position
- `parse_*` functions return structured results
- `skip_newlines()` between block members
- Keywords are contextual - same word can be keyword or name depending on position

## Database Output

Normalized tables: `symbol`, `field`, `variant`, `method`, `effect`, `reference`, `named_string`

Full JSON in `symbol.definition` for complete reconstruction.

FTS5 search on `symbol_fts`.

## Test Pattern

```rust
#[test]
fn test_something() {
    let source = r#"
        model Foo {
            field x: string
        }
    "#;
    let result = parse(source, "<test>");
    assert!(result.is_ok());
    // check structure
}
```

## Known Debt

- `get_keyword_as_string` and `keyword_as_name` have overlap (parser.rs ~200 lines)
- `parse_field_def`, `parse_input_def`, `parse_state_def` share pattern (40 lines duplicated)
- `parse_literal` is 119 lines, could extract map/set helpers

## Don't

- Add features without updating docs/syntax.md
- Leave `#[allow(dead_code)]` in committed code
- Use `unwrap()` without `expect()` explaining the invariant
