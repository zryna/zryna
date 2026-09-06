use super::*;
use fixtures::{cleanup, definition, instruction, place, returning};
use zryna_layout::TypeCategory;

// Explicit constructors prove active variants, fixed lengths and Vec operand
// counts. Incoming field/element values are authenticated parameters, not source
// producers, executed allocations or dynamically observed control counts.
fn constructor_inputs(
    record: zryna_layout::VerifiedType<'_>,
    ordinal_or_length: u32,
) -> Vec<raw::TypeId> {
    match record.category() {
        TypeCategory::Struct => {
            record.fields().iter().map(|field| raw::TypeId(field.ty().index())).collect::<Vec<_>>()
        }
        TypeCategory::Enum => record.variants()[ordinal_or_length as usize]
            .payload()
            .map(|ty| vec![raw::TypeId(ty.index())])
            .unwrap_or_default(),
        TypeCategory::FixedArray | TypeCategory::Vec => {
            let count = record.array_length().unwrap_or(u64::from(ordinal_or_length));
            vec![
                raw::TypeId(record.referenced_type().expect("element").index());
                usize::try_from(count).expect("small count")
            ]
        }
        TypeCategory::Shared => {
            vec![raw::TypeId(record.referenced_type().expect("payload").index())]
        }
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => vec![],
        TypeCategory::Weak => {
            panic!("Weak requires a retained Shared source, not a value constructor")
        }
    }
}

fn constructor_operation(
    category: TypeCategory,
    ordinal_or_length: u32,
    count: u32,
) -> raw::InstructionKind {
    let values = (0..count).map(raw::ValueId).collect::<Vec<_>>();
    let prepare = raw::CleanupPlanId(2);
    match category {
        TypeCategory::Bool => raw::InstructionKind::BoolLiteral(true),
        TypeCategory::I32 => raw::InstructionKind::I32Literal(42),
        TypeCategory::String => {
            raw::InstructionKind::StringFromUtf8 { bytes: b"payload".to_vec(), cleanup: prepare }
        }
        TypeCategory::Struct => {
            raw::InstructionKind::StructConstruct { fields: values, cleanup: None }
        }
        TypeCategory::Enum => raw::InstructionKind::EnumConstruct {
            variant: ordinal_or_length,
            payload: values.first().copied(),
            cleanup: None,
        },
        TypeCategory::FixedArray => {
            raw::InstructionKind::FixedArrayConstruct { elements: values, cleanup: None }
        }
        TypeCategory::Vec => {
            raw::InstructionKind::VecConstruct { elements: values, cleanup: prepare }
        }
        TypeCategory::Shared => {
            raw::InstructionKind::SharedConstruct { value: values[0], cleanup: prepare }
        }
        TypeCategory::Weak => unreachable!(),
    }
}

fn constructed(fixture: &Fixture, ordinal_or_length: u32) -> raw::Program {
    let record = fixture
        .linear
        .types()
        .find(|ty| ty.id().index() == fixture.payload.0)
        .expect("payload type");
    let inputs = constructor_inputs(record, ordinal_or_length);
    let count = u32::try_from(inputs.len()).expect("small fixture");
    let operation = constructor_operation(record.category(), ordinal_or_length, count);
    let mut raw = fixture.program();
    let f = &mut raw.modules[0].functions[0];
    let span = f.span;
    f.parameters = inputs
        .iter()
        .enumerate()
        .map(|(id, ty)| definition(u32::try_from(id).expect("bounded"), *ty, span))
        .collect();
    f.result = fixture.shared;
    f.places = inputs
        .iter()
        .enumerate()
        .map(|(id, ty)| {
            let id = u32::try_from(id).expect("bounded");
            place(id, *ty, raw::PlaceKind::Parameter(id), span)
        })
        .collect();
    f.places.push(place(
        count,
        fixture.payload,
        raw::PlaceKind::Temporary(raw::ValueId(count)),
        span,
    ));
    f.places.push(place(
        count + 1,
        fixture.shared,
        raw::PlaceKind::Temporary(raw::ValueId(count + 1)),
        span,
    ));
    f.blocks.truncate(1);
    f.blocks[0].instructions = vec![
        instruction(count, fixture.payload, operation, span),
        instruction(
            count + 1,
            fixture.shared,
            raw::InstructionKind::SharedConstruct {
                value: raw::ValueId(count),
                cleanup: raw::CleanupPlanId(0),
            },
            span,
        ),
    ];
    f.blocks[0].terminators = vec![returning(count + 1, 1, span)];
    f.cleanup_plans = vec![
        cleanup(0, if record.drop_kind() == 0 { vec![] } else { vec![count] }, span),
        cleanup(1, vec![], span),
    ];
    if matches!(record.category(), TypeCategory::String | TypeCategory::Vec | TypeCategory::Shared)
    {
        let owned = inputs
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, ty)| {
                fixture
                    .linear
                    .types()
                    .find(|record| record.id().index() == ty.0)
                    .is_some_and(|record| record.drop_kind() != 0)
            })
            .map(|(index, _)| u32::try_from(index).expect("bounded input"))
            .collect();
        f.cleanup_plans.push(cleanup(2, owned, span));
    }
    raw
}

#[test]
fn shared_construct_accepts_explicit_payload_shapes_and_retains_exact_prepare_owner() {
    for (payload, variants_or_lengths) in [
        (Payload::Bool, vec![0]),
        (Payload::I32, vec![0]),
        (Payload::String, vec![0]),
        (Payload::Struct, vec![0]),
        (Payload::Enum, vec![0, 1, 2]),
        (Payload::EmptyArray, vec![0]),
        (Payload::Array, vec![2]),
        (Payload::Vec, vec![0, 2]),
        (Payload::Shared, vec![0]),
        (Payload::Nested, vec![0]),
        (Payload::Recursive, vec![0, 1]),
    ] {
        let fixture = Fixture::new(payload);
        for ordinal in variants_or_lengths {
            let raw = constructed(&fixture, ordinal);
            for _ in 0..2 {
                let verified = fixture
                    .verify(raw.clone())
                    .expect("fully constructed payload before shared allocation");
                let function = verified
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function");
                let block = function.blocks().next().expect("block");
                let construct = block.instructions().nth(1).expect("Shared construction");
                assert_eq!(construct.kind(), VerifiedInstructionKind::SharedConstruct);
                assert_eq!(construct.result_type().expect("result type").index(), fixture.shared.0);
                let drops = construct
                    .derived_drop_actions()
                    .map(|action| action.root().index())
                    .collect::<Vec<_>>();
                let record = fixture
                    .linear
                    .types()
                    .find(|ty| ty.id().index() == fixture.payload.0)
                    .expect("payload");
                let expected = if record.drop_kind() == 0 {
                    vec![]
                } else {
                    vec![u32::try_from(function.parameters().count()).expect("parameters")]
                };
                assert_eq!(drops, expected, "{payload:?}/{ordinal}");
                if record.category() == TypeCategory::Enum && record.drop_kind() != 0 {
                    let action =
                        construct.derived_drop_actions().next().expect("enum payload cleanup");
                    assert_eq!(action.active_variant(), Some(ordinal));
                    assert_eq!(action.kind(), VerifiedDropActionKind::Place);
                    assert_eq!(action.moved_projections().count(), 0);
                }
                assert_eq!(
                    block.terminator().derived_drop_actions().count(),
                    0,
                    "returned Shared is transferred, not dropped"
                );
            }
        }
    }
}
