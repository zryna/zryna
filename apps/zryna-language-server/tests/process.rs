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
