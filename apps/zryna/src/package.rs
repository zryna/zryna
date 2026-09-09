use std::{path::PathBuf, process::ExitCode};

use clap::{Subcommand, ValueEnum};
use serde_json::json;
use zryna_driver::{PackageLockMode, PackageResolutionRequest};

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Resolve exact local and prepopulated Git sources without network access.
    Resolve(Options),
}

#[derive(Clone, Debug, clap::Args)]
pub(crate) struct Options {
    /// Root package locator relative to the source root.
    package: String,
    /// Declared reproduction/source root.
    #[arg(long = "source-root", default_value = ".")]
    source_root: PathBuf,
    /// Optional prepopulated exact-commit Git material cache.
    #[arg(long = "git-cache")]
    git_cache: Option<PathBuf>,
    /// Verify the existing lock or atomically publish the resolved lock.
    #[arg(long, value_enum)]
    mode: Mode,
    /// Emit one machine-readable JSON response.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Frozen,
    Update,
}

pub(crate) fn run(command: Command) -> ExitCode {
    match command {
        Command::Resolve(options) => resolve(options),
    }
}

fn resolve(options: Options) -> ExitCode {
    let Ok(source_root) = absolute(options.source_root) else {
        return render_error(options.json, "ZRYNA-P4004", "source root is unavailable");
    };
    let Ok(git_cache) = options.git_cache.map(absolute).transpose() else {
        return render_error(options.json, "ZRYNA-P4004", "Git cache is unavailable");
    };
    let request = PackageResolutionRequest {
        source_root,
        package: options.package.clone(),
        git_cache,
        mode: match options.mode {
            Mode::Frozen => PackageLockMode::Frozen,
            Mode::Update => PackageLockMode::Update,
        },
    };
    match zryna_driver::resolve_package(&request) {
        Ok(success) => {
            let relative_lock = format!("{}/zryna.lock.json", options.package);
            if options.json {
                println!(
                    "{}",
                    json!({
                        "version": 1,
                        "ok": true,
                        "command": "package-resolve",
                        "lock": relative_lock,
                        "lockSha256": success.graph().lock_sha256(),
                        "packages": success.graph().packages().len(),
                        "published": success.published(),
                    })
                );
            } else if success.published() {
                println!("published {relative_lock}");
            } else {
                println!("verified {relative_lock}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => render_error(options.json, error.code(), error.detail()),
    }
}

fn absolute(path: PathBuf) -> Result<PathBuf, ()> {
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir().map(|root| root.join(path)).map_err(|_| ())
    }
}

fn render_error(json_mode: bool, code: &str, detail: &str) -> ExitCode {
    if json_mode {
        println!(
            "{}",
            json!({
                "version": 1,
                "ok": false,
                "command": "package-resolve",
                "lock": null,
                "diagnostics": [{ "code": code, "message": detail }],
            })
        );
    } else {
        eprintln!("error[{code}]: {detail}");
    }
    ExitCode::from(3)
}
