//! Exact applications, nominal identities, and enum payload use obligations.

use super::*;

#[test]
fn opaque_barrier_precedes_all_argument_candidates() {
    let main = format!(
        "import {{bad,other}} from \"./late.zry\"; {IDENTITY} function score():i32 {{ {} return 0; }}",
        "identity(1);".repeat(256)
    );
    let input = project(&[
        ("main.zry", &main),
        (
            "late.zry",
            "export function bad<T extends ZrynaValue>(x:T):T {return clone(x);} export function other<U extends ZrynaValue>(x:U):U {return clone(x);}",
        ),
    ]);
    let context = declarations(&input);
    let BodyTypeFailure::Diagnostics(errors) =
        check_body_types(&context).expect_err("source rejection")
    else {
        panic!("diagnostic lane")
    };
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| error.code() == "ZRYNA-M7002"));
}

#[test]
fn invalid_type_hole_never_becomes_a_success_type() {
    rejects(
        "function bad<T extends ZrynaValue>(x:Option<i32>,t:T):T {return clone(match(x,{\"Option.none\":()=>t,\"Option.some\":(v)=>v}));}",
        "ZRYNA-M7006",
        "v",
    );
    rejects(
        &format!(
            "{IDENTITY} function bad<T extends ZrynaValue>(x:T):T {{ return clone(match(Option.some(x), {{\"Option.none\":()=>x,\"Option.some\":(v)=>x}})); }}"
        ),
        "ZRYNA-M7001",
        "Option",
    );
    rejects(
        &format!(
            "{IDENTITY} function bad<T extends ZrynaValue>(x:T):T {{ identity(1); return clone(x); }}"
        ),
        "ZRYNA-M7002",
        "clone",
    );
}

#[test]
fn explicit_argument_presence_count_and_categories() {
    passes(&format!("{IDENTITY} function score():i32 {{ return identity<i32>(1); }}"));
    for arguments in ["", "<i32,bool>", "<unit>", "<Borrow<i32>>", "<Vec<Borrow<i32>>>"] {
        let errors = errors(&format!(
            "{IDENTITY} function score():i32 {{ return identity{arguments}(1); }}"
        ));
        assert!(errors.iter().all(|error| error.code() == "ZRYNA-M7001"), "{errors:?}");
    }
}

#[test]
fn generic_function_value_requires_arguments() {
    rejects(
        &format!("{IDENTITY} function score():i32 {{ identity; return 0; }}"),
        "ZRYNA-M7001",
        "identity",
    );
    passes("function score():i32 { return 0; } function other():i32 { score; return score(); }");
}

#[test]
fn nominal_identity_and_argument_order_are_exact() {
    rejects(
        &format!(
            "{BOX} interface Other<T extends ZrynaValue> extends ZrynaStruct {{value:T;}} function bad<T extends ZrynaValue>(a:Box<T>):Other<T> {{return a;}}"
        ),
        "ZRYNA-M7006",
        "a",
    );
    rejects(
        "function bad<T extends ZrynaValue,E extends ZrynaValue>(a:Result<T,E>):Result<E,T> {return a;}",
        "ZRYNA-M7006",
        "a",
    );
}

#[test]
fn generic_struct_fields_use_original_declaration_order() {
    passes(
        "interface Pair<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaStruct {first:T;second:E;} function make<T extends ZrynaValue,E extends ZrynaValue>(a:T,b:E):Pair<T,E> {return Pair<T,E>({second:b,first:a});}",
    );
    for (fields, token) in
        [("value:true", "true"), ("other:1", "other"), ("value:1,value:2", "value"), ("", "Box")]
    {
        let source = format!("{BOX} function score():Box<i32> {{return Box<i32>({{{fields}}});}}");
        let errors = errors(&source);
        assert!(errors.iter().all(|error| error.code() == "ZRYNA-M7006"));
        assert!(errors.iter().any(|error| {
            let span = error.primary_span().expect("original fixture invariant");
            &source[span.start() as usize..span.end() as usize] == token
        }));
    }
}

#[test]
fn standard_and_user_enum_signatures_are_exact() {
    passes(
        "interface Choice<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;some:T;} function make(x:i32):Choice<i32> {Option.none<i32>();Option.some<i32>(x);Result.ok<i32,String>(x);Result.err<i32,String>(\"x\");return Choice.some<i32>(x);}",
    );
    rejects(
        "function score():Option<i32> {return Option.unknown<i32>();}",
        "ZRYNA-M7004",
        "unknown",
    );
    rejects("function score():Option<i32> {return Option.some<i32>(true);}", "ZRYNA-M7006", "true");
    rejects("function score():Option<i32> {return Option.none<i32>(1);}", "ZRYNA-M7006", "none");
}

#[test]
fn match_family_coverage_bindings_and_results() {
    passes(
        "function score(x:Option<i32>):i32 {return match(x,{\"Option.some\":(v)=>v,\"Option.none\":()=>0});}",
    );
    for (arms, code, token) in [
        ("\"Option.none\":()=>0", "ZRYNA-M7004", "match"),
        (
            "\"Option.none\":()=>0,\"Option.none\":()=>0,\"Option.some\":(v)=>v",
            "ZRYNA-M7004",
            "none",
        ),
        ("\"Result.ok\":(v)=>v,\"Option.none\":()=>0", "ZRYNA-M7004", "Result"),
        ("\"Option.none\":()=>true,\"Option.some\":(v)=>v", "ZRYNA-M7006", "v"),
    ] {
        has_error(
            &format!("function score(x:Option<i32>):i32 {{return match(x,{{{arms}}});}}"),
            code,
            token,
        );
    }
}

#[test]
fn borrowed_match_payload_types_remain_borrowed() {
    for access in ["Borrow", "BorrowMut"] {
        for (family, left, right, environments) in
            [("Result<T,T>", "ok", "err", 0), ("Choice<T>", "a", "b", 1)]
        {
            let name = family.split('<').next().expect("family spelling");
            let source = format!(
                "interface Choice<T extends ZrynaValue> extends ZrynaEnum {{a:T;b:T;}} function select<T extends ZrynaValue>(x:{access}<{family}>):{access}<T> {{return match(x,{{\"{name}.{left}\":(a)=>a,\"{name}.{right}\":(b)=>b}});}}"
            );
            assertions::borrowed_arms(&source, access == "BorrowMut", environments);
        }
    }
    assertions::nested_borrowed_arms(
        "interface Choice<T extends ZrynaValue> extends ZrynaEnum {a:T;b:T;} function nested<T extends ZrynaValue>(x:Borrow<Choice<Choice<T>>>):Borrow<T> {return match(x,{\"Choice.a\":(a)=>match(a,{\"Choice.a\":(v)=>v,\"Choice.b\":(v)=>v}),\"Choice.b\":(b)=>match(b,{\"Choice.a\":(v)=>v,\"Choice.b\":(v)=>v})});}",
    );
    rejects(
        "function bad<T extends ZrynaValue>(x:Borrow<Option<T>>):i32 {return match(x,{\"Option.none\":()=>0,\"Option.some\":(v)=>clone(v)});}",
        "ZRYNA-M3008",
        "clone",
    );
}
