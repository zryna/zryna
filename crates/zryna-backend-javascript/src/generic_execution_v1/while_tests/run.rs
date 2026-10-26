use super::source;
use std::process::Command;

pub(super) fn case(case: &str, observed: bool, faults: bool) {
    for provider in source::PROVIDERS {
        source::with_program(case, provider, |program, wire| {
            assert!(wire.starts_with(zryna_ir::generic_v1::execution_v1::wire::HEADER));
            let text =
                if observed {
                    let actual = super::super::emit_observed(program)
                        .expect("same NEW seal observed target");
                    assert_eq!(
                        actual,
                        super::super::emit_observed(program).expect("fresh deterministic emission")
                    );
                    actual.as_source().to_owned()
                } else {
                    let actual = super::super::emit(program)
                        .expect("same NEW seal ordinary target accepts cycles");
                    assert_eq!(
                        actual,
                        super::super::emit(program).expect("fresh deterministic emission")
                    );
                    assert!(actual.source.contains("const $runtimeManifest="));
                    assert!(actual.source.contains(
                        "987fa645b5f81a9c6fd03f7edcbd23203e7cc61e8aa206b1b08a5ef61d3b1421"
                    ));
                    for forbidden in [
                        "globalThis",
                        "Symbol.for",
                        "import ",
                        "import(",
                        "require(",
                        "eval(",
                        "new Function",
                    ] {
                        assert!(
                            !actual.source.contains(forbidden),
                            "ordinary runtime authority {forbidden}"
                        );
                    }
                    actual.source
                };
            launch(case, provider, &text, observed, faults);
        });
    }
}
fn launch(case: &str, provider: &str, text: &str, observed: bool, faults: bool) {
    let mode = if observed { "observed" } else { "ordinary" };
    let file = std::env::temp_dir()
        .join(format!("zryna-while-{}-{case}-{provider}-{mode}-{faults}.mjs", std::process::id()));
    std::fs::write(&file, text).expect("actual module bytes");
    let script = if observed { "observe.mjs" } else { "ordinary.mjs" };
    let output = Command::new("node")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/while_tests")
                .join(script),
        )
        .arg(&file)
        .arg(case)
        .arg(if faults { "faults" } else { "positive" })
        .output();
    std::fs::remove_file(file).expect("remove target artifact");
    let output = output.expect("actual Node process");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
