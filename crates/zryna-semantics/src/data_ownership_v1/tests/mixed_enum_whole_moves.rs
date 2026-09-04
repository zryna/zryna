use super::{NestedSource, nested_enum_fixture};
use crate::data_ownership_v1::tests::*;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity};
use zryna_syntax::v4::RawExpressionKind;

fn fixture(duplicate: bool) -> (String, RawProjectSyntaxSnapshot) {
    let source = NestedSource(if duplicate {
        "interface Choice extends ZrynaEnum { none: ZrynaNone; some: Vec<String>; }\nfunction make(): Vec<Choice> { const item: Choice = Choice.some(Vec<String>([\"a\"])); return Vec<Choice>([item, item]); }"
    } else {
        "interface Choice extends ZrynaEnum { none: ZrynaNone; some: Vec<String>; }\nfunction make(): Vec<Choice> { const item: Choice = Choice.some(Vec<String>([\"a\"])); return Vec<Choice>([item]); }"
    });
    let (_, original) = nested_enum_fixture();
    let declarations = original.files[0].data_declarations.clone();
    // The declaration prefix is byte-identical to the established fixture.
    let types = vec![
        source.string_type(0),
        source.vec_type("Vec<String>", 0, 0),
        source.named_type("Choice", 1),
        source.vec_type("Vec<Choice>", 0, 2),
        source.named_type("Choice", 2),
        source.string_type(1),
        source.vec_type("Vec<String>", 1, 5),
        source.named_type("Choice", 4),
        source.vec_type("Vec<Choice>", 1, 7),
    ];
    let reference = |ordinal| RawExpressionSyntax {
        span: source.token("item", ordinal),
        kind: RawExpressionKind::Reference { name: source.name("item", ordinal) },
    };
    let mut expressions = vec![
        RawExpressionSyntax {
            span: source.token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        source.vector(("Vec<String>", 1), 6, 2, 1, (0, 0), vec![0]),
        RawExpressionSyntax {
            span: source.range(("Choice.some", 0), (")", 2)),
            kind: RawExpressionKind::EnumConstruction {
                type_name: source.name("Choice", 3),
                dot_span: source.token(".", 0),
                variant: source.name("some", 1),
                open_paren_span: source.token("(", 1),
                payload: Some(1),
                close_paren_span: source.token(")", 2),
            },
        },
        reference(1),
    ];
    let children = if duplicate {
        expressions.push(reference(2));
        vec![3, 4]
    } else {
        vec![3]
    };
    let root = u32::try_from(expressions.len()).expect("bounded fixture");
    expressions.push(source.vector(("Vec<Choice>", 1), 8, 3, 3, (1, 1), children));
    let mut raw = source.snapshot(types, declarations, 3, expressions);
    let body = &mut raw.files[0].functions[0].body;
    body.statements = vec![
        RawStatementSyntax {
            span: source.range(("const", 0), (";", 2)),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: source.token("const", 0),
                mutable: false,
                name: source.name("item", 0),
                type_syntax: 4,
                equals_span: source.token("=", 0),
                initializer: 2,
                semicolon_span: source.token(";", 2),
            },
        },
        RawStatementSyntax {
            span: source.range(("return", 0), (";", 3)),
            kind: RawStatementKind::Return {
                keyword_span: source.token("return", 0),
                value: root,
                semicolon_span: source.token(";", 3),
            },
        },
    ];
    body.blocks[0].statements = vec![0, 1];
    (source.0.to_owned(), raw)
}

#[test]
fn mixed_owned_enum_local_moves_once_into_outer_vec_with_exact_cleanup() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated actual Enum local");
    let mut previous = None;
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("full owned Enum move IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::EnumConstruct,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(instructions[2].variant(), Some(1));
        assert_eq!(function.places().count(), 6);
        assert_eq!(function.cleanup_plans().count(), 4);
        assert_eq!(
            instructions[4].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![3]
        );
        let drop = instructions[5].derived_drop_actions().next().expect("moved Enum owner");
        assert_eq!((drop.root().index(), drop.active_variant()), (4, Some(1)));
        let observed = instructions
            .iter()
            .map(|i| {
                (
                    i.result().map(ValueIdentity::index),
                    i.value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
                    i.derived_drop_actions().map(|a| a.root().index()).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![
                (Some(0), vec![], vec![]),
                (Some(1), vec![0], vec![0]),
                (Some(2), vec![1], vec![]),
                (None, vec![2], vec![]),
                (Some(3), vec![], vec![]),
                (Some(4), vec![3], vec![4]),
            ]
        );
        assert_eq!(
            block.terminator().value_operands().map(ValueIdentity::index).collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
        if let Some(previous) = &previous {
            assert_eq!(&observed, previous);
        }
        previous = Some(observed);
    }
}

#[test]
fn mixed_owned_enum_second_whole_move_reports_exact_source_unavailable() {
    let (source, raw) = fixture(true);
    let bad = raw.files[0].functions[0].body.expressions[4].span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated duplicate Enum reference");
    let expected = vec![Diagnostic::error_at(
        "ZRYNA-M3014",
        span(&sources, bad),
        "aggregate value 'item' is moved or only partially available",
        "move a whole owned aggregate only before moving any of its projections",
    )];
    for _ in 0..2 {
        assert_eq!(
            lower(pair_input(&syntax, &sources)).expect_err("one unique source owner"),
            expected
        );
    }
}
