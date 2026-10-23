//! Actual private call analysis; these fixtures issue no source/execution authority.
use super::{Failure, Function, fallibility, raw};
use crate::generic_v1::execution_v1::{
    places::{self, decomposition::Decomposition},
    typed::cfg::Graph,
};
use zryna_source::UntrustedSpan;

fn fixture(count: usize, cached_suffix: bool) -> (raw::Program, Vec<Function>) {
    let span = UntrustedSpan { file: 0, start: 0, end: 0 };
    let mut raw_functions = Vec::new();
    let mut functions = Vec::new();
    for id in 0..count {
        // Reverse-indexed calls force every child subtree to have already been cached
        // by fallibility's canonical function-order walk, instead of recursive descent.
        let target = if cached_suffix {
            id.checked_sub(1)
        } else {
            id.checked_add(1).filter(|child| *child < count)
        };
        let instructions = target.map_or_else(Vec::new, |callee| {
            vec![raw::Instruction {
                site_span: span,
                result: Some(raw::ValueDefinition { id: 0, ty: raw::ValueType::Unit }),
                operation: raw::Operation::SourceCall {
                    module: 0,
                    function: u32::try_from(callee).expect("small inherited-depth fixture"),
                    arguments: vec![],
                    failure: None,
                },
            }]
        });
        let raw = raw::Function {
            key: vec![],
            original_span: span,
            executable_export: None,
            parameters: vec![],
            result_type: raw::ValueType::Unit,
            places: vec![],
            loans: vec![],
            blocks: vec![raw::Block {
                id: 0,
                value_parameters: vec![],
                instructions,
                terminator: raw::Terminator::Return { value: target.map(|_| 0), cleanup: 0 },
            }],
            cleanup_plans: vec![],
        };
        let graph = Graph::build(&raw, 0, &mut 0).expect("actual complete private CFG");
        let values: Vec<_> = graph.types().collect();
        let owners = vec![None; values.len()];
        functions.push(Function {
            graph,
            values,
            places: places::Table {
                rows: vec![],
                loans: vec![],
                owners,
                decomposition: Decomposition::default(),
            },
            callees: vec![target.map_or_else(Vec::new, |id| vec![Some(id)])],
            sites: vec![target.map_or_else(Vec::new, |_| vec![0])],
            may_fail: false,
            upgrades: vec![None],
        });
        raw_functions.push(raw);
    }
    let program = raw::Program {
        language_contract: raw::CONTRACT.into(),
        modules: vec![],
        declarations: vec![],
        type_keys: vec![],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        entry_module: 0,
        functions: raw_functions,
    };
    (program, functions)
}

fn check(count: usize, cached_suffix: bool) -> Result<(), Failure> {
    let (program, mut functions) = fixture(count, cached_suffix);
    fallibility(&program, &mut functions, &mut 0)
}

#[test]
fn inherited_depth_counts_128_functions_including_the_root() {
    assert_eq!(crate::data_ownership_v1::MAX_STATIC_CALL_DEPTH, 128);
    check(128, false).expect("exact inherited function depth");
    assert!(
        matches!(check(129, false), Err(Failure::Diagnostics(v)) if v[0].code == "ZRYNA-I3201"),
        "129 functions must fail the actual typed gate, not only earlier source replay"
    );
    check(128, false).expect("pristine same-input retry");
}

#[test]
fn cached_subtree_height_retains_the_same_exact_first_extra_frontier() {
    check(128, true).expect("exact inherited depth through cached child subtrees");
    assert!(
        matches!(check(129, true), Err(Failure::Diagnostics(v)) if v[0].code == "ZRYNA-I3201"),
        "cached subtree must include its caller root when applying inherited depth"
    );
    check(128, true).expect("pristine cached traversal retry");
}
