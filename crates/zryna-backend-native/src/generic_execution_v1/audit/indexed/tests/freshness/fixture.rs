//! Actual source/MIR/body builder; this fixture issues no substitute source or MIR authority.
use super::*;
use crate::generic_execution_v1::{self as native, body, helpers, runtime};
use cranelift_codegen::{ir::UserFuncName, settings};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::BTreeMap;
#[path = "../../../../../../../zryna-driver/tests/generic_execution_v1_dynamic_index_source_seal/source.rs"]
mod source;
pub(super) fn with_graph(
    check: impl FnOnce(
        &Frame<'_, '_, '_>,
        &mut Function,
        VerifiedIndexedLoan<'_, '_>,
        Value,
        Value,
        Inst,
    ),
) {
    assert!(source::text("vec-indexed-replace").contains("root[index]"));
    source::with_program("vec-indexed-replace", "native", |program, _, _| {
        let mir =
            zryna_native_mir::generic_execution_v1::lower(program).expect("genuine native owner");
        let (plan, view) = mir
            .functions()
            .iter()
            .find_map(|plan| {
                plan.source()
                    .blocks()
                    .find_map(|block| {
                        block.operations().find_map(
                            zryna_ir::generic_v1::execution_v1::VerifiedOperation::indexed_loan,
                        )
                    })
                    .map(|view| (plan, view))
            })
            .expect("actual indexed original");
        let triple = native::NATIVE_OBJECT_TARGET.parse().expect("pinned target");
        let isa = cranelift_codegen::isa::lookup(triple)
            .expect("ISA")
            .finish(settings::Flags::new(settings::builder()))
            .expect("flags");
        let mut module = ObjectModule::new(
            ObjectBuilder::new(isa, b"indexed-freshness".to_vec(), default_libcall_names())
                .expect("object"),
        );
        let runtime_ids = runtime::declare(&mut module, &mir).expect("retained runtime");
        let helper_ids = helpers::Ids::declare(&mut module, &mir).expect("retained helpers");
        let mut context = cranelift_codegen::Context::for_function(Function::with_name_signature(
            UserFuncName::user(7, u32::try_from(plan.source().index()).expect("function")),
            native::signature(u32::try_from(plan.source().parameters().len()).expect("parameters")),
        ));
        let runtime = runtime_ids
            .iter()
            .map(|id| module.declare_func_in_func(*id, &mut context.func))
            .collect::<Vec<_>>();
        let references =
            helper_ids.references(&mut module, &mut context.func).expect("helper refs");
        let callees = BTreeMap::new();
        body::build(
            &mir,
            plan.source().index(),
            &mut context,
            &mut FunctionBuilderContext::new(),
            body::Calls { callees: &callees, runtime: &runtime, helpers: &references },
            module.target_config(),
        )
        .expect("actual body including pristine independent audits");
        let entry = context.func.layout.entry_block().expect("entry");
        let parameters = context.func.dfg.block_params(entry).to_vec();
        let base_inst = context
            .func
            .layout
            .block_insts(entry)
            .find(|inst| context.func.dfg.insts[*inst].opcode() == O::StackAddr)
            .expect("actual frame allocation");
        let base = context.func.dfg.inst_results(base_inst)[0];
        let frame = Frame {
            transport: None,
            p: &mir,
            f: plan,
            base,
            invocation: *parameters.last().expect("context"),
            parameters,
            runtime: &runtime,
            helpers: &references,
            callees: &callees,
        };
        let (source, status, published) = locate(&frame, &context.func, view);
        check(&frame, &mut context.func, view, source, status, published);
        cranelift_codegen::verify_function(&context.func, module.isa())
            .expect("mutated graph must remain valid SSA");
    });
}
fn locate(
    c: &Frame<'_, '_, '_>,
    f: &Function,
    view: VerifiedIndexedLoan<'_, '_>,
) -> (Value, Value, Inst) {
    let LoanAddress::Prepared { slot, offset: 0 } = c.f.loan(view.loan().id()).expect("cell")
    else {
        panic!("prepared")
    };
    let published = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .find(|inst| {
            let D::Store { args: stored, offset, .. } = f.dfg.insts[*inst] else { return false };
            i32::from(offset) == 0
                && address::cell(c, f, stored[1], slot).is_ok()
                && !constant(f, stored[0], 0)
        })
        .expect("actual pointer publication");
    let target = f.layout.inst_block(published).expect("publication block");
    let status = f
        .layout
        .blocks()
        .find_map(|block| {
            let inst = f.layout.last_inst(block)?;
            let destinations =
                f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
            (destinations.len() == 2 && destinations[1].block(&f.dfg.value_lists) == target)
                .then(|| f.dfg.inst_args(inst)[0])
        })
        .expect("actual bounds status");
    let D::Store { args: stored, .. } = f.dfg.insts[published] else { panic!("store") };
    let pointer_data = args(f, stored[0], O::Iadd, 2).expect("pointer add")[0];
    let D::Load { arg: source, .. } = data(f, pointer_data).expect("actual Vec data load") else {
        panic!("data")
    };
    (*source, status, published)
}
#[test]
fn opaque_source_permission_remains_a_separate_gate() {
    let expected = source::diagnostic("opaque-indexed-consume");
    let (sources, syntax) = source::project("opaque-indexed-consume", "native");
    let zryna_ir::generic_v1::Failure::Diagnostics(errors) =
        zryna_ir::generic_v1::execution_v1::check_original_bodies(&syntax, &sources)
            .expect_err("source misuse")
    else {
        panic!("source diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), expected["code"].as_str().expect("frozen diagnostic"));
}
