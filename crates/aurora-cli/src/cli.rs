//! CLI argument parsing and definitions using Clap.

use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;

#[derive(ClapParser, Debug)]
#[command(
    name = "aurora",
    about = "Aurora: High-level ergonomics, low-level power",
    version,
    long_about = "The official compiler, runtime, and developer toolchain for the Aurora programming language."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Optional file to run directly (e.g. `aurora script.au`)
    pub file: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute an Aurora source file or script
    Run {
        /// Source file path (*.au)
        file: PathBuf,
        /// Optional arguments passed to the script
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Compile Aurora source code to native executable or bytecode
    Build {
        /// Source file path (*.au)
        file: PathBuf,
        /// Output executable path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Enable optimizations
        #[arg(short = 'O', long)]
        optimize: bool,
    },

    /// Check syntax and types without executing
    Check {
        /// Source file path (*.au)
        file: PathBuf,
    },

    /// Start the interactive Read-Eval-Print Loop (REPL)
    Repl,

    /// Show version and build info
    Version,
}
