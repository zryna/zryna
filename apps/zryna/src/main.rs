//! Zryna command-line interface.

#![forbid(unsafe_code)]

mod cli_paths;
mod command_h1;
mod installed;
mod ownership;
mod package;
mod profile;
mod project;
mod project_filesystem;
mod provider;
use provider::{provider_request, render_provider_result};
mod render;
mod server;
mod support;

use cli_paths::absolute_workspace_path;
use render::{render_cli_failure, render_failure, render_success};

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use profile::{CliProfile, CliTarget};
use zryna_abi::ScalarValue;
use zryna_driver::{CommandKind, TargetSelection};

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
    /// Resolve one standalone source package and verify or update its lockfile.
    Package {
        #[command(subcommand)]
        command: package::Command,
    },
    /// Create one deterministic standalone source project without replacing files.
    New(project::NewOptions),
    /// Compile one Zryna entrypoint into one atomic target bundle.
    Build(CompileOptions),
    /// Compile and invoke one scalar export, then commit one atomic target bundle.
    Run(RunOptions),
    /// Report the compiled public M4 contract without authenticating execution.
    Support(support::Options),
    /// Run one finite pure-source loopback server and commit its teardown record.
    Serve(server::Options),
    /// Fixed compiler-owned frontend subprocess; not a source or tool selector.
    #[command(name = "__native-frontend", hide = true)]
    NativeFrontend {
        #[arg(long)]
        protocol: u32,
    },
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
    /// Select an exact versioned profile; omission preserves M1.
    #[arg(long, value_enum)]
    profile: Option<CliProfile>,
    /// Source-checkout workspace root; defaults to the current directory.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Explicit standalone project root; omission preserves repository-local behavior.
    #[arg(long)]
    project_root: Option<PathBuf>,
    /// Portable output stem; defaults to the entrypoint stem.
    #[arg(long)]
    name: Option<String>,
    /// Exact frontend mode; omission selects native, except legacy installed packages.
    #[arg(long, value_enum)]
    provider: Option<CliProvider>,
    /// Optional direct Node.js 22.22.1 host for bootstrap or target execution.
    #[arg(long)]
    node: Option<PathBuf>,
    /// Emit one versioned JSON response.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CliProvider {
    Native,
    Bootstrap,
    Compare,
}
impl From<CliProvider> for zryna_driver::CompilerProvider {
    fn from(value: CliProvider) -> Self {
        match value {
            CliProvider::Native => Self::Native,
            CliProvider::Bootstrap => Self::Bootstrap,
            CliProvider::Compare => Self::Compare,
        }
    }
}

#[derive(Clone, Debug, clap::Args)]
struct RunOptions {
    #[command(flatten)]
    compile: CompileOptions,
    /// Exact logical scalar export.
    #[arg(long)]
    export: String,
    /// Ordered canonical typed argument, for example --arg=i32:42.
    #[arg(long = "arg", value_parser = profile::parse_scalar_argument)]
    arguments: Vec<ScalarValue>,
    /// Absolute owner-private explicit command request file.
    #[arg(long)]
    grant_file: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = match parse_cli_from(std::env::args_os()) {
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
        | Command::Doctor(options) => render::run_architecture_check(&options),
        Command::Package { command } => package::run(command),
        Command::New(options) => project::create(&options),
        Command::Build(options) => run_build(options),
        Command::Run(options) => run_command(options),
        Command::Serve(options) => server::run(&options),
        Command::Support(options) => support::report(&options),
        Command::NativeFrontend { protocol } => {
            match zryna_driver::run_native_frontend_worker(protocol) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn parse_cli_from<I, T>(arguments: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
    let typed_scalars = profile::selects_typed_scalars(&arguments);
    if !typed_scalars {
        return Cli::try_parse_from(arguments);
    }
    let mut command = Cli::command();
    command = command.mut_subcommand("run", |run| {
        run.mut_arg("arguments", |argument| {
            argument.value_parser(profile::parse_control_flow_argument)
        })
    });
    let matches = command.try_get_matches_from(arguments)?;
    Cli::from_arg_matches(&matches)
}

fn run_build(options: CompileOptions) -> ExitCode {
    if command_h1::selected(&options) {
        return render_cli_failure(
            CommandKind::Build,
            options.json,
            2,
            &[command_h1::configuration_error()],
        );
    }
    if zryna_driver::distribution::InstalledCompiler::is_distribution_build() {
        return installed::execute(options, None);
    }
    let json_mode = options.json;
    let request = match provider_request(options) {
        Ok(request) => request,
        Err(error) => return render_cli_failure(CommandKind::Build, json_mode, 2, &[error]),
    };
    let result = zryna_driver::build_with_provider(&request);
    render_provider_result(result, CommandKind::Build, json_mode)
}

fn run_command(options: RunOptions) -> ExitCode {
    if command_h1::selected(&options.compile) || options.grant_file.is_some() {
        return command_h1::run(&options);
    }
    if zryna_driver::distribution::InstalledCompiler::is_distribution_build() {
        return installed::execute(options.compile, Some((options.export, options.arguments)));
    }
    let json_mode = options.compile.json;
    let request = match provider_request(options.compile) {
        Ok(request) => request,
        Err(error) => return render_cli_failure(CommandKind::Run, json_mode, 2, &[error]),
    };
    let result = zryna_driver::run_with_provider(&request, options.export, options.arguments);
    render_provider_result(result, CommandKind::Run, json_mode)
}

#[cfg(test)]
mod tests;
