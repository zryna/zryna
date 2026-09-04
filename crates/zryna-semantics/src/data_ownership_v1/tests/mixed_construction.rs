use super::*;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedModule};
use zryna_layout::TypeCategory;
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

// Fixed protocol-v4 fixture construction only; type/layout and ownership decisions remain
// in the real compiler. These final-success tests are intentionally red on the old routes.
#[allow(clippy::too_many_lines)]
pub(super) fn mixed_fixture(vec_outer: bool) -> (String, RawProjectSyntaxSnapshot) {
    let source = if vec_outer {
        "interface Parcel extends ZrynaStruct { value: String; }\nfunction make(): Vec<Parcel> { return Vec<Parcel>([Parcel({ value: \"a\" })]); }"
    } else {
        "interface Parcel extends ZrynaStruct { value: Vec<String>; }\nfunction make(): Parcel { return Parcel({ value: Vec<String>([\"a\"]) }); }"
    }.to_owned();
    let token = |needle, ordinal| nth_untrusted_span(&source, needle, ordinal);
    let range = |start, ordinal, end, end_ordinal| {
        untrusted_range(&source, (start, ordinal), (end, end_ordinal))
    };
    let named = |at| RawTypeSyntax {
        span: at,
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Parcel".to_owned(), span: at },
        },
    };
    let string =
        |at| RawTypeSyntax { span: at, kind: RawTypeSyntaxKind::String { keyword_span: at } };
    let vector = |at: zryna_source::UntrustedSpan, argument| RawTypeSyntax {
        span: at,
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: zryna_source::UntrustedSpan {
                file: 0,
                start: at.start,
                end: at.start + 3,
            },
            less_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: at.start + 3,
                end: at.start + 4,
            },
            argument,
            greater_than_span: zryna_source::UntrustedSpan {
                file: 0,
                start: at.end - 1,
                end: at.end,
            },
        },
    };
    let types = if vec_outer {
        vec![
            string(token("String", 0)),
            named(token("Parcel", 1)),
            vector(token("Vec<Parcel>", 0), 1),
            named(token("Parcel", 2)),
            vector(token("Vec<Parcel>", 1), 3),
        ]
    } else {
        vec![
            string(token("String", 0)),
            vector(token("Vec<String>", 0), 0),
            named(token("Parcel", 1)),
            string(token("String", 1)),
            vector(token("Vec<String>", 1), 3),
        ]
    };
    let declaration = RawDataDeclaration {
        span: range("interface", 0, "}", 0),
        export_span: None,
        kind: RawDataDeclarationKind::Struct {
            interface_span: token("interface", 0),
            name: RawIdentifierSyntax { text: "Parcel".to_owned(), span: token("Parcel", 0) },
            extends_span: token("extends", 0),
            marker_span: token("ZrynaStruct", 0),
            open_brace_span: token("{", 0),
            close_brace_span: token("}", 0),
            fields: vec![RawDataField {
                span: range("value", 0, ";", 0),
                name: RawIdentifierSyntax { text: "value".to_owned(), span: token("value", 0) },
                colon_span: token(":", 0),
                type_syntax: u32::from(!vec_outer),
                semicolon_span: token(";", 0),
            }],
        },
    };
    let literal = RawExpressionSyntax {
        span: token("\"a\"", 0),
        kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
    };
    let struct_start = token("Parcel({", 0).start;
    let struct_end = token("}", 1).end + 1;
    let struct_expression = RawExpressionSyntax {
        span: zryna_source::UntrustedSpan { file: 0, start: struct_start, end: struct_end },
        kind: RawExpressionKind::StructConstruction {
            type_name: RawIdentifierSyntax {
                text: "Parcel".to_owned(),
                span: zryna_source::UntrustedSpan {
                    file: 0,
                    start: struct_start,
                    end: struct_start + 6,
                },
            },
            open_paren_span: token("(", if vec_outer { 2 } else { 1 }),
            open_brace_span: token("{", 2),
            fields: vec![RawFieldInitializer {
                span: if vec_outer {
                    range("value", 1, "\"a\"", 0)
                } else {
                    range("value", 1, ")", 1)
                },
                kind: RawFieldInitializerKind::Explicit {
                    name: RawIdentifierSyntax { text: "value".to_owned(), span: token("value", 1) },
                    colon_span: token(":", 2),
                    value: u32::from(!vec_outer),
                },
            }],
            close_brace_span: token("}", 1),
            close_paren_span: token(")", if vec_outer { 1 } else { 2 }),
        },
    };
    let vec_expression = RawExpressionSyntax {
        span: if vec_outer {
            range("Vec<Parcel>", 1, ")", 2)
        } else {
            range("Vec<String>", 1, ")", 1)
        },
        kind: RawExpressionKind::VecConstruction {
            type_syntax: 4,
            open_paren_span: token("(", if vec_outer { 1 } else { 2 }),
            open_bracket_span: token("[", 0),
            elements: vec![u32::from(vec_outer)],
            close_bracket_span: token("]", 0),
            close_paren_span: token(")", if vec_outer { 2 } else { 1 }),
        },
    };
    let body = range("{", 1, "}", 2);
    let expressions = if vec_outer {
        vec![literal, struct_expression, vec_expression]
    } else {
        vec![literal, vec_expression, struct_expression]
    };
    let function = RawFunctionSyntax {
        span: range("function", 0, "}", 2),
        export_span: None,
        function_span: token("function", 0),
        name: RawIdentifierSyntax { text: "make".to_owned(), span: token("make", 0) },
        parameters: Vec::new(),
        result_type: 2,
        body: RawFunctionBodySyntax {
            span: body,
            root_block: 0,
            blocks: vec![RawBlockSyntax {
                span: body,
                open_brace_span: token("{", 1),
                statements: vec![0],
                close_brace_span: token("}", 2),
            }],
            statements: vec![RawStatementSyntax {
                span: range("return", 0, ";", 1),
                kind: RawStatementKind::Return {
                    keyword_span: token("return", 0),
                    value: 2,
                    semicolon_span: token(";", 1),
                },
            }],
            expressions,
        },
    };
    (
        source,
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: vec![declaration],
                functions: vec![function],
            }],
            diagnostics: Vec::new(),
        },
    )
}

