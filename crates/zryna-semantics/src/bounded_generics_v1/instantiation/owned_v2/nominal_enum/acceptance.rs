//! Genuine construction, substitution, movement and sealed layout affinity.
use super::{original_ownership_errors, owned_v2, with_claim};

#[test]
fn nominal_enum_source_constructs_matches_and_seals_owned_and_copy_instances() {
    for source in [
        "interface Tag extends ZrynaEnum {none:ZrynaNone;code:i32;} export function root(input:i32):i32 {const item:Tag=Tag.code(input); return match(item,{\"Tag.none\":()=>0,\"Tag.code\":(code)=>code});}",
        "interface Box<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;value:T;} function unused<T extends ZrynaValue>(value:Box<T>):Box<T> {return value;} export function root(input:i32):i32 {const item:Box<i32> =Box.value<i32>(input); return match(item,{\"Box.none\":()=>0,\"Box.value\":(code)=>code});}",
        "interface Choice<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;left:T;right:E;} export function root(input:i32):i32 {const item:Choice<String,i32> =Choice.left<String,i32>(\"owner\"); return match(item,{\"Choice.none\":()=>0,\"Choice.left\":(text)=>7,\"Choice.right\":(code)=>code});}",
        "interface Box<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;value:T;} export function root(input:i32):i32 {const item:Box<Option<Result<String,bool>>> =Box.value<Option<Result<String,bool>>>(Option.some<Result<String,bool>>(Result.ok<String,bool>(\"owner\"))); return match(item,{\"Box.none\":()=>input,\"Box.value\":(inner)=>7});}",
        "interface Choice<T extends ZrynaValue,E extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;left:T;right:E;} export function root(input:i32):i32 {const item:Choice<Option<String>,Result<i32,bool>> =Choice.right<Option<String>,Result<i32,bool>>(Result.err<i32,bool>(true)); return match(item,{\"Choice.none\":()=>0,\"Choice.left\":(text)=>7,\"Choice.right\":(result)=>match(result,{\"Result.ok\":(code)=>code,\"Result.err\":(flag)=>9})});}",
    ] {
        with_claim(&[("main.zry", source)], |claim, syntax, sources, linear, linux| {
            assert!(linear.types().any(|ty| matches!(ty.key().first(), Some(0x11 | 0x13))));
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
            .expect("exact original nominal members, payload moves and cleanup seal");
        });
    }
}

#[test]
fn nominal_affinity_uses_all_substituted_payloads_instead_of_type_arguments() {
    let phantom = "interface Marker<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;code:i32;} export function root(input:i32):i32 {const item:Marker<String> =Marker.code<String>(input); match(item,{\"Marker.none\":()=>0,\"Marker.code\":(code)=>code}); return match(item,{\"Marker.none\":()=>0,\"Marker.code\":(code)=>code});}";
    with_claim(&[("main.zry", phantom)], |_, _, _, linear, _| {
        let marker = linear
            .types()
            .find(|ty| ty.key().first() == Some(&0x13))
            .expect("phantom String instance");
        assert_eq!(marker.drop_kind(), 0, "unused String argument stores no owner");
    });
    let fixed = "interface Fixed<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;text:String;} export function root(input:i32):i32 {const item:Fixed<i32> =Fixed.none<i32>(); match(item,{\"Fixed.none\":()=>0,\"Fixed.text\":(text)=>0}); return match(item,{\"Fixed.none\":()=>input,\"Fixed.text\":(text)=>0});}";
    let errors = original_ownership_errors(fixed);
    assert_eq!(errors[0].code, "ZRYNA-M7007");
    assert!(errors[0].message.contains("moved") || errors[0].message.contains("unavailable"));
    // The inactive owned variant must determine the independently sealed layout too.
    let valid = fixed.replace("match(item,{\"Fixed.none\":()=>0,\"Fixed.text\":(text)=>0}); ", "");
    with_claim(&[("main.zry", &valid)], |_, _, _, linear, _| {
        let fixed = linear
            .types()
            .find(|ty| ty.key().first() == Some(&0x13))
            .expect("fixed-owned scalar-argument instance");
        assert_ne!(fixed.drop_kind(), 0, "all variants determine affinity");
    });
}

#[test]
fn nominal_owned_originals_reject_reuse_and_live_loan_moves() {
    for body in [
        "match(item,{\"Box.none\":()=>0,\"Box.value\":(text)=>0}); return match(item,{\"Box.none\":()=>0,\"Box.value\":(text)=>0});",
        "const view:Borrow<Box<String>> =borrow(item); const moved:Box<String> =item; return match(view,{\"Box.none\":()=>0,\"Box.value\":(text)=>0});",
        "const view:BorrowMut<Box<String>> =borrowMut(item); const moved:Box<String> =item; return match(view,{\"Box.none\":()=>0,\"Box.value\":(text)=>0});",
    ] {
        let source = format!(
            "interface Box<T extends ZrynaValue> extends ZrynaEnum {{none:ZrynaNone;value:T;}} export function root(input:i32):i32 {{const item:Box<String> =Box.value<String>(\"owner\"); {body}}}"
        );
        let errors = original_ownership_errors(&source);
        assert_eq!(errors[0].code, "ZRYNA-M7007");
    }
}
