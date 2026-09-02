use super::*;

fn type_syntax(
    source: &str,
    spelling: &str,
    ordinal: usize,
    kind: RawTypeSyntaxKind,
) -> RawTypeSyntax {
    RawTypeSyntax { span: nth_untrusted_span(source, spelling, ordinal), kind }
}

fn named_type(source: &str, spelling: &str, ordinal: usize) -> RawTypeSyntax {
    let span = nth_untrusted_span(source, spelling, ordinal);
    RawTypeSyntax {
        span,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: spelling.to_owned(), span },
        },
    }
}

fn borrow_type(
    source: &str,
    spelling: &str,
    ordinal: usize,
    argument: u32,
    mutable: bool,
) -> RawTypeSyntax {
    let span = nth_untrusted_span(source, spelling, ordinal);
    let keyword = if mutable { "BorrowMut" } else { "Borrow" };
    let keyword_span = zryna_source::UntrustedSpan {
        file: 0,
        start: span.start,
        end: span.start + u32::try_from(keyword.len()).expect("keyword length"),
    };
    let less_than_span =
        zryna_source::UntrustedSpan { file: 0, start: keyword_span.end, end: keyword_span.end + 1 };
    let greater_than_span =
        zryna_source::UntrustedSpan { file: 0, start: span.end - 1, end: span.end };
    let kind = if mutable {
        RawTypeSyntaxKind::BorrowMut { keyword_span, less_than_span, argument, greater_than_span }
    } else {
        RawTypeSyntaxKind::Borrow { keyword_span, less_than_span, argument, greater_than_span }
    };
    type_syntax(source, spelling, ordinal, kind)
}

fn parameter(
    source: &str,
    name: &str,
    type_spelling: &str,
    type_syntax: u32,
) -> RawParameterSyntax {
    let binding = nth_untrusted_span(source, &format!("{name}:"), 0);
    let name_span = zryna_source::UntrustedSpan {
        file: 0,
        start: binding.start,
        end: binding.start + u32::try_from(name.len()).expect("parameter name length"),
    };
    let type_span = nth_untrusted_span(source, type_spelling, 0);
    RawParameterSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: name_span.start, end: type_span.end },
        name: RawIdentifierSyntax { text: name.to_owned(), span: name_span },
        type_syntax,
    }
}

fn snapshot(
    source: &str,
    name: &str,
    export_span: Option<zryna_source::UntrustedSpan>,
    type_syntax: Vec<RawTypeSyntax>,
    parameters: Vec<RawParameterSyntax>,
    result_type: u32,
) -> RawProjectSyntaxSnapshot {
    let function_span = nth_untrusted_span(source, "function", 0);
    let name_span = nth_untrusted_span(source, name, 0);
    let body_span = nth_untrusted_span(source, "{}", 0);
    RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax,
            data_declarations: Vec::new(),
            functions: vec![RawFunctionSyntax {
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: export_span.map_or(function_span.start, |at| at.start),
                    end: u32::try_from(source.len()).expect("source length"),
                },
                export_span,
                function_span,
                name: RawIdentifierSyntax { text: name.to_owned(), span: name_span },
                parameters,
                result_type,
                body: RawFunctionBodySyntax {
                    span: body_span,
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: body_span,
                        open_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_span.start,
                            end: body_span.start + 1,
                        },
                        statements: Vec::new(),
                        close_brace_span: zryna_source::UntrustedSpan {
                            file: 0,
                            start: body_span.end - 1,
                            end: body_span.end,
                        },
                    }],
                    statements: Vec::new(),
                    expressions: Vec::new(),
                },
            }],
        }],
        diagnostics: Vec::new(),
    }
}

fn catalog(
    source: &str,
    raw: RawProjectSyntaxSnapshot,
) -> (FunctionCatalog, Vec<zryna_diagnostics::Diagnostic>) {
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful borrow signature");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, declarations) = super::super::build_graph(input, &mut errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified signature layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut errors);
    let catalog = super::super::build_function_catalog(
        input,
        &declarations,
        &graph,
        &node_types,
        &mut errors,
    );
    (catalog, errors.finish())
}

