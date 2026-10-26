//! Build the real private graph under one retained original execution owner.
use super::source;
use crate::generic_execution_v1::{
    recursive::{
        calls::{Ids, Kind, Refs, signature},
        emit,
    },
    runtime,
};
use cranelift_codegen::{
    ir::{FuncRef, Function, UserFuncName},
    settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use zryna_native_mir::generic_execution_v1::{VerifiedProgram, lower};
pub(super) fn with_graph(
    case: &str,
    kind: Kind,
    inspect: impl FnOnce(&VerifiedProgram<'_, '_>, &mut Function, Refs, &[FuncRef]),
) {
    source::with_program(case, |program| {
        let mir = lower(program).expect("genuine source-derived MIR");
        assert!(mir.requires_recursive_arena(), "actual source need fact");
        let triple = crate::NATIVE_OBJECT_TARGET.parse().expect("qualified target");
        let isa = cranelift_codegen::isa::lookup(triple)
            .expect("qualified ISA")
            .finish(settings::Flags::new(settings::builder()))
            .expect("ISA flags");
        let builder = ObjectBuilder::new(
            isa,
            b"iterative-drop-owner-tests".to_vec(),
            default_libcall_names(),
        )
        .expect("private object context");
        let mut object = ObjectModule::new(builder);
        let ids = Ids::declare(&mut object, &mir)
            .expect("same-owner private declarations")
            .expect("genuine retained need");
        let runtime_ids = runtime::declare(&mut object, &mir).expect("existing canonical ABI");
        let mut f = Function::with_name_signature(UserFuncName::user(10, 0), signature(kind));
        let refs = ids.references(&mut object, &mut f);
        let runtime = runtime_ids
            .iter()
            .map(|id| object.declare_func_in_func(*id, &mut f))
            .collect::<Vec<_>>();
        let mut bc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut f, &mut bc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        emit::build(&mir, &mut b, kind, refs, &runtime, &args).expect("real protocol emitter");
        b.seal_all_blocks();
        b.finalize(object.target_config());
        inspect(&mir, &mut f, refs, &runtime);
    });
}
