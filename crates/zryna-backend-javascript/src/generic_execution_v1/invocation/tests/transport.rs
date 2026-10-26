//! Existing bounded Node child owner; retain actual artifacts/output before assertions.
use std::{path::Path, process::Command};
pub(super) fn execute(case: &str, source: &str, status: u32, scalar: u32, instrument: bool) {
    let directory =
        std::env::temp_dir().join(format!("m416-js-carrier-{}-{case}", std::process::id()));
    std::fs::create_dir(&directory).expect("new retained evidence directory");
    std::fs::write(directory.join("ordinary.mjs"), source).expect("actual audited carrier");
    if instrument {
        std::fs::write(
            directory.join("private-reentry.mjs"),
            format!("{source}\nexport function privateBusy(value){{$busy=value;}}\n"),
        )
        .expect("test-only private reentry control");
        for code in 1..=5 {
            let changed = source.replacen(
                "try{$value=",
                &format!("try{{throw $sourceTraps[{code}];$value="),
                1,
            );
            assert_ne!(changed, source);
            std::fs::write(directory.join(format!("private-trap-{code}.mjs")), changed)
                .expect("test-only private identity witness");
        }
        let forged = source.replacen(
            "try{$value=",
            "try{throw new Error('zryna.trap.bounds-v1');$value=",
            1,
        );
        assert_ne!(forged, source);
        std::fs::write(directory.join("forged.mjs"), forged).expect("test-only forged host error");
    }
    let mut command = Command::new("node");
    command
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/invocation/tests/transport.mjs"),
        )
        .arg(&directory)
        .arg(status.to_string())
        .arg(scalar.to_string())
        .arg(if instrument { "yes" } else { "no" });
    let result = super::super::super::accounting_tests::process::run(command);
    std::fs::write(directory.join("stdout.txt"), &result.output.stdout).expect("retain stdout");
    std::fs::write(directory.join("stderr.txt"), &result.output.stderr).expect("retain stderr");
    assert!(result.containment.is_none(), "bounded host failure retained at {directory:?}");
    assert!(
        result.output.status.success(),
        "{}; retained {directory:?}",
        String::from_utf8_lossy(&result.output.stderr)
    );
}
