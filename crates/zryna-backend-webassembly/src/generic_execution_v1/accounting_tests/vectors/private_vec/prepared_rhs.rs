//! Actual retained Clone/drop helpers and genuine prepared-Push source world; no handwritten clone.
use super::*;
#[path = "../../../../../../zryna-driver/tests/generic_execution_v1_native_affine_push/source.rs"]
mod push_source;
#[test]
fn failed_growth_preserves_genuine_prepared_rhs_and_old_vec_until_reverse_outer_cleanup() {
    let oracle = oracle();
    let row = oracle["cases"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["id"] == "prepared-RHS-survives-reserve-failure")
        .expect("unchanged RHS row");
    for provider in push_source::PROVIDERS {
        push_source::with_program(push_source::SETTLED[0], provider, |program, wire| {
            with_model(program, wire, |m, wire| {
                let h = Harness::new(m, wire, &format!("vec-rhs-{provider}"), None);
                let _deadline = super::super::super::deadline::Deadline::new(h.engine.clone());
                let (ty, element) = vector(m);
                let out = resource_records(m);
                let mut s = h.session();
                let left = source_literal(m, b"A");
                let right = source_literal(m, b"B");
                assert_eq!(s.call3("from", (left, 1, out + 24)), 0);
                assert_eq!(s.call3("vecAllocate", (element, 1, out)), 0);
                let old = s.record(usize::try_from(out).expect("old"));
                let mut child = [0; 12];
                s.memory
                    .read(&s.store, usize::try_from(out + 24).expect("child"), &mut child)
                    .expect("child bytes");
                s.memory
                    .write(&mut s.store, usize::try_from(old.0).expect("backing"), &child)
                    .expect("transport child credit");
                s.memory
                    .write(
                        &mut s.store,
                        usize::try_from(out + 4).expect("length"),
                        &1_i32.to_le_bytes(),
                    )
                    .expect("publish original length");
                assert_eq!(s.call3("from", (right, 1, out + 36)), 0);
                let rhs = s.record(usize::try_from(out + 36).expect("RHS"));
                assert_eq!(s.state(), row["pre"]);
                s.set("cursor", 16_777_208);
                s.poison(usize::try_from(out + 12).expect("output"));
                let status = s.call4_named("vecReserve", (element, out, 2, out + 12));
                let post = s.state();
                let old_after = s.record(usize::try_from(out).expect("old"));
                let rhs_after = s.record(usize::try_from(out + 36).expect("RHS"));
                let mut old_child = [0; 12];
                s.memory
                    .read(&s.store, usize::try_from(old.0).expect("old backing"), &mut old_child)
                    .expect("old child");
                h.retain("reserve-failure-before-commit",&serde_json::json!({"status":status,"post":post,"old":old_after,"rhs":rhs_after,"child":old_child,"private_boundary_not_source_trace":true}));
                assert_eq!(status, 1);
                assert_eq!(post, row["expected"]["post"]);
                assert_eq!(old_after, (old.0, 1, old.2));
                assert_eq!(rhs_after, rhs);
                assert_eq!(old_child, child);
                assert_eq!(s.record(usize::try_from(out + 12).expect("out")), (0, 0, 0));
                assert_eq!(s.call3("release", rhs), 0);
                s.drop_at(&format!("vecDrop{ty}"), out);
                let cleaned = s.state();
                h.retain("reverse-outer-cleanup", &cleaned);
                assert_eq!(cleaned, row["expected"]["after_cleanup"]);
            });
        });
    }
}
