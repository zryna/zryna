//! One framed operation; independently pinned approval precedes fixed daemon authority.

use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use zryna_driver::{PlaygroundDaemonOperatorApproval, approve_playground_daemon};

mod approval;
mod evaluation;
mod framing;
mod terminal;
use terminal::Failure;

pub(super) fn entry(arguments: &[OsString]) -> std::process::ExitCode {
    match run(arguments) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(failure) => {
            failure.write();
            std::process::ExitCode::from(1)
        }
    }
}

fn run(arguments: &[OsString]) -> Result<(), Failure> {
    if arguments.len() != 6
        || arguments[0] != "--daemon-approval"
        || arguments[2] != "--approval-sha256"
        || arguments[4] != "--operation"
    {
        return Err(Failure::clean("PLAYGROUND-CONFIGURATION"));
    }
    let operation = arguments[5].to_str().ok_or("PLAYGROUND-CONFIGURATION")?;
    let limit = match operation {
        "compile" => zryna_playground_compiler::MAX_REQUEST_BYTES,
        "evaluate" => evaluation::MAX_EVALUATION_BYTES,
        _ => return Err(Failure::clean("PLAYGROUND-CONFIGURATION")),
    };
    let expected = arguments[3].to_str().ok_or("PLAYGROUND-HOST-APPROVAL")?;
    if !evaluation::hash(expected) || !Path::new(&arguments[1]).is_absolute() {
        return Err(Failure::clean("PLAYGROUND-HOST-APPROVAL"));
    }
    // Validate inert input before opening an approval or contacting any host service.
    let mut input = io::stdin();
    let payload = framing::read_payload(&mut input, limit).map_err(str::to_owned)?;
    let claims = match operation {
        "compile" => {
            zryna_playground_compiler::read_request(&mut payload.as_slice())
                .map_err(str::to_owned)?;
            None
        }
        "evaluate" => Some(evaluation::validate(&payload).map_err(str::to_owned)?),
        _ => return Err(Failure::clean("PLAYGROUND-CONFIGURATION")),
    };
    let cancellation = Arc::new(AtomicBool::new(false));
    let control = Arc::clone(&cancellation);
    // The trusted bridge holds stdin open. A byte, EOF or failed control pipe cancels.
    std::thread::Builder::new()
        .name("playground-control".to_owned())
        .spawn(move || framing::control(&mut input, &control))
        .map_err(|_| "PLAYGROUND-CONTROL".to_owned())?;
    let bytes = approval::capture(Path::new(&arguments[1]), expected).map_err(str::to_owned)?;
    let record: PlaygroundDaemonOperatorApproval =
        serde_json::from_slice(&bytes).map_err(|_| "PLAYGROUND-HOST-APPROVAL".to_owned())?;
    if let Some(claims) = claims {
        claims.authenticate(&record).map_err(str::to_owned)?;
    }
    if cancellation.load(Ordering::Acquire) {
        return Err(Failure::clean("PLAYGROUND-CANCELLED"));
    }
    let mut daemon = approve_playground_daemon(record).map_err(|error| Failure::driver(&error))?;
    let response = match operation {
        "compile" => daemon.compile(&payload, &cancellation),
        "evaluate" => daemon.evaluate(&payload, &cancellation),
        _ => return Err(Failure::clean("PLAYGROUND-CONFIGURATION")),
    }
    .map_err(|error| Failure::driver(&error))?;
    if cancellation.load(Ordering::Acquire) {
        return Err(Failure::clean("PLAYGROUND-CANCELLED"));
    }
    // Daemon success exists only after confirmed removal of owned namespaces and storage.
    io::stdout()
        .lock()
        .write_all(response.stdout())
        .map_err(|_| Failure::clean("PLAYGROUND-OUTPUT-FAILED"))?;
    Ok(())
}
