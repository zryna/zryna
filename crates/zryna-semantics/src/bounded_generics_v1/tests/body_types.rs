use super::*;
use crate::bounded_generics_v1::tests::body_fixtures::project;

#[path = "body_assertions.rs"]
mod assertions;
use assertions::{declarations, errors, function, has_error, parameter, passes, rejects};

const IDENTITY: &str = "function identity<T extends ZrynaValue>(value: T): T { return value; }";
const BOX: &str = "interface Box<T extends ZrynaValue> extends ZrynaStruct { value: T; }";

#[test]
fn exact_declaration_context_retained() {
    let input = project(&[("main.zry", IDENTITY)]);
    let context = declarations(&input);
    let checked = check_body_types(&context).expect("original fixture invariant");
    assert!(std::ptr::eq(checked.declarations(), &raw const context));
    let foreign = project(&[("main.zry", IDENTITY)]);
    let foreign_owner = function(&declarations(&foreign), 0);
    assert_eq!(checked.expression_count(foreign_owner), None);
}

#[test]
fn owning_parameter_terms_are_disjoint() {
    let source = format!(
        "{IDENTITY} function relay<U extends ZrynaValue>(value: U): U {{ return identity<U>(value); }}"
    );
    let input = project(&[("main.zry", &source)]);
    let context = declarations(&input);
    let checked = check_body_types(&context).expect("original fixture invariant");
    let first = parameter(&context, 0);
    let second = parameter(&context, 1);
    assert_ne!(first, second);
    let result = checked.expression_type(function(&context, 1), 1).expect("original expression");
    assert_eq!(result.shape(), TypeShape::Parameter(second));
    let input = crate::bounded_generics_v1::tests::body_fixtures::many_owners();
    let context = declarations(&input);
    let checked = check_body_types(&context).expect("source-derived high owner count");
    let owners = context
        .modules()
        .flat_map(|module| module.data_declarations().chain(module.functions()))
        .flat_map(|declaration| {
            declaration
                .type_parameters()
                .map(|parameter| (parameter.identity().declaration(), parameter.identity().index()))
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(owners.len(), 65_528);
    assert_eq!(checked.storage().predicate_rows, 65_537);
}

#[test]
fn unused_identity_template_typechecked() {
    passes(IDENTITY);
}

#[test]
fn unused_invalid_clone_template_rejected() {
    rejects(
        "function broken<T extends ZrynaValue>(value:T):T { return clone(value); }",
        "ZRYNA-M7002",
        "clone",
    );
}

#[test]
fn concrete_i32_use_does_not_repair_clone() {
    rejects(
        "function broken<T extends ZrynaValue>(value:T):T { return clone(value); } function score():i32 { return broken<i32>(1); }",
        "ZRYNA-M7002",
        "clone",
    );
}

#[test]
fn two_parameter_template_keeps_order() {
    passes(
        "function second<T extends ZrynaValue,E extends ZrynaValue>(a:T,b:E):E { return b; } function relay<A extends ZrynaValue,B extends ZrynaValue>(a:A,b:B):B { return second<A,B>(a,b); }",
    );
    rejects(
        "function second<T extends ZrynaValue,E extends ZrynaValue>(a:T,b:E):T { return b; }",
        "ZRYNA-M7006",
        "b",
    );
}

#[test]
fn imported_template_substitution_uses_original_owner() {
    let input = project(&[
        (
            "main.zry",
            "import { identity as relay, Box as Parcel } from \"./values.zry\"; function wrap<T extends ZrynaValue>(value:T):T { const p:Parcel<T> = Parcel<T>({value:value}); return relay<T>(p.value); }",
        ),
        (
            "values.zry",
            "export interface Box<T extends ZrynaValue> extends ZrynaStruct { value:T; } export function identity<T extends ZrynaValue>(value:T):T {return value;}",
        ),
    ]);
    let context = declarations(&input);
    let checked = check_body_types(&context).expect("original fixture invariant");
    let original = context
        .modules()
        .nth(1)
        .expect("original fixture invariant")
        .functions()
        .next()
        .expect("original fixture invariant")
        .identity();
    assert!(
        checked.tables.functions[0]
            .environments
            .iter()
            .any(|environment| environment.owner == original)
    );
}

#[test]
fn local_binding_initializers_and_shadowing() {
    passes("function score(value:i32):i32 { { const value:i32 = value; value; } return value; }");
    rejects(
        "function score():i32 { { const hidden:i32 = 1; } return hidden; }",
        "ZRYNA-M3002",
        "hidden",
    );
}

#[test]
fn branch_loop_and_upgrade_lexical_types() {
    passes(
        "function score(flag:bool, weak:Weak<String>):i32 { let value:i32 = 0; if(flag) { value = 1; } else { value = 2; } while(flag) { value = value + 1; } upgradeWeak(weak,(strong)=>{ clone(strong); },()=>{ value = 3; }); return value; }",
    );
    rejects(
        "function score(weak:Weak<String>):i32 { upgradeWeak(weak,(strong)=>{ clone(strong); },()=>{ strong; }); return 0; }",
        "ZRYNA-M3002",
        "strong",
    );
}

#[test]
fn opaque_scalar_operations_rejected() {
    for operator in ["+", "-", "*", "===", "!==", "<", "<=", ">", ">="] {
        rejects(
            &format!(
                "function bad<T extends ZrynaValue>(a:T,b:T):T {{ a {operator} b; return a; }}"
            ),
            "ZRYNA-M7002",
            operator,
        );
    }
    rejects("function bad<T extends ZrynaValue>(a:T):T { -a; return a; }", "ZRYNA-M7002", "-");
}

#[test]
fn opaque_projection_index_and_callable_rejected() {
    for (body, token) in [("a.value;", "."), ("a[0];", "["), ("a();", "a")] {
        rejects(
            &format!("function bad<T extends ZrynaValue>(a:T):T {{ {body} return a; }}"),
            "ZRYNA-M7002",
            token,
        );
    }
}

#[test]
fn higher_kinded_application_is_resolved_before_code() {
    rejects(
        "function bad<F extends ZrynaValue>(value:F<i32>):i32 { return 0; }",
        "ZRYNA-M7002",
        "F<i32>",
    );
    rejects(
        &format!("{IDENTITY} function score():i32 {{ return identity<Unknown>(1); }}"),
        "ZRYNA-M7001",
        "Unknown",
    );
}

#[test]
fn all_payload_capabilities_are_required() {
    for value in ["Option.none<T>()", "Vec<T>([])", "Box<T>({value:value})", "Choice.none<T>()"] {
        let source = format!(
            "{BOX} interface Choice<T extends ZrynaValue> extends ZrynaEnum {{ none:ZrynaNone; some:T; }} function bad<T extends ZrynaValue>(value:T):T {{ clone({value}); return value; }}"
        );
        rejects(&source, "ZRYNA-M7002", "clone");
    }
}

#[test]
fn shared_weak_handle_clone_does_not_require_payload_clone() {
    passes(
        "function wrap<T extends ZrynaValue>(value:T):Weak<T> { return clone(downgrade(clone(shared(value)))); }",
    );
    rejects(
        "function bad<T extends ZrynaValue>(value:T):T { clone(borrow(value)); return value; }",
        "ZRYNA-M3008",
        "clone",
    );
}

#[test]
fn symbolic_moves_stores_borrows_passes_returns_typecheck() {
    passes(&format!(
        "{IDENTITY} {BOX} function relay<T extends ZrynaValue>(value:T):T {{ const stored:Box<T> = Box<T>({{value:value}}); borrow(stored); borrowMut(stored); return identity<T>(stored.value); }}"
    ));
}

#[test]
fn ownership_misuse_remains_an_unresolved_obligation() {
    let source = format!(
        "{IDENTITY} function reuse<T extends ZrynaValue>(value:T):T {{ identity<T>(value); return value; }}"
    );
    let input = project(&[("main.zry", &source)]);
    let context = declarations(&input);
    let checked = check_body_types(&context).expect("original fixture invariant");
    let owner = context
        .modules()
        .next()
        .expect("original fixture invariant")
        .functions()
        .nth(1)
        .expect("original fixture invariant")
        .identity();
    assert_eq!(checked.expression_count(owner), Some(3));
    let uses = resources::raw_function(&context, owner)
        .body
        .expressions
        .iter()
        .filter(|expression| {
            matches!(&expression.kind,
                zryna_syntax::v5::RawExpressionKind::Reference { name } if name.text == "value")
        })
        .count();
    assert_eq!(uses, 2);
}

#[path = "body_types/applications.rs"]
mod applications;
#[path = "body_types/boundaries.rs"]
mod boundaries;

#[path = "body_types/argument_spans.rs"]
mod argument_spans;

#[path = "body_types/match_diagnostics.rs"]
mod match_diagnostics;
