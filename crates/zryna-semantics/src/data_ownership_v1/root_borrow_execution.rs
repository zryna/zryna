use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;

use super::type_model::{
    RootBorrowArmPlan, RootBorrowCallArgumentPlan, RootBorrowCallPlan, RootBorrowInitializer,
    RootBorrowLiteral, RootBorrowPlacePlan, RootBorrowProjectionKey, RootBorrowStep,
};

pub(super) fn emit_root_borrow_initializer(
    initializer: RootBorrowInitializer,
    values: &mut u32,
    instructions: &mut Vec<raw::Instruction>,
) -> Option<raw::ValueId> {
    let (ty, at, kind) = match initializer {
        RootBorrowInitializer::Literal { literal, ty, at } => {
            let kind = match literal {
                RootBorrowLiteral::Bool(value) => raw::InstructionKind::BoolLiteral(value),
                RootBorrowLiteral::I32(value) => raw::InstructionKind::I32Literal(value),
            };
            (ty, at, kind)
        }
        RootBorrowInitializer::Struct { ty, fields, at } => {
            let fields = fields
                .into_iter()
                .map(|field| emit_root_borrow_initializer(field, values, instructions))
                .collect::<Option<Vec<_>>>()?;
            (ty, at, raw::InstructionKind::StructConstruct { fields, cleanup: None })
        }
        RootBorrowInitializer::FixedArray { ty, elements, at } => {
            let elements = elements
                .into_iter()
                .map(|element| emit_root_borrow_initializer(element, values, instructions))
                .collect::<Option<Vec<_>>>()?;
            (ty, at, raw::InstructionKind::FixedArrayConstruct { elements, cleanup: None })
        }
    };
    let value = raw::ValueId(*values);
    *values = values.checked_add(1)?;
    instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
        span: at,
        kind,
    });
    Some(value)
}

fn materialize_root_borrow_place(
    plan: &RootBorrowPlacePlan,
    root: raw::PlaceId,
    places: &mut Vec<raw::Place>,
    projected: &mut BTreeMap<Vec<RootBorrowProjectionKey>, raw::PlaceId>,
) -> Option<raw::PlaceId> {
    let mut parent = root;
    let mut prefix = Vec::with_capacity(plan.projections.len());
    for projection in &plan.projections {
        prefix.push(projection.key);
        if let Some(place) = projected.get(&prefix).copied() {
            parent = place;
            continue;
        }
        let id = raw::PlaceId(u32::try_from(places.len()).ok()?);
        let kind = match projection.key {
            RootBorrowProjectionKey::StructField(ordinal) => {
                raw::PlaceKind::StructField { base: parent, ordinal }
            }
            RootBorrowProjectionKey::FixedArrayConstant(index) => {
                raw::PlaceKind::FixedArrayConstant { base: parent, index }
            }
        };
        places.push(raw::Place { id, ty: projection.ty.ir, span: projection.at, kind });
        projected.insert(prefix.clone(), id);
        parent = id;
    }
    Some(parent)
}

fn emit_root_borrow_call(
    call: RootBorrowCallPlan,
    cleanup: raw::CleanupPlanId,
    values: &mut u32,
    places: &mut Vec<raw::Place>,
    instructions: &mut Vec<raw::Instruction>,
) -> Option<()> {
    let mut value_arguments = vec![None; call.value_parameters];
    let mut borrow_arguments = vec![None; call.borrow_parameters];
    for argument in call.arguments {
        match argument {
            RootBorrowCallArgumentPlan::Value { index, value } => {
                let value = emit_root_borrow_initializer(value, values, instructions)?;
                *value_arguments.get_mut(usize::try_from(index).ok()?)? = Some(value);
            }
            RootBorrowCallArgumentPlan::Borrow { index, id } => {
                *borrow_arguments.get_mut(usize::try_from(index).ok()?)? = Some(id);
            }
        }
    }
    let arguments = value_arguments
        .into_iter()
        .map(|value| value.map(raw::CallArgument::Value))
        .chain(borrow_arguments.into_iter().map(|borrow| borrow.map(raw::CallArgument::Borrow)))
        .collect::<Option<Vec<_>>>()?;
    let result = raw::ValueId(*values);
    *values = values.checked_add(1)?;
    instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: result, ty: call.result.ir, span: call.at }),
        span: call.at,
        kind: raw::InstructionKind::DirectCall { callee: call.callee, arguments, cleanup },
    });
    let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
    places.push(raw::Place {
        id: place,
        ty: call.result.ir,
        span: call.at,
        kind: raw::PlaceKind::Local(place.0),
    });
    instructions.push(raw::Instruction {
        result: None,
        span: call.at,
        kind: raw::InstructionKind::InitializePlace { place, value: result },
    });
    Some(())
}

