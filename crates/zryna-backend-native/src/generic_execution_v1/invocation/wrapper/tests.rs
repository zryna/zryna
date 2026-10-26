//! Genuine request/owner wrapper recipes and ordered lifecycle mutations.
use super::*;
use crate::generic_execution_v1::{self as native, recursive::check::wrapper::invocation as audit};
use cranelift_codegen::{
    ir::{Function, Inst, InstructionData as D, Opcode as O, UserFuncName, condcodes::CondCode},
    settings,
};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(in crate::generic_execution_v1) fn check(p: &VerifiedProgram<'_, '_>, plan: &Plan<'_>) {
    let triple = crate::NATIVE_OBJECT_TARGET.parse().expect("exact target");
    let isa = cranelift_codegen::isa::lookup(triple)
        .expect("ISA")
        .finish(settings::Flags::new(settings::builder()))
        .expect("flags");
    let builder =
        ObjectBuilder::new(isa, b"invocation-wrapper-owner".to_vec(), default_libcall_names())
            .expect("object context");
    let mut object = ObjectModule::new(builder);
    let source = &p.functions()[plan.target];
    let id = object
        .declare_function(
            source.symbol(),
            Linkage::Local,
            &native::signature(
                u32::try_from(source.source().parameters().len()).expect("bounded parameters"),
            ),
        )
        .expect("actual private callee");
    let mut context =
        Context::for_function(Function::with_name_signature(UserFuncName::user(7, 0), signature()));
    let callee = object.declare_func_in_func(id, &mut context.func);
    let getter = if p.requires_recursive_arena() {
        let mut s = Signature::new(CallConv::SystemV);
        s.params.push(AbiParam::new(types::I64));
        s.returns.push(AbiParam::new(types::I64));
        let id = object
            .declare_function(recursive::STUB_NAME, Linkage::Local, &s)
            .expect("exact private getter");
        Some(object.declare_func_in_func(id, &mut context.func))
    } else {
        None
    };
    build(
        plan,
        callee,
        &mut context,
        &mut FunctionBuilderContext::new(),
        object.target_config(),
        getter,
    )
    .expect("actual baked wrapper");
    let f = &mut context.func;
    let expected = 46
        + 5 * plan.request.arguments().len()
        + 4 * usize::from(plan.request.export().result() == ScalarType::Bool)
        + 30 * usize::from(getter.is_some());
    assert_eq!(
        f.layout.blocks().flat_map(|b| f.layout.block_insts(b)).count(),
        expected,
        "source-derived complete recipe bound"
    );
    let instructions = f.layout.blocks().flat_map(|b| f.layout.block_insts(b)).collect::<Vec<_>>();
    for inst in instructions {
        let original = f.dfg.insts[inst];
        let mut wrong = original;
        match &mut wrong {
            D::Load { offset, .. } | D::Store { offset, .. } => *offset = 123_456.into(),
            D::UnaryImm { opcode: O::Iconst, imm } => *imm = (i64::from(*imm) ^ 1).into(),
            D::IntCompare { cond, .. } => *cond = cond.complement(),
            _ => continue,
        }
        f.dfg.insts[inst] = wrong;
        assert!(
            audit::check(f, &plan.request, callee, getter).is_err(),
            "actual wrapper mutation {inst:?}: {original:?}"
        );
        f.dfg.insts[inst] = original;
        noncanonical_high_bits(f, inst, &plan.request, callee, getter);
        if let D::Store { .. } = original {
            let mut truncated = original;
            if let D::Store { opcode, .. } = &mut truncated {
                *opcode = O::Istore8;
            }
            f.dfg.insts[inst] = truncated;
            assert!(
                audit::check(f, &plan.request, callee, getter).is_err(),
                "partial publication/reset store"
            );
            f.dfg.insts[inst] = original;
        }
    }
    audit::check(f, &plan.request, callee, getter).expect("pristine actual wrapper retry");
    cranelift_codegen::verify_function(f, object.isa()).expect("actual wrapper valid SSA");
}

fn noncanonical_high_bits(
    f: &mut Function,
    inst: Inst,
    request: &zryna_abi::VerifiedInvocation<'_>,
    callee: FuncRef,
    getter: Option<FuncRef>,
) {
    let original = f.dfg.insts[inst];
    let D::UnaryImm { opcode: O::Iconst, imm } = original else {
        return;
    };
    let value = f.dfg.inst_results(inst)[0];
    if f.dfg.value_type(value) != types::I32 {
        return;
    }
    f.dfg.insts[inst] =
        D::UnaryImm { opcode: O::Iconst, imm: (i64::from(imm) ^ (1_i64 << 32)).into() };
    assert!(
        audit::check(f, request, callee, getter).is_err(),
        "noncanonical high bits with unchanged low32bits must reject"
    );
    f.dfg.insts[inst] = original;
}
