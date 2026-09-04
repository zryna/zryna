use super::NestedSource;
use crate::data_ownership_v1::tests::*;
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1::{PlaceIdentity, ValueIdentity};
use zryna_syntax::v4::{RawExpressionKind, RawFieldInitializer, RawFieldInitializerKind};

pub(super) fn fixture(duplicate: bool) -> (String, RawProjectSyntaxSnapshot) {
    let s = NestedSource(if duplicate {
        "interface Parcel extends ZrynaStruct { value: Vec<String>; }\nfunction make(): Vec<Parcel> { const item: Parcel = Parcel({ value: Vec<String>([\"a\"]) }); return Vec<Parcel>([item, item]); }"
    } else {
        "interface Parcel extends ZrynaStruct { value: Vec<String>; }\nfunction make(): Vec<Parcel> { const item: Parcel = Parcel({ value: Vec<String>([\"a\"]) }); return Vec<Parcel>([item]); }"
    });
    let (_, original) = mixed_construction::mixed_fixture(false);
    let declarations = original.files[0].data_declarations.clone();
    let types = vec![
        s.string_type(0),
        s.vec_type("Vec<String>", 0, 0),
        s.named_type("Parcel", 1),
        s.vec_type("Vec<Parcel>", 0, 2),
        s.named_type("Parcel", 2),
        s.string_type(1),
        s.vec_type("Vec<String>", 1, 5),
        s.named_type("Parcel", 4),
        s.vec_type("Vec<Parcel>", 1, 7),
    ];
    let reference = |ordinal| RawExpressionSyntax {
        span: s.token("item", ordinal),
        kind: RawExpressionKind::Reference { name: s.name("item", ordinal) },
    };
    let mut expressions = vec![
        RawExpressionSyntax {
            span: s.token("\"a\"", 0),
            kind: RawExpressionKind::StringLiteral { spelling: "\"a\"".into() },
        },
        s.vector(("Vec<String>", 1), 6, 2, 1, (0, 0), vec![0]),
        RawExpressionSyntax {
            span: s.range(("Parcel({", 0), (")", 2)),
            kind: RawExpressionKind::StructConstruction {
                type_name: s.name("Parcel", 3),
                open_paren_span: s.token("(", 1),
                open_brace_span: s.token("{", 2),
                fields: vec![RawFieldInitializer {
                    span: s.range(("value", 1), (")", 1)),
                    kind: RawFieldInitializerKind::Explicit {
                        name: s.name("value", 1),
                        colon_span: s.token(":", 3),
                        value: 1,
                    },
                }],
                close_brace_span: s.token("}", 1),
                close_paren_span: s.token(")", 2),
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
    expressions.push(s.vector(("Vec<Parcel>", 1), 8, 3, 3, (1, 1), children));
    let mut raw = s.snapshot(types, declarations, 3, expressions);
    let function = &mut raw.files[0].functions[0];
    function.span = s.range(("function", 0), ("}", 2));
    function.body.span = s.range(("{", 1), ("}", 2));
    function.body.blocks[0].span = function.body.span;
    function.body.blocks[0].close_brace_span = s.token("}", 2);
    function.body.blocks[0].statements = vec![0, 1];
    function.body.statements = vec![
        RawStatementSyntax {
            span: s.range(("const", 0), (";", 1)),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: s.token("const", 0),
                mutable: false,
                name: s.name("item", 0),
                type_syntax: 4,
                equals_span: s.token("=", 0),
                initializer: 2,
                semicolon_span: s.token(";", 1),
            },
        },
        RawStatementSyntax {
            span: s.range(("return", 0), (";", 2)),
            kind: RawStatementKind::Return {
                keyword_span: s.token("return", 0),
                value: root,
                semicolon_span: s.token(";", 2),
            },
        },
    ];
    (s.0.to_owned(), raw)
}

#[test]
fn mixed_owned_struct_local_moves_once_into_outer_vec_with_exact_cleanup() {
    let (source, raw) = fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated mixed Struct local");
    let mut previous = None;
    for _ in 0..2 {
        let program =
            lower(pair_input(&syntax, &sources)).expect("mixed Struct whole move full IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(
            instructions.iter().map(|i| i.kind()).collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(function.places().count(), 6);
        assert_eq!(function.cleanup_plans().count(), 4);
        assert_eq!(
            instructions[4].place_operands().map(PlaceIdentity::index).collect::<Vec<_>>(),
            vec![3]
        );
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
fn mixed_owned_struct_second_whole_move_reports_exact_source_unavailable() {
    let (source, raw) = fixture(true);
    let missing = raw.files[0].functions[0].body.expressions[4].span;
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated duplicate Struct reference");
    let expected = vec![Diagnostic::error_at(
        "ZRYNA-M3014",
        span(&sources, missing),
        "aggregate value 'item' is moved or only partially available",
        "move a whole owned aggregate only before moving any of its projections",
    )];
    for _ in 0..2 {
        assert_eq!(
            lower(pair_input(&syntax, &sources)).expect_err("unique source owner"),
            expected
        );
    }
}
