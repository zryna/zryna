//! Genuine first-six sources and independent frozen runtime oracles; no raw execution proof.
use std::process::Command;
use zryna_ir::generic_v1::execution_v1;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1 as semantic_execution, layouts::verify_layouts},
    resolve_declarations,
};
#[path = "tests/source.rs"]
mod source;

#[test]
fn first_frozen_sources_emit_and_execute_only_from_the_new_execution_seal() {
    for case in source::POSITIVE {
        for provider in source::PROVIDERS {
            with_program(case, provider, |program| {
                let artifact = super::emit_observed(program).expect("new execution emission");
                assert_eq!(
                    artifact,
                    super::emit_observed(program).expect("deterministic emission")
                );
                observe(case, provider, &artifact.source);
            });
        }
    }
}

#[test]
fn exact_artifact_budget_and_first_extra_byte_are_checked_on_a_genuine_seal() {
    with_program("scalar-i32-clone", "native", |program| {
        let artifact = super::emit(program).expect("complete artifact");
        assert_eq!(
            artifact,
            super::emit_with_budget(program, artifact.source.len()).expect("inclusive budget")
        );
        assert_eq!(
            super::emit_with_budget(program, artifact.source.len() - 1)
                .expect_err("first-extra byte cannot publish an artifact")
                .code(),
            "ZRYNA-J2003"
        );
    });
}

#[test]
fn unused_opaque_clone_cannot_reach_the_emitter_as_a_new_seal() {
    for provider in source::PROVIDERS {
        let (sources, syntax) = source::project(source::NEGATIVE, provider);
        let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
            execution_v1::check_original_bodies(&syntax, &sources).expect_err("no new authority")
        else {
            panic!("opaque original owning diagnostic")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7002");
        assert_eq!(
            errors[0].primary_span(),
            sources.verify_span(zryna_source::UntrustedSpan { file: 0, start: 65, end: 70 }).ok()
        );
    }
}

pub(super) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&execution_v1::VerifiedExecution<'_>),
) {
    let (sources, syntax) = source::project(case, provider);
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("opaque original bodies");
    let instances = semantic_execution::discover(&bodies).expect("new original ownership");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("declarations"),
        &linear,
        &linux,
    )
    .expect("same-universe runtime");
    let claim = semantic_execution::produce_claim(&instances, &linear, &linux).expect("new claim");
    let wire = execution_v1::wire::encode(&claim).expect("distinct complete carrier");
    let program = execution_v1::verify(
        execution_v1::wire::decode(&wire).expect("execution carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent source/type/state/cleanup/ABI issuer");
    check(&program);
}

fn observe(case: &str, provider: &str, source: &str) {
    let file = std::env::temp_dir()
        .join(format!("zryna-execution-v1-{}-{case}-{provider}.mjs", std::process::id()));
    std::fs::write(&file, source).expect("new artifact");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .arg(root.join("src/generic_execution_v1/tests/observe.mjs"))
        .arg(&file)
        .arg(case)
        .arg(root.join("../../tests/m7-generic-full-profile-first-vertical/SOURCE_EXPECTED.json"))
        .output();
    std::fs::remove_file(&file).expect("remove artifact");
    let output = output.expect("Node observation");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

mod enum_owner;
