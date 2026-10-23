//! Semantic extraction controls: absence of this sufficient proof grants no closed layout.
use super::super::tests::check;
use super::{one, plain_errors, with_bodies};

#[test]
fn direct_two_parameter_struct_and_inactive_enum_member_use_the_full_stored_root() {
    for source in [
        "interface Pair<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {code:i32;next:Pair<T,E>;}",
        "interface Pair<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;next:Pair<T,E>;}",
    ] {
        let start = u32::try_from(source.rfind("Pair<T,E>").expect("actual second member root"))
            .expect("span");
        with_bodies(source, |bodies| {
            one(&plain_errors(bodies), "ZRYNA-L3002", start, start + 9);
        });
    }
}

#[test]
fn wrappers_phantom_arguments_and_function_occurrences_are_not_direct_storage_edges() {
    for member in [
        "Vec<Node<T>>",
        "Shared<Node<T>>",
        "Weak<Node<T>>",
        "Option<Node<T>>",
        "FixedArray<Node<T>,0>",
        "FixedArray<Node<T>,1>",
        "Phantom<Node<T>>",
    ] {
        let source = format!(
            "interface Phantom<U extends ZrynaValue> extends ZrynaStruct {{code:i32;}} interface Node<T extends ZrynaValue> extends ZrynaStruct {{next:{member};}}"
        );
        let (types, functions, edges) = check(&[("main.zry", &source)])
            .expect("no direct-root proof; no actual closed source root is supplied");
        assert_eq!(types, [vec![0], vec![1], vec![2]], "{member}: no closed probe invented");
        assert!(functions.is_empty());
        assert_eq!(edges, 0);
        // No layout or v4 ownership claim is made for these currently unqualified wrappers.
    }
    let source = "interface Node<T extends ZrynaValue> extends ZrynaStruct {code:i32;} function unused<T extends ZrynaValue>(value:Node<T>):i32 {return 0;}";
    let (types, functions, edges) =
        check(&[("main.zry", source)]).expect("function type is not a stored member root");
    assert_eq!(types, [vec![0], vec![1], vec![2]]);
    assert!(functions.is_empty());
    assert_eq!(edges, 0);
}

#[test]
fn argument_permutation_constants_and_foreign_original_identity_do_not_prove_self_equality() {
    for arguments in ["E,T", "i32,E", "T,T"] {
        let source = format!(
            "interface Pair<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {{next:Pair<{arguments}>;}}"
        );
        let (types, functions, edges) = check(&[("main.zry", &source)])
            .expect("a possible closed unification is not universal own-slot equality");
        assert_eq!(types, [vec![0], vec![1], vec![2]]);
        assert!(functions.is_empty());
        assert_eq!(edges, 0);
    }
    let (types, functions, edges) = check(&[
        ("main.zry", "import {Loop as Foreign} from \"./values.zry\"; interface Loop<T extends ZrynaValue> extends ZrynaStruct {next:Foreign<T>;}"),
        ("values.zry", "export interface Loop<T extends ZrynaValue> extends ZrynaStruct {value:T;}"),
    ]).expect("same spelling in another source module is not the original owner");
    assert_eq!(types, [vec![0], vec![1], vec![2]]);
    assert!(functions.is_empty());
    assert_eq!(edges, 0);
}

#[test]
fn an_unrelated_phantom_descendant_never_replaces_the_real_member_span() {
    let source = "interface Phantom<U extends ZrynaValue> extends ZrynaStruct {code:i32;} interface Pair<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {code:i32;next:Pair<T,E>;unrelated:Phantom<Pair<T,E>>;}";
    let start =
        u32::try_from(source.find("next:Pair<T,E>").expect("actual by-value self member") + 5)
            .expect("span");
    with_bodies(source, |bodies| {
        one(&plain_errors(bodies), "ZRYNA-L3002", start, start + 9);
    });
}
