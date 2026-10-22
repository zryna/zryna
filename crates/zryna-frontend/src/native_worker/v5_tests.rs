//! Private native v5 wire sessions; these never admit a public compiler profile.
use std::io::Cursor;

use super::*;
use crate::syntax_v5;

fn frames(output: &[u8]) -> Vec<Value> {
    output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("complete JSON response"))
        .collect()
}

fn analyze(params: &Value) -> (Result<(), NativeWorkerError>, Vec<Value>) {
    let input = format!(
        "{{\"id\":1,\"method\":\"handshake\"}}\n{}\n",
        json!({"id":2,"method":"analyze","params":params})
    );
    let mut output = Vec::new();
    let result = run(5, &mut Cursor::new(input), &mut output);
    (result, frames(&output))
}

fn source(path: &str, text: &str) -> Value {
    json!({"path":path,"text":text})
}

#[test]
fn exact_v5_handshake_and_source_bound_generic_candidate() {
    let text = "// π\r\nexport function identity<T extends ZrynaValue>(value: T): T { return value; }\nexport function answer(): i32 { return identity<i32>(42); }";
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text: text.into() }])
            .expect("authoritative sources");
    let (result, output) =
        analyze(&json!({"schema_version":5,"files":[source("src/main.zry", text)]}));
    result.expect("private v5 native session");
    assert_eq!(output.len(), 2);
    assert_eq!(
        output[0],
        json!({"id":1,"result":{
        "provider":NATIVE_PROVIDER,"provider_version":NATIVE_PROVIDER_VERSION,
        "protocol_version":5,"capabilities":{"module_resolution":false,
        "semantic_diagnostics":false,"control_flow_v1":true,
        "data_ownership_syntax_v1":true,"bounded_generics_syntax_v1":true}}})
    );
    let bytes = serde_json::to_vec(&output[1]["result"]).expect("raw DTO bytes");
    let raw = syntax_v5::decode_snapshot(&bytes).expect("closed v5 DTO");
    let sealed = syntax_v5::verify_snapshot(raw, &sources).expect("independent source verifier");
    assert!(sealed.is_bound_to(&sources));
    assert!(output[1]["result"]["files"][0]["functions"][0]["type_parameters"].is_object());
    assert_eq!(output[1]["result"]["schema_version"], 5);
}

#[test]
fn files_are_complete_and_sorted_without_import_discovery() {
    let text = "import { identity } from \"./absent.zry\"; export function answer(): i32 { return identity<i32>(42); }";
    let files = [source("z/main.zry", text), source("a/extra.zry", "")];
    let (first, first_frames) = analyze(&json!({"schema_version":5,"files":files}));
    first.expect("unresolved imports are source syntax, never host reads");
    let (second, second_frames) = analyze(&json!({"schema_version":5,"files":[files[1],files[0]]}));
    second.expect("reversed caller order");
    assert_eq!(first_frames, second_frames);
    let raw = &first_frames[1]["result"];
    assert_eq!(raw["files"].as_array().expect("files").len(), 2);
    assert_eq!(raw["files"][0]["path"], "a/extra.zry");
    assert_eq!(raw["files"][0]["id"], 0);
    assert_eq!(raw["files"][1]["path"], "z/main.zry");
    assert_eq!(raw["files"][1]["id"], 1);
    assert_eq!(raw["files"][1]["imports"][0]["specifier"]["text"], "./absent.zry");
}

#[test]
fn request_unknown_fields_protocol_and_host_paths_reject_before_candidate() {
    for params in [
        json!({"schema_version":4,"files":[]}),
        json!({"schema_version":5,"files":[],"root":"/"}),
        json!({"schema_version":5,"files":[{"path":"a.zry","text":"","span":null}]}),
        json!({"schema_version":5,"files":[source("../outside.zry", "")]}),
        json!({"schema_version":5,"files":[source("/etc/passwd", "")]}),
        json!({"schema_version":5,"files":[source("A.zry", ""),source("a.zry", "")]}),
    ] {
        let (result, output) = analyze(&params);
        assert!(result.is_err());
        assert_eq!(output.len(), 1, "only exact handshake may precede atomic rejection");
    }
}

#[test]
fn lexical_and_parser_failures_reject_entire_project_without_recovery_dto() {
    for (text, code) in [
        ("export function broken(): i32 { return `forbidden`; }", "ZRYNA-F1501"),
        ("export const rejected = 1; export function answer(): i32 { return 42; }", "ZRYNA-F2002"),
        ("export function broken(): i32 { return Option<i32>.some(7); }", "ZRYNA-F2002"),
    ] {
        let (result, output) = analyze(&json!({"schema_version":5,"files":[
            source("a/valid.zry", "export function answer(): i32 { return 42; }"),
            source("z/broken.zry", text)]}));
        result.expect("parser failure has one correlated RPC rejection");
        assert_eq!(output.len(), 2);
        assert!(output[1].get("result").is_none());
        assert_eq!(output[1]["id"], 2);
        assert_eq!(output[1]["error"]["code"], code);
        assert!(output[1]["error"]["message"].as_str().expect("message").contains("file 1 bytes"));
    }
    let (result, output) = analyze(&json!({"schema_version":5,"files":[
        source("main.zry", "export function answer(): i32 { return 42; }")]}));
    result.expect("fresh valid session after rejection");
    assert!(output[1].get("result").is_some());
}

