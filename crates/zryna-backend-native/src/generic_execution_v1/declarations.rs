//! The same closed target and declaration order precede ordinary or private entry emission.
use super::{NATIVE_OBJECT_TARGET, codegen, invocation, signature, wrappers};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) struct Functions {
    pub source: Vec<FuncId>,
    pub wrappers: Vec<FuncId>,
}
pub(super) fn module() -> Result<ObjectModule, Diagnostic> {
    let mut flags = settings::builder();
    for (key, value) in [("opt_level", "none"), ("is_pic", "false"), ("unwind_info", "false")] {
        flags.set(key, value).map_err(codegen)?;
    }
    let triple = NATIVE_OBJECT_TARGET.parse::<target_lexicon::Triple>().map_err(codegen)?;
    let isa = cranelift_codegen::isa::lookup(triple)
        .map_err(codegen)?
        .finish(settings::Flags::new(flags))
        .map_err(codegen)?;
    let mut builder =
        ObjectBuilder::new(isa, b"zryna-gexecution-v1".to_vec(), default_libcall_names())
            .map_err(codegen)?;
    builder.per_function_section(false);
    Ok(ObjectModule::new(builder))
}
pub(super) fn functions(
    object: &mut ObjectModule,
    program: &VerifiedProgram<'_, '_>,
    transport: Option<&invocation::plan::Plan<'_>>,
) -> Result<Functions, Diagnostic> {
    let mut ids = Vec::new();
    for plan in program.functions() {
        ids.push(
            object
                .declare_function(
                    plan.symbol(),
                    Linkage::Local,
                    &signature(u32::try_from(plan.source().parameters().len()).map_err(codegen)?),
                )
                .map_err(codegen)?,
        );
    }
    let mut wrapper_ids = Vec::new();
    if transport.is_some() {
        wrapper_ids.push(
            object
                .declare_function(
                    invocation::ENTRY_SYMBOL,
                    Linkage::Export,
                    &invocation::wrapper::signature(),
                )
                .map_err(codegen)?,
        );
    } else {
        for export in program.program().scalar_abi().exports() {
            wrapper_ids.push(
                object
                    .declare_function(
                        export.native_linux_x86_64_symbol().as_str(),
                        Linkage::Export,
                        &wrappers::signature(export),
                    )
                    .map_err(codegen)?,
            );
        }
    }
    Ok(Functions { source: ids, wrappers: wrapper_ids })
}
