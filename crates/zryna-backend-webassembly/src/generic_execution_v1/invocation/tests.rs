//! Actual immutable sources and original NEW owners; no fabricated execution authority.
use super::*;
use wasmparser::{Operator, Parser, Payload};
use zryna_abi::{Invocation, ScalarType, ScalarValue};
use zryna_ir::generic_v1::execution_v1::invocation::{Outcome, decode_packed};
mod hostile;
#[path = "tests/source.rs"]
mod indexed;
fn request() -> Invocation {
    Invocation::new("entry".into(), vec![])
}
#[test]
fn same_source_invocation_audits_actual_sections_and_preserves_ordinary_bytes() {
    for case in ["scalar-i32-clone", "scalar-bool-clone", "nominal-enum-clone"] {
        super::super::tests::source::program(case, "native", |program| {
            let ordinary = super::super::emit(program).expect("ordinary before carrier");
            let carrier = emit_invocation(program, request()).expect("same NEW carrier");
            assert!(std::ptr::eq(carrier.source(), program));
            assert_eq!(carrier.invocation().export().logical_name().as_str(), "entry");
            assert_eq!(carrier.entry_export(), "zryna$invoke$v1");
            carrier.audit_bytes(carrier.as_bytes()).expect("actual ordinary+carrier audit");
            assert_eq!(
                ordinary.bytes(),
                super::super::emit(program).expect("ordinary replay").bytes()
            );
            assert_eq!(
                emit_invocation(program, Invocation::new("absent".into(), vec![]))
                    .expect_err("same source export")
                    .code(),
                "ZRYNA-B2101"
            );
            assert_eq!(
                emit_invocation(
                    program,
                    Invocation::new("entry".into(), vec![ScalarValue::I32(1)])
                )
                .expect_err("same arity")
                .code(),
                "ZRYNA-B2102"
            );
            hostile::check(&carrier);
        });
    }
}
#[test]
fn actual_private_wrapper_returns_scalar_bits_and_genuine_bounds_only_after_cleanup() {
    super::super::tests::source::program("scalar-i32-clone", "native", |program| {
        let carrier = emit_invocation(program, request()).expect("genuine scalar");
        runtime(&carrier, Outcome::Value(ScalarValue::I32(14)));
    });
    for vector in [false, true] {
        indexed::with_program("zero-length-bounds", "native", |program, _, _| {
            let carrier = emit_invocation(
                program,
                Invocation::new(
                    "entry".into(),
                    vec![ScalarValue::I32(0), ScalarValue::Bool(vector)],
                ),
            )
            .expect("genuine zero-length bounds");
            runtime(
                &carrier,
                Outcome::Trap(zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Bounds),
            );
        });
    }
}
fn runtime(carrier: &ValidatedInvocationArtifact<'_, '_>, expected: Outcome) {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).expect("host engine");
    let module =
        wasmtime::Module::new(&engine, carrier.as_bytes()).expect("audited actual carrier bytes");
    assert_eq!(module.imports().len(), 0);
    assert_eq!(
        module.exports().map(|e| e.name()).collect::<Vec<_>>(),
        vec![carrier.entry_export()]
    );
    let mut store = wasmtime::Store::new(&engine, ());
    store.set_fuel(10_000_000).expect("single finite test-only fuel bank");
    let instance =
        wasmtime::Instance::new(&mut store, &module, &[]).expect("defined memory no imports");
    let invoke = instance
        .get_typed_func::<(), i64>(&mut store, carrier.entry_export())
        .expect("exact zeroarg wide signature");
    for _ in 0..2 {
        let result =
            invoke.call(&mut store, ()).expect("normal host completion required before decoding");
        assert_eq!(decode_packed(result.cast_unsigned(), ScalarType::I32), Ok(expected));
    }
}
#[test]
fn carrier_work_precharge_checked_arithmetic_keeps_exact_and_overflow_boundaries() {
    assert_eq!(extra_work(0).expect("fixed wrapper"), 4096);
    assert_eq!(extra_work(1).expect("one verified argument"), 4224);
    assert!(extra_work(usize::MAX).is_err());
    let mut spent = super::super::MAX_BYTES - 4096;
    super::super::work::take(&mut spent, extra_work(0).expect("derived prepayment"))
        .expect("exact old work bound");
    assert!(super::super::work::take(&mut spent, 1).is_err());
    assert_eq!(spent, super::super::MAX_BYTES);
}

mod controlled;
