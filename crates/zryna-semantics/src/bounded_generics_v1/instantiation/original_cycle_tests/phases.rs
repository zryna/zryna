//! Shared-prefix precedence: real source arguments first, original ownership before pending work.
use super::{MOVE, NEST, NODE, observed, one, plain_errors, v4_errors, with_bodies};
use std::fmt::Write as _;

const INVALID_ARGUMENT: &str = "interface Bad extends ZrynaStruct {loan:Borrow<i32>;} interface Box<T extends ZrynaValue> extends ZrynaStruct {value:T;} function read(value:Box<Bad>):i32 {return 0;}";
const RECUR: &str = "function recur<U extends ZrynaValue>(value:U):U {return recur<U>(value);}";

#[test]
fn actual_supplied_nonvalue_argument_precedes_definition_and_unrelated_move_errors() {
    for definition in [NODE, NEST] {
        let source = format!("{definition} {INVALID_ARGUMENT} {MOVE}");
        let start = u32::try_from(source.find("Box<Bad>").expect("actual supplied type") + 4)
            .expect("span");
        with_bodies(&source, |bodies| {
            one(&plain_errors(bodies), "ZRYNA-M7001", start, start + 3);
            one(&v4_errors(bodies), "ZRYNA-M7001", start, start + 3);
        });
    }
}

#[test]
fn source_calls_and_definitions_share_one_canonical_collector_before_ownership() {
    for (definition, code, head) in
        [(NODE, "ZRYNA-L3002", "Node<T>"), (NEST, "ZRYNA-M7003", "Nest<Vec<T>>")]
    {
        for reversed in [false, true] {
            let source = if reversed {
                format!("{RECUR} {definition} {MOVE}")
            } else {
                format!("{definition} {RECUR} {MOVE}")
            };
            let definition_start =
                u32::try_from(source.rfind(head).expect("original member head")).expect("span");
            let call_start = u32::try_from(source.find("recur<U>").expect("closing original call"))
                .expect("span");
            let mut expected = vec![
                (
                    code.to_owned(),
                    definition_start,
                    definition_start + u32::try_from(head.len()).expect("head"),
                ),
                ("ZRYNA-M7003".into(), call_start, call_start + 8),
            ];
            expected.sort_by_key(|(_, start, end)| (*start, *end));
            with_bodies(&source, |bodies| {
                let plain = plain_errors(bodies);
                assert_eq!(observed(&plain), expected);
                assert_eq!(observed(&v4_errors(bodies)), expected);
                let recursion = plain
                    .iter()
                    .find(|error| error.message.starts_with("source-level function recursion"))
                    .expect("original call diagnostic");
                assert_eq!(
                    recursion.message,
                    "source-level function recursion is excluded before specialization; original declaration path [0, 0, 0, 0]"
                );
            });
        }
    }
}

#[test]
fn source_call_errors_keep_existing_priority_over_supplied_arguments() {
    let source = format!("{NODE} {RECUR} {INVALID_ARGUMENT} {MOVE}");
    let call = u32::try_from(source.find("recur<U>").expect("closing call")).expect("span");
    let expected = vec![("ZRYNA-L3002".into(), 65, 72), ("ZRYNA-M7003".into(), call, call + 8)];
    with_bodies(&source, |bodies| {
        assert_eq!(observed(&plain_errors(bodies)), expected);
        assert_eq!(observed(&v4_errors(bodies)), expected);
    });
}

#[test]
fn original_definition_rejections_precede_unrelated_opaque_move_misuse() {
    for (definition, code, start, end) in
        [(NODE, "ZRYNA-L3002", 65, 72), (NEST, "ZRYNA-M7003", 69, 81)]
    {
        with_bodies(&format!("{definition} {MOVE}"), |bodies| {
            one(&v4_errors(bodies), code, start, end);
        });
    }
}

fn pending_fanout() -> String {
    let mut source = String::new();
    for index in 0..14 {
        if index == 13 {
            write!(&mut source, "function f13<T extends ZrynaValue>():i32 {{return 7;}}")
                .expect("bounded source");
        } else {
            let next = index + 1;
            write!(&mut source, "function f{index}<T extends ZrynaValue>():i32 {{f{next}<Option<T>>();return f{next}<Result<bool,T>>();}}").expect("bounded source");
        }
    }
    source.push_str("export function score():i32 {return f0<i32>();}");
    source
}

#[test]
fn original_ownership_still_precedes_pending_generated_resource_failure() {
    let baseline = pending_fanout();
    with_bodies(&baseline, |bodies| {
        let plain = plain_errors(bodies);
        assert_eq!(
            plain[0].code(),
            "ZRYNA-M7201",
            "finite acyclic original calls amplify only during pending discovery"
        );
        assert_eq!(v4_errors(bodies)[0].code(), "ZRYNA-M7201");
    });
    let source = format!("{MOVE} {baseline}");
    let at = u32::try_from(
        source.find("return value").expect("original invalid consuming expression") + 7,
    )
    .expect("span");
    with_bodies(&source, |bodies| {
        one(&v4_errors(bodies), "ZRYNA-M7007", at, at + 5);
    });
}
