//! Actual String Clone graph proves semantic status predicates, not counts or liveness.
use super::fixture;
use crate::generic_execution_v1::{
    accounting,
    recursive::{calls::Kind, check},
};
use cranelift_codegen::ir::{Function, InstructionData as D, Opcode as O, Value, condcodes::IntCC};
fn pair(function: &Function, value: Value) -> [Value; 2] {
    let inst = function.dfg.value_def(value).inst().expect("actual predicate definition");
    let args = function.dfg.inst_args(inst);
    assert_eq!(args.len(), 2);
    [args[0], args[1]]
}
#[test]
fn genuine_string_clone_all_code_and_impossible_success_predicates_are_bound() {
    fixture::with_graph("clone", Kind::Clone, |program, function, refs, runtime| {
        check::function(program, function, Kind::Clone, refs, runtime)
            .expect("pristine genuine Clone");
        let canonical=function.layout.blocks().flat_map(|block|function.layout.block_insts(block))
            .find(|inst|matches!(function.dfg.insts[*inst],D::Call{func_ref,..}if func_ref==runtime[1]))
            .expect("actual source-authorized StringClone");
        let block = function.layout.inst_block(canonical).expect("admitted block");
        let after = function
            .layout
            .block_insts(block)
            .skip_while(|inst| *inst != canonical)
            .skip(1)
            .collect::<Vec<_>>();
        let valid = after
            .iter()
            .copied()
            .find(|inst| function.dfg.insts[*inst].opcode() == O::Trapz)
            .expect("real allowed-set trap");
        let impossible = after
            .iter()
            .copied()
            .find(|inst| function.dfg.insts[*inst].opcode() == O::Trapnz)
            .expect("real impossible-success trap");
        let outer = pair(function, function.dfg.inst_args(valid)[0]);
        let first = pair(function, outer[0]);
        let both = pair(function, function.dfg.inst_args(impossible)[0]);
        let entry = function.layout.entry_block().expect("function entry");
        let context = *function.dfg.block_params(entry).last().expect("original context");
        for (label, predicate) in [
            ("status0", first[0]),
            ("status1", first[1]),
            ("status2", outer[1]),
            ("possible0", both[0]),
            ("result0", both[1]),
        ] {
            let inst = function.dfg.value_def(predicate).inst().expect("actual boolean definition");
            let original = function.dfg.insts[inst];
            let mut changed = original;
            let D::IntCompare { cond, .. } = &mut changed else {
                panic!("actual equality predicate");
            };
            assert_eq!(*cond, IntCC::Equal);
            *cond = IntCC::NotEqual;
            function.dfg.insts[inst] = changed;
            assert!(
                accounting::audit_function(program, function, context, runtime).is_err(),
                "shared accounting proof rejects {label} inversion"
            );
            assert!(
                check::function(program, function, Kind::Clone, refs, runtime).is_err(),
                "reject {label} inversion"
            );
            function.dfg.insts[inst] = original;
        }
        check::function(program, function, Kind::Clone, refs, runtime).expect("pristine retry");
    });
}
