use super::*;
use zryna_layout::{StorageTarget, TypeCategory};
use zryna_syntax::v4::{RawEnumVariant, RawExpressionKind};

// Fixed source DTOs reuse the existing lexical span helpers; no semantic route is predicted here.
#[allow(clippy::too_many_lines)]
fn copy_identity_fixture(nested_enum: bool) -> (&'static str, RawProjectSyntaxSnapshot) {
    let source = if nested_enum {
        "interface Inner extends ZrynaEnum { some: i32; }\ninterface Wrapper extends ZrynaEnum { inner: Inner; }\nfunction identity(arg: Wrapper): Wrapper { return arg; }"
    } else {
        "interface Inner extends ZrynaEnum { some: i32; }\ninterface Wrapper extends ZrynaStruct { inner: Inner; }\nfunction identity(arg: Wrapper): Wrapper { return arg; }"
    };
    let token = |text: &str, ordinal: usize| nth_untrusted_span(source, text, ordinal);
    let range = |first, last| untrusted_range(source, first, last);
    let name = |text: &str, ordinal| RawIdentifierSyntax {
        text: text.to_owned(),
        span: token(text, ordinal),
    };
    let named_type = |text, ordinal| RawTypeSyntax {
        span: token(text, ordinal),
        kind: RawTypeSyntaxKind::Named { name: name(text, ordinal) },
    };
    let inner = RawDataDeclaration {
        span: range(("interface", 0), ("}", 0)),
        export_span: None,
        kind: RawDataDeclarationKind::Enum {
            interface_span: token("interface", 0),
            name: name("Inner", 0),
            extends_span: token("extends", 0),
            marker_span: token("ZrynaEnum", 0),
            open_brace_span: token("{", 0),
            close_brace_span: token("}", 0),
            variants: vec![RawEnumVariant {
                span: range(("some", 0), (";", 0)),
                name: name("some", 0),
                colon_span: token(":", 0),
                payload_type: Some(0),
                none_span: None,
                semicolon_span: token(";", 0),
            }],
        },
    };
    let wrapper_kind = if nested_enum {
        RawDataDeclarationKind::Enum {
            interface_span: token("interface", 1),
            name: name("Wrapper", 0),
            extends_span: token("extends", 1),
            marker_span: token("ZrynaEnum", 1),
            open_brace_span: token("{", 1),
            close_brace_span: token("}", 1),
            variants: vec![RawEnumVariant {
                span: range(("inner", 0), (";", 1)),
                name: name("inner", 0),
                colon_span: token(":", 1),
                payload_type: Some(1),
                none_span: None,
                semicolon_span: token(";", 1),
            }],
        }
    } else {
        RawDataDeclarationKind::Struct {
            interface_span: token("interface", 1),
            name: name("Wrapper", 0),
            extends_span: token("extends", 1),
            marker_span: token("ZrynaStruct", 0),
            open_brace_span: token("{", 1),
            close_brace_span: token("}", 1),
            fields: vec![RawDataField {
                span: range(("inner", 0), (";", 1)),
                name: name("inner", 0),
                colon_span: token(":", 1),
                type_syntax: 1,
                semicolon_span: token(";", 1),
            }],
        }
    };
    let body = range(("{", 2), ("}", 2));
    let raw = RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            imports: Vec::new(),
            type_syntax: vec![
                named_type("i32", 0),
                named_type("Inner", 1),
                named_type("Wrapper", 1),
                named_type("Wrapper", 2),
            ],
            data_declarations: vec![
                inner,
                RawDataDeclaration {
                    span: range(("interface", 1), ("}", 1)),
                    export_span: None,
                    kind: wrapper_kind,
                },
            ],
            functions: vec![RawFunctionSyntax {
                span: range(("function", 0), ("}", 2)),
                export_span: None,
                function_span: token("function", 0),
                name: name("identity", 0),
                parameters: vec![RawParameterSyntax {
                    span: range(("arg", 0), ("Wrapper", 1)),
                    name: name("arg", 0),
                    type_syntax: 2,
                }],
                result_type: 3,
                body: RawFunctionBodySyntax {
                    span: body,
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: body,
                        open_brace_span: token("{", 2),
                        statements: vec![0],
                        close_brace_span: token("}", 2),
                    }],
                    statements: vec![RawStatementSyntax {
                        span: range(("return", 0), (";", 2)),
                        kind: RawStatementKind::Return {
                            keyword_span: token("return", 0),
                            value: 0,
                            semicolon_span: token(";", 2),
                        },
                    }],
                    expressions: vec![RawExpressionSyntax {
                        span: token("arg", 1),
                        kind: RawExpressionKind::Reference { name: name("arg", 1) },
                    }],
                },
            }],
        }],
        diagnostics: Vec::new(),
    };
    (source, raw)
}

