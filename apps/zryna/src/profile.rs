use std::ffi::OsString;

use clap::error::ErrorKind;

pub(super) fn select_control_flow(arguments: &[OsString]) -> Result<bool, clap::Error> {
    let data_ownership =
        arguments.windows(2).any(|pair| pair[0] == "--profile" && pair[1] == "data-ownership-v1")
            || arguments.iter().any(|argument| argument == "--profile=data-ownership-v1");
    if data_ownership {
        return Err(clap::Error::raw(
            ErrorKind::InvalidValue,
            "ZRYNA-C3401: data-ownership-v1 is an internal candidate and is not a supported public profile",
        ));
    }
    Ok(arguments.windows(2).any(|pair| pair[0] == "--profile" && pair[1] == "control-flow-v1")
        || arguments.iter().any(|argument| argument == "--profile=control-flow-v1"))
}
