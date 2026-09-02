use super::*;

fn empty_declaration_file(file: u32, first: usize, count: usize) -> (String, RawSourceUnit) {
    let mut text = String::new();
    let mut declarations = Vec::with_capacity(count);
    let mut types = Vec::with_capacity(count);
    for number in first..first + count {
        let name = format!("T{number}");
        let start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str("interface ");
        let name_start = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(&name);
        let name_end = u32::try_from(text.len()).expect("fixture offset");
        text.push_str(" extends ZrynaStruct { x: i32; }\n");
        let end = u32::try_from(text.len() - 1).expect("fixture offset");
        let extends_start = name_end + 1;
        let marker_start = extends_start + 8;
        let open = marker_start + 12;
        let field_start = open + 2;
        let colon = field_start + 1;
        let type_start = colon + 2;
        let semicolon = type_start + 3;
        let type_id = u32::try_from(types.len()).expect("type id");
        types.push(RawTypeSyntax {
            span: zryna_source::UntrustedSpan { file, start: type_start, end: type_start + 3 },
            kind: RawTypeSyntaxKind::Named {
                name: RawIdentifierSyntax {
                    text: "i32".to_owned(),
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: type_start,
                        end: type_start + 3,
                    },
                },
            },
        });
        declarations.push(RawDataDeclaration {
            span: zryna_source::UntrustedSpan { file, start, end },
            export_span: None,
            kind: RawDataDeclarationKind::Struct {
                interface_span: zryna_source::UntrustedSpan { file, start, end: start + 9 },
                name: RawIdentifierSyntax {
                    text: name,
                    span: zryna_source::UntrustedSpan { file, start: name_start, end: name_end },
                },
                extends_span: zryna_source::UntrustedSpan {
                    file,
                    start: extends_start,
                    end: extends_start + 7,
                },
                marker_span: zryna_source::UntrustedSpan {
                    file,
                    start: marker_start,
                    end: marker_start + 11,
                },
                open_brace_span: zryna_source::UntrustedSpan { file, start: open, end: open + 1 },
                fields: vec![RawDataField {
                    span: zryna_source::UntrustedSpan {
                        file,
                        start: field_start,
                        end: semicolon + 1,
                    },
                    name: RawIdentifierSyntax {
                        text: "x".to_owned(),
                        span: zryna_source::UntrustedSpan {
                            file,
                            start: field_start,
                            end: field_start + 1,
                        },
                    },
                    colon_span: zryna_source::UntrustedSpan { file, start: colon, end: colon + 1 },
                    type_syntax: type_id,
                    semicolon_span: zryna_source::UntrustedSpan {
                        file,
                        start: semicolon,
                        end: semicolon + 1,
                    },
                }],
                close_brace_span: zryna_source::UntrustedSpan {
                    file,
                    start: semicolon + 2,
                    end: semicolon + 3,
                },
            },
        });
    }
    let path = if file == 0 { "src/a.zry" } else { "src/b.zry" };
    (
        text,
        RawSourceUnit {
            id: file,
            path: path.to_owned(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: declarations,
            functions: Vec::new(),
        },
    )
}

#[test]
fn nominal_declaration_budget_is_exact_and_plus_one_fails_m3201() {
    let (exact_text, exact_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let exact_sources =
        SourceMap::build(vec![SourceFileInput { path: "src/a.zry".to_owned(), text: exact_text }])
            .expect("exact source map");
    let exact_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![exact_file],
            diagnostics: Vec::new(),
        },
        &exact_sources,
    )
    .expect("exact v4 budget");
    let entry_path = NormalizedSourcePath::new("src/a.zry").expect("entry path");
    let entry = exact_sources.file_id(&entry_path).expect("entry");
    lower(SemanticInput::try_new(&exact_syntax, &exact_sources, entry).expect("exact input"))
        .expect("the exact nominal declaration budget must verify");

    let (first_text, first_file) =
        empty_declaration_file(0, 0, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS);
    let (last_text, last_file) =
        empty_declaration_file(1, zryna_ir::data_ownership_v1::MAX_NOMINAL_DECLARATIONS, 1);
    let plus_sources = SourceMap::build(vec![
        SourceFileInput { path: "src/a.zry".to_owned(), text: first_text },
        SourceFileInput { path: "src/b.zry".to_owned(), text: last_text },
    ])
    .expect("plus-one source map");
    let plus_syntax = verify_snapshot(
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![first_file, last_file],
            diagnostics: Vec::new(),
        },
        &plus_sources,
    )
    .expect("plus-one v4 budget");
    let entry = plus_sources.file_id(&entry_path).expect("entry");
    let plus =
        lower(SemanticInput::try_new(&plus_syntax, &plus_sources, entry).expect("plus-one input"))
            .expect_err("M3 nominal limit must fail");
    assert_eq!(plus[0].code(), "ZRYNA-M3201");
}
