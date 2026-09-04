use super::*;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::ValueIdentity;
use zryna_layout::{StorageTarget, TypeCategory};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

const ARRAY: &str = "FixedArray<Vec<String>, 0>";

fn at(start: usize, end: usize) -> zryna_source::UntrustedSpan {
    zryna_source::UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("fixture offset"),
        end: u32::try_from(end).expect("fixture offset"),
    }
}

fn string_type(types: &mut Vec<RawTypeSyntax>, start: usize) -> u32 {
    let id = u32::try_from(types.len()).expect("type ID");
    types.push(RawTypeSyntax {
        span: at(start, start + 6),
        kind: RawTypeSyntaxKind::String { keyword_span: at(start, start + 6) },
    });
    id
}

fn vector_type(types: &mut Vec<RawTypeSyntax>, start: usize, end: usize, argument: u32) -> u32 {
    let id = u32::try_from(types.len()).expect("type ID");
    types.push(RawTypeSyntax {
        span: at(start, end),
        kind: RawTypeSyntaxKind::Vec {
            keyword_span: at(start, start + 3),
            less_than_span: at(start + 3, start + 4),
            argument,
            greater_than_span: at(end - 1, end),
        },
    });
    id
}

fn zero_array_type(types: &mut Vec<RawTypeSyntax>, start: usize) -> u32 {
    let string = string_type(types, start + 15);
    let element = vector_type(types, start + 11, start + 22, string);
    let id = u32::try_from(types.len()).expect("type ID");
    types.push(RawTypeSyntax {
        span: at(start, start + 26),
        kind: RawTypeSyntaxKind::FixedArray {
            keyword_span: at(start, start + 10),
            less_than_span: at(start + 10, start + 11),
            element,
            comma_span: at(start + 22, start + 23),
            length_span: at(start + 24, start + 25),
            length: 0,
            length_spelling: "0".into(),
            greater_than_span: at(start + 25, start + 26),
        },
    });
    id
}

fn snapshot(
    source: &str,
    types: Vec<RawTypeSyntax>,
    declarations: Vec<RawDataDeclaration>,
    expressions: Vec<RawExpressionSyntax>,
    result_type: u32,
) -> RawProjectSyntaxSnapshot {
    let function = source.find("function").expect("function");
    let body = function + source[function..].find('{').expect("body");
    let returned = body + source[body..].find("return").expect("return");
    let semi = source.rfind(';').expect("return semicolon");
    let close = source.rfind('}').expect("body close");
    let value = u32::try_from(expressions.len() - 1).expect("return result");
    RawProjectSyntaxSnapshot {
        schema_version: PROTOCOL_VERSION,
        diagnostics: Vec::new(),
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".into(),
            imports: Vec::new(),
            type_syntax: types,
            data_declarations: declarations,
            functions: vec![RawFunctionSyntax {
                span: at(function, close + 1),
                export_span: None,
                function_span: at(function, function + 8),
                name: RawIdentifierSyntax {
                    text: "make".into(),
                    span: at(function + 9, function + 13),
                },
                parameters: Vec::new(),
                result_type,
                body: RawFunctionBodySyntax {
                    span: at(body, close + 1),
                    root_block: 0,
                    blocks: vec![RawBlockSyntax {
                        span: at(body, close + 1),
                        open_brace_span: at(body, body + 1),
                        statements: vec![0],
                        close_brace_span: at(close, close + 1),
                    }],
                    statements: vec![RawStatementSyntax {
                        span: at(returned, semi + 1),
                        kind: RawStatementKind::Return {
                            keyword_span: at(returned, returned + 6),
                            value,
                            semicolon_span: at(semi, semi + 1),
                        },
                    }],
                    expressions,
                },
            }],
        }],
    }
}

