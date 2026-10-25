//! Actual function construction over the same verified source and retained invocation plan.
use crate::generic_execution_v1::{
    self as native, body, helpers,
    invocation::{plan::Plan, source::trace},
    runtime,
};
use cranelift_codegen::{
    Context,
    ir::{Function, InstructionData as D, Opcode as O, UserFuncName, condcodes::IntCC},
    settings,
};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn check(
    p: &VerifiedProgram<'_, '_>,
    plan: &Plan<'_>,
    codes: &mut std::collections::BTreeSet<u32>,
) {
    let triple = crate::NATIVE_OBJECT_TARGET.parse().expect("exact target");
    let isa = cranelift_codegen::isa::lookup(triple)
        .expect("ISA")
        .finish(settings::Flags::new(settings::builder()))
        .expect("flags");
    let builder =
        ObjectBuilder::new(isa, b"invocation-owner-graphs".to_vec(), default_libcall_names())
            .expect("object context");
    let mut module = ObjectModule::new(builder);
    let runtime_ids = runtime::declare(&mut module, p).expect("actual runtime declarations");
    let helpers = helpers::Ids::declare(&mut module, p).expect("actual helper declarations");
    let ids = p
        .functions()
        .iter()
        .map(|f| {
            module
                .declare_function(
                    f.symbol(),
                    Linkage::Local,
                    &native::signature(
                        u32::try_from(f.source().parameters().len()).expect("parameters"),
                    ),
                )
                .expect("private signature")
        })
        .collect::<Vec<_>>();
    for (index, f) in p.functions().iter().enumerate() {
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(7, u32::try_from(index).expect("index")),
            native::signature(u32::try_from(f.source().parameters().len()).expect("parameters")),
        ));
        let callees = body::callees(p, index)
            .expect("real call graph")
            .into_iter()
            .map(|target| (target, module.declare_func_in_func(ids[target], &mut context.func)))
            .collect::<std::collections::BTreeMap<_, _>>();
        let runtime = runtime_ids
            .iter()
            .map(|id| module.declare_func_in_func(*id, &mut context.func))
            .collect::<Vec<_>>();
        let references = helpers.references(&mut module, &mut context.func).expect("real helpers");
        trace::start();
        body::build_with_transport(
            p,
            index,
            &mut context,
            &mut FunctionBuilderContext::new(),
            body::Calls { callees: &callees, runtime: &runtime, helpers: &references },
            module.target_config(),
            Some(plan),
        )
        .expect("real source body and independent proof");
        let tails = trace::finish();
        for tail in tails {
            codes.extend(tail.codes());
            tail.check(&context.func).expect("pristine actual tail");
            for inst in tail.instructions(&context.func) {
                let original = context.func.dfg.insts[inst];
                let mut wrong = original;
                match &mut wrong {
                    D::UnaryImm { opcode: O::Iconst, imm } => *imm = (i64::from(*imm) ^ 1).into(),
                    D::IntCompare { cond, .. } => *cond = IntCC::NotEqual,
                    _ => continue,
                }
                context.func.dfg.insts[inst] = wrong;
                assert!(
                    tail.check(&context.func).is_err(),
                    "actual changed normalization {inst:?}: {original:?}"
                );
                context.func.dfg.insts[inst] = original;
                if let D::IntCompare { args, .. } = original {
                    let mut stale = original;
                    if let D::IntCompare { args: changed, .. } = &mut stale {
                        *changed = [args[1], args[1]];
                    }
                    context.func.dfg.insts[inst] = stale;
                    assert!(tail.check(&context.func).is_err(), "wrong actual raw status root");
                    context.func.dfg.insts[inst] = original;
                }
            }
            tail.check(&context.func).expect("pristine retry");
        }
        cranelift_codegen::verify_function(&context.func, module.isa())
            .expect("actual SSA remains valid");
    }
}
