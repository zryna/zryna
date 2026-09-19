//! Standard-input entry point for the bounded Zryna language server.

use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--version" {
        println!(
            "zryna-language-server {} portable-setup-v1 {}",
            env!("CARGO_PKG_VERSION"),
            option_env!("ZRYNA_TOOLING_SOURCE_COMMIT").unwrap_or("source-build")
        );
        return ExitCode::SUCCESS;
    }
    let result = if args.len() == 2 && args[0] == "--installed-root" {
        zryna_language_server::run_installed_stdio(&PathBuf::from(&args[1]))
    } else {
        arguments().and_then(|(root, node)| zryna_language_server::run_stdio(&root, &node))
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zryna-language-server: {error}");
            ExitCode::from(1)
        }
    }
}

fn arguments() -> Result<(PathBuf, PathBuf), String> {
    let mut root = None;
    let mut node = None;
    let mut arguments = env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments.next().ok_or_else(|| "every option requires one value".to_owned())?;
        match argument.to_str() {
            Some("--compiler-root") if root.is_none() => root = Some(PathBuf::from(value)),
            Some("--node") if node.is_none() => node = Some(PathBuf::from(value)),
            _ => return Err("expected exactly --compiler-root <path> --node <path>".to_owned()),
        }
    }
    let root = root.ok_or_else(|| "missing --compiler-root".to_owned())?;
    let node = node.ok_or_else(|| "missing --node".to_owned())?;
    if !root.is_absolute() || !node.is_absolute() {
        return Err("compiler root and Node.js runtime must be absolute paths".to_owned());
    }
    Ok((root, node))
}
