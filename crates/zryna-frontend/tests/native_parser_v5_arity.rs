//! Source rejection witnesses from the accepted #415 catalogue, independent of raw DTO limits.

use serde_json::Value;
use zryna_frontend::{native_lexer::lex, native_parser::v5::parse_v5_candidate};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
use zryna_syntax::v5;

fn catalogue() -> Value {
    serde_json::from_str(include_str!("../../../spec/language/generic-review-v1-fixtures.json"))
        .expect("accepted contract catalogue")
}

fn source(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
        .expect("source map")
}

#[test]
fn first_extra_generic_source_lists_report_the_catalogue_code_and_utf8_primary_token() {
    let catalogue = catalogue();
    for (id, wrapper, extra) in [
        ("three-parameters", false, ", U extends ZrynaValue"),
        ("three-arguments", true, ", String"),
    ] {
        let case = catalogue["sourceCases"]
            .as_array()
            .expect("source cases")
            .iter()
            .find(|case| case["id"] == id)
            .expect("accepted first-extra decision");
        assert_eq!(case["parserErrors"], 0);
        assert_eq!(case["primaryOccurrence"], 0);
        let fragment = case["source"].as_str().expect("source");
        let primary = case["primaryToken"].as_str().expect("primary token");
        for prefix in ["", "// π😀\r\n", "/* π😀 */\u{2028}"] {
            let body = if wrapper {
                format!("function probe(): i32 {{ {fragment} return 0; }}")
            } else {
                fragment.to_owned()
            };
            let text = format!("{prefix}{body}");
            let sources = source(&text);
            let error = parse_v5_candidate(&sources, &lex(&sources).expect("lexable source"))
                .expect_err("first extra source item is rejected before a raw DTO");
            assert_eq!(error.diagnostic().code(), case["code"].as_str().expect("code"), "{id}");
            let start = u32::try_from(text.find(primary).expect("primary token")).expect("offset");
            let end = start + u32::try_from(primary.len()).expect("token length");
            let span = sources
                .verify_span(UntrustedSpan { file: 0, start, end })
                .expect("authoritative UTF-8 token span");
            assert_eq!(error.diagnostic().primary_span(), Some(span), "{id}: {text}");

            let valid = source(&text.replacen(extra, "", 1));
            let candidate = parse_v5_candidate(&valid, &lex(&valid).expect("pristine lex"))
                .expect("two source items remain admitted syntax");
            v5::verify_snapshot(candidate, &valid)
                .expect("complete source barrier after rejection");
        }
    }
}

#[test]
fn rejected_source_lists_do_not_relax_the_hostile_raw_two_item_cap() {
    let span = serde_json::json!({"file": 0, "start": 0, "end": 1});
    let parameter = serde_json::json!({
        "span": span, "name": {"text": "T", "span": span},
        "extends_span": span, "bound": {"text": "ZrynaValue", "span": span}
    });
    let common = serde_json::json!({
        "span": span, "less_than_span": span, "greater_than_span": span,
        "comma_spans": [span, span]
    });
    let mut parameters = common.clone();
    parameters["parameters"] = serde_json::json!([parameter, parameter, parameter]);
    assert!(serde_json::from_value::<v5::RawTypeParameterList>(parameters).is_err());
    let mut arguments = common;
    arguments["arguments"] = serde_json::json!([0, 1, 2]);
    assert!(serde_json::from_value::<v5::RawTypeArgumentList>(arguments).is_err());
}

#[test]
fn first_extra_order_and_complete_nested_argument_spans_are_source_bound() {
    for (text, code, primary) in [
        (
            "function f<T extends ZrynaValue, E extends ZrynaValue, U extends ZrynaValue, V extends ZrynaValue>(): i32 { return 0; }",
            "ZRYNA-D7001",
            "U",
        ),
        (
            "function f(): i32 { return select<i32, bool, String, i32>(7, true); }",
            "ZRYNA-M7001",
            "String",
        ),
        (
            "// π😀\r\nfunction f(): i32 { return select<i32, bool, Option<Result<i32,bool>>>(7, true); }",
            "ZRYNA-M7001",
            "Option<Result<i32,bool>>",
        ),
    ] {
        let sources = source(text);
        let error = parse_v5_candidate(&sources, &lex(&sources).expect("lexable source"))
            .expect_err("first extra item");
        assert_eq!(error.diagnostic().code(), code);
        let start = u32::try_from(text.find(primary).expect("primary token")).expect("offset");
        let end = start + u32::try_from(primary.len()).expect("token length");
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start, end })
            .expect("complete authentic argument span");
        assert_eq!(error.diagnostic().primary_span(), Some(span));
    }
}