fn assert_copy_identity(nested_enum: bool) {
    let (source, raw) = copy_identity_fixture(nested_enum);
    let sources = sources_for(source);
    let syntax = verify_snapshot(raw, &sources).expect("source-authenticated Copy identity");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, _) = crate::data_ownership_v1::build_graph(input, &mut errors);
    assert!(errors.finish().is_empty());
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let layouts = zryna_layout::verify(&graph, &sources, target).expect("Copy layouts");
        assert_eq!(layouts.types().count(), 5, "three built-ins and two nominal records");
        let nominal = layouts
            .types()
            .filter(|ty| matches!(ty.category(), TypeCategory::Struct | TypeCategory::Enum))
            .collect::<Vec<_>>();
        assert_eq!(nominal.len(), 2);
        assert!(nominal.iter().all(|ty| ty.drop_kind() == 0));
        assert_eq!(
            layouts.types().filter(|ty| ty.category() == TypeCategory::Enum).count(),
            if nested_enum { 2 } else { 1 }
        );
    }
    let mut previous = None;
    for _ in 0..2 {
        // lower returns only after the mandatory independent raw-IR verifier succeeds.
        let program = lower(input).expect("existing private Copy identity reaches verified IR");
        assert_eq!(program.modules().count(), 1);
        let module = program.modules().next().expect("one module");
        assert_eq!(module.functions().count(), 1);
        let function = module.functions().next().expect("one identity function");
        let parameters = function.parameters().collect::<Vec<_>>();
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].id().index(), 0);
        assert_eq!(parameters[0].ty(), function.result_type());
        assert_eq!(function.borrow_parameters().count(), 0);
        let places = function.places().collect::<Vec<_>>();
        assert_eq!(places.len(), 1);
        assert!(places[0].is_copy());
        assert_eq!(places[0].kind(), VerifiedPlaceKind::Parameter(0));
        assert_eq!(places[0].ty(), parameters[0].ty());
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].parameters().count(), 0);
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 1);
        let copied = instructions[0];
        assert_eq!(copied.kind(), VerifiedInstructionKind::CopyFromPlace);
        let result = copied.result().expect("one Copy result");
        assert_eq!(result.index(), 1);
        assert_eq!(copied.result_type(), Some(function.result_type()));
        assert_eq!(copied.value_operands().count(), 0);
        assert_eq!(copied.place_operands().collect::<Vec<_>>(), vec![places[0].id()]);
        assert_eq!(copied.derived_drop_actions().count(), 0);
        assert!(copied.cleanup().is_none());
        let cleanup = function.cleanup_plans().collect::<Vec<_>>();
        assert_eq!(cleanup.len(), 1);
        assert_eq!(cleanup[0].actions().count(), 0);
        let returned = blocks[0].terminator();
        assert_eq!(returned.kind(), VerifiedTerminatorKind::Return);
        assert_eq!(returned.value_operands().collect::<Vec<_>>(), vec![result]);
        assert_eq!(returned.cleanup(), Some(cleanup[0].id()));
        assert_eq!(returned.derived_drop_actions().count(), 0);
        let result_category = program
            .verified_ir()
            .linear32_layouts()
            .type_by_id(function.result_type())
            .expect("sealed result layout")
            .category();
        assert_eq!(
            result_category,
            if nested_enum { TypeCategory::Enum } else { TypeCategory::Struct }
        );
        let observation = (copied.kind(), result.index(), places[0].id().index(), result_category);
        if let Some(previous) = previous {
            assert_eq!(observation, previous);
        }
        previous = Some(observation);
    }
}

#[test]
fn copy_struct_with_enum_identity_preserves_copy_route() {
    assert_copy_identity(false);
}

#[test]
fn copy_nested_enum_identity_preserves_copy_route() {
    assert_copy_identity(true);
}
