use super::{change, emit, initialized, open, request, semantic_initialized};
use serde_json::{Value, json};
use zryna_language_server::{RevisionCompiler, Server};

const TEXT: &str = "export function a(x: i32): i32 { return x; }\n";

fn format(server: &mut Server<impl RevisionCompiler>, id: i64, range: Option<Value>) {
    let mut params = json!({"textDocument":{"uri":"file:///workspace/src/main.zry"},
        "options":{"tabSize":8,"insertSpaces":false}});
    let method = if let Some(range) = range {
        params["range"] = range;
        "textDocument/rangeFormatting"
    } else {
        "textDocument/formatting"
    };
    assert!(
        request(server, json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .is_empty()
    );
}

fn finish(server: &mut Server<impl RevisionCompiler>) -> Vec<Value> {
    let output = server.finish_pending();
    emit(server, output)
}

#[test]
fn formatting_returns_exact_document_and_range_edits() {
    let mut server = semantic_initialized();
    open(&mut server, 1, TEXT);
    format(&mut server, 2, None);
    let output = finish(&mut server);
    assert_eq!(
        output[0]["result"],
        json!([{
            "range":{"start":{"line":0,"character":0},"end":{"line":1,"character":0}},
            "newText":"export function a(x: i32): i32 {\n  return x;\n}\n"
        }])
    );
    format(
        &mut server,
        3,
        Some(json!({"start":{"line":0,"character":0},"end":{"line":0,"character":44}})),
    );
    let output = finish(&mut server);
    assert_eq!(
        output[0]["result"][0]["newText"],
        "export function a(x: i32): i32 {\n  return x;\n}"
    );
    assert_eq!(output[0]["result"][0]["range"]["end"]["character"], 44);
}

#[test]
fn formatting_rejects_unready_partial_foreign_and_invalid_coordinates_without_result() {
    let mut server = initialized();
    open(&mut server, 1, "class Unknown {}");
    format(&mut server, 2, None);
    let output = finish(&mut server);
    assert_eq!(output[0]["error"]["data"]["code"], "ZRYNA-D4001");
    assert!(output[0].get("result").is_none());
    let mut server = semantic_initialized();
    open(&mut server, 1, TEXT);
    format(
        &mut server,
        3,
        Some(json!({"start":{"line":0,"character":1},"end":{"line":0,"character":44}})),
    );
    assert_eq!(finish(&mut server)[0]["error"]["data"]["code"], "ZRYNA-D4003");
    let output = request(
        &mut server,
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/rangeFormatting",
        "params":{"textDocument":{"uri":"file:///workspace/src/main.zry"},"options":{"tabSize":2,"insertSpaces":true},
        "range":{"start":{"line":9,"character":0},"end":{"line":9,"character":0}}}}),
    );
    assert_eq!(output[0]["error"]["data"]["code"], "ZRYNA-D4003");
}

#[test]
fn formatting_cancel_change_close_and_recovery_are_revision_bound() {
    let mut server = semantic_initialized();
    open(&mut server, 1, TEXT);
    format(&mut server, 2, None);
    request(&mut server, json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":2}}));
    assert_eq!(finish(&mut server)[0]["error"]["code"], -32800);
    format(&mut server, 3, None);
    change(&mut server, 2, &TEXT.replace("function a", "function b"));
    assert_eq!(finish(&mut server)[0]["error"]["code"], -32801);
    format(&mut server, 4, None);
    assert!(
        finish(&mut server)[0]["result"][0]["newText"]
            .as_str()
            .expect("text")
            .contains("function b")
    );
    format(&mut server, 5, None);
    request(
        &mut server,
        json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":"file:///workspace/src/main.zry"}}}),
    );
    assert_eq!(finish(&mut server)[0]["error"]["code"], -32801);
}

#[test]
fn formatting_rechecks_revision_at_actual_transport_emission() {
    let mut server = semantic_initialized();
    open(&mut server, 1, TEXT);
    format(&mut server, 2, None);
    let mut outgoing = server.finish_pending();
    let response = outgoing.pop().expect("completed format");
    assert!(response.value().get("result").is_some());
    change(&mut server, 2, &TEXT.replace("function a", "function b"));
    let mut bytes = Vec::new();
    server.write_outgoing(&mut bytes, response).expect("emission");
    let frame = zryna_language_server::read_frame(&mut std::io::Cursor::new(bytes))
        .expect("frame")
        .expect("response");
    let value: Value = serde_json::from_slice(&frame).expect("JSON");
    assert_eq!(value["error"]["code"], -32801);
    assert!(value.get("result").is_none());
}
