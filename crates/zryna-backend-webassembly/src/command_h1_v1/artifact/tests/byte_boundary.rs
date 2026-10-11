//! Complete public H1 factory evidence from ordinary, independently authenticated source.

use super::Artifact;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use wasm_encoder::{Component, RawSection};
use wasmparser::{Parser, Payload, Validator, WasmFeatures};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::{command_h1_v1, v4};

const LIMIT: usize = 1_048_576;
const LITERAL_BYTES: usize = 87_044;

fn span(start: usize, end: usize) -> Value {
    json!({"file": 0, "start": start, "end": end})
}

fn token(text: &str, start: usize, spelling: &str) -> Value {
    let at = start + text[start..].find(spelling).expect("actual source token");
    span(at, at + spelling.len())
}

fn identifier(text: &str, start: usize, spelling: &str) -> Value {
    json!({"text": spelling, "span": token(text, start, spelling)})
}

fn return_statement(text: &str, start: usize, value: u32) -> Value {
    let at = start + text[start..].find("return").expect("source return");
    let end = at + text[at..].find(';').expect("source semicolon") + 1;
    json!({"span": span(at, end), "kind": {
        "kind": "return", "keyword_span": span(at, at + 6), "value": value,
        "semicolon_span": span(end - 1, end)
    }})
}

fn function(
    text: &str,
    start: usize,
    name: &str,
    result_type: u32,
    statements: Vec<Value>,
    expressions: Vec<Value>,
) -> Value {
    let open = start + text[start..].find('{').expect("body opening");
    let close = open + text[open..].find('}').expect("body closing");
    let ids: Vec<usize> = (0..statements.len()).collect();
    json!({
        "span": span(start, close + 1),
        "export_span": if name == "main" { token(text, start, "export") } else { Value::Null },
        "function_span": token(text, start, "function"),
        "name": identifier(text, start, name), "parameters": [], "result_type": result_type,
        "body": {"span": span(open, close + 1), "root_block": 0,
            "blocks": [{"span": span(open, close + 1),
                "open_brace_span": span(open, open + 1), "statements": ids,
                "close_brace_span": span(close, close + 1)}],
            "statements": Value::Array(statements), "expressions": Value::Array(expressions)}
    })
}

fn source(wide_bytes: usize) -> (SourceMap, zryna_semantics::command_h1_v1::VerifiedProgram) {
    // Both are ordinary ASCII UTF-8 bytes. 'a' requires one more signed-LEB byte
    // than '!' in the emitted I32Const, without adding a source expression.
    let literal = format!("{}{}", "a".repeat(wide_bytes), "!".repeat(LITERAL_BYTES - wide_bytes));
    let text = format!(
        "function retained(): bool {{ const payload: String = \"{literal}\"; return true; }}\n\
         export function main(): bool {{ return true; }}\n"
    );
    assert!(text.len() < zryna_source::MAX_SOURCE_FILE_BYTES);
    let main = text.find("export").expect("main source declaration");
    let local = text.find("const").expect("owned local declaration");
    let quote = text.find('"').expect("String literal opening");
    let quote_end = quote + literal.len() + 2;
    let named_bool = |start| {
        json!({"span": token(&text, start, "bool"), "kind": {
            "kind": "named", "name": identifier(&text, start, "bool")
        }})
    };
    let bool_expression = |start| {
        json!({"span": token(&text, start, "true"),
        "kind": {"kind": "bool-literal", "value": true}})
    };
    let raw = json!({"schema_version": 4, "diagnostics": [], "files": [{
        "id": 0, "path": "src/main.zry", "imports": [], "data_declarations": [],
        "type_syntax": [named_bool(0), {"span": token(&text, local, "String"),
            "kind": {"kind": "string", "keyword_span": token(&text, local, "String")}},
            named_bool(main)],
        "functions": [function(&text, 0, "retained", 0, vec![
            json!({"span": span(local, quote_end + 1), "kind": {
                "kind": "local-declaration", "keyword_span": span(local, local + 5),
                "mutable": false, "name": identifier(&text, local, "payload"),
                "type_syntax": 1, "equals_span": token(&text, local, "="),
                "initializer": 0, "semicolon_span": span(quote_end, quote_end + 1)
            }}), return_statement(&text, 0, 1)], vec![
                json!({"span": span(quote, quote_end), "kind": {
                    "kind": "string-literal", "spelling": &text[quote..quote_end]
                }}), bool_expression(0)]),
            function(&text, main, "main", 2, vec![return_statement(&text, main, 0)],
                vec![bool_expression(main)])]
    }]});
    let sources = SourceMap::build(vec![SourceFileInput { path: "src/main.zry".into(), text }])
        .expect("bounded actual source map");
    let bytes = serde_json::to_vec(&raw).expect("untrusted provider-shaped claim");
    let syntax =
        v4::verify_snapshot(v4::decode_snapshot(&bytes).expect("bounded decode"), &sources)
            .expect("every provider span and literal is authenticated against source");
    let admitted = command_h1_v1::admit(&syntax, &sources).expect("whole public H1 source");
    assert!(admitted.environment().is_none(), "pure source needs no grant");
    let program = zryna_semantics::command_h1_v1::lower(&admitted, &sources)
        .expect("whole body, owned cleanup and mandatory source-bound IR");
    (sources, program)
}

