//! Genuine frozen source owners; private trap instrumentation is never an emitted product path.
use super::*;
use zryna_abi::{Invocation, ScalarValue};
mod cleanup;
#[path = "tests/source.rs"]
mod indexed;
mod transport;
fn request() -> Invocation {
    Invocation::new("entry".into(), vec![])
}
#[test]
fn source_request_actual_text_audit_and_ordinary_bytes_stay_bound() {
    for case in ["scalar-i32-clone", "scalar-bool-clone", "nominal-enum-clone"] {
        super::super::tests::with_program(case, "native", |program| {
            let ordinary = super::super::emit(program).expect("ordinary preimage");
            let carrier = emit_invocation(program, request()).expect("same NEW invocation");
            assert!(std::ptr::eq(carrier.source(), program));
            assert_eq!(carrier.invocation().export().logical_name().as_str(), "entry");
            assert_eq!(carrier.entry_export(), "zryna$invoke$v1");
            assert_eq!(carrier.encode_export(), "zryna$encodeInvocation$v1");
            carrier.audit_source(carrier.as_source()).expect("actual complete text");
            for (before, after) in [
                ("$pendingInvocation=null;", "$pendingInvocation=undefined;"),
                ("$failure===$sourceTraps[1]", "$failure.message==='zryna.trap.bounds-v1'"),
                ("$runtimeExit();", "void 0;"),
                ("$result!==$current", "false"),
                ("setUint32(16,", "setUint32(15,"),
            ] {
                let bad = carrier.as_source().replacen(before, after, 1);
                assert_ne!(bad, carrier.as_source());
                assert!(carrier.audit_source(&bad).is_err());
            }
            carrier.audit_source(carrier.as_source()).expect("pristine retry");
            assert_eq!(ordinary, super::super::emit(program).expect("ordinary byte replay"));
            assert_eq!(
                emit_invocation(program, Invocation::new("absent".into(), vec![]))
                    .expect_err("unknown exact source export")
                    .code(),
                "ZRYNA-B2101"
            );
            assert_eq!(
                emit_invocation(
                    program,
                    Invocation::new("entry".into(), vec![ScalarValue::I32(1)])
                )
                .expect_err("exact arity")
                .code(),
                "ZRYNA-B2102"
            );
            let count = carrier.as_source().len();
            assert_eq!(
                build(program, carrier.invocation(), count).expect("inclusive output budget"),
                carrier.as_source()
            );
            assert!(build(program, carrier.invocation(), count - 1).is_err());
        });
    }
}
#[test]
fn trusted_fragments_do_not_search_user_literal_or_formatted_text() {
    let mut w = Writer::output(4096, 4096).expect("bounded writer");
    w.invocation();
    let user = "throw new Error('zryna.trap.bounds-v1')";
    w.format(format_args!("{user}")).expect("untouched user text");
    w.trusted(";throw new Error('zryna.trap.bounds-v1');")
        .expect("explicit static compiler fragment");
    assert_eq!(w.finish(), "throw new Error('zryna.trap.bounds-v1');throw $sourceTraps[1];");
    for (identity, text, code) in [
        (zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Bounds, "zryna.trap.bounds-v1", 1),
        (
            zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Allocation,
            "zryna.trap.allocation-v1",
            2,
        ),
        (
            zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Capacity,
            "zryna.trap.capacity-v1",
            3,
        ),
        (
            zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Refcount,
            "zryna.trap.refcount-v1",
            4,
        ),
        (zryna_ir::generic_v1::execution_v1::raw::TrapIdentity::Utf8, "zryna.trap.utf8-v1", 5),
    ] {
        assert_eq!(traps::identity(text), Some(identity));
        let mut w = Writer::output(4096, 4096).expect("bounded");
        w.invocation();
        w.throws_source(identity).expect("typed identity");
        assert_eq!(w.finish(), format!("throw $sourceTraps[{code}];\n"));
    }
    assert_eq!(traps::identity("forged zryna.trap.bounds-v1"), None);
    let mut work = crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES - 8;
    super::super::writer::template_work(&mut work, 1).expect("exact eight scan units");
    assert!(super::super::writer::template_work(&mut work, 1).is_err());
    assert_eq!(work, crate::prelude::MAX_CONTROL_FLOW_JAVASCRIPT_BYTES);
    assert!(super::super::writer::template_work(&mut work, usize::MAX).is_err());
}
#[test]
fn genuine_scalar_and_bounds_transport_rejects_forgery_stale_and_duplicate_records() {
    super::super::tests::with_program("scalar-i32-clone", "native", |program| {
        let carrier = emit_invocation(program, request()).expect("genuine original scalar");
        transport::execute("scalar", carrier.as_source(), 0, 14, true);
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
            .expect("genuine empty indexed owner");
            transport::execute(
                if vector { "vec-bounds" } else { "array-bounds" },
                carrier.as_source(),
                1,
                0,
                false,
            );
        });
    }
}
