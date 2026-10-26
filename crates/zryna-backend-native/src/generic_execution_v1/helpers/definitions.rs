//! Compile finite helpers; recursive-profile Clone/Drop use independently checked flat forwards.
use super::{
    Context, Diagnostic, FuncId, Function, FunctionBuilder, FunctionBuilderContext, Helper, Ids,
    InstBuilder, Module, ObjectModule, UserFuncName, VerifiedProgram, cloning, codegen, dropping,
    invariant, reserve, signature, types,
};
use crate::generic_execution_v1::{accounting, recursive};
use cranelift_codegen::ir::Value;
struct Definition<'c, 'p, 'a> {
    object: &'c mut ObjectModule,
    program: &'c VerifiedProgram<'p, 'a>,
    runtime: &'c [FuncId],
    builder: &'c mut FunctionBuilderContext,
    traversal: Option<&'c recursive::calls::Ids>,
}
impl Ids {
    pub fn define(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        bc: &mut FunctionBuilderContext,
        traversal: Option<&recursive::calls::Ids>,
    ) -> Result<(), Diagnostic> {
        let mut definition =
            Definition { object, program: p, runtime: runtime_ids, builder: bc, traversal };
        for (clone, ids) in [(true, &self.clones), (false, &self.drops)] {
            for (index, id) in ids.iter().enumerate() {
                if let Some(id) = id {
                    definition.one(self, clone, index, *id)?;
                }
            }
        }
        Ok(())
    }
}
impl Definition<'_, '_, '_> {
    fn one(&mut self, ids: &Ids, clone: bool, index: usize, id: FuncId) -> Result<(), Diagnostic> {
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(if clone { 8 } else { 9 }, u32::try_from(index).map_err(codegen)?),
            signature(clone),
        ));
        let references = ids.references(self.object, &mut context.func)?;
        let traversal = self.traversal.map(|ids| ids.references(self.object, &mut context.func));
        let mut runtime = reserve(self.runtime.len())?;
        for id in self.runtime {
            let reference = self.object.declare_func_in_func(*id, &mut context.func);
            context.func.dfg.ext_funcs[reference].colocated = true;
            runtime.push(reference);
        }
        let mut b = FunctionBuilder::new(&mut context.func, self.builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params = b.block_params(entry).to_vec();
        let helper = Helper {
            p: self.program,
            refs: &references,
            runtime: &runtime,
            invocation: *params.last().ok_or_else(invariant)?,
            ty: u32::try_from(index).map_err(codegen)?,
        };
        body(&helper, &mut b, &params, clone, traversal)?;
        accounting::audit_function(self.program, b.func, helper.invocation, helper.runtime)?;
        b.seal_all_blocks();
        b.finalize(self.object.target_config());
        self.object.define_function(id, &mut context).map_err(codegen)
    }
}
fn body(
    helper: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    params: &[Value],
    clone: bool,
    traversal: Option<recursive::calls::Refs>,
) -> Result<(), Diagnostic> {
    if clone {
        if let Some(refs) = traversal {
            let key = b.ins().iconst(types::I32, i64::from(helper.ty));
            let call = b.ins().call(refs.clone, &[key, params[0], params[1], helper.invocation]);
            let status = b.inst_results(call)[0];
            b.ins().return_(&[status]);
            recursive::check::clone_forward::check(helper.p, b.func, helper.ty, refs.clone)?;
            return Ok(());
        }
        cloning::emit(helper, b, params[0], params[1])?;
    } else if let Some(refs) = traversal {
        let flat = refs.drop;
        let key = b.ins().iconst(types::I32, i64::from(helper.ty));
        let size = u32::try_from(helper.p.stored_type(helper.ty).ok_or_else(invariant)?.size())
            .map_err(codegen)?;
        let size = b.ins().iconst(types::I32, i64::from(size));
        let call = b.ins().call(flat, &[key, params[0], size, helper.invocation]);
        let status = b.inst_results(call)[0];
        b.ins().return_(&[status]);
        recursive::check::forward::check(helper.p, b.func, helper.ty, flat)?;
        return Ok(());
    } else {
        dropping::emit(helper, b, params[0])?;
    }
    let ok = b.ins().iconst(types::I32, 0);
    b.ins().return_(&[ok]);
    Ok(())
}
