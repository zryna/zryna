//! Define all private protocol functions after declaring their exact closed call inventory.
use super::{
    calls::{Ids, KINDS, signature},
    check, emit,
};
use crate::generic_execution_v1::codegen;
use cranelift_codegen::{
    Context,
    ir::{Function, UserFuncName},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Module};
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
impl Ids {
    pub(in crate::generic_execution_v1) fn define(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        bc: &mut FunctionBuilderContext,
    ) -> Result<(), Diagnostic> {
        for (index, kind) in KINDS.into_iter().enumerate() {
            let mut context = Context::for_function(Function::with_name_signature(
                UserFuncName::user(10, u32::try_from(index).map_err(codegen)?),
                signature(kind),
            ));
            let refs = self.references(object, &mut context.func);
            let mut runtime = Vec::new();
            runtime.try_reserve_exact(runtime_ids.len()).map_err(codegen)?;
            for id in runtime_ids {
                let reference = object.declare_func_in_func(*id, &mut context.func);
                context.func.dfg.ext_funcs[reference].colocated = true;
                runtime.push(reference);
            }
            let mut b = FunctionBuilder::new(&mut context.func, bc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            emit::build(p, &mut b, kind, refs, &runtime, &args)?;
            check::function(p, b.func, kind, refs, &runtime)?;
            b.seal_all_blocks();
            b.finalize(object.target_config());
            object.define_function(self.functions[index], &mut context).map_err(codegen)?;
        }
        Ok(())
    }
}
