//! Opt-in retention of the already checked compiler-only report; no execution grant.
use super::*;

pub(super) fn retain_if_requested(
    parent: &Path,
    report: &serde_json::Value,
) -> Result<(), Diagnostic> {
    match std::env::var_os("ZRYNA_RUST_ORDINARY_PREFIX_CAPTURE") {
        None => return Ok(()),
        Some(value) if value == std::ffi::OsStr::new("1") => {}
        Some(_) => return Err(rejected()),
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|_| rejected())?;
    if bytes.len() > 1024 * 1024 {
        return Err(rejected());
    }
    let held = export::Directory::open(parent)?;
    held.write("compiler-prefix-report.json", &bytes)?;
    let sha256 = digest(&bytes);
    if held.hash("compiler-prefix-report.json", 1024 * 1024)? != sha256 {
        return Err(rejected());
    }
    println!(
        "[zryna-m5-prefix-report] {}",
        serde_json::json!({"path":"compiler-prefix-report.json","bytes":bytes.len(),"sha256":sha256})
    );
    Ok(())
}
