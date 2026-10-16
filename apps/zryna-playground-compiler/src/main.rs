//! One-source compiler transport and independently approved fixed daemon host entry point.

#![forbid(unsafe_code)]

use std::{env, io, path::PathBuf, process::ExitCode};

use zryna_driver::RestrictedBrowserCompiler;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod daemon_host;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod material_audit;

fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--version" {
        println!(
            "zryna-playground-compiler {} restricted-browser-v1 {}",
            env!("CARGO_PKG_VERSION"),
            option_env!("ZRYNA_PLAYGROUND_SOURCE_COMMIT").unwrap_or("source-build")
        );
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && args[0] == "--audit-materials" {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return material_audit::entry();
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            eprintln!("PLAYGROUND-HOST-UNSUPPORTED");
            return ExitCode::from(1);
        }
    }
    if args.first().is_some_and(|argument| argument == "--daemon-approval") {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return daemon_host::entry(&args);
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        {
            eprintln!("PLAYGROUND-HOST-UNSUPPORTED");
            return ExitCode::from(1);
        }
    }
    let result = (|| {
        if args.len() != 2 || args[0] != "--materials-root" {
            return Err("PLAYGROUND-CONFIGURATION".to_owned());
        }
        let root = PathBuf::from(&args[1]);
        if !root.is_absolute() {
            return Err("PLAYGROUND-CONFIGURATION".to_owned());
        }
        let request = zryna_playground_compiler::read_request(&mut io::stdin().lock())
            .map_err(str::to_owned)?;
        let compiler = RestrictedBrowserCompiler::capture_isolated(&root)
            .map_err(|error| error.to_string())?;
        let response = compiler
            .compile_isolated(request.revision(), request.source())
            .map_err(|error| error.to_string())?;
        response
            .write_frame(&mut io::stdout().lock())
            .map_err(|_| "PLAYGROUND-OUTPUT-FAILED".to_owned())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(1)
        }
    }
}
