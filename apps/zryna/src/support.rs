//! Static public M4 contract rendering; no execution or grant authority is constructed.

use std::{
    io::{self, Write},
    process::ExitCode,
};

const CONTRACT: &str = include_str!("m4_support.json");

#[derive(Clone, Debug, clap::Args)]
pub(super) struct Options {
    /// Emit the compiled canonical support contract JSON.
    #[arg(long)]
    json: bool,
}

pub(super) fn report(options: &Options) -> ExitCode {
    let mut output = io::stdout().lock();
    let result = if options.json {
        output.write_all(CONTRACT.as_bytes())
    } else {
        output.write_all(
            b"M4 static support contract; execution and publication are not authenticated.\n\
Checkout component/browser, checkout or fresh verified-installation command, and checkout \
status/clock server contracts have bounded host rows.\n\
Installed component/browser/server and existing immutable beta H1 remain excluded.\n\
Use zryna support --json for the complete matrix and registry digest.\n",
        )
    };
    if result.is_ok() { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
