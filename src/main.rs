use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "armature")]
#[command(about = "Armature Schema Language - Architecture specification for AI agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show tokens from an Armature file (debugging)
    Tokens {
        /// Armature source file (.arm or .ac)
        file: PathBuf,
    },

    /// Show AST from an Armature file (debugging)
    Ast {
        /// Armature source file (.arm or .ac)
        file: PathBuf,
    },

    /// Check an Armature file for errors
    Check {
        /// Armature source file (.arm or .ac)
        file: PathBuf,
    },

    /// Compile an Armature file to SQLite database
    Compile {
        /// Armature source file (.arm or .ac)
        file: PathBuf,

        /// Output database file
        #[arg(short, long, default_value = "spec.db")]
        output: PathBuf,

        /// Allow compilation with warnings (errors still fail)
        #[arg(long)]
        allow_warnings: bool,
    },

    /// List symbols in an Armature file
    Ls {
        /// Armature source file (.arm or .ac)
        file: PathBuf,

        /// Filter by symbol kind (model, enum, interface, operation, event, config, portal, surface)
        #[arg(short, long)]
        kind: Option<String>,

        /// Show detailed info including context
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Tokens { file } => cmd_tokens(file),
        Commands::Ast { file } => cmd_ast(file),
        Commands::Check { file } => cmd_check(file),
        Commands::Compile { file, output, allow_warnings } => cmd_compile(file, output, allow_warnings),
        Commands::Ls { file, kind, verbose } => cmd_ls(file, kind, verbose),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_tokens(file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&file)?;
    let filename = file.to_string_lossy();

    let tokens = armature::lexer::tokenize(&source, &filename)?;

    for token in &tokens {
        println!("{}", token);
    }

    println!("\n{} tokens", tokens.len());
    Ok(())
}

fn cmd_ast(file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&file)?;
    let filename = file.to_string_lossy();

    let parsed = armature::parser::parse_file(&source, &filename)?;

    match parsed {
        armature::parser::ParsedFile::Arm(ast) => {
            // Print namespace
            if let Some(ns) = &ast.namespace {
                println!("namespace: {}", ns.name);
            }

            // Print imports
            for import in &ast.imports {
                println!("import: {}", import.path);
            }

            // Print symbols
            println!("\n{} symbols:", ast.symbols.len());
            for symbol in &ast.symbols {
                print_symbol(symbol, 0);
            }
        }
        armature::parser::ParsedFile::Ac(ac_file) => {
            println!(".ac project file\n");

            // Print project
            if let Some(project) = &ac_file.project {
                println!("project \"{}\" {{", project.name);
                for field in &project.fields {
                    print_project_field(field, 1);
                }
                println!("}}");
            }

            // Print platforms
            for platform in &ac_file.platforms {
                println!("\nplatform {} {{", platform.name);
                for field in &platform.fields {
                    println!("  {}: {:?}", field.name, field.value);
                }
                println!("}}");
            }

            // Print aliases
            for alias in &ac_file.aliases {
                println!("\nalias {} = {:?}", alias.name, alias.target_type);
            }

            // Print imports
            if !ac_file.imports.is_empty() {
                println!("\nimports:");
                for import in &ac_file.imports {
                    println!("  {}", import.path);
                }
            }

            // Print build
            if let Some(build) = &ac_file.build {
                println!("\nbuild {{");
                for field in &build.fields {
                    println!("  {}: \"{}\"", field.name, field.value);
                }
                println!("}}");
            }
        }
    }

    Ok(())
}

fn print_project_field(field: &armature::parser::ProjectField, indent: usize) {
    let pad = "  ".repeat(indent);
    match &field.value {
        armature::parser::ProjectValue::String(s) => {
            println!("{}{} \"{}\"", pad, field.name, s);
        }
        armature::parser::ProjectValue::Block(nested) => {
            println!("{}{} {{", pad, field.name);
            for f in nested {
                print_project_field(f, indent + 1);
            }
            println!("{}}}", pad);
        }
    }
}

