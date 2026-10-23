//! Hand-authored phase controls exercise owner replay without source replay repairing claims.
use super::super::struct_v4::{claim, layouts};
use super::*;
fn fixture() -> (graph::Program, Vec<Vec<raw::Extension>>, VerifiedLayouts) {
    let (mut graph, mut effects) = claim();
    let block = &mut graph.functions[0].blocks[0];
    let mut returned = block.instructions[4].clone();
    returned.result.id = 5;
    block.instructions[4].result.ty = graph::Type::Stored(3);
    block.instructions[4].operation = graph::Operation::Unit;
    block.instructions.push(returned);
    block.terminator = graph::Terminator::Return(5);
    effects[0]
        .push(raw::Extension { result: 4, operation: raw::Operation::CloneStruct { value: 3 } });
    (graph, effects, layouts())
}
#[test]
fn clone_snapshot_retains_source_excludes_result_and_return_releases_both() {
    let (graph, effects, layouts) = fixture();
    let (plans, _, mut table) =
        derive_counted(&graph, &effects, &layouts).expect("independent owner proof");
    let failure = &plans[0].steps[3];
    assert_eq!((failure.block, failure.position, failure.failure), (0, 4, true));
    assert_eq!(failure.cleanup, [3, 0]);
    assert_eq!(plans[0].steps[4].cleanup, [4, 3, 0]);
    let view = table.view(0, 4, &plans).expect("bound clone");
    assert_eq!((view.source(), view.result(), view.ty()), (3, 4, 3));
    assert_eq!(view.string_occurrences(), 2);
    assert_eq!(view.failure_step(), Some(failure));
    for cleanup in [vec![4, 3, 0], vec![0]] {
        let mut hostile = plans.clone();
        hostile[0].steps[3].cleanup = cleanup;
        assert!(table.validate_plans(&hostile, &mut 0).is_err());
    }
    table.validate_plans(&plans, &mut 0).expect("fresh pristine snapshot");
    assert!(
        super::super::derive(&graph, &effects, &layouts).is_err(),
        "old admission stays closed"
    );
}
#[test]
fn clone_replay_rejects_moved_and_exclusively_loaned_source_before_result_publication() {
    for exclusive in [false, true] {
        let (mut graph, mut effects, layouts) = fixture();
        let block = &mut graph.functions[0].blocks[0];
        block.instructions[4].result.ty = if exclusive {
            graph::Type::Borrow { referent: 3, exclusive: true }
        } else {
            graph::Type::Stored(3)
        };
        block.instructions[5].result.ty = graph::Type::Stored(3);
        block.instructions[5].operation = graph::Operation::Unit;
        let mut returned = block.instructions[5].clone();
        returned.result = graph::Definition { id: 6, ty: graph::Type::Stored(1) };
        returned.operation = graph::Operation::I32Literal(7);
        block.instructions.push(returned);
        block.terminator = graph::Terminator::Return(6);
        effects[0][4].operation = if exclusive {
            raw::Operation::Borrow { value: 3, exclusive: true }
        } else {
            raw::Operation::Move(3)
        };
        effects[0].push(raw::Extension {
            result: 5,
            operation: raw::Operation::CloneStruct { value: 3 },
        });
        assert!(derive_counted(&graph, &effects, &layouts).is_err());
        let (graph, effects, layouts) = fixture();
        derive_counted(&graph, &effects, &layouts).expect("fresh recovery after hostile owner use");
    }
}
#[test]
fn clone_table_rejects_changed_result_identity_and_nonunit_placeholder() {
    for change_type in [false, true] {
        let (mut graph, effects, layouts) = fixture();
        let result = &mut graph.functions[0].blocks[0].instructions[4];
        if change_type {
            result.result.ty = graph::Type::Stored(2);
        } else {
            result.operation = graph::Operation::I32Literal(7);
        }
        assert!(Table::derive(&graph, &effects, &layouts, &mut 0).is_err());
    }
}
#[test]
fn recipe_retention_expanded_occurrences_and_owner_replay_share_exact_credit() {
    let (graph, effects, layouts) = fixture();
    let (plans, spent, table) =
        derive_counted(&graph, &effects, &layouts).expect("cold full proof");
    let view = table.view(0, 4, &plans).expect("exact site");
    let fields = view.root().fields().collect::<Vec<_>>();
    assert_eq!(fields.len(), 2);
    assert_eq!((fields[0].1, fields[1].1), (0, 1));
    assert_eq!((fields[0].2.ty(), fields[1].2.ty()), (2, 2));
    for extra in [0, 1] {
        let mut used = super::super::MAX_OWNER_UNITS - spent + extra;
        let result = (|| {
            let mut table = Table::derive(&graph, &effects, &layouts, &mut used)?;
            let (plans, replayed) =
                super::super::replay_with(&graph, &effects, &layouts, Some(&table), used)?;
            used = replayed;
            table.validate_plans(&plans, &mut used)?;
            Ok::<_, Failure>(plans)
        })();
        if extra == 0 {
            assert_eq!(result.expect("exact inherited remaining credits"), plans);
            assert_eq!(used, super::super::MAX_OWNER_UNITS);
        } else {
            assert!(matches!(result, Err(Failure::Diagnostics(v)) if v[0].code == "ZRYNA-I3201"));
        }
    }
}
