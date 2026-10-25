//! Source/law-derived status, context and actual canonical observations, with freed-safe census.
fn numbers(v: &serde_json::Value) -> Vec<u64> {
    v.as_array().expect("counter vector").iter().map(|n| n.as_u64().expect("u64")).collect()
}
pub(super) fn check(row: &serde_json::Value, actual: &serde_json::Value) {
    assert_eq!(actual["exit_code"], 0);
    assert_eq!(actual["signal"], serde_json::Value::Null);
    assert_eq!(actual["stdout_bytes"], 0);
    assert_eq!(actual["stderr_bytes"], 0);
    assert_eq!(actual["binary_bytes"], 480);
    let fields = numbers(&actual["fields"]);
    assert_eq!(fields.len(), 60);
    let pre = &fields[..30];
    let post = &fields[30..];
    let expected = &row["expected"];
    assert_eq!(pre[0], 0);
    assert_eq!(post[0], 1);
    assert_eq!(&pre[3..7], numbers(&row["pre"]));
    assert_eq!(
        pre[7],
        row["malloc_fault_ordinal"].as_u64().expect("fault pending at zero attempts")
    );
    assert_eq!(post[1], expected["status"].as_u64().expect("actual returned source status"));
    assert_eq!(post[2], expected["out"].as_u64().expect("exact output"));
    assert_eq!(&post[3..7], numbers(&expected["post"]));
    assert_eq!(post[7], expected["fault_pending"].as_u64().expect("deny-before-sideeffect"));
    let summary = &post[8..];
    assert_eq!(summary[0], 0x4d34_3136_4143_4331);
    assert_eq!(summary[2], 1);
    for (slot, name) in [
        (3, "allocate"),
        (4, "strongClone"),
        (5, "weakDowngrade"),
        (6, "weakClone"),
        (7, "weakUpgrade"),
        (8, "strongReleaseBegin"),
        (9, "strongReleaseFinish"),
        (10, "weakRelease"),
    ] {
        assert_eq!(
            summary[slot],
            expected["entries"][name].as_u64().expect("pre-output role census"),
            "{name}"
        );
    }
    for (slot, name) in [
        (11, "attempts"),
        (12, "allocations"),
        (13, "frees"),
        (14, "live"),
        (15, "peak_live"),
        (16, "strong"),
        (17, "explicit_weak"),
        (18, "canonical_status_entries"),
        (19, "zero_outputs"),
        (20, "expired"),
        (21, "allocation_failures"),
    ] {
        assert_eq!(
            summary[slot],
            expected[name].as_u64().expect("frozen physical/count observation"),
            "{name}"
        );
    }
    assert_eq!(
        summary[18],
        summary[3..=10].iter().sum::<u64>(),
        "no internal C factoring as an outer event"
    );
}
