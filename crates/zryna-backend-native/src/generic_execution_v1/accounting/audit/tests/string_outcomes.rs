//! All three real default String routes bind every status bit and impossible-success bit.
use super::{Op, fixture};
use cranelift_codegen::ir::{
    Function, Inst, InstructionData as D, Opcode as O, Value, condcodes::IntCC,
};
fn pair(function: &Function, value: Value) -> [Value; 2] {
    let inst = function.dfg.value_def(value).inst().expect("actual predicate definition");
    let args = function.dfg.inst_args(inst);
    assert_eq!(args.len(), 2);
    [args[0], args[1]]
}
fn predicates(function: &Function, call: Inst, operation: Op) -> Vec<(&'static str, Value)> {
    let block = function.layout.inst_block(call).expect("actual admitted block");
    let after = function
        .layout
        .block_insts(block)
        .skip_while(|inst| *inst != call)
        .skip(1)
        .collect::<Vec<_>>();
    let valid = after
        .iter()
        .copied()
        .find(|inst| function.dfg.insts[*inst].opcode() == O::Trapz)
        .expect("actual status guard");
    let impossible = after
        .iter()
        .copied()
        .find(|inst| function.dfg.insts[*inst].opcode() == O::Trapnz)
        .expect("actual impossible-success guard");
    let mut root = function.dfg.inst_args(valid)[0];
    let mut extra = None;
    if operation == Op::StringFromUtf8Copy {
        let extension = pair(function, root);
        root = extension[0];
        extra = Some(extension[1]);
    }
    let outer = pair(function, root);
    let first = pair(function, outer[0]);
    let both = pair(function, function.dfg.inst_args(impossible)[0]);
    let mut values = vec![
        ("status0", first[0]),
        ("status1", first[1]),
        ("status2", outer[1]),
        ("possible0", both[0]),
        ("result0", both[1]),
    ];
    if let Some(extra) = extra {
        values.push(("utf8-status4", extra));
    }
    values
}
#[test]
fn actual_nonrecursive_string_operations_bind_all_outcome_components() {
    for operation in [Op::StringFromUtf8Copy, Op::StringClone, Op::StringConcat] {
        fixture::with_function(
            "nested-temporaries",
            operation,
            |program, function, context, runtime, call| {
                assert!(!program.requires_recursive_arena(), "default nonrecursive issuer route");
                assert!(program.containers().is_none(), "ordinary inline String helpers");
                super::super::function(program, function, context, runtime)
                    .expect("genuine pristine shared accounting");
                let values = predicates(function, call, operation);
                assert_eq!(values.len(), if operation == Op::StringFromUtf8Copy { 6 } else { 5 });
                for (label, value) in values {
                    let inst =
                        function.dfg.value_def(value).inst().expect("actual predicate definition");
                    let original = function.dfg.insts[inst];
                    let mut changed = original;
                    let D::IntCompare { cond, .. } = &mut changed else {
                        panic!("actual equality predicate");
                    };
                    assert_eq!(*cond, IntCC::Equal);
                    *cond = IntCC::NotEqual;
                    function.dfg.insts[inst] = changed;
                    assert!(
                        super::super::function(program, function, context, runtime).is_err(),
                        "{operation:?}: {label}"
                    );
                    function.dfg.insts[inst] = original;
                }
                super::super::function(program, function, context, runtime)
                    .expect("pristine shared-proof retry");
            },
        );
    }
}
