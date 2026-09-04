use super::child_preparation_red::{replace_literal_with_reference, state, unresolved};
use super::*;
use serde_json::{Value, json};

// This fixture writer emits only two fixed source shapes and their untrusted syntax.
// Every use authenticates the complete source/snapshot; the valid controls additionally
// require the ordinary independent full IR verifier, not a test interpretation.
#[derive(Default)]
struct FixtureWriter {
    source: String,
    types: Vec<Value>,
    expressions: Vec<Value>,
    leaves: Vec<usize>,
}

fn at(start: usize, end: usize) -> Value {
    json!({"file": 0, "start": start, "end": end})
}

impl FixtureWriter {
    fn token(&mut self, text: &str) -> Value {
        let start = self.source.len();
        self.source.push_str(text);
        at(start, self.source.len())
    }

    fn identifier(&mut self, text: &str) -> Value {
        json!({"text": text, "span": self.token(text)})
    }

    fn array_type(&mut self, depth: usize) -> usize {
        let start = self.source.len();
        let kind = if depth == 0 {
            json!({"kind": "string", "keyword_span": self.token("String")})
        } else {
            let keyword = self.token("FixedArray");
            let less = self.token("<");
            let element = self.array_type(depth - 1);
            let comma = self.token(",");
            self.token(" ");
            let length = self.token("2");
            let greater = self.token(">");
            json!({"kind": "fixed-array", "keyword_span": keyword,
                "less_than_span": less, "element": element, "comma_span": comma,
                "length_span": length, "length": 2, "length_spelling": "2",
                "greater_than_span": greater})
        };
        let id = self.types.len();
        self.types.push(json!({"span": at(start, self.source.len()), "kind": kind}));
        id
    }

    fn array_expression(&mut self, depth: usize) -> usize {
        let start = self.source.len();
        let kind = if depth == 0 {
            let spelling = format!(
                "\"{}\"",
                char::from(
                    b'a' + u8::try_from(self.leaves.len()).expect("at most four fixture leaves")
                )
            );
            self.token(&spelling);
            self.leaves.push(self.expressions.len());
            json!({"kind": "string-literal", "spelling": spelling})
        } else {
            let ty = self.array_type(depth);
            let open_paren = self.token("(");
            let open_bracket = self.token("[");
            let first = self.array_expression(depth - 1);
            self.token(", ");
            let second = self.array_expression(depth - 1);
            let close_bracket = self.token("]");
            let close_paren = self.token(")");
            json!({"kind": "fixed-array-construction", "type_syntax": ty,
                "open_paren_span": open_paren, "open_bracket_span": open_bracket,
                "elements": [first, second], "close_bracket_span": close_bracket,
                "close_paren_span": close_paren})
        };
        let id = self.expressions.len();
        self.expressions.push(json!({"span": at(start, self.source.len()), "kind": kind}));
        id
    }

    fn declaration(&mut self) -> Value {
        let start = self.source.len();
        let interface = self.token("interface");
        self.token(" ");
        let name = self.identifier("Bundle");
        self.token(" ");
        let extends = self.token("extends");
        self.token(" ");
        let marker = self.token("ZrynaEnum");
        self.token(" ");
        let open = self.token("{");
        self.token(" ");
        let variant_start = self.source.len();
        let variant = self.identifier("some");
        let colon = self.token(":");
        self.token(" ");
        let payload = self.array_type(1);
        let semi = self.token(";");
        let variant_end = self.source.len();
        self.token(" ");
        let close = self.token("}");
        let end = self.source.len();
        self.token("\n");
        json!({"span": at(start, end), "export_span": null, "kind": {
            "kind": "enum", "interface_span": interface, "name": name,
            "extends_span": extends, "marker_span": marker, "open_brace_span": open,
            "close_brace_span": close, "variants": [{"span": at(variant_start, variant_end),
                "name": variant, "colon_span": colon, "semicolon_span": semi,
                "payload_type": payload, "none_span": null}]}})
    }

    fn enum_expression(&mut self) -> usize {
        let start = self.source.len();
        let name = self.identifier("Bundle");
        let dot = self.token(".");
        let variant = self.identifier("some");
        let open = self.token("(");
        let payload = self.array_expression(1);
        let close = self.token(")");
        let id = self.expressions.len();
        self.expressions.push(json!({"span": at(start, self.source.len()), "kind": {
            "kind": "enum-construction", "type_name": name, "dot_span": dot,
            "variant": variant, "open_paren_span": open, "payload": payload,
            "close_paren_span": close}}));
        id
    }
}

