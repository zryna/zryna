//! Actual complete bytes are checked without trusting the encoder's body table.
use super::{harness::transport, with_model};
use crate::generic_execution_v1::{
    artifact, code,
    model::{Model, containers as C},
};
mod mutations;
#[test]
fn full_decoded_control_guards_routes_refunds_and_prefree_predicates_reject_mutation() {
    with_model("control-live-count-call-loop", "native", |m, wire| {
        assert!(m.accounted(), "same-source eligibility owns every control effect");
        let code = code::build(m).expect("actual code");
        let bytes = artifact::encode(m, &code).expect("actual ordinary module");
        artifact::audit(&bytes, m, &code).expect("pristine full audit");
        artifact::independent_audit(&bytes, m).expect("pristine decoded authority");
        let directory =
            std::env::temp_dir().join(format!("m416-wasm-control-hostiles-{}", std::process::id()));
        std::fs::create_dir(&directory).expect("new evidence");
        std::fs::write(directory.join("original.wasm"), &bytes).expect("pristine bytes");
        std::fs::write(directory.join("program.zir"), wire).expect("genuine source proof input");
        let mut receipts = Vec::new();
        for kind in 0..15 {
            let target = mutations::target(m, kind);
            let wrong = transport::body_replacement(&bytes, target, |body| {
                mutations::change(body, kind, m)
            });
            wasmparser::Validator::new()
                .validate_all(&wrong)
                .expect("actual hostile remains a valid typed Wasm module");
            std::fs::write(directory.join(format!("hostile-{kind}.wasm")), &wrong)
                .expect("retain actual bytes");
            let result = artifact::independent_audit(&wrong, m);
            receipts.push(serde_json::json!({"kind":kind,"target":target,"error":result.as_ref().err().map(ToString::to_string)}));
            std::fs::write(
                directory.join("actual-before-checks.json"),
                serde_json::to_vec_pretty(&receipts).expect("receipt"),
            )
            .expect("retain before assertion");
            assert!(
                result.is_err(),
                "changed complete artifact {kind} cannot inherit genuine authority"
            );
        }
    });
}
#[test]
fn control_code_writer_exact_first_extra_overflow_and_retry_are_owning() {
    with_model("control-live-count-call-loop", "native", |m, _| {
        assert!(m.accounted(), "resource owner includes appended control effects");
        let start = m.writer.get();
        let work = m.work.get();
        let expected = code::build(m).expect("actual owned complete code");
        let cost = m.writer.get().checked_sub(start).expect("nonnegative actual writer cost");
        assert!(cost > 0);
        m.writer.set(crate::generic_execution_v1::MAX_BYTES - cost);
        m.work.set(work);
        code::build(m).expect("exact actual writer frontier");
        assert_eq!(m.writer.get(), crate::generic_execution_v1::MAX_BYTES);
        m.writer.set(crate::generic_execution_v1::MAX_BYTES - cost + 1);
        m.work.set(work);
        assert!(code::build(m).is_err(), "first extra rejected");
        m.writer.set(usize::MAX);
        m.work.set(work);
        assert!(code::build(m).is_err(), "writer arithmetic overflow rejects");
        m.writer.set(start);
        m.work.set(work);
        let retry = code::build(m).expect("same-source retry after failed attempts");
        assert_eq!(expected.len(), retry.len());
        for (before, after) in expected.iter().zip(&retry) {
            assert_eq!(
                before.body.bytes, after.body.bytes,
                "retry preserves exact source-bound instructions"
            );
        }
    });
}
