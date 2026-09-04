use super::*;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::RawExpressionKind;

fn at(start: usize, end: usize) -> UntrustedSpan {
    UntrustedSpan {
        file: 0,
        start: u32::try_from(start).expect("span"),
        end: u32::try_from(end).expect("span"),
    }
}

pub(in crate::data_ownership_v1) fn local_commit_fixture(
    invalid: bool,
) -> (String, RawProjectSyntaxSnapshot) {
    let (mut source, mut raw) = struct_whole_moves::fixture(false);
    let start = source.find("return ").expect("final return");
    let text = if invalid { "const copy: Parcel = lost; " } else { "const copy: Parcel = item; " };
    source.insert_str(start, text);
    raw = shift_snapshot(
        raw,
        u32::try_from(start).expect("span"),
        u32::try_from(text.len()).expect("length"),
    );
    let local_type = u32::try_from(raw.files[0].type_syntax.len()).expect("type");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(start + 12, start + 18),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "Parcel".into(), span: at(start + 12, start + 18) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 5);
    let RawExpressionKind::Reference { name } = &mut body.expressions[3].kind else {
        panic!("return local")
    };
    assert_eq!(name.text, "item");
    source.replace_range(name.span.start as usize..name.span.end as usize, "copy");
    name.text = "copy".into();
    let name = if invalid { "lost" } else { "item" };
    body.expressions.push(RawExpressionSyntax {
        span: at(start + 21, start + 25),
        kind: RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: name.into(), span: at(start + 21, start + 25) },
        },
    });
    body.statements.insert(
        1,
        RawStatementSyntax {
            span: at(start, start + 26),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(start, start + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "copy".into(), span: at(start + 6, start + 10) },
                type_syntax: local_type,
                equals_span: at(start + 19, start + 20),
                initializer: 5,
                semicolon_span: at(start + 25, start + 26),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1, 2];
    (source, raw)
}

#[test]
fn mixed_local_commit_fixture_preserves_real_prior_statement_and_full_ir_move_chain() {
    let (source, raw) = local_commit_fixture(false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("actual two-local source");
    for _ in 0..2 {
        let program = lower(pair_input(&syntax, &sources)).expect("two-local full IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        assert_eq!(
            block
                .instructions()
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>(),
            vec![
                VerifiedInstructionKind::StringFromUtf8,
                VerifiedInstructionKind::VecConstruct,
                VerifiedInstructionKind::StructConstruct,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::InitializePlace,
                VerifiedInstructionKind::MoveFromPlace,
                VerifiedInstructionKind::VecConstruct,
            ]
        );
        assert_eq!(function.places().count(), 8);
        assert_eq!(function.cleanup_plans().count(), 4);
        assert_eq!(block.terminator().derived_drop_actions().count(), 0);
    }
}
