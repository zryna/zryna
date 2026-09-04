use super::*;
use zryna_ir::data_ownership_v1::{ValueIdentity, VerifiedModule};
use zryna_layout::TypeCategory;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{RawEnumVariant, RawExpressionKind};

// Fixed syntax DTOs only: no layout, semantic type, ownership, or resource inference.
struct NestedSource(&'static str);

impl NestedSource {
    fn token(&self, spelling: &str, ordinal: usize) -> UntrustedSpan {
        nth_untrusted_span(self.0, spelling, ordinal)
    }

    fn range(&self, first: (&str, usize), last: (&str, usize)) -> UntrustedSpan {
        untrusted_range(self.0, first, last)
    }

    fn name(&self, spelling: &str, ordinal: usize) -> RawIdentifierSyntax {
        RawIdentifierSyntax { text: spelling.to_owned(), span: self.token(spelling, ordinal) }
    }

    fn named_type(&self, spelling: &str, ordinal: usize) -> RawTypeSyntax {
        RawTypeSyntax {
            span: self.token(spelling, ordinal),
            kind: RawTypeSyntaxKind::Named { name: self.name(spelling, ordinal) },
        }
    }

    fn string_type(&self, ordinal: usize) -> RawTypeSyntax {
        let at = self.token("String", ordinal);
        RawTypeSyntax { span: at, kind: RawTypeSyntaxKind::String { keyword_span: at } }
    }

    fn vec_type(&self, spelling: &str, ordinal: usize, argument: u32) -> RawTypeSyntax {
        let span = self.token(spelling, ordinal);
        let part = |start, end| UntrustedSpan { file: 0, start, end };
        RawTypeSyntax {
            span,
            kind: RawTypeSyntaxKind::Vec {
                keyword_span: part(span.start, span.start + 3),
                less_than_span: part(span.start + 3, span.start + 4),
                argument,
                greater_than_span: part(span.end - 1, span.end),
            },
        }
    }

    // Punctuation ordinals describe these exact fixtures, not a source parser.
    fn vector(
        &self,
        start: (&str, usize),
        ty: u32,
        open: usize,
        close: usize,
        brackets: (usize, usize),
        elements: Vec<u32>,
    ) -> RawExpressionSyntax {
        RawExpressionSyntax {
            span: self.range(start, (")", close)),
            kind: RawExpressionKind::VecConstruction {
                type_syntax: ty,
                open_paren_span: self.token("(", open),
                open_bracket_span: self.token("[", brackets.0),
                elements,
                close_bracket_span: self.token("]", brackets.1),
                close_paren_span: self.token(")", close),
            },
        }
    }

    fn snapshot(
        &self,
        types: Vec<RawTypeSyntax>,
        declarations: Vec<RawDataDeclaration>,
        result_type: u32,
        expressions: Vec<RawExpressionSyntax>,
    ) -> RawProjectSyntaxSnapshot {
        let braces = declarations.len();
        let body = self.range(("{", braces), ("}", braces));
        let semicolon = if declarations.is_empty() { 0 } else { 2 };
        RawProjectSyntaxSnapshot {
            schema_version: PROTOCOL_VERSION,
            files: vec![RawSourceUnit {
                id: 0,
                path: "src/main.zry".to_owned(),
                imports: Vec::new(),
                type_syntax: types,
                data_declarations: declarations,
                functions: vec![RawFunctionSyntax {
                    span: self.range(("function", 0), ("}", braces)),
                    export_span: None,
                    function_span: self.token("function", 0),
                    name: self.name("make", 0),
                    parameters: Vec::new(),
                    result_type,
                    body: RawFunctionBodySyntax {
                        span: body,
                        root_block: 0,
                        blocks: vec![RawBlockSyntax {
                            span: body,
                            open_brace_span: self.token("{", braces),
                            statements: vec![0],
                            close_brace_span: self.token("}", braces),
                        }],
                        statements: vec![RawStatementSyntax {
                            span: self.range(("return", 0), (";", semicolon)),
                            kind: RawStatementKind::Return {
                                keyword_span: self.token("return", 0),
                                value: 3,
                                semicolon_span: self.token(";", semicolon),
                            },
                        }],
                        expressions,
                    },
                }],
            }],
            diagnostics: Vec::new(),
        }
    }
}

fn nested_vec_fixture() -> (NestedSource, RawProjectSyntaxSnapshot) {
    let s = NestedSource(
        "function make(): Vec<Vec<i32>> { return Vec<Vec<i32>>([Vec<i32>([7]), Vec<i32>([])]); }",
    );
    let types = vec![
        s.named_type("i32", 0),
        s.vec_type("Vec<i32>", 0, 0),
        s.vec_type("Vec<Vec<i32>>", 0, 1),
        s.named_type("i32", 1),
        s.vec_type("Vec<i32>", 1, 3),
        s.vec_type("Vec<Vec<i32>>", 1, 4),
        s.named_type("i32", 2),
        s.vec_type("Vec<i32>", 2, 6),
        s.named_type("i32", 3),
        s.vec_type("Vec<i32>", 3, 8),
    ];
    let expressions = vec![
        RawExpressionSyntax {
            span: s.token("7", 0),
            kind: RawExpressionKind::I32Literal { spelling: "7".to_owned() },
        },
        s.vector(("Vec<i32>", 2), 7, 2, 1, (1, 0), vec![0]),
        s.vector(("Vec<i32>", 3), 9, 3, 2, (2, 1), vec![]),
        s.vector(("Vec<Vec<i32>>", 1), 5, 1, 3, (0, 2), vec![1, 2]),
    ];
    let raw = s.snapshot(types, vec![], 2, expressions);
    (s, raw)
}

fn nested_enum_fixture() -> (NestedSource, RawProjectSyntaxSnapshot) {
    let s = NestedSource(
        "interface Choice extends ZrynaEnum { none: ZrynaNone; some: Vec<String>; }\nfunction make(): Vec<Choice> { return Vec<Choice>([Choice.some(Vec<String>([\"a\"]))]); }",
    );
    let types = vec![
        s.string_type(0),
        s.vec_type("Vec<String>", 0, 0),
        s.named_type("Choice", 1),
        s.vec_type("Vec<Choice>", 0, 2),
        s.named_type("Choice", 2),
        s.vec_type("Vec<Choice>", 1, 4),
        s.string_type(1),
        s.vec_type("Vec<String>", 1, 6),
    ];
    let declaration = RawDataDeclaration {
        span: s.range(("interface", 0), ("}", 0)),
        export_span: None,
        kind: RawDataDeclarationKind::Enum {
            interface_span: s.token("interface", 0),
            name: s.name("Choice", 0),
            extends_span: s.token("extends", 0),
            marker_span: s.token("ZrynaEnum", 0),
            open_brace_span: s.token("{", 0),
            close_brace_span: s.token("}", 0),
            variants: vec![
                RawEnumVariant {
                    span: s.range(("none", 0), (";", 0)),
                    name: s.name("none", 0),
                    colon_span: s.token(":", 0),
                    payload_type: None,
                    none_span: Some(s.token("ZrynaNone", 0)),
                    semicolon_span: s.token(";", 0),
                },
                RawEnumVariant {
                    span: s.range(("some", 0), (";", 1)),
                    name: s.name("some", 0),
                    colon_span: s.token(":", 1),
                    payload_type: Some(1),
                    none_span: None,
                    semicolon_span: s.token(";", 1),
                },
            ],
        },
    };
    let expressions = vec![
        RawExpressionSyntax {
            span: s.token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".to_owned() },
        },
        s.vector(("Vec<String>", 1), 7, 3, 1, (1, 0), vec![0]),
        RawExpressionSyntax {
            span: s.range(("Choice.some", 0), (")", 2)),
            kind: RawExpressionKind::EnumConstruction {
                type_name: s.name("Choice", 3),
                dot_span: s.token(".", 0),
                variant: s.name("some", 1),
                open_paren_span: s.token("(", 2),
                payload: Some(1),
                close_paren_span: s.token(")", 2),
            },
        },
        s.vector(("Vec<Choice>", 1), 5, 1, 3, (0, 1), vec![2]),
    ];
    let raw = s.snapshot(types, vec![declaration], 3, expressions);
    (s, raw)
}

#[allow(clippy::too_many_lines)]
fn assert_nested(selected_enum: bool) {
    let (source, raw) = if selected_enum { nested_enum_fixture() } else { nested_vec_fixture() };
    let sources = sources_for(source.0);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated nested constructor source");
    let input = pair_input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, _) = crate::data_ownership_v1::build_graph(input, &mut errors);
    assert!(errors.finish().is_empty());
    for target in
        [zryna_layout::StorageTarget::Linear32V1, zryna_layout::StorageTarget::LinuxX8664V1]
    {
        let layouts = zryna_layout::verify(&graph, &sources, target).expect("real nested layouts");
        assert_eq!(layouts.target(), target);
    }
    let expected_kinds = if selected_enum {
        [
            VerifiedInstructionKind::StringFromUtf8,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::EnumConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    } else {
        [
            VerifiedInstructionKind::I32Literal,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::VecConstruct,
            VerifiedInstructionKind::VecConstruct,
        ]
    };
    let expected_types = if selected_enum {
        [TypeCategory::String, TypeCategory::Vec, TypeCategory::Enum, TypeCategory::Vec]
    } else {
        [TypeCategory::I32, TypeCategory::Vec, TypeCategory::Vec, TypeCategory::Vec]
    };
    let expected_operands = if selected_enum {
        vec![vec![], vec![0], vec![1], vec![2]]
    } else {
        vec![vec![], vec![0], vec![], vec![1, 2]]
    };
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(input).expect("nested source must reach mandatory independent IR verification");
        let functions = program.modules().flat_map(VerifiedModule::functions).collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let function = functions[0];
        assert_eq!(function.parameters().count(), 0);
        let blocks = function.blocks().collect::<Vec<_>>();
        assert_eq!(blocks.len(), 1);
        let instructions = blocks[0].instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 4);
        let kinds = instructions.iter().map(|i| i.kind()).collect::<Vec<_>>();
        assert_eq!(kinds, expected_kinds);
        let results = instructions.iter().map(|i| i.result().expect("result")).collect::<Vec<_>>();
        assert_eq!(results.iter().map(|v| v.index()).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        let operands = instructions
            .iter()
            .map(|i| i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(operands, expected_operands);
        let layouts = program.verified_ir().linear32_layouts();
        let categories = instructions
            .iter()
            .map(|i| {
                layouts.type_by_id(i.result_type().expect("type")).expect("sealed type").category()
            })
            .collect::<Vec<_>>();
        assert_eq!(categories, expected_types);
        if selected_enum {
            assert_eq!(instructions[0].string_utf8_bytes(), Some(&b"a"[..]));
            assert_eq!(instructions[2].variant(), Some(1), "select some, never none");
        } else {
            assert_eq!(instructions[0].i32_literal(), Some(7));
            assert_eq!(instructions[1].result_type(), instructions[2].result_type());
        }
        let places = function.places().collect::<Vec<_>>();
        let first_owned = usize::from(!selected_enum);
        assert_eq!(places.len(), 4 - first_owned);
        for (index, place) in places.iter().enumerate() {
            let result = index + first_owned;
            assert!(!place.is_copy());
            assert_eq!(place.kind(), VerifiedPlaceKind::Temporary(results[result]));
            assert_eq!(place.ty(), instructions[result].result_type().expect("owned type"));
        }
        // Fallible sites retain exact pending owners in reverse; constructors transfer children.
        let cleanup_roots = instructions
            .iter()
            .map(|i| i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let expected_cleanup = if selected_enum {
            vec![vec![], vec![0], vec![], vec![2]]
        } else {
            vec![vec![], vec![], vec![0], vec![1, 0]]
        };
        assert_eq!(cleanup_roots, expected_cleanup);
        assert_eq!(function.cleanup_plans().count(), 4);
        let returned = blocks[0].terminator();
        assert_eq!(returned.kind(), VerifiedTerminatorKind::Return);
        assert_eq!(returned.value_operands().collect::<Vec<_>>(), vec![results[3]]);
        assert_eq!(returned.derived_drop_actions().count(), 0);
        let observation = (kinds, categories, operands, cleanup_roots);
        if let Some(previous) = &previous {
            assert_eq!(&observation, previous);
        }
        previous = Some(observation);
    }
}

#[test]
fn mixed_nested_vec_with_empty_inner_reaches_verified_ir() {
    assert_nested(false);
}

#[test]
fn mixed_nested_selected_enum_vec_payload_reaches_verified_ir() {
    assert_nested(true);
}

// Append inside the nested mixed fixture module; no production visibility changes.
pub(in crate::data_ownership_v1) fn cleanup_frontier_fixture() -> (String, RawProjectSyntaxSnapshot)
{
    let s = NestedSource(
        "function make(): Vec<Vec<i32>> { return Vec<Vec<i32>>([Vec<i32>([]), Vec<i32>([7])]); }",
    );
    let types = vec![
        s.named_type("i32", 0),
        s.vec_type("Vec<i32>", 0, 0),
        s.vec_type("Vec<Vec<i32>>", 0, 1),
        s.named_type("i32", 1),
        s.vec_type("Vec<i32>", 1, 3),
        s.vec_type("Vec<Vec<i32>>", 1, 4),
        s.named_type("i32", 2),
        s.vec_type("Vec<i32>", 2, 6),
        s.named_type("i32", 3),
        s.vec_type("Vec<i32>", 3, 8),
    ];
    let expressions = vec![
        s.vector(("Vec<i32>", 2), 7, 2, 1, (1, 0), vec![]),
        RawExpressionSyntax {
            span: s.token("7", 0),
            kind: RawExpressionKind::I32Literal { spelling: "7".to_owned() },
        },
        s.vector(("Vec<i32>", 3), 9, 3, 2, (2, 1), vec![1]),
        s.vector(("Vec<Vec<i32>>", 1), 5, 1, 3, (0, 2), vec![0, 2]),
    ];
    let raw = s.snapshot(types, vec![], 2, expressions);
    (s.0.to_owned(), raw)
}
#[path = "mixed_constructor_faults.rs"]
mod faults;
#[path = "mixed_struct_whole_moves.rs"]
mod struct_whole_moves;
#[path = "mixed_enum_whole_moves.rs"]
mod whole_moves;
