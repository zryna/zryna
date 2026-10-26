//! The actual source body builder, retained private callgraph and runtime function references.
use super::helper_fixture::Graph;
use super::*;
use crate::generic_execution_v1::{self as native, body, helpers, runtime};
use cranelift_codegen::{Context, ir::UserFuncName};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{FuncId, Module};
use cranelift_object::ObjectModule;
pub(super) struct Functions<'a> {
    pub private: &'a [FuncId],
    pub runtime: &'a [FuncId],
    pub helpers: &'a helpers::Ids,
}
pub(super) fn build(
    p: &VerifiedProgram<'_, '_>,
    module: &mut ObjectModule,
    calls: &Functions<'_>,
    index: usize,
    operation: Op,
) -> Option<Graph> {
    let plan = &p.functions()[index];
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(7, u32::try_from(index).expect("bounded function")),
        native::signature(
            u32::try_from(plan.source().parameters().len()).expect("bounded parameters"),
        ),
    ));
    let callees = body::callees(p, index)
        .expect("actual callgraph")
        .into_iter()
        .map(|target| {
            (target, module.declare_func_in_func(calls.private[target], &mut context.func))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let runtime = calls
        .runtime
        .iter()
        .map(|id| module.declare_func_in_func(*id, &mut context.func))
        .collect::<Vec<_>>();
    let references =
        calls.helpers.references(module, &mut context.func).expect("actual helper refs");
    body::build(
        p,
        index,
        &mut context,
        &mut FunctionBuilderContext::new(),
        body::Calls { callees: &callees, runtime: &runtime, helpers: &references },
        module.target_config(),
    )
    .expect("actual body including pristine audit");
    let call=context.func.layout.blocks().flat_map(|b|context.func.layout.block_insts(b))
        .find(|inst|matches!(context.func.dfg.insts[*inst],cranelift_codegen::ir::InstructionData::Call{func_ref,..}
            if runtime::string_operation(p,&runtime,func_ref)==Some(operation)
            || p.containers().is_some_and(|plan|plan.operations().iter().position(|op|*op==operation)
                .is_some_and(|index|runtime.get(runtime::NAMES.len()+index)==Some(&func_ref)))))?;
    let entry = context.func.layout.entry_block().expect("entry");
    let invocation = *context.func.dfg.block_params(entry).last().expect("actual appended context");
    Some(Graph { function: context.func, runtime, invocation, call })
}
