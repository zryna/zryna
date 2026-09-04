use super::super::constructor_resources::tests::with_snapshot;
use super::super::preparation_plan::PreparationFacts;
use super::super::statements::StatementOutcome;
use crate::data_ownership_v1 as ownership;
use crate::data_ownership_v1::owner_state::OwnerState;
use crate::data_ownership_v1::tests::constructor_envelope_fixtures::{self as fixtures, Fixture};
use std::collections::BTreeMap;
use zryna_ir::data_ownership_v1::{VerifiedInstructionKind, raw};
use zryna_layout::TypeCategory;
use zryna_syntax::v4::verify_snapshot;

#[test]
fn constructor_byte_facts_follow_real_string_local_and_preserve_survivor() {
    let (source, snapshot) = fixtures::snapshot(Fixture::Enum);
    let sources = fixtures::sources(&source);
    let syntax = verify_snapshot(snapshot.clone(), &sources).expect("authenticated enum source");
    for _ in 0..2 {
        let program = ownership::lower(fixtures::input(&syntax, &sources))
            .expect("whole source reaches independent verified IR");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        assert_eq!(block.instructions().count(), 8);
        assert_eq!(
            block.instructions().nth(1).expect("local initialization").kind(),
            VerifiedInstructionKind::InitializePlace
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            vec![1]
        );
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
            assert!(lowerer.preparation_facts.string_bytes.is_empty());
            for index in 0..4 {
                let statement = lowerer.function.body.statements[index].clone();
                let outcome = lowerer
                    .lower_statement(
                        u32::try_from(index).expect("bounded statement"),
                        &statement,
                        result,
                        Some(3),
                        1,
                    )
                    .expect("real statement lowers");
                let survivor = lowerer.bindings.get("survivor").expect("real String local");
                assert_eq!(survivor.ty.category, TypeCategory::String);
                assert!(lowerer.owners.contains(survivor.place));
                assert_eq!(
                    lowerer.preparation_facts.string_bytes,
                    BTreeMap::from([(survivor.place, 1)]),
                    "only the live String local retains its exact byte length; no temporary or enum fact"
                );
                if index == 3 {
                    let StatementOutcome::Return(value, _) = outcome else {
                        panic!("final return");
                    };
                    let returned = lowerer.owners.owner(value).expect("returned enum owner");
                    assert!(!lowerer.preparation_facts.string_bytes.contains_key(&returned));
                } else {
                    assert!(matches!(outcome, StatementOutcome::Continue));
                }
            }
            assert!(lowerer.errors.is_empty());
        });
        assert!(errors.is_empty());
    }
}

#[test]
fn constructor_byte_facts_follow_actual_move_rename_and_transfer_deltas() {
    // Owner-map unit control, not a source-reachability or runtime-memory proof.
    let mut owners = OwnerState::default();
    let mut facts = PreparationFacts::default();
    facts.apply(owners.register(raw::ValueId(0), raw::PlaceId(5)).expect("source owner"));
    facts.string_bytes.insert(raw::PlaceId(5), 7);
    facts.apply(owners.register(raw::ValueId(1), raw::PlaceId(9)).expect("move result"));
    facts.apply(
        owners.rehome_move_result(raw::ValueId(1), raw::PlaceId(5)).expect("real move delta"),
    );
    assert_eq!(facts.string_bytes, BTreeMap::from([(raw::PlaceId(9), 7)]));
    facts.apply(owners.rename(raw::ValueId(1), raw::PlaceId(12)).expect("real local rename"));
    assert_eq!(facts.string_bytes, BTreeMap::from([(raw::PlaceId(12), 7)]));
    facts.apply(owners.register(raw::ValueId(2), raw::PlaceId(15)).expect("next move result"));
    facts.apply(
        owners.rehome_move_result(raw::ValueId(2), raw::PlaceId(12)).expect("next real move"),
    );
    assert_eq!(facts.string_bytes, BTreeMap::from([(raw::PlaceId(15), 7)]));
    facts.apply(owners.transfer(raw::ValueId(2)).expect("constructor child transfer"));
    assert!(facts.string_bytes.is_empty());
    assert!(owners.pending().is_empty());
}

#[test]
fn constructor_byte_facts_unknown_replacement_clears_stale_target() {
    // Type-agnostic bookkeeping control: use returned owner deltas, not a fabricated delta.
    for incoming in [None, Some(0), Some(13)] {
        let mut owners = OwnerState::default();
        let mut facts = PreparationFacts::default();
        facts.apply(owners.register(raw::ValueId(0), raw::PlaceId(5)).expect("target"));
        facts.string_bytes.insert(raw::PlaceId(5), 99);
        facts.apply(owners.register(raw::ValueId(1), raw::PlaceId(9)).expect("prepared owner"));
        if let Some(bytes) = incoming {
            facts.string_bytes.insert(raw::PlaceId(9), bytes);
        }
        facts.apply(owners.replace(raw::ValueId(1), raw::PlaceId(5)).expect("real replacement"));
        assert_eq!(
            facts.string_bytes,
            incoming.map(|bytes| (raw::PlaceId(5), bytes)).into_iter().collect()
        );
        assert_eq!(owners.pending(), &[raw::PlaceId(5)]);
    }
}
