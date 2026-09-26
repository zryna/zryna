use std::{ffi::OsString, path::PathBuf, process::Command};

use serde_json::Value;
use zryna_frontend::{ProviderExpectationV3, WorkerFrontendV3, WorkerLimitsV3, WorkerSpecV3};
use zryna_source::{SourceFileInput, SourceMap};

use super::{FormattingDocument, layout};

fn frontend() -> WorkerFrontendV3 {
    let output = Command::new("node").args(["-p", "process.execPath"]).output().expect("node path");
    assert!(output.status.success());
    let node = PathBuf::from(String::from_utf8(output.stdout).expect("node UTF-8").trim());
    let expected = ProviderExpectationV3::new("typescript-6", "6.0.3").expect("provider");
    WorkerFrontendV3::new(
        WorkerSpecV3::new(
            node,
            vec![OsString::from("src/worker-v3.mjs")],
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../adapters/typescript-6"),
            expected,
            WorkerLimitsV3::default(),
        )
        .expect("worker"),
    )
}

fn sources(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("source map")
}

fn erase_coordinates(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|key, _| !key.ends_with("span") && key != "source_map_identity");
            for value in object.values_mut() {
                erase_coordinates(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                erase_coordinates(value);
            }
        }
        _ => {}
    }
}

#[test]
fn m2_formatting_is_canonical_and_preserves_verified_meaning() {
    let input = concat!(
        "// 🌱 keep\r\nfunction step(x:i32):i32{return x+1;}\n",
        "export function total(limit:i32):i32{let n:i32=0;let acc:i32=0;",
        "while(n<limit){const next:i32=step(n);if(next>0){acc=acc+next;}else{acc=acc-1;}",
        "n=n+1; // exact  comment\r\n}return acc;}\n",
    );
    let expected = concat!(
        "// 🌱 keep\nfunction step(x: i32): i32 {\n  return x + 1;\n}\n",
        "export function total(limit: i32): i32 {\n  let n: i32 = 0;\n",
        "  let acc: i32 = 0;\n  while (n < limit) {\n",
        "    const next: i32 = step(n);\n    if (next > 0) {\n",
        "      acc = acc + next;\n    } else {\n      acc = acc - 1;\n    }\n",
        "    n = n + 1;\n    // exact  comment\n  }\n  return acc;\n}\n",
    );
    let frontend = frontend();
    let original = sources(input);
    let syntax = frontend.analyze_verified_v3(&original).expect("verified M2 syntax");
    let entry = original.verify_file_id(0).expect("entry");
    let semantic_input =
        zryna_semantics::control_flow_v1::SemanticInput::try_new(&syntax, &original, entry)
            .expect("semantic input");
    let original_ir =
        zryna_semantics::control_flow_v1::lower(semantic_input).expect("M2 semantics");
    let document = FormattingDocument::prepare_control_flow(&syntax, &original).expect("plan");
    assert_eq!(document.complete, expected);
    assert_eq!(layout::format_control_flow(expected).as_deref(), Some(expected));

    let formatted = sources(&document.complete);
    let reparsed = frontend.analyze_verified_v3(&formatted).expect("reparsed M2 syntax");
    let entry = formatted.verify_file_id(0).expect("entry");
    let semantic_input =
        zryna_semantics::control_flow_v1::SemanticInput::try_new(&reparsed, &formatted, entry)
            .expect("reparsed semantic input");
    let formatted_ir =
        zryna_semantics::control_flow_v1::lower(semantic_input).expect("reparsed M2 semantics");
    let before = zryna_backend_javascript::emit_control_flow(&original_ir).expect("original ESM");
    let after = zryna_backend_javascript::emit_control_flow(&formatted_ir).expect("formatted ESM");
    assert_eq!(before.source, after.source);
    let mut before = serde_json::to_value(&syntax).expect("original syntax value");
    let mut after = serde_json::to_value(&reparsed).expect("formatted syntax value");
    erase_coordinates(&mut before);
    erase_coordinates(&mut after);
    assert_eq!(before, after);
    assert_eq!(
        FormattingDocument::prepare_control_flow(&reparsed, &formatted)
            .expect("second plan")
            .complete,
        expected
    );
}

#[test]
fn m2_range_layout_preserves_bytes_outside_the_selected_function() {
    let input = "// 😀\r\nfunction step(x:i32):i32{return x+1;} /*keep*/ export function run(x:i32):i32{return step(x);}\r\n";
    let source = sources(input);
    let syntax = frontend().analyze_verified_v3(&source).expect("verified syntax");
    let document = FormattingDocument::prepare_control_flow(&syntax, &source).expect("plan");
    let first = &document.functions[0];
    let original = &input[first.start as usize..first.end as usize];
    let formatted = layout::format_control_flow(original).expect("first function layout");
    let replacement = formatted.strip_suffix('\n').expect("trailing newline");
    let result = format!(
        "{}{}{}",
        &input[..first.start as usize],
        replacement,
        &input[first.end as usize..]
    );
    assert!(result.starts_with("// 😀\r\nfunction step(x: i32): i32 {\n"));
    assert!(result.ends_with(" /*keep*/ export function run(x:i32):i32{return step(x);}\r\n"));
    assert_eq!(document.functions.len(), 2);
}

#[test]
fn m2_operator_boundaries_survive_formatting() {
    let cases = [
        (
            "export function neg(x:i32):i32{return - -x;}",
            "export function neg(x: i32): i32 {\n  return - -x;\n}\n",
        ),
        (
            "export function compare(a:i32,b:i32):bool{const equal:bool=a===b;if(equal){return true;}else{return a!==b;}}",
            "export function compare(a: i32, b: i32): bool {\n  const equal: bool = a === b;\n  if (equal) {\n    return true;\n  } else {\n    return a !== b;\n  }\n}\n",
        ),
    ];
    let frontend = frontend();
    for (input, expected) in cases {
        let original = sources(input);
        let syntax = frontend.analyze_verified_v3(&original).expect("verified operators");
        let original_input = zryna_semantics::control_flow_v1::SemanticInput::try_new(
            &syntax,
            &original,
            original.verify_file_id(0).expect("entry"),
        )
        .expect("original semantic input");
        let original_ir =
            zryna_semantics::control_flow_v1::lower(original_input).expect("semantics");
        let document = FormattingDocument::prepare_control_flow(&syntax, &original).expect("plan");
        assert_eq!(document.complete, expected);
        let formatted = sources(expected);
        let reparsed = frontend.analyze_verified_v3(&formatted).expect("reparsed operators");
        let formatted_input = zryna_semantics::control_flow_v1::SemanticInput::try_new(
            &reparsed,
            &formatted,
            formatted.verify_file_id(0).expect("entry"),
        )
        .expect("formatted semantic input");
        let formatted_ir =
            zryna_semantics::control_flow_v1::lower(formatted_input).expect("semantics");
        assert_eq!(
            zryna_backend_javascript::emit_control_flow(&original_ir).expect("original ESM").source,
            zryna_backend_javascript::emit_control_flow(&formatted_ir)
                .expect("formatted ESM")
                .source,
        );
    }
}
