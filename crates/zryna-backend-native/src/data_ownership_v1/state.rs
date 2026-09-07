use cranelift_codegen::ir::{Block, BlockArg, FuncRef};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{
    VerifiedBlock, VerifiedBorrowParameter, VerifiedFunction, VerifiedValue,
};

use super::invariant_error;

pub(super) fn value_capacity(function: VerifiedFunction<'_>) -> Result<usize, Diagnostic> {
    function
        .parameters()
        .map(VerifiedValue::id)
        .chain(function.blocks().flat_map(|block| {
            block.parameters().map(VerifiedValue::id).chain(
                block
                    .operations()
                    .filter_map(|operation| operation.result().map(VerifiedValue::id)),
            )
        }))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(invariant_error)
}

pub(super) fn borrow_capacity(function: VerifiedFunction<'_>) -> Result<usize, Diagnostic> {
    function
        .borrow_parameters()
        .map(VerifiedBorrowParameter::id)
        .chain(
            function
                .blocks()
                .flat_map(VerifiedBlock::operations)
                .flat_map(|operation| operation.borrows().iter().copied()),
        )
        .max()
        .map_or(Ok(0), |value| {
            value
                .checked_add(1)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(invariant_error)
        })
}

pub(super) fn set_value(
    values: &mut [Option<cranelift_codegen::ir::Value>],
    id: u32,
    value: cranelift_codegen::ir::Value,
) -> Result<(), Diagnostic> {
    let slot = values
        .get_mut(usize::try_from(id).map_err(|_| invariant_error())?)
        .ok_or_else(invariant_error)?;
    if slot.replace(value).is_some() {
        return Err(invariant_error());
    }
    Ok(())
}

pub(super) fn get_value(
    values: &[Option<cranelift_codegen::ir::Value>],
    id: u32,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    values
        .get(usize::try_from(id).map_err(|_| invariant_error())?)
        .and_then(|value| *value)
        .ok_or_else(invariant_error)
}

pub(super) fn set_borrow(
    borrows: &mut [Option<cranelift_codegen::ir::Value>],
    id: u32,
    value: cranelift_codegen::ir::Value,
) -> Result<(), Diagnostic> {
    let slot = borrows
        .get_mut(usize::try_from(id).map_err(|_| invariant_error())?)
        .ok_or_else(invariant_error)?;
    if slot.replace(value).is_some() {
        return Err(invariant_error());
    }
    Ok(())
}

pub(super) fn set_borrow_type(
    types: &mut [Option<u32>],
    id: u32,
    ty: u32,
) -> Result<(), Diagnostic> {
    let slot = types
        .get_mut(usize::try_from(id).map_err(|_| invariant_error())?)
        .ok_or_else(invariant_error)?;
    *slot = Some(ty);
    Ok(())
}

pub(super) fn get_borrow_type(types: &[Option<u32>], id: u32) -> Result<u32, Diagnostic> {
    types
        .get(usize::try_from(id).map_err(|_| invariant_error())?)
        .and_then(|ty| *ty)
        .ok_or_else(invariant_error)
}

pub(super) fn get_borrow(
    borrows: &[Option<cranelift_codegen::ir::Value>],
    id: u32,
) -> Result<cranelift_codegen::ir::Value, Diagnostic> {
    borrows
        .get(usize::try_from(id).map_err(|_| invariant_error())?)
        .and_then(|value| *value)
        .ok_or_else(invariant_error)
}

pub(super) fn runtime_function(
    runtime: &super::failure::Runtime<'_>,
    symbol: &str,
) -> Result<FuncRef, Diagnostic> {
    runtime.get(symbol).copied().ok_or_else(invariant_error)
}

pub(super) fn encoded_block(blocks: &[Block], id: u32) -> Result<Block, Diagnostic> {
    blocks
        .get(usize::try_from(id).map_err(|_| invariant_error())?)
        .copied()
        .ok_or_else(invariant_error)
}

pub(super) fn edge_arguments(
    values: &[Option<cranelift_codegen::ir::Value>],
    ids: &[u32],
) -> Result<Vec<BlockArg>, Diagnostic> {
    ids.iter().map(|id| get_value(values, *id).map(Into::into)).collect()
}