fn print_symbol(symbol: &armature::parser::SymbolDef, indent: usize) {
    let pad = "  ".repeat(indent);
    match symbol {
        armature::parser::SymbolDef::Model(m) => {
            println!("{}model {} {{", pad, m.name);
            if let Some(ctx) = &m.context {
                println!("{}  context: {:?}", pad, ctx);
            }
            for field in &m.fields {
                println!("{}  field {}: {:?}", pad, field.name, field.field_type);
            }
            for rel in &m.relations {
                println!("{}  relation {}: {:?}", pad, rel.name, rel.kind);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Enum(e) => {
            println!("{}enum {} {{", pad, e.name);
            for v in &e.variants {
                println!("{}  variant {}", pad, v.name);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Interface(i) => {
            println!("{}interface {} {{", pad, i.name);
            for m in &i.methods {
                let params: Vec<_> = m.params.iter().map(|p| format!("{}: {:?}", p.name, p.param_type)).collect();
                println!("{}  method {}({}) -> {:?}", pad, m.name, params.join(", "), m.return_type);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Operation(o) => {
            println!("{}operation {} {{", pad, o.name);
            for input in &o.inputs {
                println!("{}  input {}: {:?}", pad, input.name, input.field_type);
            }
            if let Some(out) = &o.output {
                println!("{}  output: {:?}", pad, out);
            }
            for e in &o.errors {
                println!("{}  error {}: {:?}", pad, e.name, e.value);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Event(e) => {
            println!("{}event {} {{", pad, e.name);
            for field in &e.fields {
                println!("{}  field {}: {:?}", pad, field.name, field.field_type);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Config(c) => {
            println!("{}config {} {{", pad, c.name);
            for field in &c.fields {
                println!("{}  field {}: {:?}", pad, field.name, field.field_type);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Portal(p) => {
            println!("{}portal {} {{", pad, p.name);
            for prop in &p.properties {
                println!("{}  {}: {:?}", pad, prop.name, prop.value);
            }
            println!("{}}}", pad);
        }
        armature::parser::SymbolDef::Surface(s) => {
            println!("{}surface {} {{", pad, s.name);
            for d in &s.displays {
                println!("{}  display {}: {:?}", pad, d.name, d.display_type);
            }
            for state in &s.states {
                println!("{}  state {}: {:?}", pad, state.name, state.field_type);
            }
            for output in &s.outputs {
                println!("{}  output {}: {}", pad, output.name, output.value);
            }
            if let Some(mapping) = &s.mapping {
                println!("{}  mapping {{", pad);
                for entry in &mapping.entries {
                    println!("{}    {}: {}", pad, entry.key, entry.value);
                }
                println!("{}  }}", pad);
            }
            println!("{}}}", pad);
        }
    }
}

fn cmd_check(file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&file)?;
    let filename = file.to_string_lossy();

    let parsed = armature::parser::parse_file(&source, &filename)?;

    match parsed {
        armature::parser::ParsedFile::Arm(ast) => {
            // Expand clones (merge base fields into cloned symbols)
            let (ast, expansion_errors) = armature::expand::expand_clones(ast);
            for error in &expansion_errors {
                eprintln!("{}: error: {}", filename, error);
            }
            if !expansion_errors.is_empty() {
                eprintln!("\ncheck: {}: FAILED ({} clone expansion errors)", file.display(), expansion_errors.len());
                std::process::exit(1);
            }

            // Analyze
            let (symbols, result) = armature::analyzer::analyze(&ast);

            // Report errors
            for error in &result.errors {
                eprintln!("{}: error: {}", filename, error);
            }

            // Report warnings
            for warning in &result.warnings {
                eprintln!("{}: {}", filename, warning);
            }

            if result.has_errors() {
                eprintln!("\ncheck: {}: FAILED ({} errors, {} warnings)",
                    file.display(), result.errors.len(), result.warnings.len());
                std::process::exit(1);
            }

            println!("check: {}: OK", file.display());
            println!("  namespace: {}", symbols.namespace().unwrap_or("(none)"));
            println!("  imports: {}", ast.imports.len());
            println!("  symbols: {}", symbols.len());

            // Count by kind
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for (_, info) in symbols.iter() {
                *counts.entry(info.kind.as_str()).or_insert(0) += 1;
            }
            for (kind, count) in counts {
                println!("    {}: {}", kind, count);
            }

            if !result.warnings.is_empty() {
                println!("  warnings: {}", result.warnings.len());
            }
        }
        armature::parser::ParsedFile::Ac(ac_file) => {
            // .ac files have simpler validation - just parsing is enough for now
            println!("check: {}: OK (.ac project file)", file.display());

            if let Some(project) = &ac_file.project {
                println!("  project: {}", project.name);
            }
            println!("  platforms: {}", ac_file.platforms.len());
            println!("  aliases: {}", ac_file.aliases.len());
            println!("  imports: {}", ac_file.imports.len());
        }
    }

    Ok(())
}

fn cmd_compile(file: PathBuf, output: PathBuf, allow_warnings: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&file)?;
    let filename = file.to_string_lossy();

    let parsed = armature::parser::parse_file(&source, &filename)?;

    match parsed {
        armature::parser::ParsedFile::Arm(ast) => {
            // Expand clones (merge base fields into cloned symbols)
            let (ast, expansion_errors) = armature::expand::expand_clones(ast);
            for error in &expansion_errors {
                eprintln!("{}: error: {}", filename, error);
            }
            if !expansion_errors.is_empty() {
                eprintln!("\ncompile: {}: FAILED ({} clone expansion errors)", file.display(), expansion_errors.len());
                std::process::exit(1);
            }

            // Analyze
            let (symbols, result) = armature::analyzer::analyze(&ast);

            // Report errors
            for error in &result.errors {
                eprintln!("{}: error: {}", filename, error);
            }

            // Report warnings
            for warning in &result.warnings {
                eprintln!("{}: {}", filename, warning);
            }

            if result.has_errors() {
                eprintln!("\ncompile: {}: FAILED ({} errors, {} warnings)",
                    file.display(), result.errors.len(), result.warnings.len());
                std::process::exit(1);
            }

            if !allow_warnings && !result.warnings.is_empty() {
                eprintln!("\ncompile: {}: FAILED (has {} warnings, use --allow-warnings to proceed)",
                    file.display(), result.warnings.len());
                std::process::exit(1);
            }

            // Compile to SQLite
            armature::compiler::compile(&ast, &symbols, &filename, &output)?;

            println!("compile: {} -> {}", file.display(), output.display());
            println!("  namespace: {}", symbols.namespace().unwrap_or("(none)"));
            println!("  symbols: {}", symbols.len());

            // Count by kind
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for (_, info) in symbols.iter() {
                *counts.entry(info.kind.as_str()).or_insert(0) += 1;
            }
            for (kind, count) in counts {
                println!("    {}: {}", kind, count);
            }

            if !result.warnings.is_empty() {
                println!("  warnings: {} (allowed)", result.warnings.len());
            }
        }
        armature::parser::ParsedFile::Ac(ac_file) => {
            // Compile .ac file
            armature::compiler::compile_ac(&ac_file, &filename, &output)?;

            println!("compile: {} -> {}", file.display(), output.display());
            if let Some(project) = &ac_file.project {
                println!("  project: {}", project.name);
            }
            println!("  platforms: {}", ac_file.platforms.len());
            println!("  aliases: {}", ac_file.aliases.len());
            println!("  imports: {}", ac_file.imports.len());
        }
    }

    Ok(())
}

fn cmd_ls(file: PathBuf, kind_filter: Option<String>, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&file)?;
    let filename = file.to_string_lossy();

    let parsed = armature::parser::parse_file(&source, &filename)?;

    match parsed {
        armature::parser::ParsedFile::Arm(ast) => {
            // Analyze to get symbol table
            let (symbols, _result) = armature::analyzer::analyze(&ast);

            // Collect and filter symbols
            let mut entries: Vec<_> = symbols.iter()
                .filter(|(_, info)| {
                    if let Some(ref filter) = kind_filter {
                        info.kind.as_str() == filter
                    } else {
                        true
                    }
                })
                .collect();

            // Sort by kind, then name
            entries.sort_by(|a, b| {
                let kind_cmp = a.1.kind.as_str().cmp(b.1.kind.as_str());
                if kind_cmp == std::cmp::Ordering::Equal {
                    a.0.cmp(b.0)
                } else {
                    kind_cmp
                }
            });

            // Print symbols
            if verbose {
                for (name, info) in &entries {
                    println!("{} {}", info.kind.as_str(), name);
                    if let Some(ctx) = &info.context {
                        println!("  context: {}", ctx);
                    }
                    if !info.references.is_empty() {
                        println!("  references: {}", info.references.join(", "));
                    }
                }
            } else {
                // Group by kind for compact output
                let mut current_kind = "";
                for (name, info) in &entries {
                    let kind = info.kind.as_str();
                    if kind != current_kind {
                        if !current_kind.is_empty() {
                            println!();
                        }
                        println!("{}:", kind);
                        current_kind = kind;
                    }
                    println!("  {}", name);
                }
            }

            println!("\n{} symbols", entries.len());
        }
        armature::parser::ParsedFile::Ac(ac_file) => {
            // List .ac file contents
            if let Some(project) = &ac_file.project {
                println!("project: {}", project.name);
            }

            if !ac_file.platforms.is_empty() {
                println!("\nplatforms:");
                for platform in &ac_file.platforms {
                    println!("  {}", platform.name);
                }
            }

            if !ac_file.aliases.is_empty() {
                println!("\naliases:");
                for alias in &ac_file.aliases {
                    if verbose {
                        println!("  {} = {:?}", alias.name, alias.target_type);
                    } else {
                        println!("  {}", alias.name);
                    }
                }
            }

            if !ac_file.imports.is_empty() {
                println!("\nimports:");
                for import in &ac_file.imports {
                    println!("  {}", import.path);
                }
            }

            let total = ac_file.platforms.len() + ac_file.aliases.len() + ac_file.imports.len();
            println!("\n{} items", total);
        }
    }

    Ok(())
}