#[test]
fn private_borrow_signature_preserves_source_order_access_referents_and_spans() {
    const SOURCE: &str = "function inspect(copy: i32, shared: Borrow<i32>, flag: bool, exclusive: BorrowMut<bool>): bool {}";
    let types = vec![
        named_type(SOURCE, "i32", 0),
        named_type(SOURCE, "i32", 1),
        borrow_type(SOURCE, "Borrow<i32>", 0, 1, false),
        named_type(SOURCE, "bool", 0),
        named_type(SOURCE, "bool", 1),
        borrow_type(SOURCE, "BorrowMut<bool>", 0, 4, true),
        named_type(SOURCE, "bool", 2),
    ];
    let parameters = vec![
        parameter(SOURCE, "copy", "i32", 0),
        parameter(SOURCE, "shared", "Borrow<i32>", 2),
        parameter(SOURCE, "flag", "bool", 3),
        parameter(SOURCE, "exclusive", "BorrowMut<bool>", 5),
    ];
    let (catalog, diagnostics) =
        catalog(SOURCE, snapshot(SOURCE, "inspect", None, types, parameters, 6));
    assert!(diagnostics.is_empty());
    let signature = catalog.modules[0][0].as_ref().expect("valid private signature");
    assert_eq!(signature.id.module.0, 0);
    assert_eq!(signature.id.declaration, 0);
    assert_eq!(signature.parameters.len(), 2);
    assert_eq!(signature.parameters[0].category, zryna_layout::TypeCategory::I32);
    assert_eq!(signature.parameters[1].category, zryna_layout::TypeCategory::Bool);
    assert_eq!(signature.borrow_parameters.len(), 2);
    assert_eq!(
        signature.parameter_order,
        [
            FunctionParameterOrder::Value(0),
            FunctionParameterOrder::Borrow(0),
            FunctionParameterOrder::Value(1),
            FunctionParameterOrder::Borrow(1),
        ]
    );
    let FunctionBorrowParameter { referent, access, span: at } = signature.borrow_parameters[0];
    assert_eq!(referent.category, zryna_layout::TypeCategory::I32);
    assert_eq!(access, raw::BorrowAccess::Shared);
    let expected = nth_untrusted_span(SOURCE, "Borrow<i32>", 0);
    assert_eq!((at.start(), at.end()), (expected.start, expected.end));
    let FunctionBorrowParameter { referent, access, span: at } = signature.borrow_parameters[1];
    assert_eq!(referent.category, zryna_layout::TypeCategory::Bool);
    assert_eq!(access, raw::BorrowAccess::Exclusive);
    let expected = nth_untrusted_span(SOURCE, "BorrowMut<bool>", 0);
    assert_eq!((at.start(), at.end()), (expected.start, expected.end));
}

#[test]
fn invalid_borrow_signature_keeps_slot_and_exact_sorted_diagnostics() {
    const SOURCE: &str =
        "export function invalid(a: Borrow<String>, b: Borrow<Borrow<i32>>): BorrowMut<bool> {}";
    let types = vec![
        type_syntax(
            SOURCE,
            "String",
            0,
            RawTypeSyntaxKind::String { keyword_span: nth_untrusted_span(SOURCE, "String", 0) },
        ),
        borrow_type(SOURCE, "Borrow<String>", 0, 0, false),
        named_type(SOURCE, "i32", 0),
        borrow_type(SOURCE, "Borrow<i32>", 0, 2, false),
        borrow_type(SOURCE, "Borrow<Borrow<i32>>", 0, 3, false),
        named_type(SOURCE, "bool", 0),
        borrow_type(SOURCE, "BorrowMut<bool>", 0, 5, true),
    ];
    let parameters = vec![
        parameter(SOURCE, "a", "Borrow<String>", 1),
        parameter(SOURCE, "b", "Borrow<Borrow<i32>>", 4),
    ];
    let export_span = nth_untrusted_span(SOURCE, "export", 0);
    let (catalog, diagnostics) =
        catalog(SOURCE, snapshot(SOURCE, "invalid", Some(export_span), types, parameters, 6));
    assert!(catalog.modules[0][0].is_none());
    assert_eq!(diagnostics.len(), 4);
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.code() == "ZRYNA-M3016"));
    assert_eq!(
        diagnostics.iter().map(zryna_diagnostics::Diagnostic::message).collect::<Vec<_>>(),
        vec![
            "borrow-parameter functions must remain private",
            "borrow parameters require one direct Copy referent",
            "borrow parameters require one direct Copy referent",
            "borrow results are outside the nonescaping ownership profile",
        ]
    );
    assert_eq!(
        diagnostics.iter().map(zryna_diagnostics::Diagnostic::guidance).collect::<Vec<_>>(),
        vec![
            "remove export because borrow authority cannot cross the public ABI",
            "borrow bool, i32, or a recursively Copy aggregate type",
            "borrow bool, i32, or a recursively Copy aggregate type",
            "return an exact Copy value read through the borrow instead",
        ]
    );
    let expected = [
        export_span,
        nth_untrusted_span(SOURCE, "Borrow<String>", 0),
        nth_untrusted_span(SOURCE, "Borrow<Borrow<i32>>", 0),
        nth_untrusted_span(SOURCE, "BorrowMut<bool>", 0),
    ];
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| {
                let at = diagnostic.primary_span().expect("source diagnostic");
                (at.start(), at.end())
            })
            .collect::<Vec<_>>(),
        expected.iter().map(|at| (at.start, at.end)).collect::<Vec<_>>()
    );
}

#[test]
fn unknown_borrow_referent_keeps_the_existing_name_diagnostic_only() {
    const SOURCE: &str = "function unknown(a: Borrow<Missing>): bool {}";
    let types = vec![
        named_type(SOURCE, "Missing", 0),
        borrow_type(SOURCE, "Borrow<Missing>", 0, 0, false),
        named_type(SOURCE, "bool", 0),
    ];
    let parameters = vec![parameter(SOURCE, "a", "Borrow<Missing>", 1)];
    let sources = sources_for(SOURCE);
    let syntax = verify_snapshot(snapshot(SOURCE, "unknown", None, types, parameters, 2), &sources)
        .expect("source-faithful unknown referent");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("unknown referent");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3002");
    assert_eq!(diagnostics[0].message(), "type 'Missing' does not name a module-local aggregate");
}
