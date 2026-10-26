//! Linux x86-64 physical-frame execution of the independent full execution MIR.

mod accounting;
mod audit;
mod body;
mod clone;
mod containers;
mod control;
mod declarations;
mod helpers;
mod invocation;
pub use invocation::{ValidatedInvocationArtifact, emit_invocation};
mod operations;
mod records;
mod recursive;
mod runtime;
mod wrappers;

use crate::{LinuxX8664ObjectTarget, NATIVE_OBJECT_TARGET, ValidatedNativeObjectArtifact};
use cranelift_codegen::{
    Context,
    ir::{AbiParam, Function, Signature, UserFuncName, types},
    isa::CallConv,
};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::Module;
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;

/// Emits audited ELF with private stack result transport and exact scalar ABI wrappers.
///
/// ```compile_fail
/// fn fake(raw: zryna_ir::generic_v1::raw::Program, target: zryna_backend_native::LinuxX8664ObjectTarget) {
///     let _ = zryna_backend_native::generic_execution_v1::emit_object(&raw, target);
/// }
/// ```
/// # Errors
/// Returns a stable invariant/code generation diagnostic or final closed object audit rejection.
pub fn emit_object(
    program: &VerifiedProgram<'_, '_>,
    _target: LinuxX8664ObjectTarget,
) -> Result<ValidatedNativeObjectArtifact, Diagnostic> {
    Ok(ValidatedNativeObjectArtifact { bytes: emit(program, None)? })
}
fn emit(
    program: &VerifiedProgram<'_, '_>,
    transport: Option<&invocation::plan::Plan<'_>>,
) -> Result<Vec<u8>, Diagnostic> {
    let entry = invocation::Entry::from_plan(transport);
    let mut object = declarations::module()?;
    let arena = recursive::Arena::declare(&mut object, program)?;
    let traversal = recursive::calls::Ids::declare(&mut object, program)?;
    let runtime_ids = runtime::declare(&mut object, program)?;
    let helper_ids = helpers::Ids::declare(&mut object, program)?;
    let declarations::Functions { source: ids, wrappers: wrapper_ids } =
        declarations::functions(&mut object, program, transport)?;
    let mut builder_context = FunctionBuilderContext::new();
    for (index, plan) in program.functions().iter().enumerate() {
        let mut context = context(
            index,
            signature(u32::try_from(plan.source().parameters().len()).map_err(codegen)?),
        )?;
        let mut callees = std::collections::BTreeMap::new();
        for target in body::callees(program, index)? {
            callees.insert(target, object.declare_func_in_func(ids[target], &mut context.func));
        }
        let runtime = runtime_ids
            .iter()
            .map(|id| {
                let reference = object.declare_func_in_func(*id, &mut context.func);
                context.func.dfg.ext_funcs[reference].colocated = true;
                reference
            })
            .collect::<Vec<_>>();
        let references = helper_ids.references(&mut object, &mut context.func)?;
        let calls = body::Calls { callees: &callees, runtime: &runtime, helpers: &references };
        if let Some(plan) = transport {
            body::build_with_transport(
                program,
                index,
                &mut context,
                &mut builder_context,
                calls,
                object.target_config(),
                Some(plan),
            )?;
        } else {
            body::build(
                program,
                index,
                &mut context,
                &mut builder_context,
                calls,
                object.target_config(),
            )?;
        }
        object.define_function(ids[index], &mut context).map_err(codegen)?;
    }
    if let Some(traversal) = &traversal {
        traversal.define(&mut object, program, &runtime_ids, &mut builder_context)?;
    }
    helper_ids.define(
        &mut object,
        program,
        &runtime_ids,
        &mut builder_context,
        traversal.as_ref(),
    )?;
    if let Some(plan) = transport {
        let mut context = context(program.functions().len(), invocation::wrapper::signature())?;
        let callee = object.declare_func_in_func(ids[plan.target], &mut context.func);
        let getter = traversal
            .as_ref()
            .map(|ids| object.declare_func_in_func(ids.getter, &mut context.func));
        invocation::wrapper::build(
            plan,
            callee,
            &mut context,
            &mut builder_context,
            object.target_config(),
            getter,
        )?;
        object.define_function(wrapper_ids[0], &mut context).map_err(codegen)?;
    } else {
        define_wrappers(
            &mut object,
            program,
            &ids,
            &wrapper_ids,
            &mut builder_context,
            traversal.as_ref(),
        )?;
    }
    if let Some(arena) = &arena {
        arena.define(&mut object)?;
    }
    let mut product = object.finish();
    if let Some(arena) = &arena {
        arena.relocate(&mut product)?;
    }
    let bytes = product.emit().map_err(codegen)?;
    audit::check_entry(&bytes, program, entry)?;
    Ok(bytes)
}

fn signature(parameters: u32) -> Signature {
    let mut s = Signature::new(CallConv::SystemV);
    s.params.push(AbiParam::new(types::I64));
    for _ in 0..parameters {
        s.params.push(AbiParam::new(types::I64));
    }
    s.params.push(AbiParam::new(types::I64));
    s.returns.push(AbiParam::new(types::I32));
    s
}

/// Checks the closed ELF inventory and direct-call bindings against exact sealed generic MIR.
///
/// This diagnostic utility grants no executable artifact authority and does not authenticate
/// arbitrary machine instructions. Only emission constructs the opaque validated artifact.
/// # Errors
/// Rejects target, section, symbol, extent, relocation and direct-call instruction drift.
pub fn validate_object_inventory(
    bytes: &[u8],
    program: &VerifiedProgram<'_, '_>,
) -> Result<(), Diagnostic> {
    audit::check(bytes, program)
}

fn context(index: usize, s: Signature) -> Result<Context, Diagnostic> {
    Ok(Context::for_function(Function::with_name_signature(
        UserFuncName::user(7, u32::try_from(index).map_err(codegen)?),
        s,
    )))
}
fn codegen(error: impl std::fmt::Display) -> Diagnostic {
    let _ = error;
    Diagnostic::error(
        "ZRYNA-N7102",
        None,
        "generic native code generation rejected a sealed invariant",
        "report the smallest reproducible source",
    )
}
fn invariant() -> Diagnostic {
    codegen("sealed invariant")
}

fn define_wrappers(
    object: &mut ObjectModule,
    program: &VerifiedProgram<'_, '_>,
    ids: &[cranelift_module::FuncId],
    wrapper_ids: &[cranelift_module::FuncId],
    builder_context: &mut FunctionBuilderContext,
    traversal: Option<&recursive::calls::Ids>,
) -> Result<(), Diagnostic> {
    for (index, export) in program.program().scalar_abi().exports().enumerate() {
        let target = program.program().export_functions()[index];
        let mut context = context(program.functions().len() + index, wrappers::signature(export))?;
        let callee = object.declare_func_in_func(ids[target], &mut context.func);
        let getter =
            traversal.map(|ids| object.declare_func_in_func(ids.getter, &mut context.func));
        wrappers::build(
            export,
            callee,
            &mut context,
            builder_context,
            object.target_config(),
            getter,
        )?;
        object.define_function(wrapper_ids[index], &mut context).map_err(codegen)?;
    }
    Ok(())
}

#[cfg(test)]
mod accounting_tests;