#[test]
fn input_duplicate_fields_framing_and_extra_request_fail_closed() {
    let handshake = "{\"id\":1,\"method\":\"handshake\"}\n";
    for input in [
        "{\"id\":1,\"method\":\"handshake\",\"capabilities\":{}}\n".to_owned(),
        "{\"id\":1,\"id\":1,\"method\":\"handshake\"}\n".to_owned(),
        format!(
            "{handshake}{{\"id\":2,\"method\":\"analyze\",\"params\":{{\"schema_version\":5,\"schema_version\":5,\"files\":[]}}}}\n"
        ),
        format!(
            "{handshake}{{\"id\":2,\"method\":\"analyze\",\"params\":{{\"schema_version\":5,\"files\":[{{\"path\":\"a.zry\",\"path\":\"b.zry\",\"text\":\"\"}}]}}}}\n"
        ),
        format!(
            "{handshake}{{\"id\":2,\"method\":\"analyze\",\"params\":{{\"schema_version\":5,\"files\":[]}}}}"
        ),
        format!("{handshake}{{}} {{}}\n"),
    ] {
        let mut output = Vec::new();
        assert!(run(5, &mut Cursor::new(input), &mut output).is_err());
        assert!(frames(&output).len() <= 1);
    }
    let analysis =
        "{\"id\":2,\"method\":\"analyze\",\"params\":{\"schema_version\":5,\"files\":[]}}\n";
    let mut output = Vec::new();
    assert!(
        run(5, &mut Cursor::new(format!("{handshake}{analysis}{analysis}")), &mut output).is_err()
    );
    assert_eq!(frames(&output).len(), 2, "consumer must require clean session exit");
}

#[test]
fn source_first_extra_limits_reject_before_analysis_output() {
    for files in [
        vec![source("a.zry", ""); MAX_SOURCE_FILES + 1],
        vec![source(&"a".repeat(MAX_SOURCE_PATH_BYTES + 1), "")],
        vec![source("a.zry", &" ".repeat(MAX_SOURCE_FILE_BYTES + 1))],
    ] {
        let (result, output) = analyze(&json!({"schema_version":5,"files":files}));
        assert!(result.is_err());
        assert_eq!(output.len(), 1);
    }
}

#[test]
fn emitted_dtos_remain_untrusted_at_unknown_field_and_span_barriers() {
    let text = "// π\nexport function answer(): i32 { return 42; }";
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
            .expect("exact source authority");
    let (result, output) = analyze(&json!({"schema_version":5,"files":[source("main.zry", text)]}));
    result.expect("native raw candidate");
    let original = output[1]["result"].clone();
    let mut unknown = original.clone();
    unknown["files"][0]["functions"][0]["semantic_authority"] = json!(true);
    assert!(syntax_v5::decode_snapshot(&serde_json::to_vec(&unknown).expect("JSON")).is_err());
    for span in [
        json!({"file":1,"start":6,"end":text.len()}),
        json!({"file":0,"start":4,"end":text.len()}),
        json!({"file":0,"start":6,"end":u32::MAX}),
    ] {
        let mut hostile = original.clone();
        hostile["files"][0]["functions"][0]["span"] = span;
        let raw = syntax_v5::decode_snapshot(&serde_json::to_vec(&hostile).expect("JSON"))
            .expect("closed raw source claim only");
        assert!(syntax_v5::verify_snapshot(raw, &sources).is_err());
    }
}

#[test]
fn inherited_project_source_budget_accepts_exact_and_rejects_first_extra() {
    let text = " ".repeat(MAX_SOURCE_FILE_BYTES);
    let count = native_lexer::MAX_SOURCE_BYTES_PER_PROJECT / MAX_SOURCE_FILE_BYTES;
    let mut files: Vec<_> =
        (0..count).map(|index| source(&format!("f{index}.zry"), &text)).collect();
    let (result, output) = analyze(&json!({"schema_version":5,"files":files}));
    result.expect("exact inherited project source-byte ceiling");
    assert_eq!(output[1]["result"]["files"].as_array().expect("files").len(), count);
    files.push(source("extra.zry", " "));
    let (result, output) = analyze(&json!({"schema_version":5,"files":files}));
    assert!(result.is_err());
    assert_eq!(output.len(), 1, "first-extra source byte prevents candidate output");
}

#[test]
fn existing_driver_stdio_entrypoint_keeps_protocol_five_closed() {
    assert_eq!(
        run_native_worker_stdio(5).expect_err("existing CLI route cannot activate v5").to_string(),
        "native worker protocol is unsupported"
    );
}
