//! UTS command-line interface.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "uts", version, about = "Strict UTS compiler workspace tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate the canonical repository architecture.
    Architecture {
        #[command(subcommand)]
        command: ArchitectureCommand,
    },
    /// Report local compiler-workspace health.
    Doctor {
        /// Workspace root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ArchitectureCommand {
    /// Run the mandatory fail-closed architecture gate.
    Check {
        /// Workspace root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Architecture { command: ArchitectureCommand::Check { root, json } }
        | Command::Doctor { root, json } => run_architecture_check(&root, json),
    }
}

fn run_architecture_check(root: &Path, json: bool) -> Result<()> {
    let report = uts_driver::check_workspace(root);
    if json {
        let output =
            serde_json::to_string_pretty(&report).context("serialize architecture report")?;
        println!("{output}");
    } else if report.is_valid() {
        println!("UTS architecture check passed");
    } else {
        for diagnostic in &report.diagnostics {
            eprintln!("{diagnostic}");
        }
    }
    if report.is_valid() { Ok(()) } else { bail!("UTS architecture check failed") }
}
