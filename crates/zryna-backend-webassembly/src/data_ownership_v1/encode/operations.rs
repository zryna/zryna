use wasm_encoder::{Function, Instruction, MemArg};
use zryna_ir::data_ownership_v1::{
    VerifiedBackendInstruction as B, VerifiedCallArgument, VerifiedFunction, VerifiedInstruction,
    VerifiedInstructionKind as K, VerifiedPlaceKind,
};
use zryna_layout::{TypeCategory, VerifiedLayouts};

use super::{Context, Locals, index_error, memory};

const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };

pub(super) fn store(body: &mut Function) {
    body.instruction(&Instruction::I32Store(WORD));
}
pub(super) fn load(body: &mut Function) {
    body.instruction(&Instruction::I32Load(WORD));
}

pub(super) fn place_address(
    function: VerifiedFunction<'_>,
    id: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let place = function.places().find(|place| place.id().index() == id).ok_or_else(index_error)?;
    match place.kind() {
        VerifiedPlaceKind::Parameter(_)
        | VerifiedPlaceKind::Local(_)
        | VerifiedPlaceKind::Temporary(_) => {
            body.instruction(&Instruction::LocalGet(locals.frame));
            body.instruction(&Instruction::I32Const(
                i32::try_from(id.checked_mul(4).ok_or_else(index_error)?)
                    .map_err(|_| index_error())?,
            ));
            body.instruction(&Instruction::I32Add);
        }
        VerifiedPlaceKind::StructField { base, ordinal } => {
            place_value(function, base.index(), locals, layouts, body)?;
            let base_type = function
                .places()
                .find(|candidate| candidate.id() == base)
                .ok_or_else(index_error)?
                .ty();
            let offset = layouts
                .type_by_id(base_type)
                .and_then(|ty| ty.fields().iter().find(|field| field.ordinal() == ordinal))
                .map(|field| field.offset())
                .ok_or_else(index_error)?;
            body.instruction(&Instruction::I32Const(
                i32::try_from(offset).map_err(|_| index_error())?,
            ));
            body.instruction(&Instruction::I32Add);
        }
        VerifiedPlaceKind::EnumPayload { base, .. } => {
            place_value(function, base.index(), locals, layouts, body)?;
            let base_type = function
                .places()
                .find(|candidate| candidate.id() == base)
                .ok_or_else(index_error)?
                .ty();
            let offset = layouts
                .type_by_id(base_type)
                .and_then(zryna_layout::VerifiedType::enum_payload_layout)
                .map(|layout| layout.0)
                .ok_or_else(index_error)?;
            body.instruction(&Instruction::I32Const(
                i32::try_from(offset).map_err(|_| index_error())?,
            ));
            body.instruction(&Instruction::I32Add);
        }
        VerifiedPlaceKind::FixedArrayConstant { base, index } => {
            place_value(function, base.index(), locals, layouts, body)?;
            let base_type = function
                .places()
                .find(|candidate| candidate.id() == base)
                .ok_or_else(index_error)?
                .ty();
            let stride = layouts
                .type_by_id(base_type)
                .and_then(zryna_layout::VerifiedType::array_stride)
                .ok_or_else(index_error)?;
            let offset = stride.checked_mul(u64::from(index)).ok_or_else(index_error)?;
            body.instruction(&Instruction::I32Const(
                i32::try_from(offset).map_err(|_| index_error())?,
            ));
            body.instruction(&Instruction::I32Add);
        }
    }
    Ok(())
}

pub(super) fn place_value(
    function: VerifiedFunction<'_>,
    id: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let place = function.places().find(|place| place.id().index() == id).ok_or_else(index_error)?;
    let ty = layouts.type_by_id(place.ty()).ok_or_else(index_error)?;
    place_address(function, id, locals, layouts, body)?;
    match place.kind() {
        VerifiedPlaceKind::Parameter(_)
        | VerifiedPlaceKind::Local(_)
        | VerifiedPlaceKind::Temporary(_) => load(body),
        _ => memory::load_value(ty, body),
    }
    Ok(())
}