#[test]
fn outer_arity_precedes_nested_arity_and_type_depth_work() {
    let deep = format!("{}i32{}", "Option<".repeat(129), ">".repeat(129));
    for (first, second, third) in [
        ("i32", "bool", "Option<i32, bool, String>"),
        ("Option<i32, bool, String>", "bool", "String"),
        ("i32", "Option<i32, bool, String>", "String"),
        (deep.as_str(), "bool", "String"),
        ("i32", deep.as_str(), "String"),
        ("i32", "bool", deep.as_str()),
    ] {
        let prefix = format!("// π😀\r\nfunction f(): i32 {{ return select<{first}, {second}, ");
        let text = format!("{prefix}{third}>(7, true); }}");
        let sources = source(&text);
        let error = parse_v5_candidate(&sources, &lex(&sources).expect("bounded original tokens"))
            .expect_err("outer third argument rejects before any type normalization");
        assert_eq!(error.diagnostic().code(), "ZRYNA-M7001", "{first}, {second}, {third}");
        let start = u32::try_from(prefix.len()).expect("UTF-8 start");
        let end = start + u32::try_from(third.len()).expect("complete third occurrence");
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start, end })
            .expect("authentic complete third argument");
        assert_eq!(error.diagnostic().primary_span(), Some(span), "{first}, {second}, {third}");

        let valid = source("function f(): i32 { return select<i32, bool>(7, true); }");
        let candidate = parse_v5_candidate(&valid, &lex(&valid).expect("fresh lexical inventory"))
            .expect("pristine two-item recovery");
        v5::verify_snapshot(candidate, &valid)
            .expect("complete source verification after rejection");
    }
}

#[test]
fn malformed_generic_prefixes_defer_to_syntax_and_valid_trailing_commas_recover() {
    for text in [
        "function f(): i32 { return select<, i32, bool, String>(7, true); }",
        "function f(): i32 { return select<i32, , bool, String>(7, true); }",
        "function f(): i32 { return select<i32, bool, , String>(7, true); }",
        "function f(value: Box<i32, bool, , String>): i32 { return 0; }",
        "function f<T extends ZrynaValue, , E extends ZrynaValue, U extends ZrynaValue>(): i32 { return 0; }",
        "function f(): i32 { return select<i32, bool, Option<String>(7, true); }",
        "function f(value: Box<i32, bool): i32 { return 0; }",
    ] {
        let sources = source(text);
        let error = parse_v5_candidate(&sources, &lex(&sources).expect("lexable malformed source"))
            .expect_err("malformed prefix cannot authenticate an arity rejection");
        assert_eq!(error.diagnostic().code(), "ZRYNA-F2002", "{text}");
        for valid in [
            "function f(): i32 { return select<i32,>(7); }",
            "function f(): i32 { return select<i32, bool,>(7, true); }",
            "function f<T extends ZrynaValue,>(value: T): T { return value; }",
            "function f<T extends ZrynaValue, E extends ZrynaValue,>(value: T): T { return value; }",
        ] {
            let sources = source(valid);
            let candidate =
                parse_v5_candidate(&sources, &lex(&sources).expect("fresh lexical inventory"))
                    .expect("one/two-item trailing comma remains admitted syntax");
            v5::verify_snapshot(candidate, &sources)
                .expect("pristine complete source verification");
        }
    }
}

#[test]
fn arity_preflight_does_not_promote_malformed_nonempty_source_prefixes() {
    for text in [
        "function f(): i32 { return select<i32 +, bool, String>(7, true); }",
        "function f(): i32 { return select<i32 foo, bool, String>(7, true); }",
        "function f<T ZrynaValue, E extends ZrynaValue, U extends ZrynaValue>(): i32 { return 0; }",
        "function f(value: Box<i32, bool, String, ): i32 { return 0; }",
        "function f(): i32 { return select<function<i32>, bool, String>(7, true); }",
        "function f(value: Box<function<i32>, bool, String>): i32 { return 0; }",
    ] {
        let sources = source(text);
        let error = parse_v5_candidate(&sources, &lex(&sources).expect("bounded malformed tokens"))
            .expect_err("malformed nonempty syntax cannot justify an arity diagnostic");
        assert_eq!(error.diagnostic().code(), "ZRYNA-F2002", "{text}");
    }
}

#[test]
fn arity_preflight_covers_fixed_array_before_any_argument_normalization() {
    for (first, third) in [("i32", "String"), ("Option<i32, bool, String>", "String")] {
        let prefix = format!("// π😀\r\nfunction f(value: FixedArray<{first}, 2, ");
        let text = format!("{prefix}{third}>): i32 {{ return 0; }}");
        let sources = source(&text);
        let error = parse_v5_candidate(&sources, &lex(&sources).expect("bounded original tokens"))
            .expect_err("outer FixedArray third argument rejects before element replay");
        assert_eq!(error.diagnostic().code(), "ZRYNA-M7001");
        let start = u32::try_from(prefix.len()).expect("UTF-8 start");
        let end = start + u32::try_from(third.len()).expect("third type occurrence");
        let span = sources
            .verify_span(UntrustedSpan { file: 0, start, end })
            .expect("outer third argument span");
        assert_eq!(error.diagnostic().primary_span(), Some(span));
    }
}
