//! Genuine source façade and bounded process owner for private backend units.
//! Complete ordinary String runtime frontiers live in the driver's M7 integration target.
#[path = "../../../zryna-driver/tests/generic_execution_v1_native_accounting_string/source.rs"]
mod source;
use std::process::Command;
pub(super) mod process;
// Private test façade reuses this single authenticated helper module and bounded process owner.
pub(super) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    source::with_program(case, provider, check);
}
pub(super) fn private_boundary(group: &str, text: &str) -> serde_json::Value {
    let file = std::env::temp_dir()
        .join(format!("zryna-private-accounting-{}-{group}.mjs", std::process::id()));
    std::fs::write(&file, text).expect("actual emitted private ledger and frozen seeded controls");
    let mut command = Command::new("node");
    command.arg(&file).arg(group);
    let outcome = process::run(command);
    let output = outcome.output;
    std::fs::write(file.with_extension("stdout"), &output.stdout).expect("retain private output");
    std::fs::write(file.with_extension("stderr"), &output.stderr)
        .expect("retain private diagnostics");
    let receipt = serde_json::json!({"group":group,"module":file,"status":output.status.to_string(),"exit_code":output.status.code(),"containment_failure":outcome.containment,"seeded_private_evidence":true});
    std::fs::write(
        file.with_extension("status.json"),
        serde_json::to_vec_pretty(&receipt).expect("private status receipt"),
    )
    .expect("retain before assertions");
    assert!(outcome.containment.is_none(), "private fixture containment, retained {file:?}");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).expect("private seeded evidence summary")
}
