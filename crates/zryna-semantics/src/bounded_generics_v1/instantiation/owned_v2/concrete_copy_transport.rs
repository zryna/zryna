//! Source-bound recursive concrete Copy transport; opaque and owned places stay affine.
use super::tests::{claim, with_claim};
use zryna_ir::generic_v1::{Failure, owned_v2, raw};

#[test]
fn recursive_concrete_copy_branch_and_loop_sources_seal() {
    for (ty, initial, replacement) in [
        ("Option<bool>", "Option.some<bool>(true)", "Option.none<bool>()"),
        ("Result<bool,i32>", "Result.ok<bool,i32>(true)", "Result.err<bool,i32>(7)"),
        (
            "Option<Option<i32>>",
            "Option.none<Option<i32>>()",
            "Option.some<Option<i32>>(Option.some<i32>(7))",
        ),
        (
            "Option<Result<bool,i32>>",
            "Option.none<Result<bool,i32>>()",
            "Option.some<Result<bool,i32>>(Result.err<bool,i32>(7))",
        ),
    ] {
        for transport in [
            format!("if(flag){{item={replacement};}}"),
            format!("let go:bool=flag; while(go){{item={replacement}; go=false;}}"),
        ] {
            let source = format!(
                "export function root(flag:bool):i32 {{let item:{ty} ={initial}; {transport} return 7;}}"
            );
            with_claim(&[("main.zry", &source)], |claim, syntax, sources, linear, linux| {
                let family_parameters = claim
                    .graph
                    .functions
                    .iter()
                    .flat_map(|f| &f.blocks)
                    .filter(|b| b.id != 0)
                    .flat_map(|b| &b.parameters)
                    .filter_map(|p| match p.ty {
                        raw::Type::Stored(id) => linear.types().find(|t| t.id().index() == id),
                        _ => None,
                    })
                    .filter(|t| t.key().len() > 1)
                    .collect::<Vec<_>>();
                assert!(!family_parameters.is_empty(), "complete family block parameter");
                assert!(family_parameters.iter().all(|t| t.drop_kind() == 0));
                let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
                    zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux)
                        .expect("runtime claims"),
                    linear,
                    linux,
                )
                .expect("runtime authority");
                owned_v2::verify(
                    owned_v2::wire::decode(&owned_v2::wire::encode(&claim).expect("wire"))
                        .expect("decode"),
                    syntax,
                    sources,
                    sources.verify_file_id(0).expect("entry"),
                    linear,
                    linux,
                    &runtime,
                )
                .expect("independent source/type/owner seal");
            });
        }
    }
}

#[test]
fn recursively_concrete_transport_never_grants_opaque_owned_or_borrowed_copy() {
    for control in ["if(flag){item=next;}", "while(flag){item=next;}"] {
        for ty in ["T", "Option<T>", "Result<bool,T>", "Option<Option<T>>"] {
            for demand in [false, true] {
                let (first, next) = match ty {
                    "T" => ("1", "2"),
                    "Option<T>" => ("Option.none<i32>()", "Option.some<i32>(2)"),
                    "Result<bool,T>" => ("Result.ok<bool,i32>(true)", "Result.err<bool,i32>(2)"),
                    _ => (
                        "Option.none<Option<i32>>()",
                        "Option.some<Option<i32>>(Option.some<i32>(2))",
                    ),
                };
                let call =
                    if demand { format!("bad<i32>(flag,{first},{next})") } else { "0".into() };
                let source = format!(
                    "function bad<T extends ZrynaValue>(flag:bool,first:{ty},next:{ty}):i32 {{let item:{ty} =first; {control} return 0;}} export function root(flag:bool):i32 {{return {call};}}"
                );
                let Failure::Diagnostics(errors) =
                    claim(&[("main.zry", &source)]).expect_err("opaque original remains affine")
                else {
                    panic!("diagnostic");
                };
                assert_eq!(errors[0].code, "ZRYNA-M7007");
            }
        }
        for (ty, initial, replacement) in [
            ("Option<String>", "Option.none<String>()", "Option.some<String>(\"next\")"),
            (
                "Option<Result<bool,String>>",
                "Option.none<Result<bool,String>>()",
                "Option.some<Result<bool,String>>(Result.err<bool,String>(\"next\"))",
            ),
        ] {
            let control = control.replace("next", replacement);
            let source = format!(
                "export function root(flag:bool):i32 {{let item:{ty} ={initial}; {control} return 0;}}"
            );
            let Failure::Diagnostics(errors) =
                claim(&[("main.zry", &source)]).expect_err("all possible payloads govern Copy")
            else {
                panic!("diagnostic");
            };
            assert_eq!(errors[0].code, "ZRYNA-M7007");
        }
    }
    for access in ["Borrow", "BorrowMut"] {
        let borrow = if access == "Borrow" { "borrow" } else { "borrowMut" };
        let source = format!(
            "export function root(flag:bool):i32 {{let item:Option<Option<i32>> =Option.none<Option<i32>>(); const view:{access}<Option<Option<i32>>> ={borrow}(item); while(flag){{item=Option.none<Option<i32>>();}} return 0;}}"
        );
        let Failure::Diagnostics(errors) =
            claim(&[("main.zry", &source)]).expect_err("borrowed Copy destination")
        else {
            panic!("diagnostic");
        };
        assert_eq!(errors[0].code, "ZRYNA-M7007");
    }
}

