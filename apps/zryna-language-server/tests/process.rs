//! Process-level coverage for the language-server transport boundary.

use std::{
    io::{BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use zryna_language_server::{read_frame, write_frame};

fn send(input: &mut impl Write, value: impl serde::Serialize) {
    let bytes = serde_json::to_vec(&value).unwrap_or_else(|error| panic!("fixture: {error}"));
    write_frame(input, &bytes).unwrap_or_else(|error| panic!("write frame: {error}"));
    input.flush().unwrap_or_else(|error| panic!("flush frame: {error}"));
}

fn receive(receiver: &mpsc::Receiver<Value>) -> Value {
    receiver
        .recv_timeout(Duration::from_secs(40))
        .unwrap_or_else(|error| panic!("server response: {error}"))
}

fn node_executable() -> PathBuf {
    if let Some(configured) = std::env::var_os("ZRYNA_NODE") {
        return std::fs::canonicalize(configured)
            .unwrap_or_else(|error| panic!("configured node path: {error}"));
    }
    let output = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .unwrap_or_else(|error| panic!("run node: {error}"));
    assert!(output.status.success(), "node path query failed");
    let path = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("node path encoding: {error}"));
    std::fs::canonicalize(path.trim()).unwrap_or_else(|error| panic!("node path: {error}"))
}

#[test]
fn stdio_process_preserves_revision_query_and_cancellation() {
    let compiler_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|error| panic!("compiler root: {error}"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_zryna-language-server"))
        .args(["--compiler-root"])
        .arg(compiler_root)
        .args(["--node"])
        .arg(node_executable())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn server: {error}"));
    let mut input = child.stdin.take().unwrap_or_else(|| panic!("server stdin"));
    let output = child.stdout.take().unwrap_or_else(|| panic!("server stdout"));
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut output = BufReader::new(output);
        while let Ok(Some(bytes)) = read_frame(&mut output) {
            let value = serde_json::from_slice(&bytes)
                .unwrap_or_else(|error| panic!("server JSON: {error}"));
            if sender.send(value).is_err() {
                break;
            }
        }
    });

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "rootUri":"file:///workspace","capabilities":{"general":{"positionEncodings":["utf-16"]}}
            }
        }),
    );
    assert_eq!(receive(&receiver)["result"]["capabilities"]["positionEncoding"], "utf-16");
    send(&mut input, json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    send(
        &mut input,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
                "uri":"file:///workspace/src/main.zry","languageId":"zryna","version":1,
                "text":"export function identity(x: i32): i32 { return x; }\n"
            }}
        }),
    );
    assert_eq!(receive(&receiver)["method"], "zryna/publishDiagnostics");
    assert_eq!(receive(&receiver)["method"], "textDocument/publishDiagnostics");

    assert_formatting(&mut input, &receiver);

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry"},
                "position":{"line":0,"character":47}
            }
        }),
    );
    let definition = receive(&receiver);
    assert_eq!(definition["result"]["uri"], "file:///workspace/src/main.zry");
    assert_eq!(
        definition["result"]["range"],
        json!({
            "start":{"line":0,"character":25},"end":{"line":0,"character":26}
        })
    );

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry"},
                "position":{"line":0,"character":47}
            }
        }),
    );
    send(&mut input, json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":3}}));
    assert_eq!(receive(&receiver)["error"]["code"], -32800);

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry","version":2},
                "contentChanges":[{"text":"export function identity(x): i32 { return x; }\n"}]
            }
        }),
    );
    let exact = receive(&receiver);
    assert_eq!(exact["method"], "zryna/publishDiagnostics");
    assert_eq!(exact["params"]["documents"][0]["version"], 2);
    assert_eq!(exact["params"]["report"]["diagnostics"][0]["code"], "ZRYNA-M1003");
    let standard = receive(&receiver);
    assert_eq!(standard["params"]["version"], 2);
    assert_eq!(standard["params"]["diagnostics"][0]["code"], "ZRYNA-M1003");

    send(&mut input, json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    assert!(receive(&receiver)["result"].is_null());
    send(&mut input, json!({"jsonrpc":"2.0","method":"exit"}));
    drop(input);
    let status = child.wait().unwrap_or_else(|error| panic!("wait server: {error}"));
    assert!(status.success(), "server failed: {status}");
    reader.join().unwrap_or_else(|_| panic!("reader thread"));
}