fn fixture(enum_root: bool) -> (String, RawProjectSyntaxSnapshot, Vec<usize>) {
    let mut writer = FixtureWriter::default();
    let declarations = if enum_root { vec![writer.declaration()] } else { vec![] };
    let function_start = writer.source.len();
    let function = writer.token("function");
    writer.token(" ");
    let name = writer.identifier("make");
    writer.token("(): ");
    let result_type = if enum_root {
        let start = writer.source.len();
        let name = writer.identifier("Bundle");
        let id = writer.types.len();
        writer.types.push(json!({"span": at(start, writer.source.len()),
            "kind": {"kind": "named", "name": name}}));
        id
    } else {
        writer.array_type(2)
    };
    writer.token(" ");
    let body_start = writer.source.len();
    let open = writer.token("{");
    writer.token(" ");
    let return_start = writer.source.len();
    let keyword = writer.token("return");
    writer.token(" ");
    let value = if enum_root { writer.enum_expression() } else { writer.array_expression(2) };
    let semi = writer.token(";");
    let return_end = writer.source.len();
    writer.token(" ");
    let close = writer.token("}");
    let end = writer.source.len();
    let snapshot = serde_json::from_value(json!({"schema_version": 4, "files": [{
        "id": 0, "path": "src/main.zry", "imports": [], "type_syntax": writer.types,
        "data_declarations": declarations, "functions": [{
            "span": at(function_start, end), "export_span": null, "function_span": function,
            "name": name, "parameters": [], "result_type": result_type, "body": {
                "span": at(body_start, end), "root_block": 0, "blocks": [{
                    "span": at(body_start, end), "open_brace_span": open,
                    "statements": [0], "close_brace_span": close}],
                "statements": [{"span": at(return_start, return_end), "kind": {
                    "kind": "return", "keyword_span": keyword, "value": value,
                    "semicolon_span": semi}}], "expressions": writer.expressions}}]}],
        "diagnostics": []}))
    .expect("fixed source fixture schema");
    (writer.source, snapshot, writer.leaves)
}

fn failure(enum_root: bool, first_invalid: bool) {
    let (mut source, mut snapshot, leaves) = fixture(enum_root);
    let expressions = &mut snapshot.files[0].functions[0].body.expressions;
    let later = replace_literal_with_reference(
        &mut source,
        &mut expressions[*leaves.last().expect("later leaf")],
        "bad",
    );
    let (failed, name) = if first_invalid {
        (replace_literal_with_reference(&mut source, &mut expressions[leaves[0]], "nil"), "nil")
    } else {
        (later, "bad")
    };
    let (mut before, mut after, mut expected) = (None, None, None);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        assert!(lowerer.errors.is_empty());
        before = Some(state(lowerer));
        expected = Some(unresolved(span(lowerer.input.sources(), failed), name));
        assert!(lowerer.value(root_value(lowerer, 0), result).is_none());
        after = Some(state(lowerer));
    });
    assert_eq!(errors, [expected.expect("exact source-bound first diagnostic")]);
    // Later failures are intentionally red before complete C2 preparation; first-child
    // controls pin unchanged state and diagnostic precedence without partial emission.
    assert_eq!(after, before, "C2 rejected child preparation must preserve complete real state");
}

#[test]
fn constructor_child_matrix_nested_array_later_child_is_atomic() {
    failure(false, false);
}

#[test]
fn constructor_child_matrix_enum_array_payload_later_child_is_atomic() {
    failure(true, false);
}

#[test]
fn constructor_child_matrix_nested_array_first_error_precedes_later_error() {
    failure(false, true);
}

#[test]
fn constructor_child_matrix_enum_array_payload_first_error_precedes_later_error() {
    failure(true, true);
}

#[test]
fn constructor_child_matrix_valid_nested_sources_replay_through_full_verifier() {
    use zryna_ir::data_ownership_v1::VerifiedInstructionKind::{
        EnumConstruct, FixedArrayConstruct, StringFromUtf8,
    };
    for enum_root in [false, true] {
        let (source, snapshot, _) = fixture(enum_root);
        let sources = fixtures::sources(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("source-authenticated matrix");
        let mut previous = None;
        for _ in 0..2 {
            let program = ownership::lower(fixtures::input(&syntax, &sources))
                .expect("ordinary mandatory full IR verification");
            let kinds = program
                .modules()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
                .flat_map(zryna_ir::data_ownership_v1::VerifiedFunction::blocks)
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>();
            let expected = if enum_root {
                vec![StringFromUtf8, StringFromUtf8, FixedArrayConstruct, EnumConstruct]
            } else {
                vec![
                    StringFromUtf8,
                    StringFromUtf8,
                    FixedArrayConstruct,
                    StringFromUtf8,
                    StringFromUtf8,
                    FixedArrayConstruct,
                    FixedArrayConstruct,
                ]
            };
            assert_eq!(kinds, expected, "exact admitted nested constructor route and order");
            if let Some(previous) = &previous {
                assert_eq!(&kinds, previous);
            }
            previous = Some(kinds);
        }
    }
}
