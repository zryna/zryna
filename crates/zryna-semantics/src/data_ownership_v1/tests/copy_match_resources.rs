use super::*;
use super::generic_vec_fixture::ordinary_array_composition_fixture::copy_match_expression_fixture::{Arm, padded_fixture};

#[test]
#[ignore = "proportional authenticated 16,384-value cross-arm boundary; included in preflight"]
fn copy_match_expressions_value_budget_counts_every_arm_and_accepts_only_exact_limit() {
    let limit = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    let width = u32::try_from((limit - 4) / 4).expect("bounded authenticated fixture");
    let mut accepted = None;
    for padding in [1, 2, 1] {
        let (source, raw) = padded_fixture(Arm::LargeArray(width), true, false, padding);
        assert_eq!(derived_value_count(&raw.files[0].functions[0]), limit + padding - 1,);
        let sources = sources_for(&source);
        let syntax =
            verify_snapshot(raw, &sources).expect("authenticated proportional arm fixture");
        if padding == 1 {
            let program =
                lower(pair_input(&syntax, &sources)).expect("exact cross-arm value limit");
            let function = program
                .modules()
                .next()
                .expect("bounded authenticated fixture")
                .functions()
                .next()
                .expect("bounded authenticated fixture");
            let results = function
                .blocks()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
                .filter_map(FaultVerifiedInstruction::result)
                .collect::<Vec<_>>();
            let observation = function
                .blocks()
                .map(|block| {
                    let instructions = block
                        .instructions()
                        .map(|instruction| {
                            format!(
                                "{:?}",
                                (
                                    instruction.kind(),
                                    instruction
                                        .result()
                                        .map(zryna_ir::data_ownership_v1::ValueIdentity::index),
                                    instruction.result_type().map(zryna_layout::TypeId::index),
                                    instruction
                                        .value_operands()
                                        .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
                                        .collect::<Vec<_>>(),
                                    instruction
                                        .place_operands()
                                        .map(zryna_ir::data_ownership_v1::PlaceIdentity::index)
                                        .collect::<Vec<_>>()
                                )
                            )
                        })
                        .collect::<Vec<_>>();
                    (
                        instructions,
                        format!(
                            "{:?}",
                            (
                                block.terminator().kind(),
                                block
                                    .terminator()
                                    .value_operands()
                                    .map(zryna_ir::data_ownership_v1::ValueIdentity::index)
                                    .collect::<Vec<_>>(),
                                block
                                    .terminator()
                                    .cleanup()
                                    .map(zryna_ir::data_ownership_v1::CleanupPlanIdentity::index)
                            )
                        ),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(before_rejection) = &accepted {
                assert_eq!(
                    &observation, before_rejection,
                    "exact fixture recovers after rejected preparation"
                );
            } else {
                accepted = Some(observation);
            }
            assert_eq!(results.len() + function.parameters().len(), limit);
            assert_eq!(
                results.last().expect("bounded authenticated fixture").index(),
                u32::try_from(limit - 1).expect("bounded authenticated fixture")
            );
        } else {
            let first = lower(pair_input(&syntax, &sources)).expect_err("first extra value");
            let second = lower(pair_input(&syntax, &sources)).expect_err("replayed first extra");
            assert_eq!(first, second);
            assert_eq!(first[0].code(), "ZRYNA-M3201");
        }
    }
}
