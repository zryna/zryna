use super::super::generic_clone_fixture::Fixture;
use super::super::*;
use zryna_layout::TypeCategory;

pub(super) struct Authority {
    fixture: Fixture,
    enumeration: raw::TypeId,
    payload: raw::TypeId,
}

impl Authority {
    pub(super) fn new() -> Self {
        let fixture = Fixture::new(TypeCategory::Enum);
        let enumeration = fixture
            .linear
            .types()
            .find(|ty| ty.category() == TypeCategory::Enum)
            .expect("enum type");
        let payload = enumeration.variants()[1].payload().expect("struct payload");
        Self {
            enumeration: raw::TypeId(enumeration.id().index()),
            payload: raw::TypeId(payload.index()),
            fixture,
        }
    }

    pub(super) fn check(
        &self,
        raw: raw::Program,
    ) -> Result<crate::data_ownership_v1::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
        verify(
            raw,
            &self.fixture.sources,
            self.fixture.sources.verify_file_id(0).expect("entry"),
            self.fixture.linear.clone(),
            self.fixture.linux.clone(),
        )
    }
}

fn instruction(
    span: zryna_source::Span,
    result: Option<(u32, raw::TypeId)>,
    kind: raw::InstructionKind,
) -> raw::Instruction {
    raw::Instruction {
        result: result.map(|(id, ty)| raw::ValueDefinition { id: raw::ValueId(id), ty, span }),
        span,
        kind,
    }
}

fn jump(span: zryna_source::Span, target: u32) -> raw::SpannedTerminator {
    raw::SpannedTerminator {
        span,
        kind: raw::Terminator::Jump(raw::Edge { target: raw::BlockId(target), arguments: vec![] }),
    }
}

fn arm(variant: u32, target: u32) -> raw::EnumArm {
    raw::EnumArm { variant, edge: raw::Edge { target: raw::BlockId(target), arguments: vec![] } }
}

fn move_and_discharge(
    span: zryna_source::Span,
    payload: raw::PlaceId,
    result: u32,
    ty: raw::TypeId,
    temporary: raw::PlaceId,
    roots: &[raw::PlaceId],
) -> Vec<raw::Instruction> {
    let move_kind = if ty.0 == 2 {
        raw::InstructionKind::MoveFromPlace { place: payload }
    } else {
        raw::InstructionKind::GenericMoveFromPlace { place: payload }
    };
    let mut instructions = vec![
        instruction(span, Some((result, ty)), move_kind),
        instruction(span, None, raw::InstructionKind::DropPlace { place: temporary }),
    ];
    instructions.extend(
        roots.iter().map(|place| {
            instruction(span, None, raw::InstructionKind::DropPlace { place: *place })
        }),
    );
    instructions
}

#[allow(clippy::too_many_lines)]
pub(super) fn seed(authority: &Authority) -> raw::Program {
    let mut raw = authority.fixture.seed();
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let string = authority.fixture.string;
    let integer = authority.fixture.integer;
    let enumeration = authority.enumeration;
    let payload = authority.payload;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.parameters = vec![
        value(0, enumeration),
        value(1, enumeration),
        value(2, string),
        value(3, string),
        value(4, integer),
    ];
    function.result = integer;
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: enumeration,
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: enumeration,
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place { id: raw::PlaceId(2), ty: string, span, kind: raw::PlaceKind::Parameter(2) },
        raw::Place { id: raw::PlaceId(3), ty: string, span, kind: raw::PlaceKind::Parameter(3) },
        raw::Place {
            id: raw::PlaceId(4),
            ty: string,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: payload,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: string,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: integer,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(8),
            ty: string,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
        raw::Place {
            id: raw::PlaceId(9),
            ty: payload,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(6)),
        },
        raw::Place {
            id: raw::PlaceId(10),
            ty: string,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(11),
            ty: payload,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(12),
            ty: string,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(11), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(13),
            ty: integer,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(11), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(14),
            ty: string,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(7)),
        },
        raw::Place {
            id: raw::PlaceId(15),
            ty: payload,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(8)),
        },
    ];
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::EnumMatch {
                    place: raw::PlaceId(0),
                    arms: vec![arm(0, 1), arm(1, 2)],
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![],
            instructions: move_and_discharge(
                span,
                raw::PlaceId(4),
                5,
                string,
                raw::PlaceId(8),
                &[raw::PlaceId(0), raw::PlaceId(1)],
            ),
            terminators: vec![jump(span, 5)],
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![],
            instructions: move_and_discharge(
                span,
                raw::PlaceId(5),
                6,
                payload,
                raw::PlaceId(9),
                &[raw::PlaceId(0)],
            ),
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::EnumMatch {
                    place: raw::PlaceId(1),
                    arms: vec![arm(0, 3), arm(1, 4)],
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(3),
            parameters: vec![],
            instructions: move_and_discharge(
                span,
                raw::PlaceId(10),
                7,
                string,
                raw::PlaceId(14),
                &[raw::PlaceId(1)],
            ),
            terminators: vec![jump(span, 5)],
        },
        raw::Block {
            id: raw::BlockId(4),
            parameters: vec![],
            instructions: move_and_discharge(
                span,
                raw::PlaceId(11),
                8,
                payload,
                raw::PlaceId(15),
                &[raw::PlaceId(1)],
            ),
            terminators: vec![jump(span, 5)],
        },
        raw::Block {
            id: raw::BlockId(5),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(4),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    function.cleanup_plans = vec![raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span,
        actions: vec![
            raw::DropAction::DropPlace(raw::PlaceId(3)),
            raw::DropAction::DropPlace(raw::PlaceId(2)),
        ],
    }];
    raw
}
