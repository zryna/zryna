//! Actual retained Clone/drop helpers and genuine prepared-Push source world; no handwritten clone.
use super::*;
#[test]
fn actual_vec_string_clone_second_child_failure_rolls_back_completed_prefix_then_original() {
    let oracle = oracle();
    let row = oracle["cases"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["id"] == "clone-prefix-second-string-failure")
        .expect("unchanged prefix row");
    for provider in ["native", "typescript"] {
        crate::generic_execution_v1::tests::container_source::with_program_wire(
            "vec-owned-prepare",
            provider,
            |program, wire| {
                with_model(program, wire, |m, wire| {
                    let h = Harness::new(m, wire, &format!("vec-prefix-{provider}"), None);
                    let _deadline = super::super::super::deadline::Deadline::new(h.engine.clone());
                    let (ty, element) = vector(m);
                    let out = resource_records(m);
                    let mut s = h.session();
                    let left = source_literal(m, b"A");
                    let right = source_literal(m, b"B");
                    assert_eq!(s.call3("from", (left, 1, out + 24)), 0);
                    assert_eq!(s.call3("from", (right, 1, out + 36)), 0);
                    assert_eq!(s.call3("vecAllocate", (element, 2, out)), 0);
                    let original = s.record(usize::try_from(out).expect("old vector"));
                    let stride = usize::try_from(m.element(ty).expect("element metadata").stride())
                        .expect("stride");
                    for (index, address) in [(0, out + 24), (1, out + 36)] {
                        let mut bytes = [0; 12];
                        s.memory
                            .read(
                                &s.store,
                                usize::try_from(address).expect("child record"),
                                &mut bytes,
                            )
                            .expect("child bytes");
                        s.memory
                            .write(
                                &mut s.store,
                                usize::try_from(original.0).expect("backing") + index * stride,
                                &bytes,
                            )
                            .expect("transport existing child debt");
                    }
                    s.memory
                        .write(
                            &mut s.store,
                            usize::try_from(out + 4).expect("length"),
                            &2_i32.to_le_bytes(),
                        )
                        .expect("publish initialized prefix");
                    assert_eq!(s.state(), row["pre"]);
                    let storage_round = (2 * stride + 16 + 7) & !7;
                    let available = storage_round + 24;
                    s.set(
                        "cursor",
                        u32::try_from(16_777_216 - available)
                            .expect("exact finite available bytes"),
                    );
                    s.poison(usize::try_from(out + 12).expect("destination"));
                    let status = s.call2(&format!("vecClone{ty}"), (out, out + 12));
                    let post = s.state();
                    let output = s.record(usize::try_from(out + 12).expect("output"));
                    // Frozen row.post is before mandatory local-prefix cleanup. The actual helper has already done it.
                    let after_local = serde_json::json!({"A":6,"L":3,"S":8,"H":3});
                    h.retain("second-child-failure",&serde_json::json!({"status":status,"after_local_cleanup":post,"law_derived_after_local":after_local,"frozen_pre_local_checkpoint":row["expected"]["post"],"out":output,"cleanup_order_authority":"independent decoded retained Clone/drop roles; no runtime trace claim"}));
                    assert_eq!(status, 1);
                    assert_eq!(post, after_local);
                    assert_eq!(output, (0, 0, 0));
                    assert_eq!(
                        s.record(usize::try_from(out).expect("old")),
                        (original.0, 2, original.2)
                    );
                    s.drop_at(&format!("vecDrop{ty}"), out);
                    let cleaned = s.state();
                    h.retain("after-original-cleanup", &cleaned);
                    assert_eq!(cleaned, row["expected"]["after_cleanup"]);
                });
            },
        );
    }
}
