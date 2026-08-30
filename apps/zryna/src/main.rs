//! Zryna command-line interface.

#![forbid(unsafe_code)]

use std::{path::PathBuf, process::ExitCode};

use clap::error::ErrorKind;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use zryna_abi::{ScalarOutcome, ScalarValue};
use zryna_diagnostics::Diagnostic;
use zryna_driver::{
    BuildRequest, CommandFailure, CommandKind, CommandSuccess, RunRequest, TargetSelection,
};

#[derive(Debug, Parser)]
#[command(name = "zryna", version, about = "Strict Zryna compiler workspace tools")]
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
    Doctor(ArchitectureOptions),
    /// Compile one Zryna entrypoint into one atomic target bundle.
    Build(CompileOptions),
    /// Compile and invoke one scalar export, then commit one atomic target bundle.
    Run(RunOptions),
}

#[derive(Debug, Subcommand)]
enum ArchitectureCommand {
    /// Run the mandatory fail-closed architecture gate.
    Check(ArchitectureOptions),
}

#[derive(Clone, Debug, clap::Args)]
struct ArchitectureOptions {
    /// Workspace root.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Debug, clap::Args)]
struct CompileOptions {
    /// One portable workspace-relative .zry entrypoint.
    entrypoint: String,
    /// Explicit target selection.
    #[arg(long, value_enum)]
    target: CliTarget,
    /// Workspace root.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Portable output stem; defaults to the entrypoint stem.
    #[arg(long)]
    name: Option<String>,
    /// Absolute direct Node.js 22.22.1 executable.
    #[arg(long)]
    node: PathBuf,
    /// Emit one versioned JSON response.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Debug, clap::Args)]
struct RunOptions {
    #[command(flatten)]
    compile: CompileOptions,
    /// Exact logical scalar export.
    #[arg(long)]
    export: String,
    /// Ordered canonical typed argument, for example --arg=i32:42.
    #[arg(long = "arg", value_parser = parse_scalar_argument)]
    arguments: Vec<ScalarValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
enum CliTarget {
    JavaScript,
    WebAssembly,
    Native,
    All,
}

impl From<CliTarget> for TargetSelection {
    fn from(value: CliTarget) -> Self {
        match value {
            CliTarget::JavaScript => Self::JavaScript,
            CliTarget::WebAssembly => Self::WebAssembly,
            CliTarget::Native => Self::Native,
            CliTarget::All => Self::All,
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let successful_display =
                matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);
            let _ = error.print();
            return if successful_display { ExitCode::SUCCESS } else { ExitCode::from(2) };
        }
    };
    match cli.command {
        Command::Architecture { command: ArchitectureCommand::Check(options) }
        | Command::Doctor(options) => run_architecture_check(&options),
        Command::Build(options) => run_build(options),
        Command::Run(options) => run_command(options),
    }
}

fn run_architecture_check(options: &ArchitectureOptions) -> ExitCode {
    let root = match absolute_workspace_path(&options.root) {
        Ok(root) => root,
        Err(diagnostic) => {
            return render_cli_failure(CommandKind::Build, options.json, 2, &[diagnostic]);
        }
    };
    let report = zryna_driver::check_workspace(&root);
    if options.json {
        match serde_json::to_string_pretty(&report) {
            Ok(output) => println!("{output}"),
            Err(_) => return ExitCode::from(70),
        }
    } else if report.is_valid() {
        println!("Zryna architecture check passed");
    } else {
        for diagnostic in &report.diagnostics {
            eprintln!("{diagnostic}");
        }
    }
    if report.is_valid() { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn run_build(options: CompileOptions) -> ExitCode {
    let json_mode = options.json;
    let request = match build_request(options) {
        Ok(request) => request,
        Err(diagnostic) => {
            return render_cli_failure(CommandKind::Build, json_mode, 2, &[diagnostic]);
        }
    };
    match zryna_driver::build_workspace(&request) {
        Ok(success) => render_success(&success, json_mode),
        Err(failure) => render_failure(CommandKind::Build, json_mode, &failure),
    }
}

fn run_command(options: RunOptions) -> ExitCode {
    let json_mode = options.compile.json;
    let export = options.export;
    let arguments = options.arguments;
    let build = match build_request(options.compile) {
        Ok(request) => request,
        Err(diagnostic) => {
            return render_cli_failure(CommandKind::Run, json_mode, 2, &[diagnostic]);
        }
    };
    match zryna_driver::run_workspace(RunRequest { build, logical_export: export, arguments }) {
        Ok(success) => render_success(&success, json_mode),
        Err(failure) => render_failure(CommandKind::Run, json_mode, &failure),
    }
}

fn build_request(options: CompileOptions) -> Result<BuildRequest, Diagnostic> {
    let root = absolute_workspace_path(&options.root)?;
    if !options.node.is_absolute() {
        return Err(cli_path_error());
    }
    let node = options.node;
    let stem = options.name.unwrap_or_else(|| default_stem(&options.entrypoint));
    Ok(BuildRequest {
        workspace_root: root,
        entrypoint: options.entrypoint,
        artifact_stem: stem,
        targets: options.target.into(),
        node_runtime: node,
    })
}

fn absolute_workspace_path(path: &PathBuf) -> Result<PathBuf, Diagnostic> {
    if path.is_absolute() {
        Ok(path.clone())
    } else {
        Ok(std::env::current_dir().map_err(|_| cli_path_error())?.join(path))
    }
}

fn default_stem(entrypoint: &str) -> String {
    std::path::Path::new(entrypoint)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_owned()
}

fn parse_scalar_argument(value: &str) -> Result<ScalarValue, String> {
    let Some(decimal) = value.strip_prefix("i32:") else {
        return Err("expected canonical i32:<decimal> argument".to_owned());
    };
    let canonical = if decimal == "0" {
        true
    } else if let Some(digits) = decimal.strip_prefix('-') {
        !digits.is_empty()
            && !digits.starts_with('0')
            && digits.bytes().all(|byte| byte.is_ascii_digit())
    } else {
        !decimal.is_empty()
            && !decimal.starts_with('0')
            && decimal.bytes().all(|byte| byte.is_ascii_digit())
    };
    if !canonical {
        return Err("expected canonical signed base-ten i32 without whitespace or leading zeroes"
            .to_owned());
    }
    decimal
        .parse::<i32>()
        .map(ScalarValue::I32)
        .map_err(|_| "i32 argument is outside the signed 32-bit range".to_owned())
}

fn render_success(success: &CommandSuccess, json_mode: bool) -> ExitCode {
    if json_mode {
        let results = success
            .results()
            .iter()
            .map(|result| {
                json!({
                    "target": result.target(),
                    "outcome": result.outcome(),
                })
            })
            .collect::<Vec<_>>();
        let response = json!({
            "version": 1,
            "ok": true,
            "command": success.command(),
            "manifest": success.manifest_portable_path(),
            "results": results,
            "diagnostics": success.diagnostics(),
        });
        match serde_json::to_string_pretty(&response) {
            Ok(output) => println!("{output}"),
            Err(_) => return render_json_serialization_failure(success.command()),
        }
    } else if success.command() == CommandKind::Build {
        println!("{}", success.manifest_portable_path());
        for diagnostic in success.diagnostics() {
            eprintln!("{diagnostic}");
        }
    } else {
        for result in success.results() {
            match result.outcome() {
                ScalarOutcome::Returned { value: ScalarValue::I32(value) } => {
                    println!("{}: i32 {value}", result.target());
                }
                ScalarOutcome::Returned { value: ScalarValue::Bool(value) } => {
                    println!("{}: bool {value}", result.target());
                }
                ScalarOutcome::Trapped { code } => {
                    println!("{}: trapped {code:?}", result.target());
                }
                ScalarOutcome::HostError { code } => {
                    println!("{}: host-error {code:?}", result.target());
                }
            }
        }
        for diagnostic in success.diagnostics() {
            eprintln!("{diagnostic}");
        }
    }
    ExitCode::SUCCESS
}

fn render_failure(command: CommandKind, json_mode: bool, failure: &CommandFailure) -> ExitCode {
    render_cli_failure(command, json_mode, failure.kind().exit_code(), failure.diagnostics())
}

fn render_cli_failure(
    command: CommandKind,
    json_mode: bool,
    exit_code: u8,
    diagnostics: &[Diagnostic],
) -> ExitCode {
    if json_mode {
        let response = json!({
            "version": 1,
            "ok": false,
            "command": command,
            "manifest": null,
            "results": [],
            "diagnostics": diagnostics,
        });
        match serde_json::to_string_pretty(&response) {
            Ok(output) => println!("{output}"),
            Err(_) => return render_json_serialization_failure(command),
        }
    } else {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
    }
    ExitCode::from(exit_code)
}

fn render_json_serialization_failure(command: CommandKind) -> ExitCode {
    let command = match command {
        CommandKind::Build => "build",
        CommandKind::Run => "run",
    };
    println!(
        "{{\"version\":1,\"ok\":false,\"command\":\"{command}\",\"manifest\":null,\"results\":[],\"diagnostics\":[{{\"code\":\"ZRYNA-C1011\",\"severity\":\"error\",\"primary\":{{\"kind\":\"global\"}},\"message\":\"CLI JSON serialization failed\",\"guidance\":\"report this compiler invariant failure\"}}]}}"
    );
    ExitCode::from(4)
}

fn cli_path_error() -> Diagnostic {
    Diagnostic::error(
        "ZRYNA-C1001",
        None,
        "CLI path could not be resolved to an existing absolute path",
        "pass an existing workspace root and direct Node.js executable path",
    )
}

#[cfg(test)]
mod tests {
    use super::{Cli, parse_scalar_argument};
    use clap::Parser;
    use zryna_abi::ScalarValue;

    #[test]
    fn target_and_node_are_required() {
        let error = Cli::try_parse_from(["zryna", "build", "src/main.zry"])
            .expect_err("target and node must be required");
        let rendered = error.to_string();
        assert!(rendered.contains("--target"));
        assert!(rendered.contains("--node"));
    }

    #[test]
    fn scalar_arguments_are_canonical() {
        assert_eq!(parse_scalar_argument("i32:-2147483648"), Ok(ScalarValue::I32(i32::MIN)));
        assert_eq!(parse_scalar_argument("i32:2147483647"), Ok(ScalarValue::I32(i32::MAX)));
        for rejected in ["i32:+1", "i32:01", "i32:-0", "i32: 1", "bool:true"] {
            assert!(parse_scalar_argument(rejected).is_err(), "{rejected}");
        }
    }
}