fn replace_language(artifact: &Artifact, language: &[u8]) -> Vec<u8> {
    // Reencode the exact retained component sections, replacing only the whole
    // source-emitted language core; no padding, custom section or appended bytes.
    let bytes = artifact.bytes();
    let mut component = Component::new();
    let mut depth = 0;
    let mut replacements = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.expect("retained component syntax");
        if depth == 0
            && let Some((id, range)) = payload.as_section()
        {
            let start = usize::try_from(range.start).expect("bounded section start");
            let end = usize::try_from(range.end).expect("bounded section end");
            let original = &bytes[start..end];
            let data = if id == 1 && original == artifact.language() {
                replacements += 1;
                language
            } else {
                original
            };
            component.section(&RawSection { id, data });
        }
        match payload {
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => depth += 1,
            Payload::End(_) if depth > 0 => depth -= 1,
            _ => {}
        }
    }
    assert_eq!(replacements, 1);
    component.finish()
}

#[test]
fn public_h1_factory_accepts1048576_and_rejects_source_emitted1048577_then_recovers() {
    let (sources, program) = source(4);
    let wit = crate::pinned_wit_sources();
    let artifact = Artifact::emit(program.verified_ir(), program.runtime_abi(), &sources, &wit)
        .expect("public canonical factory admits exact component byte ceiling");
    assert_eq!(artifact.bytes().len(), LIMIT);
    assert!(artifact.language().len() < LIMIT, "component guard owns this boundary");
    assert_eq!(artifact.issuer(), program.verified_ir().identity());
    assert!(artifact.program().source().is_bound_to(&sources));
    let source_text = sources
        .source(program.verified_ir().source().syntax().files()[0].id())
        .expect("retained source text")
        .text();
    assert_eq!(artifact.source_digest(), &<[u8; 32]>::from(Sha256::digest(source_text.as_bytes())));
    assert_eq!(artifact.component_digest(), &<[u8; 32]>::from(Sha256::digest(artifact.bytes())));
    artifact
        .revalidate(program.verified_ir(), program.runtime_abi(), &sources, &wit)
        .expect("every component byte reconstructed by the exact issuing public factory");
    assert_eq!(replace_language(&artifact, artifact.language()), artifact.bytes());

    let (extra_sources, extra_program) = source(5);
    let language = crate::data_ownership_v1::encode::command(extra_program.verified_ir())
        .expect("next source emits a genuine canonical language core");
    super::super::language_audit::audit(&language, extra_program.verified_ir())
        .expect("source-emitted first extra independently passes language admission");
    assert_eq!(language.len(), artifact.language().len() + 1);
    let extra = replace_language(&artifact, &language);
    assert_eq!(extra.len(), LIMIT + 1);
    Validator::new_with_features(WasmFeatures::WASM1.union(WasmFeatures::COMPONENT_MODEL))
        .validate_all(&extra)
        .expect("first extra is a valid complete component");
    let world = crate::wit_world_audit::AuthenticatedCommandWorld::new(&wit)
        .expect("exact authenticated world");
    assert_eq!(
        super::super::component_audit::audit(&extra, artifact.storage(), &language, &world)
            .expect_err("independent complete-component guard rejects first extra")
            .code(),
        "ZRYNA-W4104"
    );
    assert_eq!(
        Artifact::emit(
            extra_program.verified_ir(),
            extra_program.runtime_abi(),
            &extra_sources,
            &wit
        )
        .err()
        .expect("public factory rejects source-emitted first extra")
        .code(),
        "ZRYNA-W4102"
    );
    artifact
        .revalidate(program.verified_ir(), program.runtime_abi(), &sources, &wit)
        .expect("source-bound exact-boundary factory recovers after rejection");
}
