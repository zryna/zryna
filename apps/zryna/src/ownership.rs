//! Presentation of the shared manifest-v3 driver result.

use std::process::ExitCode;

use serde_json::json;
use zryna_abi::{ScalarOutcome, ScalarValue};
use zryna_driver::{CommandFailure, CommandKind, PublishedOwnershipBundle};

pub(super) fn render(
    result: Result<PublishedOwnershipBundle, CommandFailure>,
    command: CommandKind,
    json_mode: bool,
) -> ExitCode {
    let bundle = match result {
        Ok(bundle) => bundle,
        Err(failure) => return super::render_failure(command, json_mode, &failure),
    };
    let name = bundle.path().file_name().expect("published bundle has a portable filename");
    let manifest = format!(".zryna/out/{}/zryna-manifest-v3.json", name.to_string_lossy());
    if json_mode {
        let response = json!({
            "version": 1,
            "ok": true,
            "command": bundle.command(),
            "manifest": manifest,
            "results": bundle.results(),
            "diagnostics": bundle.diagnostics(),
        });
        match serde_json::to_string_pretty(&response) {
            Ok(output) => println!("{output}"),
            Err(_) => return ExitCode::from(4),
        }
    } else if command == CommandKind::Build {
        println!("{manifest}");
    } else {
        for result in bundle.results() {
            let target = match result.target() {
                zryna_driver::OwnershipTarget::JavaScript => "javascript",
                zryna_driver::OwnershipTarget::WebAssembly => "webassembly",
                zryna_driver::OwnershipTarget::Native => "native",
            };
            match result.outcome() {
                ScalarOutcome::Returned { value: ScalarValue::I32(value) } => {
                    println!("{target}: i32 {value}");
                }
                ScalarOutcome::Returned { value: ScalarValue::Bool(value) } => {
                    println!("{target}: bool {value}");
                }
                ScalarOutcome::Trapped { code } => println!("{target}: trapped {code:?}"),
                ScalarOutcome::HostError { code } => println!("{target}: host-error {code:?}"),
            }
        }
    }
    if !json_mode {
        for diagnostic in bundle.diagnostics() {
            eprintln!("{diagnostic}");
        }
    }
    ExitCode::SUCCESS
}
