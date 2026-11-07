//! Aurora CLI entrypoint.

mod cli;

use clap::Parser as ClapParser;
use cli::{Cli, Commands};
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args = Cli::parse();

    let exit_code = match args.command {
        Some(Commands::Run { file, args: _ }) => run_file(&file),
        Some(Commands::Build {
            file,
            output,
            optimize,
        }) => build_file(&file, output.as_deref(), optimize),
        Some(Commands::Check { file }) => check_file(&file),
        Some(Commands::Repl) => run_repl(),
        Some(Commands::Version) => {
            print_banner();
            0
        }
        None => {
            if let Some(file) = args.file {
                run_file(&file)
            } else {
                print_banner();
                println!("Use `aurora --help` to view available commands.\n");
                0
            }
        }
    };

    if exit_code != 0 {
        process::exit(exit_code);
    }
}

fn print_banner() {
    println!(
        r"
      /\                                
     /  \  _   _ _ __ ___  _ __ __ _ 
    / /\ \| | | | '__/ _ \| '__/ _` |
   / ____ \ |_| | | | (_) | | | (_| |
  /_/    \_\__,_|_|  \___/|_|  \__,_|
  Aurora Language Toolchain v{}
  High-level elegance, low-level power.
",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_file(path: &Path) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error: Cannot read source file '{}': {err}", path.display());
            return 1;
        }
    };

    let file_id = aurora_ast::FileId(0);
    let source_file = aurora_ast::SourceFile::new(
        file_id,
        path.file_name().unwrap_or_default().to_string_lossy(),
        &source,
        Some(path.to_path_buf()),
    );

    println!("Running Aurora file: {}", path.display());

    // Tokenization
    let mut lexer = aurora_lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    // Check lexical diagnostics
    let mut has_lex_errors = false;
    for diag in lexer.diagnostics() {
        eprint!("{}", diag.render(&source_file));
        if diag.level == aurora_lexer::DiagnosticLevel::Error {
            has_lex_errors = true;
        }
    }

    if has_lex_errors {
        eprintln!("Execution aborted due to lexical errors.");
        return 1;
    }

    // Parsing
    let mut parser = aurora_parser::Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(err) => {
            let diag = aurora_lexer::Diagnostic::error(err.to_string(), err.span());
            eprint!("{}", diag.render(&source_file));
            return 1;
        }
    };

    println!(
        "Successfully parsed {} items and {} statements.",
        program.items.len(),
        program.statements.len()
    );

    // Semantic Analysis & Type Checking
    let mut analyzer = aurora_sema::SemanticAnalyzer::new();
    if let Err(sema_errors) = analyzer.analyze_program(&program) {
        for err in sema_errors {
            eprint!("{}", err.to_diagnostic().render(&source_file));
        }
        eprintln!("Execution aborted due to semantic/type errors.");
        return 1;
    }

    0
}

fn build_file(path: &Path, output: Option<&Path>, optimize: bool) -> i32 {
    println!(
        "Building Aurora file {} (Output: {:?}, Optimize: {optimize})",
        path.display(),
        output.map(Path::display)
    );
    // Future compiler pipeline integration
    0
}

fn check_file(path: &Path) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Error reading '{}': {err}", path.display());
            return 1;
        }
    };

    let file_id = aurora_ast::FileId(0);
    let source_file = aurora_ast::SourceFile::new(
        file_id,
        path.file_name().unwrap_or_default().to_string_lossy(),
        &source,
        Some(path.to_path_buf()),
    );

    let mut lexer = aurora_lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut has_errors = false;
    for diag in lexer.diagnostics() {
        eprint!("{}", diag.render(&source_file));
        if diag.level == aurora_lexer::DiagnosticLevel::Error {
            has_errors = true;
        }
    }

    if has_errors {
        eprintln!("Check failed due to syntax errors.");
        return 1;
    }

    let mut parser = aurora_parser::Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(err) => {
            let diag = aurora_lexer::Diagnostic::error(err.to_string(), err.span());
            eprint!("{}", diag.render(&source_file));
            return 1;
        }
    };

    // Semantic Analysis & Type Checking
    let mut analyzer = aurora_sema::SemanticAnalyzer::new();
    if let Err(sema_errors) = analyzer.analyze_program(&program) {
        for err in sema_errors {
            eprint!("{}", err.to_diagnostic().render(&source_file));
        }
        eprintln!("Check failed due to semantic/type errors.");
        return 1;
    }

    println!(
        "Check passed: syntax and types are valid ({} items, {} statements).",
        program.items.len(),
        program.statements.len()
    );
    0
}

fn run_repl() -> i32 {
    println!(
        "Aurora REPL v{} (type 'exit' to quit)",
        env!("CARGO_PKG_VERSION")
    );
    // REPL loop implementation will expand in Faz 6
    0
}
