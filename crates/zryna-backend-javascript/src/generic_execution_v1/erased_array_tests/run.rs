use super::{POSITIVE, source};
use std::process::Command;
pub(super) fn execute(observed: bool) {
    for provider in source::PROVIDERS {
        source::with_program(POSITIVE, provider, |program, _| {
            let text = if observed {
                let artifact = super::super::emit_observed(program)
                    .expect("genuine observed erased-parent target");
                assert_eq!(
                    artifact,
                    super::super::emit_observed(program).expect("fresh deterministic artifact")
                );
                artifact.as_source().to_owned()
            } else {
                let artifact =
                    super::super::emit(program).expect("genuine ordinary erased-parent target");
                for forbidden in ["globalThis", "Symbol.for", "import ", "import(", "require("] {
                    assert!(!artifact.source.contains(forbidden));
                }
                artifact.source
            };
            let mode = if observed { "observed" } else { "ordinary" };
            let file = std::env::temp_dir()
                .join(format!("zryna-erased-array-{}-{provider}-{mode}.mjs", std::process::id()));
            std::fs::write(&file, text).expect("actual module");
            let output = Command::new("node")
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("src/generic_execution_v1/erased_array_tests")
                        .join(if observed { "observe.mjs" } else { "ordinary.mjs" }),
                )
                .arg(&file)
                .output();
            std::fs::remove_file(file).expect("remove actual module");
            let output = output.expect("Node target process");
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        });
    }
}
