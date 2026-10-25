//! Actual private canonical status, rollback and separate-range concat controls.
use super::*;
#[test]
fn canonical_capacity_status_is_admitted_separately_from_source_denial_and_corrupt_i32_is_atomic() {
    with_empty(|m, wire| {
        let h = Harness::new(m, wire, "canonical-capacity", None);
        let _deadline = super::super::deadline::Deadline::new(h.engine.clone());
        let out = i32::try_from(m.memory.claims().scratch_start).expect("scratch");
        let mut s = h.session();
        s.poison(usize::try_from(out).expect("out"));
        let status = s.call3("from", (0, i32::MIN, out));
        let post = s.state();
        h.retain("known-canonical-size",&serde_json::json!({"status":status,"post":post,"out":s.record(usize::try_from(out).expect("out")),"synthetic_boundary_not_source_reachability":true}));
        assert_eq!(status, 2);
        assert_eq!(post, serde_json::json!({"A":0,"L":0,"S":1,"H":0}));
        assert_eq!(s.record(usize::try_from(out).expect("out")), (0, 0, 0));
        let mut bad = h.session();
        bad.set("S", u32::MAX);
        bad.set("H", 1);
        let before = bad.state();
        let status = bad.call3("from", (0, 0, out));
        h.retain(
            "corrupt-i32",
            &serde_json::json!({"status":status,"before":before,"post":bad.state()}),
        );
        assert_eq!(status, 255);
        assert_eq!(bad.state(), before);
    });
}
#[test]
fn impossible_clone_status_rolls_back_new_credit_and_keeps_the_old_empty_owner() {
    with_case("retained-clone-call-loop", |m, wire| {
        let h = Harness::new(m, wire, "impossible-clone-status", Some((2, 4)));
        let _deadline = super::super::deadline::Deadline::new(h.engine.clone());
        let out = i32::try_from(m.memory.claims().scratch_start).expect("scratch");
        let mut s = h.session();
        assert_eq!(s.call3("from", (0, 0, out + 16)), 0);
        s.poison(usize::try_from(out).expect("out"));
        let status = s.call4((0, 0, 0, out));
        let post = s.state();
        h.retain("status4-invalid-for-clone",&serde_json::json!({"status":status,"post":post,"out":s.record(usize::try_from(out).expect("out")),"shim_untrusted":true}));
        assert_eq!(status, 255);
        assert_eq!(post, serde_json::json!({"A":0,"L":0,"S":2,"H":1}));
        assert_eq!(s.record(usize::try_from(out).expect("out")), (0, 0, 0));
        assert_eq!(s.call3("release", (0, 0, 0)), 0);
        assert_eq!(s.state(), serde_json::json!({"A":0,"L":0,"S":3,"H":0}));
    });
}
#[test]
fn actual_concat_adapter_preserves_separate_inputs_and_one_outer_acquisition() {
    crate::generic_execution_v1::tests::concat_program("same-source", |program, wire| {
        let memory = memory::verify(memory::derive(program, false).expect("claim"), program)
            .expect("memory");
        let m = Model::new(program, &memory).expect("source model");
        assert!(m.accounted() && m.concat);
        let h = Harness::new(&m, wire, "concat", None);
        let _deadline = super::super::deadline::Deadline::new(h.engine.clone());
        let out = i32::try_from(m.memory.claims().scratch_start).expect("scratch");
        let mut s = h.session();
        s.memory
            .write(&mut s.store, usize::try_from(out + 31).expect("input"), b"A")
            .expect("private input");
        assert_eq!(s.call3("from", (out + 31, 1, out + 16)), 0);
        let original = s.record(usize::try_from(out + 16).expect("record"));
        let status = s.call3("concat", (out + 16, out + 16, out));
        let copy = s.record(usize::try_from(out).expect("output"));
        let post = s.state();
        let mut bytes = [0; 2];
        s.memory
            .read(&s.store, usize::try_from(copy.0).expect("copy pointer"), &mut bytes)
            .expect("actual bytes");
        h.retain("concat",&serde_json::json!({"status":status,"post":post,"original":original,"copy":copy,"bytes":bytes}));
        assert_eq!(status, 0);
        assert_eq!(bytes, *b"AA");
        assert_eq!(s.record(usize::try_from(out + 16).expect("input record")), original);
        assert_eq!(post, serde_json::json!({"A":2,"L":2,"S":2,"H":2}));
        assert_eq!(s.call3("release", copy), 0);
        assert_eq!(s.call3("release", original), 0);
        assert_eq!(s.state(), serde_json::json!({"A":2,"L":0,"S":4,"H":0}));
    });
}