fn positive_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let source = "interface Parcel extends ZrynaStruct { value: FixedArray<Vec<String>, 0>; label: String; } function make(): Parcel { return Parcel({ value: FixedArray<Vec<String>, 0>([]), label: \"a\" }); }".to_owned();
    let token = |text, ordinal| nth_untrusted_span(&source, text, ordinal);
    let field_array = usize::try_from(token(ARRAY, 0).start).expect("span");
    let field_label = usize::try_from(token("String", 1).start).expect("span");
    let mut types = Vec::new();
    let array_ty = zero_array_type(&mut types, field_array);
    let label_ty = string_type(&mut types, field_label);
    let result_type = u32::try_from(types.len()).expect("result type");
    types.push(RawTypeSyntax {
        span: token("Parcel", 1),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Parcel".into(), span: token("Parcel", 1) },
        },
    });
    let construct = usize::try_from(token(ARRAY, 1).start).expect("span");
    let construct_ty = zero_array_type(&mut types, construct);
    let fields = [("value", array_ty, 0), ("label", label_ty, 1)]
        .into_iter()
        .map(|(name, type_syntax, ordinal)| RawDataField {
            span: untrusted_range(&source, (name, 0), (";", ordinal)),
            name: RawIdentifierSyntax { text: name.into(), span: token(name, 0) },
            colon_span: token(":", ordinal),
            type_syntax,
            semicolon_span: token(";", ordinal),
        })
        .collect();
    let declaration = RawDataDeclaration {
        span: untrusted_range(&source, ("interface", 0), ("}", 0)),
        export_span: None,
        kind: RawDataDeclarationKind::Struct {
            interface_span: token("interface", 0),
            name: RawIdentifierSyntax { text: "Parcel".into(), span: token("Parcel", 0) },
            extends_span: token("extends", 0),
            marker_span: token("ZrynaStruct", 0),
            open_brace_span: token("{", 0),
            close_brace_span: token("}", 0),
            fields,
        },
    };
    let expressions = positive_expressions(&source, construct, construct_ty);
    let raw = snapshot(&source, types, vec![declaration], expressions, result_type);
    (source, raw)
}

