//! Genuine source-owned transport controls; no provider, compiler or guest run by the author.
use crate::generic_execution_v1::{
    self as native,
    invocation::{self, plan::Plan},
};
use zryna_abi::{Invocation, ScalarType, ScalarValue};
use zryna_native_mir::generic_execution_v1::{MAX_CODEGEN_UNITS, VerifiedProgram, lower};
mod graphs;
fn request(p: &VerifiedProgram<'_, '_>) -> Invocation {
    let export = p.program().scalar_abi().exports().next().expect("genuine scalar export");
    Invocation::new(
        export.logical_name().as_str().to_owned(),
        export
            .parameters()
            .iter()
            .map(|ty| match ty {
                ScalarType::Bool => ScalarValue::Bool(false),
                ScalarType::I32 => ScalarValue::I32(0),
            })
            .collect(),
    )
}
fn with_program(
    case: &str,
    provider: &str,
    inspect: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>, &[u8]),
) {
    match case {
        "nested-temporaries" => super::concat_source::with_program(case, provider, inspect),
        "reserve-vec-call-loop" => super::vector_source::with_program(case, provider, inspect),
        "control-live-count-call-loop" => {
            super::control_source::with_program(case, provider, inspect);
        }
        _ => panic!("fixed genuine source catalogue"),
    }
}
#[test]
fn invocation_object_retains_exact_owner_and_ordinary_object_bytes() {
    for provider in ["native", "typescript"] {
        for case in ["nested-temporaries", "reserve-vec-call-loop", "control-live-count-call-loop"]
        {
            with_program(case, provider, |p, _| {
                let mir = lower(p).expect("actual MIR");
                let target = crate::select_object_target(crate::NATIVE_OBJECT_TARGET)
                    .expect("exact supported target");
                let ordinary = native::emit_object(&mir, target).expect("ordinary baseline");
                let artifact = native::emit_invocation(&mir, request(&mir), target)
                    .expect("typed invocation object");
                assert!(std::ptr::eq(artifact.source(), p));
                assert!(std::ptr::eq(artifact.mir(), &raw const mir));
                assert_eq!(artifact.entry_symbol(), invocation::ENTRY_SYMBOL);
                artifact.audit_bytes(artifact.bytes()).expect("same immutable staged bytes");
                let mut changed = artifact.bytes().to_vec();
                changed[0] ^= 1;
                assert!(artifact.audit_bytes(&changed).is_err());
                assert_eq!(
                    native::emit_object(&mir, target).expect("ordinary retry").bytes(),
                    ordinary.bytes()
                );
                assert!(artifact.codegen_work_units() > mir.codegen_work_units());
            });
        }
    }
}
#[test]
fn real_invocation_plan_exact_first_extra_overflow_and_request_rejection() {
    with_program("nested-temporaries", "native", |p, _| {
        let mir = lower(p).expect("actual MIR");
        let baseline = Plan::derive(&mir, request(&mir)).expect("owning plan");
        let addition = baseline.work - mir.codegen_work_units();
        assert!(addition > 0);
        let exact = Plan::from_work(&mir, request(&mir), MAX_CODEGEN_UNITS - addition)
            .expect("exact remaining units");
        assert_eq!(exact.work, MAX_CODEGEN_UNITS);
        assert_eq!(
            Plan::from_work(&mir, request(&mir), MAX_CODEGEN_UNITS - addition + 1)
                .err()
                .expect("first extra")
                .code,
            "ZRYNA-N7001"
        );
        assert_eq!(
            Plan::from_work(&mir, request(&mir), u64::MAX).err().expect("overflow").code,
            "ZRYNA-N7001"
        );
        assert_eq!(Plan::derive(&mir, request(&mir)).expect("pristine retry").work, baseline.work);
        assert!(Plan::derive(&mir, Invocation::new("absent".to_owned(), vec![])).is_err());
        let name = baseline.request.export().logical_name().as_str().to_owned();
        assert!(Plan::derive(&mir, Invocation::new(name, vec![ScalarValue::Bool(true)])).is_err());
    });
}
#[test]
fn all_five_source_codes_have_genuine_failure_recipe_mutation_and_retry_controls() {
    let mut codes = std::collections::BTreeSet::new();
    for provider in ["native", "typescript"] {
        for case in ["nested-temporaries", "reserve-vec-call-loop", "control-live-count-call-loop"]
        {
            with_program(case, provider, |p, _| {
                let mir = lower(p).expect("actual MIR");
                let plan = Plan::derive(&mir, request(&mir)).expect("actual plan");
                graphs::check(&mir, &plan, &mut codes);
                invocation::wrapper::tests::check(&mir, &plan);
            });
        }
    }
    assert_eq!(codes, std::collections::BTreeSet::from([1, 2, 3, 4, 5]));
}

#[test]
fn signed_i32_requests_retain_exact_bits_and_reject_noncanonical_immediates() {
    for provider in ["native", "typescript"] {
        with_program("reserve-vec-call-loop", provider, |p, _| {
            let mir = lower(p).expect("genuine original i32-parameter MIR");
            let target = crate::select_object_target(crate::NATIVE_OBJECT_TARGET)
                .expect("exact supported object target");
            let ordinary = native::emit_object(&mir, target).expect("ordinary byte baseline");
            for value in [-1, i32::MIN, -17, 0, 1, i32::MAX] {
                let request = || Invocation::new("entry".into(), vec![ScalarValue::I32(value)]);
                let artifact = native::emit_invocation(&mir, request(), target)
                    .expect("genuine signed request has a canonical32bit wrapper");
                assert_eq!(artifact.invocation().arguments(), &[ScalarValue::I32(value)]);
                artifact.audit_bytes(artifact.bytes()).expect("actual typed object audit");
                let plan = Plan::derive(&mir, request()).expect("same source/request plan");
                invocation::wrapper::tests::check(&mir, &plan);
            }
            assert_eq!(
                ordinary,
                native::emit_object(&mir, target).expect("ordinary replay after signed requests")
            );
        });
    }
}