fn assert_mixed(vec_outer: bool) {
    let (source, raw) = mixed_fixture(vec_outer);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed constructor source");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, _) = crate::data_ownership_v1::build_graph(input, &mut errors);
    assert!(errors.finish().is_empty());
    for target in
        [zryna_layout::StorageTarget::Linear32V1, zryna_layout::StorageTarget::LinuxX8664V1]
    {
        let layouts = zryna_layout::verify(&graph, &sources, target).expect("real mixed layouts");
        assert_eq!(layouts.target(), target);
    }
    let expected_kinds = if vec_outer {
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::StructConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    } else {
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::StructConstruct,
        ]
    };
    let expected_types = if vec_outer {
        [TypeCategory::String, TypeCategory::Struct, TypeCategory::Vec]
    } else {
        [TypeCategory::String, TypeCategory::Vec, TypeCategory::Struct]
    };
    let mut previous = None;
    for _ in 0..2 {
        // Final success oracle: old category rejection must remain a red test, never success.
        let program =
            lower(input).expect("mixed source must reach mandatory independent IR verification");
        let functions = program.modules().flat_map(VerifiedModule::functions).collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let function = functions[0];
        assert_eq!(function.parameters().count(), 0);
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 1);
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 3);
        let kinds = instructions.iter().map(|i| i.kind()).collect::<Vec<_>>();
        assert_eq!(kinds, expected_kinds);
        let results =
            instructions.iter().map(|i| i.result().expect("typed result")).collect::<Vec<_>>();
        assert_eq!(results.iter().map(|v| v.index()).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert!(instructions[0].value_operands().next().is_none());
        assert_eq!(instructions[0].string_utf8_bytes(), Some(&b"a"[..]));
        assert_eq!(instructions[1].value_operands().collect::<Vec<_>>(), vec![results[0]]);
        assert_eq!(instructions[2].value_operands().collect::<Vec<_>>(), vec![results[1]]);
        let layouts = program.verified_ir().linear32_layouts();
        let categories = instructions
            .iter()
            .map(|i| {
                layouts
                    .type_by_id(i.result_type().expect("result type"))
                    .expect("sealed type")
                    .category()
            })
            .collect::<Vec<_>>();
        assert_eq!(categories, expected_types);
        let places = function.places().collect::<Vec<_>>();
        assert_eq!(places.len(), 3);
        for (index, place) in places.iter().enumerate() {
            assert!(!place.is_copy());
            assert_eq!(place.kind(), VerifiedPlaceKind::Temporary(results[index]));
            assert_eq!(place.ty(), instructions[index].result_type().expect("result type"));
        }
        let returned = blocks[0].terminator();
        assert_eq!(returned.kind(), VerifiedTerminatorKind::Return);
        assert_eq!(returned.value_operands().collect::<Vec<_>>(), vec![results[2]]);
        assert_eq!(
            returned.derived_drop_actions().count(),
            0,
            "children transferred into returned root"
        );
        let observation = (
            kinds,
            categories,
            results.iter().map(|v| v.index()).collect::<Vec<_>>(),
            instructions
                .iter()
                .map(|i| i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        );
        if let Some(previous) = &previous {
            assert_eq!(&observation, previous);
        }
        previous = Some(observation);
    }
}

#[test]
fn mixed_construction_vec_of_owned_struct_reaches_verified_ir() {
    assert_mixed(true);
}

#[test]
fn mixed_construction_owned_struct_containing_vec_reaches_verified_ir() {
    assert_mixed(false);
}
