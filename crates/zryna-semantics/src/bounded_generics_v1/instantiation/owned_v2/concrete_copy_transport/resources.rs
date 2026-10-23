//! Complete-key source-state credits and logical header boundaries.

use super::{Failure, claim};

#[test]
fn recursive_copy_loop_complete_key_credit_has_fixed_last_first_extra_and_recovery() {
    // Each source has two bindings, one scalar key and one complete family key of L bytes.
    // Its initializer contributes two IDs; each loop adds one complete header parameter.
    // Two snapshots plus index/edges/both key copies cost 4*L+19+2*j for loop j.
    // These arithmetic witnesses count original source operations, independent of emitted IR.
    for (ty, initial, key_bytes, last, last_units, extra_units) in [
        ("Option<bool>", "Option.none<bool>()", 10usize, 995usize, 1_047_735usize, 1_049_784usize),
        ("Option<Option<i32>>", "Option.none<Option<i32>>()", 19, 978, 1_048_416, 1_050_467),
    ] {
        for (count, units, accepted) in [
            (last, last_units, true),
            (last + 1, extra_units, false),
            (1, if last == 995 { 59 } else { 95 }, true),
        ] {
            assert_eq!(accepted, units <= 1_048_576);
            assert_eq!(units, count * (4 * key_bytes + 19) + count * (count - 1));
            let source = format!(
                "export function root(flag:bool):i32 {{let item:{ty} ={initial}; {} return 0;}}",
                "while(flag){}".repeat(count)
            );
            let result = claim(&[("main.zry", &source)]);
            if accepted {
                result.expect("fixed last whole-loop count or pristine recovery");
            } else {
                let Failure::Diagnostics(errors) = result
                    .expect_err("complete key copies exceed the unchanged source-state ceiling")
                else {
                    panic!("diagnostic");
                };
                assert_eq!(errors[0].code, "ZRYNA-M7201");
            }
        }
    }
}

#[test]
fn recursive_copy_branch_complete_key_credit_has_fixed_last_first_extra_and_recovery() {
    use std::fmt::Write;
    // 401 bindings, 801 initial value IDs and 7,601 complete key bytes give a snapshot
    // cost of 8,804. Six snapshots have deltas 0,0,2,2,2,2; capture costs 22 and join 42.
    // Each branch adds three IDs, hence 52,896 + 18*j units at original branch j.
    for (branches, units, accepted) in
        [(19usize, 1_008_102usize, true), (20, 1_061_340, false), (1, 52_896, true)]
    {
        assert_eq!(units, branches * 52_896 + 18 * branches * (branches - 1) / 2);
        assert_eq!(accepted, units <= 1_048_576);
        let mut source = String::from("export function root(flag:bool):i32 {");
        for index in 0..400 {
            write!(source, "let item{index}:Option<Option<i32>> =Option.none<Option<i32>>();")
                .expect("source");
        }
        source.push_str(&"if(flag){item0=Option.none<Option<i32>>();}".repeat(branches));
        source.push_str("return 0;}");
        let result = claim(&[("main.zry", &source)]);
        if accepted {
            result.expect("fixed last whole-branch count or pristine recovery");
        } else {
            let Failure::Diagnostics(errors) =
                result.expect_err("complete nested key snapshot/capture/join credit")
            else {
                panic!("diagnostic");
            };
            assert_eq!(errors[0].code, "ZRYNA-M7201");
        }
    }
}

#[test]
fn recursive_copy_loop_logical_header_parameter_limit_remains_256() {
    use std::fmt::Write;
    for count in [256, 257, 1] {
        let mut source = String::from("export function root(flag:bool):i32 {");
        for index in 0..count {
            write!(source, "let item{index}:Option<Option<i32>> =Option.none<Option<i32>>();")
                .expect("source");
        }
        source.push_str("while(flag){} return 0;}");
        let result = claim(&[("main.zry", &source)]);
        if count == 257 {
            let Failure::Diagnostics(errors) =
                result.expect_err("first extra complete family header parameter")
            else {
                panic!("diagnostic");
            };
            assert_eq!(errors[0].code, "ZRYNA-M7201");
            assert!(errors[0].message.contains("header parameter ceiling"));
        } else {
            result.expect("exact logical header limit or pristine recovery");
        }
    }
}