#[test]
fn source_first_recursive_copy_target_packet_authenticates_both_module_forms() {
    const VALUES: &str =
        include_str!("../../../../../../tests/m7-generic-owned-concrete-copy/values.zry");
    const MAIN: &str =
        include_str!("../../../../../../tests/m7-generic-owned-concrete-copy/main.zry");
    for cross in [false, true] {
        let joined = format!("{VALUES}\n{MAIN}");
        let imported = format!("import {{transport}} from \"./values.zry\";\n{MAIN}");
        let files = if cross {
            vec![("main.zry", imported.as_str()), ("values.zry", VALUES)]
        } else {
            vec![("main.zry", joined.as_str())]
        };
        if let Some(dir) = std::env::var_os("ZRYNA_GENERIC_CONCRETE_COPY_FIXTURE_DIR") {
            std::fs::write(
                std::path::PathBuf::from(dir).join(format!("{}-reference.json", files.len())),
                serde_json::to_vec_pretty(
                    &crate::bounded_generics_v1::tests::body_fixtures::snapshot(&files),
                )
                .expect("source DTO"),
            )
            .expect("explicit fixture output");
        }
        with_claim(&files, |claim, syntax, sources, linear, linux| {
            let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
                zryna_ownership_runtime_abi::generic_v1::raw_v1(linear, linux)
                    .expect("runtime claims"),
                linear,
                linux,
            )
            .expect("runtime");
            let check = |claim: &owned_v2::raw::Program| {
                owned_v2::verify(
                    owned_v2::wire::decode(&owned_v2::wire::encode(claim).expect("wire"))
                        .expect("decode"),
                    syntax,
                    sources,
                    sources.verify_file_id(0).expect("entry"),
                    linear,
                    linux,
                    &runtime,
                )
                .map(|_| ())
            };
            check(&claim).expect("original nested transport source seals");
            for mutation in 0..4 {
                let mut hostile = claim.clone();
                let function = hostile
                    .graph
                    .functions
                    .iter_mut()
                    .find(|f| f.key[0] == 0x40)
                    .expect("template instance");
                let edge = function
                    .blocks
                    .iter_mut()
                    .enumerate()
                    .find_map(|(i, b)| match &mut b.terminator {
                        raw::Terminator::Jump(e)
                            if e.target as usize <= i && e.arguments.len() >= 4 =>
                        {
                            Some(e)
                        }
                        _ => None,
                    })
                    .expect("nested family backedge");
                match mutation {
                    0 => {
                        edge.arguments.pop();
                    }
                    1 => edge.arguments[0] = u32::MAX,
                    2 => edge.arguments.swap(0, 1),
                    _ => edge.target = 0,
                }
                assert!(check(&hostile).is_err(), "source identity attack {mutation}");
                check(&claim).expect("pristine source replay after attack");
            }
        });
    }
}

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
