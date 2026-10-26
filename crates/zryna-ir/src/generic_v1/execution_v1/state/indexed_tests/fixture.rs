//! Independently built private phase inputs; these publish no source or execution proof.
use crate::generic_v1::execution_v1::{
    plan, raw,
    typed::{
        container_test_support,
        continuation_test_support::{Types, instruction, push, value},
    },
};
pub(super) fn fixture(role: raw::RegionRole) -> (Types, raw::Program) {
    let w = Types::new();
    let mut p = push(&w, None);
    let f = &mut p.functions[0];
    f.parameters.push(raw::Parameter::Value(value(2, w.integer)));
    f.loans = vec![raw::Loan {
        id: 0,
        referent: w.string,
        access: raw::Access::Exclusive,
        region: raw::Region { span: instruction(raw::Operation::Unit).site_span, role },
        origin: raw::LoanOrigin::Indexed { base: raw::PlaceOrLoan::Place(0), index: 2 },
    }];
    f.blocks[0].instructions = vec![
        instruction(raw::Operation::BeginLoan { loan: 0, failure: Some(0) }),
        instruction(raw::Operation::ReplaceLoan { loan: 0, value: 1 }),
        instruction(raw::Operation::EndLoan { loan: 0 }),
    ];
    (w, p)
}
pub(super) fn derive(
    w: &Types,
    p: &raw::Program,
) -> Result<plan::Plan, crate::generic_v1::Failure> {
    let typed = container_test_support::phase_inputs(p, &w.world);
    plan::derive(p, &typed, &mut 0)
}
pub(super) fn across_edge(p: &mut raw::Program) {
    let f = &mut p.functions[0];
    let tail = f.blocks[0].instructions.split_off(1);
    let end = f.blocks[0].terminator.clone();
    f.blocks[0].terminator = raw::Terminator::Jump(raw::Edge { target: 1, arguments: vec![] });
    f.blocks.push(raw::Block {
        id: 1,
        value_parameters: vec![],
        instructions: tail,
        terminator: end,
    });
}
