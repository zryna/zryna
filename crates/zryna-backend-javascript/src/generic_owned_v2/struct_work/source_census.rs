//! Phase-A test runs against current production with no proposed accounting helper.
use zryna_ir::generic_v1::owned_v2::raw::Operation;
pub(super) fn census(
    program: &zryna_ir::generic_v1::owned_v2::VerifiedOwnedProgram<'_>,
    scopes: usize,
) {
    assert_eq!(program.functions().len(), 1);
    let f = &program.functions()[0];
    assert_eq!(f.blocks.len(), 1);
    let definitions = f.blocks[0].parameters.len() + f.blocks[0].instructions.len();
    assert_eq!(definitions, 34 + 2 * scopes);
    let drops = f.blocks[0]
        .instructions
        .iter()
        .filter(|i| matches!(program.extension(0, i.result.id), Some(Operation::Drop(_))))
        .count();
    assert_eq!(drops, scopes);
    let plan = program.plan(0).expect("source-derived complete plan");
    assert_eq!(plan.steps.len(), 33 + scopes);
    assert_eq!(plan.steps.iter().map(|step| step.cleanup.len()).sum::<usize>(), 497 + scopes);
}

#[test]
fn genuine_wide_sources_reach_v4_with_fixed_definition_and_cleanup_counts() {
    for scopes in [1, 1024] {
        super::source_fixture::with_wide(scopes, |program| census(program, scopes));
    }
}
