//! Independently fixed observations, authored before compiling or executing target code.
pub(super) const SUCCESS: &[&str] = &[
    "alloc:1:earlier-root",
    "alloc:2:left",
    "alloc:3:earlier-root",
    "drop:3:earlier-root",
    "drop:2:left",
    "drop:1:earlier-root",
];
pub(super) const FAILURE_ONE: &[&str] = &["fail:1"];
pub(super) const FAILURE_TWO: &[&str] = &["alloc:1:earlier-root", "fail:2", "drop:1:earlier-root"];
pub(super) const FAILURE_THREE: &[&str] =
    &["alloc:1:earlier-root", "alloc:2:left", "fail:3", "drop:2:left", "drop:1:earlier-root"];
pub(super) const VALUE: i32 = 7;

pub(super) fn check_frozen_authority() {
    let fault: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/m7-generic-struct-v4/fault-oracle.json"
    ))
    .expect("accepted constructor fault oracle");
    assert_eq!(fault["accepted_case"]["outcome"], "zryna.trap.allocation-v1");
    assert_eq!(
        fault["accepted_case"]["trace"],
        serde_json::json!(["release:left", "release:earlier-root"])
    );
    assert_eq!(fault["failure_steps"][2]["cleanup"], serde_json::json!([1, 0]));
    assert_eq!(fault["successful_return"]["cleanup"], serde_json::json!([3, 0]));
}

pub(super) fn events(owned: bool, fault: usize) -> &'static [&'static str] {
    match (owned, fault) {
        (false, 0) => &[],
        (true, 0) => SUCCESS,
        (true, 1) => FAILURE_ONE,
        (true, 2) => FAILURE_TWO,
        (true, 3) => FAILURE_THREE,
        _ => panic!("case outside the independently authored allocation oracle"),
    }
}
