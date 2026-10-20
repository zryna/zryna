//! Source admission shared by ownership closure discovery.

use std::path::Path;

use zryna_source::NormalizedSourcePath;

use super::{
    ModuleClosureError,
    support::{diagnostic, rejected},
};

pub(super) fn validate_entrypoint_extension(
    entrypoint: &NormalizedSourcePath,
) -> Result<(), ModuleClosureError> {
    if Path::new(entrypoint.as_str()).extension().and_then(|value| value.to_str()) != Some("zry") {
        return Err(rejected(diagnostic(
            "ZRYNA-D3301",
            Some(entrypoint),
            "entry module must use the exact lowercase .zry extension",
        )));
    }
    Ok(())
}
