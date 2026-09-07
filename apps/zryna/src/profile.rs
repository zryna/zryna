use std::ffi::OsString;

pub(super) fn selects_typed_scalars(arguments: &[OsString]) -> bool {
    ["control-flow-v1", "data-ownership-v1"].iter().any(|profile| {
        arguments.windows(2).any(|pair| pair[0] == "--profile" && pair[1] == *profile)
            || arguments.iter().any(|argument| argument == format!("--profile={profile}").as_str())
    })
}
