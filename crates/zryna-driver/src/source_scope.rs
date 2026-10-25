//! Borrowed NEW execution exists only inside this independently verified original-source scope.
use crate::{CommandFailure, generic_closure::VerifiedGenericClosure};
use zryna_ir::generic_v1::execution_v1::{self as execution, VerifiedExecution};
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
mod failure;
mod receipt;
mod records;
mod selection;
#[cfg(test)]
pub(crate) use receipt::GenericScalarExport;
pub(crate) use receipt::GenericSourceReceipt;
#[cfg(test)]
pub(crate) use records::{GenericSourceEdge, GenericSourceModule};

#[cfg(test)]
pub(crate) fn with_execution<R>(
    closure: &VerifiedGenericClosure,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
    inspect: impl FnOnce(
        &VerifiedExecution<'_>,
        &GenericSourceReceipt,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    with_execution_for_export(closure, None, checkpoint, inspect)
}

pub(crate) fn with_execution_for_export<R>(
    closure: &VerifiedGenericClosure,
    selected_export: Option<&str>,
    checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
    inspect: impl FnOnce(
        &VerifiedExecution<'_>,
        &GenericSourceReceipt,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    checkpoint()?;
    let sources = closure.sources();
    let syntax = closure.syntax();
    let entry = sources.file_id(closure.entrypoint()).ok_or_else(failure::invariant)?;
    let input = SemanticInput::try_new(syntax, sources, entry).ok_or_else(failure::invariant)?;
    let declarations = resolve_declarations(input).map_err(failure::source)?;
    let bodies = check_execution_body_types(&declarations).map_err(failure::body)?;
    selection::check(&declarations, selected_export)?;
    checkpoint()?;
    let instances = execution_v1::discover(&bodies).map_err(failure::execution)?;
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .map_err(failure::layout)?;
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .map_err(failure::layout)?;
    checkpoint()?;
    let raw_runtime = runtime::raw_v1(&linear, &linux).map_err(failure::runtime)?;
    let runtime = runtime::verify_v1(raw_runtime, &linear, &linux).map_err(failure::runtime)?;
    let claim =
        execution_v1::produce_claim(&instances, &linear, &linux).map_err(failure::execution)?;
    let wire = execution::wire::encode(&claim).map_err(failure::execution)?;
    let decoded = execution::wire::decode(&wire).map_err(failure::execution)?;
    let program = execution::verify(decoded, syntax, sources, entry, &linear, &linux, &runtime)
        .map_err(failure::execution)?;
    let receipt = GenericSourceReceipt::capture(closure, &program, &wire)?;
    checkpoint()?;
    let result = inspect(&program, &receipt, checkpoint)?;
    checkpoint()?;
    Ok(result)
}

#[cfg(test)]
mod tests;
