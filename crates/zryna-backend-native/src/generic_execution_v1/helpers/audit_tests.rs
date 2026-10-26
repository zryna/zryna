//! Test-only access to the real finite helper emitter with a genuine owning MIR.
use super::*;
impl Ids {
    pub(in crate::generic_execution_v1) fn audit_drop_body(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        ty: u32,
    ) -> (Function, Vec<FuncRef>, Value) {
        self.audit_body(object, p, runtime_ids, ty, false)
    }
    pub(in crate::generic_execution_v1) fn audit_clone_body(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        ty: u32,
    ) -> (Function, Vec<FuncRef>, Value) {
        assert!(
            p.containers().and_then(|plan| plan.clone_node(ty)).is_some(),
            "actual source Clone frontier"
        );
        self.audit_body(object, p, runtime_ids, ty, true)
    }
    fn audit_body(
        &self,
        object: &mut ObjectModule,
        p: &VerifiedProgram<'_, '_>,
        runtime_ids: &[FuncId],
        ty: u32,
        clone: bool,
    ) -> (Function, Vec<FuncRef>, Value) {
        let mut function = Function::with_name_signature(
            UserFuncName::user(if clone { 8 } else { 9 }, ty),
            signature(clone),
        );
        let references = self.references(object, &mut function).expect("same-seal helper refs");
        let runtime = runtime_ids
            .iter()
            .map(|id| object.declare_func_in_func(*id, &mut function))
            .collect::<Vec<_>>();
        let mut bc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut bc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params = b.block_params(entry).to_vec();
        let invocation = *params.last().expect("actual appended context");
        let helper = Helper { p, refs: &references, runtime: &runtime, ty, invocation };
        if clone {
            cloning::emit(&helper, &mut b, params[0], params[1])
                .expect("actual same-seal Clone helper");
        } else {
            dropping::emit(&helper, &mut b, params[0]).expect("actual same-seal drop helper");
        }
        let zero = b.ins().iconst(types::I32, 0);
        b.ins().return_(&[zero]);
        if let Err(error) =
            crate::generic_execution_v1::accounting::audit_function(p, b.func, invocation, &runtime)
        {
            let path = std::env::temp_dir().join(format!(
                "m416-native-pristine-helper-{}-{ty}-{clone}.clif",
                std::process::id()
            ));
            std::fs::write(&path, b.func.display().to_string())
                .expect("retain actual failed pristine helper before assertion");
            panic!("actual pristine helper audit {error:?}; SSA: {}", path.display());
        }
        b.seal_all_blocks();
        b.finalize(object.target_config());
        (function, runtime, invocation)
    }
}