fn assert_formatting(input: &mut impl Write, receiver: &mpsc::Receiver<Value>) {
    send(
        input,
        json!({"jsonrpc":"2.0","id":88,"method":"textDocument/formatting",
            "params":{"textDocument":{"uri":"file:///workspace/src/main.zry"},
                "options":{"tabSize":2,"insertSpaces":true}}}),
    );
    assert_eq!(
        receive(receiver)["result"][0]["newText"],
        "export function identity(x: i32): i32 {\n  return x;\n}\n"
    );
}

#[test]
fn stdio_m2_routes_control_flow_and_rejects_invalid_edits() {
    let compiler_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("compiler root");
    let mut child = Command::new(env!("CARGO_BIN_EXE_zryna-language-server"))
        .args(["--compiler-root"])
        .arg(compiler_root)
        .args(["--node"])
        .arg(node_executable())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server");
    let mut input = child.stdin.take().expect("server stdin");
    let output = child.stdout.take().expect("server stdout");
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut output = BufReader::new(output);
        while let Ok(Some(bytes)) = read_frame(&mut output) {
            let value = serde_json::from_slice(&bytes).expect("server JSON");
            if sender.send(value).is_err() {
                break;
            }
        }
    });

    send(
        &mut input,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "rootUri":"file:///workspace","capabilities":{"general":{"positionEncodings":["utf-16"]}},
            "initializationOptions":{"zrynaProfile":"control-flow-v1"}
        }}),
    );
    let initialized = receive(&receiver);
    assert_eq!(
        initialized["result"]["capabilities"]["experimental"]["zrynaAnalysisProfile"],
        "control-flow-v1"
    );
    assert_eq!(initialized["result"]["capabilities"]["definitionProvider"], false);
    send(&mut input, json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    let loop_source = "export function main(count: i32): i32 { let i: i32 = 0; let sum: i32 = 0; while (i < count) { i = i + 1; sum = sum + i; } return sum; }\n";
    send(
        &mut input,
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
            "uri":"file:///workspace/src/main.zry","languageId":"zryna","version":1,"text":loop_source
        }}}),
    );
    let accepted = receive(&receiver);
    assert_eq!(accepted["method"], "zryna/publishDiagnostics");
    assert_eq!(accepted["params"]["report"]["diagnostics"], json!([]));
    assert_eq!(receive(&receiver)["params"]["version"], 1);

    send(
        &mut input,
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting",
        "params":{"textDocument":{"uri":"file:///workspace/src/main.zry"},
        "options":{"tabSize":2,"insertSpaces":true}}}),
    );
    let formatted = receive(&receiver);
    assert!(
        formatted["result"][0]["newText"].as_str().is_some_and(|text| text.contains("while ("))
    );

    let branch_and_call = "function twice(value: i32): i32 { return value * 2; }\nexport function main(positive: bool, value: i32): i32 { const doubled: i32 = twice(value); if (positive) { return doubled; } else { return -doubled; } }\n";
    send(
        &mut input,
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":"file:///workspace/src/main.zry","version":2},
            "contentChanges":[{"text":branch_and_call}]
        }}),
    );
    let accepted_branch = receive(&receiver);
    assert_eq!(accepted_branch["params"]["report"]["diagnostics"], json!([]));
    assert_eq!(receive(&receiver)["params"]["version"], 2);

    let invalid =
        "export function main(): i32 { const value: i32 = 3; value = 4; return value; }\n";
    send(
        &mut input,
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":"file:///workspace/src/main.zry","version":3},
            "contentChanges":[{"text":invalid}]
        }}),
    );
    let report = receive(&receiver);
    assert_eq!(report["params"]["documents"][0]["version"], 3);
    assert!(
        report["params"]["report"]["diagnostics"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["code"] == "ZRYNA-M2005"))
    );
    let standard = receive(&receiver);
    assert_eq!(standard["params"]["version"], 3);
    assert!(
        standard["params"]["diagnostics"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["code"] == "ZRYNA-M2005"))
    );
    send(
        &mut input,
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting",
        "params":{"textDocument":{"uri":"file:///workspace/src/main.zry"},
        "options":{"tabSize":2,"insertSpaces":true}}}),
    );
    let rejected = receive(&receiver);
    assert_eq!(rejected["error"]["data"]["code"], "ZRYNA-D4001");
    assert!(rejected.get("result").is_none());

    send(&mut input, json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}));
    assert!(receive(&receiver)["result"].is_null());
    send(&mut input, json!({"jsonrpc":"2.0","method":"exit"}));
    drop(input);
    assert!(child.wait().expect("wait server").success());
    reader.join().expect("reader thread");
}
