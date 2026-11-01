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

    println!("Running Aurora file: {}", path.display());

    // Tokenization
    let mut lexer = aurora_lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();

    // Parsing
    let mut parser = aurora_parser::Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!(
                "Successfully parsed {} statements.",
                program.statements.len()
            );
            0
        }
        Err(err) => {
            eprintln!("Parse Error: {err}");
            1
        }
    }
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

    let mut lexer = aurora_lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = aurora_parser::Parser::new(tokens);

    match parser.parse_program() {
        Ok(_) => {
            println!("Check passed: syntax is valid.");
            0
        }
        Err(err) => {
            eprintln!("Syntax Error: {err}");
            1
        }
    }
}

fn run_repl() -> i32 {
    println!(
        "Aurora REPL v{} (type 'exit' to quit)",
        env!("CARGO_PKG_VERSION")
    );
    // REPL loop implementation will expand in Faz 6
    0
}
