use super::super::*;

pub(super) const STRING_A: u32 = 0;
pub(super) const STRING_B: u32 = 1;
pub(super) const VEC: u32 = 2;
pub(super) const STRUCT: u32 = 3;
pub(super) const ENUM: u32 = 4;
pub(super) const SHARED: u32 = 5;
pub(super) const WEAK: u32 = 6;
pub(super) const FLAG_A: u32 = 7;
pub(super) const FLAG_B: u32 = 8;
pub(super) const JOIN_A: u32 = 9;
pub(super) const JOIN_B: u32 = 10;
pub(super) const HEADER_A: u32 = 11;

fn edge(target: u32, arguments: Vec<u32>) -> raw::Edge {
    raw::Edge {
        target: raw::BlockId(target),
        arguments: arguments.into_iter().map(raw::ValueId).collect(),
    }
}

fn terminator(span: zryna_source::Span, kind: raw::Terminator) -> raw::SpannedTerminator {
    raw::SpannedTerminator { span, kind }
}

fn block(id: u32, kind: raw::Terminator, span: zryna_source::Span) -> raw::Block {
    raw::Block {
        id: raw::BlockId(id),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminators: vec![terminator(span, kind)],
    }
}

fn mixed_blocks(span: zryna_source::Span) -> Vec<raw::Block> {
    let mut blocks = vec![
        block(
            0,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(1, vec![]),
                when_false: edge(2, vec![]),
            },
            span,
        ),
        block(1, raw::Terminator::Jump(edge(3, vec![STRING_A, STRING_B])), span),
        block(2, raw::Terminator::Jump(edge(3, vec![STRING_A, STRING_B])), span),
        block(3, raw::Terminator::Jump(edge(4, vec![JOIN_A])), span),
        block(
            4,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(5, vec![]),
                when_false: edge(6, vec![]),
            },
            span,
        ),
        block(
            5,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_B),
                when_true: edge(7, vec![]),
                when_false: edge(8, vec![]),
            },
            span,
        ),
        block(
            6,
            raw::Terminator::EnumMatch {
                place: raw::PlaceId(ENUM),
                arms: vec![raw::EnumArm { variant: 0, edge: edge(9, vec![]) }],
            },
            span,
        ),
        block(7, raw::Terminator::Jump(edge(4, vec![HEADER_A])), span),
        block(8, raw::Terminator::Jump(edge(4, vec![HEADER_A])), span),
        block(
            9,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_A),
                when_true: edge(10, vec![]),
                when_false: edge(11, vec![]),
            },
            span,
        ),
        block(
            10,
            raw::Terminator::Return { value: raw::ValueId(FLAG_A), cleanup: raw::CleanupPlanId(0) },
            span,
        ),
        block(
            11,
            raw::Terminator::Branch {
                condition: raw::ValueId(FLAG_B),
                when_true: edge(12, vec![]),
                when_false: edge(13, vec![]),
            },
            span,
        ),
        block(
            12,
            raw::Terminator::Return { value: raw::ValueId(FLAG_B), cleanup: raw::CleanupPlanId(1) },
            span,
        ),
        block(
            13,
            raw::Terminator::Trap {
                identity: raw::TrapIdentity::BoundsV1,
                cleanup: raw::CleanupPlanId(2),
            },
            span,
        ),
    ];
    blocks[3].parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(JOIN_A), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(JOIN_B), ty: raw::TypeId(2), span },
    ];
    blocks[4].parameters =
        vec![raw::ValueDefinition { id: raw::ValueId(HEADER_A), ty: raw::TypeId(2), span }];
    blocks
}

pub(super) fn mixed_cfg(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = [
        (STRING_A, 2),
        (STRING_B, 2),
        (VEC, 5),
        (STRUCT, 3),
        (ENUM, 4),
        (SHARED, 6),
        (WEAK, 7),
        (FLAG_A, 0),
        (FLAG_B, 0),
    ]
    .into_iter()
    .map(|(id, ty)| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(ty), span })
    .collect();
    function.result = raw::TypeId(0);
    function.places = [
        (0, 2, raw::PlaceKind::Parameter(STRING_A)),
        (1, 2, raw::PlaceKind::Parameter(STRING_B)),
        (2, 5, raw::PlaceKind::Parameter(VEC)),
        (3, 3, raw::PlaceKind::Parameter(STRUCT)),
        (4, 4, raw::PlaceKind::Parameter(ENUM)),
        (5, 6, raw::PlaceKind::Parameter(SHARED)),
        (6, 7, raw::PlaceKind::Parameter(WEAK)),
        (7, 2, raw::PlaceKind::Temporary(raw::ValueId(JOIN_A))),
        (8, 2, raw::PlaceKind::Temporary(raw::ValueId(JOIN_B))),
        (9, 2, raw::PlaceKind::Temporary(raw::ValueId(HEADER_A))),
        (10, 2, raw::PlaceKind::StructField { base: raw::PlaceId(STRUCT), ordinal: 0 }),
    ]
    .into_iter()
    .map(|(id, ty, kind)| raw::Place { id: raw::PlaceId(id), ty: raw::TypeId(ty), span, kind })
    .collect();

    function.blocks = mixed_blocks(span);
    let actions = [6, 5, 4, 3, 2, 8, 9]
        .into_iter()
        .map(|id| raw::DropAction::DropPlace(raw::PlaceId(id)))
        .collect::<Vec<_>>();
    function.cleanup_plans = (0..3)
        .map(|id| raw::CleanupPlan { id: raw::CleanupPlanId(id), span, actions: actions.clone() })
        .collect();
    program
}

pub(super) fn verify_mixed(
    program: raw::Program,
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> Result<super::super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
    verify(
        program,
        sources,
        sources.verify_file_id(0).expect("entry"),
        linear.clone(),
        linux.clone(),
    )
}

pub(super) fn trace(entries: &[(&str, &str)]) -> DiagnosticTrace {
    entries
        .iter()
        .map(|(code, message)| (code.to_string(), message.to_string(), Some((0, 53))))
        .collect()
}

pub(super) fn reject_replay_recover(
    hostile: raw::Program,
    expected: &DiagnosticTrace,
    authorities: &(SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts),
) {
    let (sources, linear, linux) = authorities;
    let first = diagnostic_trace(
        verify_mixed(hostile.clone(), sources, linear, linux).expect_err("hostile IR must reject"),
    );
    let second = diagnostic_trace(
        verify_mixed(hostile, sources, linear, linux).expect_err("hostile replay must reject"),
    );
    assert_eq!(&first, expected);
    assert_eq!(&second, expected);
    verify_mixed(mixed_cfg(sources, linear, linux), sources, linear, linux)
        .expect("valid authority recovers after rejection");
}
