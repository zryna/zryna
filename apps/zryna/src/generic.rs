//! Presentation of an actually committed generic driver result.
use serde_json::json;
use std::process::ExitCode;
use zryna_abi::{ScalarOutcome, ScalarValue};
use zryna_driver::PublishedGenericBundle;

pub(super) fn render(bundle: &PublishedGenericBundle, json_mode: bool) -> ExitCode {
    let Some(name) = bundle.bundle_path().file_name().and_then(|name| name.to_str()) else {
        return ExitCode::from(4);
    };
    let manifest = format!(".zryna/out/{name}/zryna-generic-manifest-v1.json");
    if json_mode {
        let response = json!({
            "version":1, "ok":true, "command":bundle.command(), "manifest":manifest,
            "results":bundle.results(), "diagnostics":bundle.diagnostics(),
        });
        match serde_json::to_string_pretty(&response) {
            Ok(output) => println!("{output}"),
            Err(_) => return ExitCode::from(4),
        }
    } else {
        if bundle.command() == "build" {
            println!("{manifest}");
        }
        for result in bundle.results() {
            let target = result.target();
            match result.outcome() {
                ScalarOutcome::Returned { value: ScalarValue::I32(value) } => {
                    println!("{target}: i32 {value}");
                }
                ScalarOutcome::Returned { value: ScalarValue::Bool(value) } => {
                    println!("{target}: bool {value}");
                }
                ScalarOutcome::Trapped { code } => println!("{target}: trapped {}", code.as_str()),
                ScalarOutcome::HostError { code } => println!("{target}: host-error {code:?}"),
            }
        }
        for diagnostic in bundle.diagnostics() {
            eprintln!("{diagnostic}");
        }
    }
    ExitCode::SUCCESS
}