pub(super) fn emit_root_borrow_arm(
    arm: RootBorrowArmPlan,
    root_place: raw::PlaceId,
    materialize_reads: bool,
    values: &mut u32,
    places: &mut Vec<raw::Place>,
    instructions: &mut Vec<raw::Instruction>,
    call_cleanup: &mut Option<raw::CleanupPlanId>,
) -> Option<()> {
    let mut begun = Vec::with_capacity(arm.aliases);
    let mut projected = BTreeMap::new();
    for step in arm.steps {
        match step {
            RootBorrowStep::Begin { id, place, access, at } => {
                let place =
                    materialize_root_borrow_place(&place, root_place, places, &mut projected)?;
                instructions.push(raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                        id,
                        place,
                        access,
                        span: at,
                    }),
                });
                begun.push(id);
            }
            RootBorrowStep::Read { id, ty, at } => {
                let value = raw::ValueId(*values);
                *values = values.checked_add(1)?;
                instructions.push(raw::Instruction {
                    result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
                    span: at,
                    kind: raw::InstructionKind::BorrowRead { borrow: id },
                });
                if materialize_reads {
                    let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                    places.push(raw::Place {
                        id: place,
                        ty: ty.ir,
                        span: at,
                        kind: raw::PlaceKind::Local(place.0),
                    });
                    instructions.push(raw::Instruction {
                        result: None,
                        span: at,
                        kind: raw::InstructionKind::InitializePlace { place, value },
                    });
                }
            }
            RootBorrowStep::OwnerRead { place, at } => {
                let ty = place.ty;
                let place =
                    materialize_root_borrow_place(&place, root_place, places, &mut projected)?;
                let value = raw::ValueId(*values);
                *values = values.checked_add(1)?;
                instructions.push(raw::Instruction {
                    result: Some(raw::ValueDefinition { id: value, ty: ty.ir, span: at }),
                    span: at,
                    kind: raw::InstructionKind::CopyFromPlace { place },
                });
                if materialize_reads {
                    let place = raw::PlaceId(u32::try_from(places.len()).ok()?);
                    places.push(raw::Place {
                        id: place,
                        ty: ty.ir,
                        span: at,
                        kind: raw::PlaceKind::Local(place.0),
                    });
                    instructions.push(raw::Instruction {
                        result: None,
                        span: at,
                        kind: raw::InstructionKind::InitializePlace { place, value },
                    });
                }
            }
            RootBorrowStep::Write { id, value, at } => {
                let value = emit_root_borrow_initializer(value, values, instructions)?;
                instructions.push(raw::Instruction {
                    result: None,
                    span: at,
                    kind: raw::InstructionKind::BorrowWrite { borrow: id, value },
                });
            }
            RootBorrowStep::Call(call) => {
                emit_root_borrow_call(call, call_cleanup.take()?, values, places, instructions)?;
            }
        }
    }
    for borrow in begun.into_iter().rev() {
        instructions.push(raw::Instruction {
            result: None,
            span: arm.block_exit,
            kind: raw::InstructionKind::EndBorrow { borrow },
        });
    }
    Some(())
}
