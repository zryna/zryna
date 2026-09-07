//! Executed `DataOwnershipV1` JavaScript backend checks.

use std::process::Command;

use zryna_semantics::data_ownership_v1::{SemanticInput, lower};
use zryna_source::{NormalizedSourcePath, SourceFileInput, SourceMap};
use zryna_syntax::v4::{decode_snapshot, verify_snapshot};

const SOURCE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.zry");
const RESPONSE: &str = include_str!("../../../tests/m3-fixtures/pair-score-v4.json");

fn verified(source: &str, response: &str) -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: source.to_owned(),
    }])
    .expect("source map");
    let raw = decode_snapshot(response.as_bytes()).expect("protocol-v4 snapshot");
    let syntax = verify_snapshot(raw, &sources).expect("verified syntax");
    let path = NormalizedSourcePath::new("src/main.zry").expect("path");
    let entry = sources.file_id(&path).expect("entry");
    lower(SemanticInput::try_new(&syntax, &sources, entry).expect("semantic input"))
        .expect("verified DataOwnershipV1")
}

fn fixture() -> zryna_semantics::data_ownership_v1::VerifiedProgram {
    verified(SOURCE, RESPONSE)
}

#[test]
fn deterministic_owned_aggregate_javascript_executes() {
    let program = fixture();
    let first =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    let second =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("repeated artifact");
    assert_eq!(first, second);
    assert!(first.source.contains("{$k:2,$v:[v[2],v[3]]}"));

    let script = format!("{}\nconsole.log($zryna$d0f0(2, 3));\n", first.source);
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8 output"), "65\n");
}

#[test]
fn exclusive_borrow_write_executes_before_lexical_end() {
    let program = verified(
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.zry"),
        include_str!("../../../tests/m3-fixtures/exclusive-root-borrow.json"),
    );
    let artifact =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    let script = format!("{}\nconsole.log($zryna$d0f0());\n", artifact.source);
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8 output"), "9\n");
}

#[test]
fn generated_source_is_capability_minimal() {
    let program = fixture();
    let artifact =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    for forbidden in ["eval(", "Function(", "globalThis", "process.", "require(", "import("] {
        assert!(!artifact.source.contains(forbidden), "forbidden capability {forbidden}");
    }
    assert!(artifact.source.ends_with('\n'));
}

#[test]
fn owned_string_vec_and_borrow_helpers_execute() {
    let program = verified(
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.zry"),
        include_str!("../../../tests/m3-fixtures/owned-root-borrow-reads.json"),
    );
    let artifact =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    let script = format!(
        "{}\nconsole.log($zryna$d0f0().$v);console.log(JSON.stringify($zryna$d0f1().$v));\n",
        artifact.source,
    );
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8"), "z\n[7,9]\n");
}

#[test]
fn recursive_clone_failure_releases_completed_prefix() {
    let program = fixture();
    let artifact =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    let script = format!(
        "{}\nconst a={{$s:1,$w:1,$p:1}},z={{$s:4294967295,$w:1,$p:2}};try{{$zryna$clone({{$k:2,$v:[{{$k:4,$c:a}},{{$k:4,$c:z}}]}})}}catch{{}}console.log(a.$s,z.$s);\n",
        artifact.source,
    );
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout).expect("UTF-8"), "1 4294967295\n");
}

#[test]
fn string_allocation_classification_is_bounded_and_recoverable() {
    let program = fixture();
    let artifact =
        zryna_backend_javascript::emit_data_ownership(program.verified_ir(), program.runtime_abi())
            .expect("JavaScript artifact");
    let script = format!(
        r#"{}
const cases = [
  [67108863, 0], [67108864, 0], [67108865, 0],
  [2147483647, 0], [2147483647, 1], [-1, 0], [1.5, 0]
];
const rows = cases.map(([left, right]) => {{
  $zryna$status = 0;
  try {{ return [$zryna$stringAllocationSize(left, right), $zryna$status]; }}
  catch (failure) {{
    return [failure === $zryna$sentinel ? $zryna$status : failure.message, $zryna$status];
  }}
}});
$zryna$status = 0;
rows.push([$zryna$concat({{$k:1,$v:"hé"}},{{$k:1,$v:"!"}}).$v, $zryna$status]);
console.log(JSON.stringify(rows));
"#,
        artifact.source,
    );
    let output = Command::new("node")
        .args(["--input-type=module", "--eval", &script])
        .output()
        .expect("Node.js");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8(output.stdout).expect("classification rows"),
        "[[67108863,0],[67108864,0],[2,2],[2,2],[3,3],[\"ZRYNA-R3ABI\",0],[\"ZRYNA-R3ABI\",0],[\"hé!\",0]]\n"
    );
}
