//! Current private execution qualification boundaries do not narrow the accepted language contract.
use super::{
    SemanticInput, assert_original_types, check_body_types, original_ownership_errors, owned_v2,
    project, resolve_declarations,
};

#[test]
fn nominal_enum_admission_keeps_unqualified_operations_and_opaque_payloads_rejected() {
    for ty in ["Vec<i32>", "Shared<i32>", "Weak<i32>"] {
        let source = format!(
            "interface Bad<T extends ZrynaValue> extends ZrynaEnum {{none:ZrynaNone;bad:{ty};}} export function root(input:i32):i32 {{return input;}}"
        );
        assert_original_types(&[("main.zry", &source)]);
        let p = project(&[("main.zry", &source)]);
        assert!(
            owned_v2::check_original_bodies(&p.syntax, &p.sources).is_err(),
            "even unused declarations must have qualified stored members"
        );
    }
    let source_with = |operation: &str| {
        format!(
            "interface Box<T extends ZrynaValue> extends ZrynaEnum {{none:ZrynaNone;value:T;}} export function root(flag:bool):i32 {{let item:Box<i32> =Box.none<i32>(); {operation} return 0;}}"
        )
    };
    let baseline = source_with("");
    assert_original_types(&[("main.zry", &baseline)]);
    let p = project(&[("main.zry", &baseline)]);
    owned_v2::check_original_bodies(&p.syntax, &p.sources)
        .expect("the same nominal declaration, constructor and local need no successor operation");
    for operation in
        ["clone(item);", "if(flag){item=Box.none<i32>();}", "while(flag){item=Box.none<i32>();}"]
    {
        let source = source_with(operation);
        let errors = original_ownership_errors(&source);
        // These remain private execution exclusions, not accepted-language diagnostics.
        // Body typing above succeeds; the operation alone requires a successor IR proof.
        assert_eq!(errors.len(), 1, "{operation}: {errors:?}");
        assert_eq!(errors[0].code(), "ZRYNA-I7001", "{operation}: {errors:?}");
    }
    let opaque = "interface Box<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;value:T;} function bad<T extends ZrynaValue>(value:Box<T>):i32 {return match(value,{\"Box.none\":()=>0,\"Box.value\":(item)=>item+item});} export function root(input:i32):i32 {return input;}";
    let p = project(&[("main.zry", opaque)]);
    let entry = p.sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&p.syntax, &p.sources, entry).expect("source authority"),
    )
    .expect("opaque nominal declaration resolves");
    let Err(crate::bounded_generics_v1::body_types::BodyTypeFailure::Diagnostics(errors)) =
        check_body_types(&declarations)
    else {
        panic!("unused nominal payload must retain opaque T arithmetic rejection");
    };
    assert!(errors.iter().any(|error| error.code() == "ZRYNA-M7002"));
}

#[test]
fn nominal_structs_and_nested_nominal_arguments_need_separate_execution_qualification() {
    for source in [
        "interface Box<T extends ZrynaValue> extends ZrynaStruct {value:T;} export function root(input:i32):i32 {const item:Box<i32> =Box<i32>({value:input}); return input;}",
        "interface Inner extends ZrynaEnum {none:ZrynaNone;code:i32;} interface Outer<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;value:T;} export function root(input:i32):i32 {const item:Outer<Inner> =Outer.value<Inner>(Inner.code(input)); return input;}",
    ] {
        assert_original_types(&[("main.zry", source)]);
        let p = project(&[("main.zry", source)]);
        assert!(owned_v2::check_original_bodies(&p.syntax, &p.sources).is_err());
    }
}
