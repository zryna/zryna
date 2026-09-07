use super::{Locals, index_error, memory};
use wasm_encoder::{Function, Instruction, MemArg};
use zryna_ir::data_ownership_v1::{VerifiedFunction, VerifiedPlaceKind};
use zryna_layout::{TypeCategory, VerifiedLayouts};

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

pub(super) fn place_storage_address(
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
