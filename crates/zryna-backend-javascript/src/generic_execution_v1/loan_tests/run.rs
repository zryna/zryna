use super::{concat_source, source};
use std::process::Command;
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::StorageTarget;

pub(super) fn loan(case: &str, observed: bool, faults: bool) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, _wire| {
            execute(program, case, provider, observed, faults);
        });
    }
}
pub(super) fn concat(case: &str, observed: bool, faults: bool) {
    for provider in ["native", "typescript"] {
        concat_source::with_program(case, provider, |program, _wire| {
            execute(program, case, provider, observed, faults);
        });
    }
}
fn execute(
    program: &VerifiedExecution<'_>,
    case: &str,
    provider: &str,
    observed: bool,
    faults: bool,
) {
    if observed {
        let artifact =
            super::super::emit_observed(program).expect("genuine loan/concat observed emission");
        assert_eq!(
            artifact,
            super::super::emit_observed(program).expect("deterministic fresh emission")
        );
        launch(program, case, provider, artifact.as_source(), true, faults);
    } else {
        let artifact = super::super::emit(program).expect("genuine loan/concat ordinary emission");
        launch(program, case, provider, &artifact.source, false, faults);
    }
}
fn launch(
    program: &VerifiedExecution<'_>,
    case: &str,
    provider: &str,
    text: &str,
    observed: bool,
    faults: bool,
) {
    let mode = if observed { "observed" } else { "ordinary" };
    let stem = format!("zryna-loans-{}-{case}-{provider}-{mode}-{faults}", std::process::id());
    let file = std::env::temp_dir().join(format!("{stem}.mjs"));
    let metadata_file = std::env::temp_dir().join(format!("{stem}.json"));
    std::fs::write(&file, text).expect("actual emitted new-seal module");
    let elements: Vec<_> = program.runtime().element_layouts().iter()
        .filter(|record| record.target() == StorageTarget::Linear32V1)
        .map(|record| serde_json::json!({"id":record.element().index(),"stride":record.stride(),"alignment":record.alignment()})).collect();
    let controls: Vec<_> = program.runtime().control_layouts().iter()
        .filter(|record| record.target() == StorageTarget::Linear32V1)
        .map(|record| serde_json::json!({"size":record.size(),"alignment":record.alignment(),"payload_offset":record.payload_offset()})).collect();
    std::fs::write(
        &metadata_file,
        serde_json::to_vec(&serde_json::json!({"elements":elements,"controls":controls}))
            .expect("actual retained metadata serialization"),
    )
    .expect("same-seal test metadata only");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("node");
    if observed {
        command
            .arg(root.join("src/generic_execution_v1/loan_tests/observe.mjs"))
            .arg(&file)
            .arg(case)
            .arg(&metadata_file)
            .arg(if faults { "faults" } else { "positive" });
    } else {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("RUNTIME_ORACLE.json")).expect("pre-output oracle");
        let result = oracle["cases"]
            .as_array()
            .expect("frozen rows")
            .iter()
            .find(|row| row["case"] == case)
            .expect("unchanged original")["observation"]
            .as_i64()
            .expect("frozen scalar result");
        command
            .arg(root.join("src/generic_execution_v1/ordinary_tests/execute.mjs"))
            .arg(&file)
            .arg(result.to_string());
    }
    let output = command.output();
    std::fs::remove_file(&file).expect("remove test artifact");
    std::fs::remove_file(&metadata_file).expect("remove metadata");
    let output = output.expect("Node loan/concat process");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
