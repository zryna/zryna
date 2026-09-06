use super::generic_vec_fixture::shared_weak_fixture::{Case, fixture_case};
use super::*;
use zryna_layout::TypeCategory;

fn verified_trace(case: Case) -> (Vec<Vec<u32>>, usize) {
    let (source, raw) = fixture_case(case);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("authenticated payload-closure fixture");
    let program = lower(pair_input(&syntax, &sources)).expect("mandatory verified IR lowering");
    let replay = lower(pair_input(&syntax, &sources)).expect("deterministic verified IR replay");
    assert_eq!(format!("{program:?}"), format!("{replay:?}"));
    let function = program.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let construct = block
        .instructions()
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::SharedConstruct)
        .expect("source shared construction");
    let layouts = program.verified_ir().linear32_layouts();
    let shared = layouts
        .type_by_id(construct.result_type().expect("Shared result type"))
        .expect("sealed Shared layout");
    let payload_id = shared.referenced_type().expect("Shared referent");
    let payload = layouts.type_by_id(payload_id).expect("sealed nominal payload");
    assert_eq!(payload.category(), TypeCategory::Enum);
    assert_eq!(payload.variants().len(), 3);
    assert_eq!(payload.variants()[0].payload(), None, "first variant is payloadless");
    if matches!(case, Case::RecursiveEnum) {
        let recursive = layouts
            .type_by_id(payload.variants()[1].payload().expect("recursive payload"))
            .expect("sealed recursive indirection");
        assert_eq!(recursive.category(), TypeCategory::Shared);
        assert_eq!(recursive.referenced_type(), Some(payload_id));
    }
    let cleanup = block
        .instructions()
        .filter(|instruction| {
            matches!(
                instruction.kind(),
                VerifiedInstructionKind::SharedConstruct
                    | VerifiedInstructionKind::SharedClone
                    | VerifiedInstructionKind::WeakDowngrade
                    | VerifiedInstructionKind::WeakClone
            )
        })
        .map(|instruction| {
            instruction
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let exit_cleanup = block.terminator().derived_drop_actions().count();
    (cleanup, exit_cleanup)
}

#[test]
fn recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay() {
    for case in [Case::MultiVariantEnum, Case::RecursiveEnum] {
        let first = verified_trace(case);
        assert_eq!(
            first.0,
            [vec![1], vec![3], vec![5, 3], vec![7, 5, 3]],
            "each failed allocation/count step retains its source and reverse-cleans earlier handles"
        );
        assert_eq!(
            first.1, 3,
            "the returned Shared transfers while the cloned Shared and both Weak handles release"
        );
    }
}