#[allow(clippy::match_same_arms, clippy::too_many_lines)]
pub(super) fn instruction(
    function: VerifiedFunction<'_>,
    instruction: VerifiedInstruction<'_>,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let kind = instruction.kind();
    match (kind, instruction.backend_instruction()) {
        (K::BoolLiteral, B::BoolLiteral(value)) => {
            body.instruction(&Instruction::I32Const(i32::from(value)));
        }
        (K::I32Literal, B::I32Literal(value)) => {
            body.instruction(&Instruction::I32Const(value));
        }
        (
            K::I32Add
            | K::I32Sub
            | K::I32Mul
            | K::Eq
            | K::Ne
            | K::I32LtS
            | K::I32LeS
            | K::I32GtS
            | K::I32GeS,
            B::Binary(left, right),
        ) => {
            body.instruction(&Instruction::LocalGet(left.index()));
            body.instruction(&Instruction::LocalGet(right.index()));
            body.instruction(&match kind {
                K::I32Add => Instruction::I32Add,
                K::I32Sub => Instruction::I32Sub,
                K::I32Mul => Instruction::I32Mul,
                K::Eq => Instruction::I32Eq,
                K::Ne => Instruction::I32Ne,
                K::I32LtS => Instruction::I32LtS,
                K::I32LeS => Instruction::I32LeS,
                K::I32GtS => Instruction::I32GtS,
                _ => Instruction::I32GeS,
            });
        }
        (K::I32Neg, B::Unary(value)) => {
            body.instruction(&Instruction::I32Const(0));
            body.instruction(&Instruction::LocalGet(value.index()));
            body.instruction(&Instruction::I32Sub);
        }
        (K::DirectCall, B::DirectCall { callee, arguments }) => {
            for argument in arguments {
                match argument {
                    VerifiedCallArgument::Value(value) => {
                        body.instruction(&Instruction::LocalGet(value.index()))
                    }
                    VerifiedCallArgument::Borrow(borrow) => {
                        body.instruction(&Instruction::LocalGet(locals.borrows + borrow.index()))
                    }
                };
            }
            body.instruction(&Instruction::Call(context.function_index(callee)?));
        }
        (
            K::StructConstruct | K::FixedArrayConstruct | K::EnumConstruct,
            B::Construct { operands, variant },
        ) => {
            let result_type = instruction.result_type().ok_or_else(index_error)?;
            let layout = context.layouts.type_by_id(result_type).ok_or_else(index_error)?;
            body.instruction(&Instruction::I32Const(
                i32::try_from(layout.size().max(1)).map_err(|_| index_error())?,
            ));
            body.instruction(&Instruction::Call(0));
            let result = instruction.result().ok_or_else(index_error)?;
            body.instruction(&Instruction::LocalSet(result.index()));
            if layout.category() == TypeCategory::Enum {
                body.instruction(&Instruction::LocalGet(result.index()));
                body.instruction(&Instruction::I32Const(
                    i32::try_from(variant.ok_or_else(index_error)?).map_err(|_| index_error())?,
                ));
                store(body);
                if let Some(value) = operands.first() {
                    body.instruction(&Instruction::LocalGet(result.index()));
                    body.instruction(&Instruction::I32Const(
                        i32::try_from(layout.enum_payload_layout().ok_or_else(index_error)?.0)
                            .map_err(|_| index_error())?,
                    ));
                    body.instruction(&Instruction::I32Add);
                    body.instruction(&Instruction::LocalGet(value.index()));
                    let variant = layout
                        .variants()
                        .iter()
                        .find(|candidate| candidate.ordinal() == variant.unwrap_or_default())
                        .and_then(|candidate| candidate.payload())
                        .and_then(|ty| context.layouts.type_by_id(ty))
                        .ok_or_else(index_error)?;
                    memory::store_value(variant, body);
                }
                return sync_result(function, instruction, locals, context.layouts, body);
            }
            for (index, value) in operands.iter().enumerate() {
                body.instruction(&Instruction::LocalGet(result.index()));
                let offset = if layout.category() == TypeCategory::Struct {
                    layout.fields()[index].offset()
                } else {
                    layout
                        .array_stride()
                        .ok_or_else(index_error)?
                        .checked_mul(u64::try_from(index).map_err(|_| index_error())?)
                        .ok_or_else(index_error)?
                };
                body.instruction(&Instruction::I32Const(
                    i32::try_from(offset).map_err(|_| index_error())?,
                ));
                body.instruction(&Instruction::I32Add);
                body.instruction(&Instruction::LocalGet(value.index()));
                let element = if layout.category() == TypeCategory::Struct {
                    context.layouts.type_by_id(layout.fields()[index].ty())
                } else {
                    layout.referenced_type().and_then(|ty| context.layouts.type_by_id(ty))
                }
                .ok_or_else(index_error)?;
                memory::store_value(element, body);
            }
            return sync_result(function, instruction, locals, context.layouts, body);
        }
        (K::CopyFromPlace | K::MoveFromPlace | K::GenericMoveFromPlace, B::Place(place)) => {
            place_value(function, place.index(), locals, context.layouts, body)?;
        }
        (
            K::ClonePlace
            | K::GenericClonePlace
            | K::HandleAwareClonePlace
            | K::StringClone
            | K::VecClone,
            B::Place(place),
        ) => {
            place_value(function, place.index(), locals, context.layouts, body)?;
            body.instruction(&Instruction::Call(Context::clone_index(
                instruction.result_type().ok_or_else(index_error)?,
            )));
        }
        (K::EnumDiscriminant, B::Place(place)) => {
            place_value(function, place.index(), locals, context.layouts, body)?;
            load(body);
        }
        (
            K::InitializePlace | K::ReplacePlace | K::GenericReplacePlace,
            B::PlaceValue { place, value },
        ) => {
            if kind != K::InitializePlace {
                emit_instruction_drops(function, instruction, locals, context, body)?;
            }
            let target = place_type(function, place.index(), context.layouts)?;
            place_address(function, place.index(), locals, context.layouts, body)?;
            body.instruction(&Instruction::LocalGet(value.index()));
            store_place_value(function, place.index(), target, body)?;
            return Ok(());
        }
        (K::DropPlace, B::Place(_)) => {
            emit_instruction_drops(function, instruction, locals, context, body)?;
            return Ok(());
        }
        (K::EndBorrow, B::BorrowUse(_)) => return Ok(()),
        (K::FixedArrayIndexCopy | K::VecIndexCopy, B::IndexedPlace { place, index }) => {
            super::values::indexed_value(
                function,
                place.index(),
                index.index(),
                locals,
                context.layouts,
                body,
            )?;
        }
        (K::StringFromUtf8, B::String(bytes)) => {
            super::values::string_value(bytes, locals.scratch, body)?;
        }
        (K::StringConcat, B::StringConcat { left, right }) => super::values::string_concat(
            function,
            left.index(),
            right.index(),
            locals,
            context.layouts,
            body,
        )?,
        (K::VecConstruct, B::VecConstruct(values)) => super::values::vec_value(
            &values.iter().map(|value| value.index()).collect::<Vec<_>>(),
            instruction.result_type().ok_or_else(index_error)?,
            locals.scratch,
            context,
            body,
        )?,
        (K::VecPush, B::VecPush { vector, value }) => {
            super::values::vec_push(
                function,
                vector.index(),
                value.index(),
                locals,
                context,
                body,
            )?;
            return Ok(());
        }
        (K::SharedConstruct, B::Unary(value)) => super::values::shared_value(
            value.index(),
            instruction.result_type().ok_or_else(index_error)?,
            locals.scratch,
            context,
            body,
        )?,
        (K::SharedClone | K::WeakClone | K::WeakDowngrade, B::Place(place)) => {
            place_value(function, place.index(), locals, context.layouts, body)?;
            body.instruction(&Instruction::Call(Context::clone_index(
                instruction.result_type().ok_or_else(index_error)?,
            )));
        }
        (K::BeginBorrow, B::BeginBorrow(definition)) => {
            place_storage_address(
                function,
                definition.place().index(),
                locals,
                context.layouts,
                body,
            )?;
            body.instruction(&Instruction::LocalSet(locals.borrows + definition.id().index()));
            return Ok(());
        }
        (K::BeginIndexedBorrow | K::BeginIndexedAccess, B::IndexedBorrow { definition, index }) => {
            super::values::indexed_address(
                function,
                definition.place().index(),
                index.index(),
                locals,
                context.layouts,
                body,
            )?;
            body.instruction(&Instruction::LocalSet(locals.borrows + definition.id().index()));
            return Ok(());
        }
        (K::BindIndexedBorrow, B::BindIndexedBorrow { parent, borrow }) => {
            body.instruction(&Instruction::LocalGet(locals.borrows + parent.index()));
            body.instruction(&Instruction::LocalSet(locals.borrows + borrow.index()));
            return Ok(());
        }
        (K::ProjectIndexedBorrow, B::ProjectIndexedBorrow { parent, borrow, index }) => {
            super::values::projected_address(
                function,
                parent.index(),
                index.index(),
                locals,
                context,
                body,
            )?;
            body.instruction(&Instruction::LocalSet(locals.borrows + borrow.index()));
            return Ok(());
        }
        (
            K::BorrowRead | K::GenericCloneBorrow | K::HandleAwareCloneBorrow,
            B::BorrowUse(borrow),
        ) => {
            body.instruction(&Instruction::LocalGet(locals.borrows + borrow.index()));
            let ty = instruction.result_type().ok_or_else(index_error)?;
            memory::load_value(context.layouts.type_by_id(ty).ok_or_else(index_error)?, body);
            if kind != K::BorrowRead {
                body.instruction(&Instruction::Call(Context::clone_index(ty)));
            }
        }
        (K::BorrowWrite | K::BorrowReplace, B::BorrowValue { borrow, value }) => {
            let ty = function.backend_borrow_type(borrow.index()).ok_or_else(index_error)?;
            let layout = context.layouts.type_by_id(ty).ok_or_else(index_error)?;
            if kind == K::BorrowReplace && layout.drop_kind() != 0 {
                body.instruction(&Instruction::LocalGet(locals.borrows + borrow.index()));
                memory::load_value(layout, body);
                body.instruction(&Instruction::Call(context.drop_index(ty)));
            }
            body.instruction(&Instruction::LocalGet(locals.borrows + borrow.index()));
            body.instruction(&Instruction::LocalGet(value.index()));
            memory::store_value(layout, body);
            return Ok(());
        }
        _ => return Err(index_error()),
    }
    if let Some(result) = instruction.result() {
        body.instruction(&Instruction::LocalSet(result.index()));
        sync_result(function, instruction, locals, context.layouts, body)?;
    }
    Ok(())
}

