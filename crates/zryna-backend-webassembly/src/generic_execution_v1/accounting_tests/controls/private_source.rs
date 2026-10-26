//! Actual retained source-step reason ABI with constant-event seeded accounting frontier.
use super::{harness::Harness, source, with_model};
use crate::generic_execution_v1::accounting_tests::deadline::Deadline;
use sha2::{Digest, Sha256};
#[test]
fn birth_private_reason_frontiers_and_early_physical_failure() {
    actual("control-birth-call-loop", 4);
}
#[test]
fn live_private_count_denial_keeps_parent_cleanup_funded() {
    actual("control-live-count-call-loop", 2);
}
#[test]
fn expired_private_source_branch_fits_exact_held_frontier() {
    actual("control-expired-call-loop", 2);
}
fn actual(case: &str, count: usize) {
    let bytes = include_bytes!("PRIVATE_SOURCE_EXPECTED_FREEZE.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        "09bc9aac613ce7165aaa7acb214aab8e6296a40459b9d37eac282b92273f52dd"
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(bytes).expect("independent pre-output source reason oracle");
    for provider in source::PROVIDERS {
        with_model(case, provider, |m, wire| {
            let h = Harness::new(m, wire, &format!("source-{case}-{provider}"));
            let _deadline = Deadline::new(h.engine());
            let mut all = Vec::new();
            for row in
                oracle["cases"].as_array().expect("eight").iter().filter(|r| r["case"] == case)
            {
                let mut s = h.session();
                for (name, value) in
                    ["A", "L", "S", "H"].into_iter().zip(row["pre"].as_array().expect("four seeds"))
                {
                    s.set(
                        name,
                        u32::try_from(value.as_u64().expect("representable seed"))
                            .expect("exact u32 transport"),
                    );
                }
                if row["cursor"] == "MEMORY_BYTES-8" {
                    s.set("cursor", crate::generic_execution_v1::memory::MEMORY_BYTES - 8);
                }
                let before_cursor = s.get("cursor");
                s.put(h.out, 0);
                let source = s
                    .instance
                    .get_typed_func::<(i32, i32), i32>(&mut s.store, "sourceStep")
                    .expect("actual retained source ABI");
                let result =
                    source.call(&mut s.store, (h.frame.cast_signed(), h.out.cast_signed()));
                let actual = serde_json::json!({"id":row["id"],"returned_reason":result.as_ref().ok().copied(),"error":result.as_ref().err().map(ToString::to_string),"post":s.state(),"out":s.word(h.out),"reason_global":s.get("reason"),"attempts":s.get("attempts"),"cursor":s.get("cursor"),"before_cursor":before_cursor});
                h.retain(row["id"].as_str().expect("id"), &actual);
                all.push(actual.clone());
                let expected = &row["expected"];
                assert_eq!(actual["returned_reason"], expected["source_reason"]);
                assert_eq!(actual["error"], serde_json::Value::Null);
                assert_eq!(actual["out"], expected["out"]);
                assert_eq!(actual["post"], expected["post"]);
                assert_eq!(actual["attempts"], expected["attempts"]);
                if expected["cursor_unchanged"] == true {
                    assert_eq!(actual["cursor"], actual["before_cursor"]);
                }
                let entry = s
                    .instance
                    .get_typed_func::<i32, i32>(&mut s.store, "entry")
                    .expect("unchanged public wrapper");
                let retry = entry.call(&mut s.store, 3);
                let observed = serde_json::json!({"returned":retry.as_ref().ok().copied(),"error":retry.as_ref().err().map(ToString::to_string),"post":s.state()});
                h.retain(&format!("{}-public-retry", row["id"].as_str().expect("id")), &observed);
                assert_eq!(observed["returned"], expected["public_retry_value"]);
                assert_eq!(observed["error"], serde_json::Value::Null);
                assert_eq!(observed["post"], expected["public_retry_post"]);
            }
            h.retain("all-source-observations", &serde_json::json!(all));
            assert_eq!(all.len(), count);
        });
    }
}
