//! Portable current-byte mixed recursive VM proof; observed evidence is String-only.
use super::source;
use crate::generic_execution_v1::{self as execution, memory};
use std::collections::BTreeSet;
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, VerifiedExecution, VerifiedOperation};
use zryna_layout::StorageTarget;
mod projection;
mod session;
fn catalogue(program: &VerifiedExecution<'_>, oracle: &serde_json::Value) {
    let kinds = program
        .operations()
        .filter_map(VerifiedOperation::clone_frontier)
        .map(|frontier| match frontier.frontier().kind() {
            CloneNodeKind::Copy => "Copy",
            CloneNodeKind::String => "String",
            CloneNodeKind::Struct => "Struct",
            CloneNodeKind::Enum => "Enum",
            CloneNodeKind::FixedArray { .. } => "FixedArray",
            CloneNodeKind::VecEach => "VecEach",
            CloneNodeKind::SharedCount => "SharedCount",
            CloneNodeKind::WeakCount => "WeakCount",
        })
        .collect::<BTreeSet<_>>();
    let expected = oracle["required_Clone_kinds"]
        .as_array()
        .expect("eight source kinds")
        .iter()
        .map(|kind| kind.as_str().expect("named kind"))
        .collect::<BTreeSet<_>>();
    assert_eq!(kinds, expected);
    assert_eq!(kinds.len(), 8);
    let categories = program
        .layouts(StorageTarget::Linear32V1)
        .types()
        .map(|ty| format!("{:?}", ty.category()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        categories,
        ["Bool", "I32", "String", "Struct", "Enum", "FixedArray", "Vec", "Shared", "Weak"]
            .map(str::to_owned)
            .into_iter()
            .collect()
    );
    for observed in [false, true] {
        let claim = memory::derive(program, observed).expect("actual mixed recursive memory");
        let plan = memory::verify(claim, program).expect("same-owner checked memory");
        assert!(plan.is_bound_to(program));
        assert!(plan.recursive_arena().is_some(), "catalogue must use recursive VM");
    }
}
fn pristine(session: &mut session::Session, oracle: &serde_json::Value) {
    assert_eq!(session.get(0x2000_0001), 0, "prior ownership/busy state empty");
    assert_eq!(session.entry().expect("pristine return; host fuel exhaustion fails test"), 97);
    projection::check(session, oracle, &oracle["pristine"], None);
    assert_eq!(session.get(0x2000_0001), 0, "all counted storage cleaned");
}
#[test]
fn portable_mixed_catalogue_faults_clean_and_retry_on_same_wasmtime_instances() {
    let oracle = source::mixed_oracle();
    assert_eq!(oracle["ordinary_return_i32"], 97);
    assert_eq!(oracle["StringClone_faults"].as_array().expect("Clone rows").len(), 9);
    assert_eq!(oracle["Allocate_faults"].as_array().expect("Allocate rows").len(), 26);
    source::with_mixed(|program| {
        catalogue(program, &oracle);
        let ordinary = execution::emit(program).expect("audited ordinary same-program bytes");
        let observed =
            execution::emit_observed(program).expect("audited observed same-program bytes");
        let mut session = session::Session::new(ordinary.bytes(), observed.as_bytes());
        for _ in 0..2 {
            assert_eq!(session.ordinary().expect("ordinary return; engine failure fails test"), 97);
        }
        pristine(&mut session, &oracle);
        for fault in oracle["StringClone_faults"]
            .as_array()
            .expect("Clone rows")
            .iter()
            .chain(oracle["Allocate_faults"].as_array().expect("Allocate rows"))
        {
            let argument = u32::from_str_radix(
                fault["configured_wasm_getter_argument"]
                    .as_str()
                    .expect("frozen hex argument")
                    .strip_prefix("0x")
                    .expect("hex prefix"),
                16,
            )
            .expect("u32 argument");
            assert_eq!(session.get(argument), 0, "clean fault configuration gate");
            let failure = session.entry().expect_err("controlled source failure has no scalar");
            assert_eq!(
                failure.downcast_ref::<wasmtime::Trap>(),
                Some(&wasmtime::Trap::UnreachableCodeReached),
                "host OutOfFuel/interruption/other trap is TEST FAILURE, never source ALLOCATION"
            );
            projection::check(&mut session, &oracle, fault, Some(fault));
            assert_eq!(session.get(0x2000_0001), 0, "fault cleanup releases counted storage");
            pristine(&mut session, &oracle);
        }
        session.funded();
    });
}