fn sync_result(
    function: VerifiedFunction<'_>,
    instruction: VerifiedInstruction<'_>,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let Some(result) = instruction.result() else { return Ok(()) };
    if let Some(place) =
        function.places().find(|place| place.kind() == VerifiedPlaceKind::Temporary(result))
    {
        place_address(function, place.id().index(), locals, layouts, body)?;
        body.instruction(&Instruction::LocalGet(result.index()));
        store(body);
    }
    Ok(())
}

pub(super) fn place_type<'a>(
    function: VerifiedFunction<'_>,
    id: u32,
    layouts: &'a VerifiedLayouts,
) -> Result<zryna_layout::VerifiedType<'a>, zryna_diagnostics::Diagnostic> {
    function
        .places()
        .find(|place| place.id().index() == id)
        .and_then(|place| layouts.type_by_id(place.ty()))
        .ok_or_else(index_error)
}

fn place_storage_address(
    function: VerifiedFunction<'_>,
    id: u32,
    locals: Locals,
    layouts: &VerifiedLayouts,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let place = function.places().find(|place| place.id().index() == id).ok_or_else(index_error)?;
    let ty = layouts.type_by_id(place.ty()).ok_or_else(index_error)?;
    if matches!(
        place.kind(),
        VerifiedPlaceKind::Parameter(_)
            | VerifiedPlaceKind::Local(_)
            | VerifiedPlaceKind::Temporary(_)
    ) && matches!(
        ty.category(),
        TypeCategory::Struct | TypeCategory::Enum | TypeCategory::FixedArray
    ) {
        place_value(function, id, locals, layouts, body)
    } else {
        place_address(function, id, locals, layouts, body)
    }
}