fn positive_expressions(
    source: &str,
    construct: usize,
    construct_ty: u32,
) -> Vec<RawExpressionSyntax> {
    let token = |text, ordinal| nth_untrusted_span(source, text, ordinal);
    let structure = usize::try_from(token("Parcel({", 0).start).expect("span");
    let end = usize::try_from(token("}", 1).end).expect("span") + 1;
    vec![
        RawExpressionSyntax {
            span: at(construct, construct + 30),
            kind: RawExpressionKind::FixedArrayConstruction {
                type_syntax: construct_ty,
                open_paren_span: at(construct + 26, construct + 27),
                open_bracket_span: at(construct + 27, construct + 28),
                elements: Vec::new(),
                close_bracket_span: at(construct + 28, construct + 29),
                close_paren_span: at(construct + 29, construct + 30),
            },
        },
        RawExpressionSyntax {
            span: token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        RawExpressionSyntax {
            span: at(structure, end),
            kind: RawExpressionKind::StructConstruction {
                type_name: RawIdentifierSyntax {
                    text: "Parcel".into(),
                    span: at(structure, structure + 6),
                },
                open_paren_span: at(structure + 6, structure + 7),
                open_brace_span: at(structure + 7, structure + 8),
                fields: vec![
                    RawFieldInitializer {
                        span: at(
                            usize::try_from(token("value", 1).start).expect("span"),
                            construct + 30,
                        ),
                        kind: RawFieldInitializerKind::Explicit {
                            name: RawIdentifierSyntax {
                                text: "value".into(),
                                span: token("value", 1),
                            },
                            colon_span: token(":", 3),
                            value: 0,
                        },
                    },
                    RawFieldInitializer {
                        span: untrusted_range(source, ("label", 1), ("\"a\"", 0)),
                        kind: RawFieldInitializerKind::Explicit {
                            name: RawIdentifierSyntax {
                                text: "label".into(),
                                span: token("label", 1),
                            },
                            colon_span: token(":", 4),
                            value: 1,
                        },
                    },
                ],
                close_brace_span: token("}", 1),
                close_paren_span: at(end - 1, end),
            },
        },
    ]
}

fn negative_fixture() -> (String, RawProjectSyntaxSnapshot) {
    let source = "function make(): Vec<FixedArray<Vec<String>, 0>> { return Vec<FixedArray<Vec<String>, 0>>([]); }".to_owned();
    let mut types = Vec::new();
    let first = source.find("Vec<FixedArray").expect("result type");
    let array = zero_array_type(&mut types, first + 4);
    let result = vector_type(&mut types, first, first + 31, array);
    let second = source.rfind("Vec<FixedArray").expect("construct type");
    let array = zero_array_type(&mut types, second + 4);
    let construct = vector_type(&mut types, second, second + 31, array);
    let expression = RawExpressionSyntax {
        span: at(second, second + 35),
        kind: RawExpressionKind::VecConstruction {
            type_syntax: construct,
            open_paren_span: at(second + 31, second + 32),
            open_bracket_span: at(second + 32, second + 33),
            elements: Vec::new(),
            close_bracket_span: at(second + 33, second + 34),
            close_paren_span: at(second + 34, second + 35),
        },
    };
    let raw = snapshot(&source, types, Vec::new(), vec![expression], result);
    (source, raw)
}

#[test]
fn mixed_zero_array_of_vec_keeps_alignment_ownership_and_nonzero_struct_layout() {
    let (source, raw) = positive_fixture();
    let sources = sources_for(&source);
    let syntax =
        verify_snapshot(raw, &sources).expect("authenticated zero-length array field source");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, _) = super::super::build_graph(input, &mut errors);
    assert!(errors.finish().is_empty());
    for (target, size, alignment) in
        [(StorageTarget::Linear32V1, 12, 4), (StorageTarget::LinuxX8664V1, 24, 8)]
    {
        let layouts =
            zryna_layout::verify(&graph, &sources, target).expect("independent target layout");
        let array = layouts
            .types()
            .find(|ty| ty.category() == TypeCategory::FixedArray)
            .expect("zero array");
        let vector =
            layouts.types().find(|ty| ty.category() == TypeCategory::Vec).expect("Vec<String>");
        let structure =
            layouts.types().find(|ty| ty.category() == TypeCategory::Struct).expect("Parcel");
        assert_eq!(
            (array.size(), array.alignment(), array.array_length(), array.array_stride()),
            (0, alignment, Some(0), Some(size))
        );
        assert_ne!(
            array.drop_kind(),
            0,
            "zero count does not turn an owned element type into Copy"
        );
        assert_eq!(array.referenced_type(), Some(vector.id()));
        assert_eq!((structure.size(), structure.alignment()), (size, alignment));
        assert_eq!(structure.fields().len(), 2);
        assert_eq!(structure.fields()[0].ty(), array.id());
        assert_eq!(
            structure.fields().iter().map(|field| field.offset()).collect::<Vec<_>>(),
            vec![0, 0]
        );
    }
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(input).expect("independent zero-array full IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::FixedArrayConstruct,
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::StructConstruct
            ]
        );
        assert_eq!(instructions[0].value_operands().count(), 0);
        assert_eq!(function.places().count(), 3);
        assert_eq!(
            instructions[2].value_operands().collect::<Vec<_>>(),
            vec![
                instructions[0].result().expect("owned zero array"),
                instructions[1].result().expect("String")
            ]
        );
        assert_eq!(
            block.terminator().value_operands().collect::<Vec<_>>(),
            vec![instructions[2].result().expect("Parcel")]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        let observed = instructions
            .iter()
            .map(|instruction| {
                (
                    instruction.kind(),
                    instruction.result().map(ValueIdentity::index),
                    instruction.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(prior) = previous.replace(observed.clone()) {
            assert_eq!(observed, prior);
        }
    }
}

#[test]
fn mixed_vec_of_zero_stored_array_is_rejected_by_layout_before_lowering() {
    let (source, raw) = negative_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated nested Vec type source");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, _) = super::super::build_graph(input, &mut errors);
    assert!(errors.finish().is_empty());
    let expected = vec![Diagnostic::error(
        "ZRYNA-L3003",
        None,
        "Vec does not admit a zero-sized element type",
        "use a non-zero-sized element type",
    )];
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        assert_eq!(
            zryna_layout::verify(&graph, &sources, target).expect_err("zero stored stride"),
            expected
        );
    }
    for _ in 0..2 {
        assert_eq!(lower(input).expect_err("layout blocks IR lowering"), expected);
    }
}
#[path = "mixed_positive_arrays.rs"]
mod positive_arrays;
