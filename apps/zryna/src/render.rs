use std::process::ExitCode;

use serde_json::json;
use zryna_abi::{ScalarOutcome, ScalarValue};
use zryna_diagnostics::Diagnostic;
use zryna_driver::{CommandFailure, CommandKind, CommandSuccess};

pub(super) fn render_success(success: &CommandSuccess, json_mode: bool) -> ExitCode {
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

pub(super) fn render_failure(
    command: CommandKind,
    json_mode: bool,
    failure: &CommandFailure,
) -> ExitCode {
    render_cli_failure(command, json_mode, failure.kind().exit_code(), failure.diagnostics())
}

pub(super) fn render_cli_failure(
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
