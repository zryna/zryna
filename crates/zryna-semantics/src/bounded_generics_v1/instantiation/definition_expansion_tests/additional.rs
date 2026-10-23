//! Alias-sensitive occurs checks and independent source/phase ordering controls.
use super::super::tests::check;
use super::InstantiationFailure;
use std::fmt::Write as _;

fn errors(source: &str) -> Vec<zryna_diagnostics::Diagnostic> {
    let InstantiationFailure::Diagnostics(errors) =
        check(&[("main.zry", source)]).expect_err("owning semantic definition rejection")
    else {
        panic!("source diagnostic required");
    };
    errors
}

#[test]
fn two_variable_alias_cycle_occurs_but_a_constructor_equation_can_still_unify() {
    let growing = "interface Loop<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {next:Vec<Loop<E,Vec<T>>>;}";
    let diagnostics = errors(growing);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M7003");
    let span = diagnostics[0].primary_span().expect("generated argument head");
    assert_eq!(&growing[span.start() as usize..span.end() as usize], "Loop<E,Vec<T>>");
    let solvable = "interface Loop<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {next:Vec<Loop<Vec<E>,i32>>;}";
    let (types, functions, edges) =
        check(&[("main.zry", solvable)]).expect("T=Vec<E>,E=i32 has a finite solution");
    assert_eq!(types, [vec![0], vec![1], vec![2]]);
    assert!(functions.is_empty());
    assert_eq!(edges, 0);
    let equal = format!("{solvable} function read(value:Loop<Vec<i32>,i32>):i32 {{return 0;}}");
    let (types, _, _) = check(&[("main.zry", &equal)]).expect("equal actual arguments terminate");
    assert_eq!(types.iter().filter(|key| key[0] == 0x12).count(), 1);
    let unequal = format!("{solvable} function read(value:Loop<i32,bool>):i32 {{return 0;}}");
    assert_eq!(errors(&unequal)[0].code(), "ZRYNA-M7003");
}

#[test]
fn same_spelling_in_another_module_is_not_a_self_application() {
    let (types, _, _) = check(&[
        ("main.zry", "import {Loop as Foreign} from \"./values.zry\"; interface Loop<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Foreign<Vec<T>>>;} function read(value:Loop<i32>):i32 {return 0;}"),
        ("values.zry", "export interface Loop<T extends ZrynaValue> extends ZrynaStruct {value:T;}"),
    ]).expect("distinct original modules survive matching source names");
    assert_eq!(types.iter().filter(|key| key[0] == 0x12).count(), 2);
}

#[test]
fn nested_heads_and_fixed_array_lengths_retain_generated_identity() {
    for argument in ["Vec<T>", "FixedArray<T,0>", "FixedArray<T,2>"] {
        let source = format!(
            "interface Phantom<U extends ZrynaValue> extends ZrynaEnum {{none:ZrynaNone;}} interface Loop<T extends ZrynaValue> extends ZrynaStruct {{next:Phantom<Loop<{argument}>>;}}"
        );
        let diagnostics = errors(&source);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M7003", "{argument}");
    }
    let declaration = "interface Loop<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {next:Vec<Loop<FixedArray<E,2>,i32>>;}";
    let equal =
        format!("{declaration} function read(value:Loop<FixedArray<i32,2>,i32>):i32 {{return 0;}}");
    check(&[("main.zry", &equal)]).expect("exact array length retains the same complete key");
    let unequal =
        format!("{declaration} function read(value:Loop<FixedArray<i32,3>,i32>):i32 {{return 0;}}");
    assert_eq!(errors(&unequal)[0].code(), "ZRYNA-M7003");
}

#[test]
fn mixed_recursion_and_definition_errors_follow_source_order_without_changing_call_bytes() {
    let definition =
        "interface Grow<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Grow<Vec<T>>>;}";
    let recursion = "function recur<U extends ZrynaValue>(value:U):U {return recur<U>(value);}";
    for reversed in [false, true] {
        let source = if reversed {
            format!("{recursion} {definition}")
        } else {
            format!("{definition} {recursion}")
        };
        let diagnostics = errors(&source);
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| diagnostic.code() == "ZRYNA-M7003"));
        let spans = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.primary_span().expect("exact source head"))
            .collect::<Vec<_>>();
        assert!(spans[0].start() < spans[1].start(), "one collector sorts both phase candidates");
        let call = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message.starts_with("source-level function recursion"))
            .expect("original call diagnostic");
        assert_eq!(
            call.message,
            "source-level function recursion is excluded before specialization; original declaration path [0, 0, 0, 0]"
        );
        let span = call.primary_span().expect("closing generic call");
        assert_eq!(&source[span.start() as usize..span.end() as usize], "recur<U>");
    }
}

#[test]
fn mixed_candidates_share_one_reserved_terminal_diagnostic() {
    let mut members = String::new();
    for index in 0..255 {
        write!(&mut members, "field{index}:Vec<Grow<Vec<T>>>;")
            .expect("bounded original field spelling");
    }
    let source = format!(
        "interface Grow<T extends ZrynaValue> extends ZrynaStruct {{{members}}} function recur<U extends ZrynaValue>(value:U):U {{return recur<U>(value);}}"
    );
    let diagnostics = errors(&source);
    assert_eq!(diagnostics.len(), 256);
    assert!(diagnostics[..255].iter().all(|diagnostic| diagnostic.code() == "ZRYNA-M7003"));
    assert_eq!(diagnostics[255].code(), "ZRYNA-M7201");
    let span = diagnostics[255].primary_span().expect("retained first-extra candidate");
    assert_eq!(&source[span.start() as usize..span.end() as usize], "recur<U>");
}

#[test]
fn universally_growing_demanded_case_retains_the_fixed_full_original_head() {
    let literal = super::catalogue_source("expanding-declaration");
    let source = format!("{literal} function read(value:Nest<i32>):i32 {{return 0;}}");
    let diagnostics = errors(&source);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M7003");
    let span = diagnostics[0].primary_span().expect("same fixed original head");
    assert_eq!((span.file().index(), span.start(), span.end()), (0, 69, 81));
}