pub(super) fn store_place_value(
    function: VerifiedFunction<'_>,
    id: u32,
    ty: zryna_layout::VerifiedType<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let place = function.places().find(|place| place.id().index() == id).ok_or_else(index_error)?;
    match place.kind() {
        VerifiedPlaceKind::Parameter(_)
        | VerifiedPlaceKind::Local(_)
        | VerifiedPlaceKind::Temporary(_) => store(body),
        _ => memory::store_value(ty, body),
    }
    Ok(())
}

fn emit_instruction_drops(
    function: VerifiedFunction<'_>,
    instruction: VerifiedInstruction<'_>,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    for action in instruction.derived_drop_actions() {
        let root = action.root().index();
        let ty = place_type(function, root, context.layouts)?;
        place_value(function, root, locals, context.layouts, body)?;
        body.instruction(&Instruction::Call(context.drop_index(ty.id())));
    }
    Ok(())
}

pub(super) fn emit_terminator_drops(
    function: VerifiedFunction<'_>,
    terminator: zryna_ir::data_ownership_v1::VerifiedTerminator<'_>,
    locals: Locals,
    context: &Context<'_>,
    body: &mut Function,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    for action in terminator.derived_drop_actions() {
        let root = action.root().index();
        let ty = place_type(function, root, context.layouts)?;
        place_value(function, root, locals, context.layouts, body)?;
        body.instruction(&Instruction::Call(context.drop_index(ty.id())));
    }
    Ok(())
}
