//! Generate the actual private body; only the final audit is invoked after mutation.
use super::*;
use crate::generic_execution_v1::{self as native, helpers, runtime};
use cranelift_codegen::settings;
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
pub(super) fn with_function(
    case: &str,
    operation: Op,
    check: impl FnOnce(&VerifiedProgram<'_, '_>, &mut Function, Value, &[FuncRef], Inst),
) {
    let mut check = Some(check);
    let inspect = |program: &VerifiedExecution<'_>, _: &[u8]| {
        let mir = zryna_native_mir::generic_execution_v1::lower(program)
            .expect("genuine MIR effect owner");
        let triple = crate::NATIVE_OBJECT_TARGET.parse().expect("qualified triple");
        let isa = cranelift_codegen::isa::lookup(triple)
            .expect("qualified ISA")
            .finish(settings::Flags::new(settings::builder()))
            .expect("ISA flags");
        let builder =
            ObjectBuilder::new(isa, b"accounting-owner-test".to_vec(), default_libcall_names())
                .expect("private graph builder");
        let mut module = ObjectModule::new(builder);
        let runtime_ids = runtime::declare(&mut module, &mir).expect("same-seal runtime");
        let helper_ids = helpers::Ids::declare(&mut module, &mir).expect("same-seal helpers");
        let ids = mir
            .functions()
            .iter()
            .map(|plan| {
                module
                    .declare_function(
                        plan.symbol(),
                        Linkage::Local,
                        &native::signature(
                            u32::try_from(plan.source().parameters().len())
                                .expect("bounded parameters"),
                        ),
                    )
                    .expect("actual private function signature")
            })
            .collect::<Vec<_>>();
        if let Some(mut graph) =
            helper_fixture::build(&mir, &mut module, &helper_ids, &runtime_ids, operation)
        {
            check.take().expect("one actual helper graph")(
                &mir,
                &mut graph.function,
                graph.invocation,
                &graph.runtime,
                graph.call,
            );
            return;
        }
        let calls = source_fixture::Functions {
            private: &ids,
            runtime: &runtime_ids,
            helpers: &helper_ids,
        };
        for index in 0..mir.functions().len() {
            if let Some(mut graph) =
                source_fixture::build(&mir, &mut module, &calls, index, operation)
            {
                check.take().expect("one actual graph")(
                    &mir,
                    &mut graph.function,
                    graph.invocation,
                    &graph.runtime,
                    graph.call,
                );
                return;
            }
        }
        panic!("genuine fixture must contain requested canonical operation");
    };
    if matches!(operation, Op::VecAllocate | Op::VecReserve | Op::VecReleaseStorage) {
        vector_source::with_program(case, "native", inspect);
    } else if matches!(
        operation,
        Op::Allocate
            | Op::StrongClone
            | Op::WeakDowngrade
            | Op::WeakClone
            | Op::WeakUpgrade
            | Op::StrongReleaseBegin
            | Op::StrongReleaseFinish
            | Op::WeakRelease
    ) {
        control_source::with_program(case, "native", inspect);
    } else if operation == Op::StringConcat || case == "nested-temporaries" {
        concat_source::with_program(case, "native", inspect);
    } else {
        string_source::with_program(case, "native", inspect);
    }
}
