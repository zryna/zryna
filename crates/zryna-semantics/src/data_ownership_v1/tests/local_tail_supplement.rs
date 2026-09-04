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

pub(in crate::data_ownership_v1) fn tail_fixture(copy: bool) -> (String, RawProjectSyntaxSnapshot) {
    if !copy {
        return mixed_local_construction::string_clone_fixture();
    }
    let (mut source, mut raw) = mixed_construction::mixed_fixture(true);
    let start = source.find("return ").expect("return");
    let prefix = "const count: i32 = 7; ";
    source.insert_str(start, prefix);
    raw = shift_snapshot(
        raw,
        u32::try_from(start).expect("span"),
        u32::try_from(prefix.len()).expect("length"),
    );
    let ty = u32::try_from(raw.files[0].type_syntax.len()).expect("type ID");
    raw.files[0].type_syntax.push(RawTypeSyntax {
        span: at(start + 13, start + 16),
        kind: RawTypeSyntaxKind::Named {
            name: RawIdentifierSyntax { text: "i32".into(), span: at(start + 13, start + 16) },
        },
    });
    let body = &mut raw.files[0].functions[0].body;
    assert_eq!(body.expressions.len(), 3);
    body.expressions.push(RawExpressionSyntax {
        span: at(start + 19, start + 20),
        kind: RawExpressionKind::I32Literal { spelling: "7".into() },
    });
    body.statements.insert(
        0,
        RawStatementSyntax {
            span: at(start, start + 21),
            kind: RawStatementKind::LocalDeclaration {
                keyword_span: at(start, start + 5),
                mutable: false,
                name: RawIdentifierSyntax { text: "count".into(), span: at(start + 6, start + 11) },
                type_syntax: ty,
                equals_span: at(start + 17, start + 18),
                initializer: 3,
                semicolon_span: at(start + 20, start + 21),
            },
        },
    );
    body.blocks[0].statements = vec![0, 1];
    (source, raw)
}

#[test]
fn mixed_local_tail_copy_and_string_source_controls_verify_complete_programs() {
    for copy in [true, false] {
        let (source, raw) = tail_fixture(copy);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated local tail source");
        for _ in 0..2 {
            let program = lower(pair_input(&syntax, &sources)).expect("local tail full IR");
            let function =
                program.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            assert_eq!(
                block
                    .instructions()
                    .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                    .collect::<Vec<_>>(),
                if copy {
                    vec![
                        VerifiedInstructionKind::I32Literal,
                        VerifiedInstructionKind::InitializePlace,
                        VerifiedInstructionKind::StringFromUtf8,
                        VerifiedInstructionKind::StructConstruct,
                        VerifiedInstructionKind::VecConstruct,
                    ]
                } else {
                    vec![
                        VerifiedInstructionKind::StringFromUtf8,
                        VerifiedInstructionKind::InitializePlace,
                        VerifiedInstructionKind::StringClone,
                        VerifiedInstructionKind::StructConstruct,
                        VerifiedInstructionKind::VecConstruct,
                    ]
                }
            );
            assert_eq!(function.places().count(), if copy { 4 } else { 5 });
            assert_eq!(block.terminator().derived_drop_actions().count(), usize::from(!copy));
        }
    }
}
