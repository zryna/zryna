//! Genuine scalar-entry new seals must execute without any injected runtime authority.
use std::process::Command;
#[test]
fn ordinary_first_verticals_execute_without_private_runtime_injection() {
    for (case, result) in [
        ("scalar-i32-clone", 14),
        ("scalar-bool-clone", 7),
        ("nominal-enum-clone", 7),
        ("option-nominal-enum-clone", 7),
        ("fixed-array-identity", 18),
    ] {
        for provider in ["native", "typescript"] {
            super::tests::with_program(case, provider, |program| {
                let artifact = super::emit(program).expect("ordinary new-seal emission");
                execute(case, provider, &artifact.source, result);
            });
        }
    }
}
#[test]
fn ordinary_containers_ignore_poisoned_private_injection_and_retry() {
    for (case, result) in [
        ("vec-copy", 14),
        ("vec-owned-prepare", 7),
        ("shared-weak-count", 7),
        ("opaque-count-unused", 7),
    ] {
        for provider in ["native", "typescript"] {
            super::container_tests::source::with_program(case, provider, |program| {
                let artifact = super::emit(program).expect("ordinary whole-seal container");
                execute(case, provider, &artifact.source, result);
            });
        }
    }
}
#[test]
fn ordinary_manifest_and_helper_inventory_have_no_ambient_runtime_or_imports() {
    super::container_tests::source::with_program("shared-weak-count", "native", |program| {
        let artifact = super::emit(program).expect("ordinary same-seal artifact");
        assert!(
            artifact.source.contains("const $runtimeManifest="),
            "ordinary implementation identity is retained"
        );
        for forbidden in [
            "globalThis",
            "Symbol.for",
            "import ",
            "import(",
            "require(",
            "eval(",
            "new Function",
            "WeakRef",
            "FinalizationRegistry",
        ] {
            assert!(
                !artifact.source.contains(forbidden),
                "forbidden ordinary authority {forbidden}"
            );
        }
    });
}
fn execute(case: &str, provider: &str, source: &str, result: i32) {
    let file = std::env::temp_dir()
        .join(format!("zryna-ordinary-{}-{case}-{provider}.mjs", std::process::id()));
    std::fs::write(&file, source).expect("actual ordinary module");
    let output = Command::new("node")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/generic_execution_v1/ordinary_tests/execute.mjs"),
        )
        .arg(&file)
        .arg(result.to_string())
        .output();
    std::fs::remove_file(&file).expect("remove module");
    let output = output.expect("Node ordinary process");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn genuine_container_quota_keeps_count_handles_separate_from_physical_allocations() {
    for (case, allocations, transitions) in [
        ("vec-copy", 2, 4),
        ("vec-owned-prepare", 6, 12),
        ("shared-weak-count", 2, 11),
        ("opaque-count-unused", 0, 0),
    ] {
        super::container_tests::source::with_program(case, "native", |program| {
            let c = super::Context::new(program).expect("actual retained indices");
            let proof =
                super::runtime::ordinary::quota::prove(&c).expect("actual perentry resource proof");
            assert_eq!(proof.allocations, allocations, "frozen language acquisition trace");
            assert_eq!(
                proof.transitions, transitions,
                "acquisitions plus already funded complete cleanup"
            );
        });
    }
}
#[test]
fn ordinary_actual_bytes_reject_changed_identity_status_inventory_body_and_extra_import() {
    super::container_tests::source::with_program("shared-weak-count", "native", |program| {
        let artifact = super::emit(program).expect("ordinary exact artifact");
        for (before, after) in [
            ("helpers:17", "helpers:18"),
            ("mode:'ordinary'", "mode:'observed'"),
            ("status:255", "status:254"),
            ("strongClone:function", "wrongClone:function"),
        ] {
            let bad = artifact.source.replacen(before, after, 1);
            assert_ne!(bad, artifact.source, "independent hostile mutation must occur");
            reject_bytes(program, &bad);
        }
        reject_bytes(program, &format!("{}\nimport 'foreign-runtime';", artifact.source));
    });
}
fn reject_bytes(program: &zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, source: &str) {
    let c = super::Context::new(program).expect("same exact retained authority");
    let mode = super::Mode::Ordinary(
        super::runtime::ordinary::quota::prove(&c).expect("same finite quota proof"),
    );
    assert_eq!(
        super::runtime::ordinary::audit(&c, source, mode)
            .expect_err("actual bytes differ from closed same-program emission")
            .code(),
        "ZRYNA-I7001"
    );
}
