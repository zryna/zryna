use super::generic_vec_fixture::handle_frontier_fixture::fixture;
use super::handle_frontier_model::{self as model, Claim, Failure, PathPart};
use super::*;
use zryna_ir::data_ownership_v1::{VerifiedBlock, VerifiedDropAction};

fn assert_hostiles(events: &[model::Acquisition], claim: &Claim, source: FaultPlaceIdentity) {
    let mut cases = Vec::new();
    let mut bad = claim.clone();
    bad.ordinal = events.len();
    cases.push((bad, Failure::Ordinal));
    let mut bad = claim.clone();
    bad.operation = LogicalOperation::Allocate;
    cases.push((bad, Failure::Operation));
    let mut bad = claim.clone();
    bad.status = RuntimeStatus::Ok;
    cases.push((bad, Failure::Status));
    let mut bad = claim.clone();
    bad.destination = source;
    cases.push((bad, Failure::Destination));
    let mut bad = claim.clone();
    bad.survivors.remove(0);
    cases.push((bad, Failure::Survivors));
    let mut bad = claim.clone();
    bad.survivors.swap(0, 1);
    cases.push((bad, Failure::Survivors));
    if !claim.reverse_prefix.is_empty() {
        let mut bad = claim.clone();
        bad.reverse_prefix.pop();
        cases.push((bad, Failure::Prefix));
    }
    if claim.reverse_prefix.len() > 1 {
        let mut bad = claim.clone();
        bad.reverse_prefix.swap(0, 1);
        cases.push((bad, Failure::Prefix));
    }
    for (bad, error) in cases {
        for _ in 0..2 {
            assert_eq!(
                model::validate(events, claim.destination, &claim.survivors, &bad),
                Err(error)
            );
        }
        assert_eq!(model::validate(events, claim.destination, &claim.survivors, claim), Ok(()));
    }
}

#[test]
fn handle_frontier_source_vec_enum_occurrences_retain_replacement_owners() {
    for populated in [false, true] {
        let (source, raw) = fixture(populated);
        let sources = sources_for(&source);
        let syntax = verify_snapshot(raw, &sources).expect("authenticated nested handle source");
        let program = lower(pair_input(&syntax, &sources)).expect("verified nested handle source");
        let function =
            program.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().flat_map(VerifiedBlock::instructions).collect::<Vec<_>>();
        let vector = instructions
            .iter()
            .find(|i| i.kind() == VerifiedInstructionKind::VecConstruct)
            .expect("source Vec construction");
        let instruction = instructions
            .iter()
            .find(|i| i.handle_aware_clone().is_some())
            .expect("replacement clone");
        let clone = instruction.handle_aware_clone().expect("sealed clone");
        let events = model::occurrences(function, clone, vector.result().expect("constructed Vec"))
            .expect("concrete constructor occurrences instantiate sealed recipe");
        assert_occurrences(&events, populated);
        let survivors = assert_replacement(&instructions, *instruction);
        let VerifiedHandleAwareCloneSourceAuthority::Root(source_root) = clone.source_authority()
        else {
            panic!("local source root");
        };
        assert_faults(program.runtime_abi(), &events, clone.destination(), &survivors, source_root);
        assert_eq!(
            function
                .blocks()
                .next()
                .expect("block")
                .terminator()
                .derived_drop_actions()
                .map(|drop| drop.root().index())
                .collect::<Vec<_>>(),
            [source_root.index(), 3, 2, 1]
        );
        assert_eq!(model::occurrences(function, clone, clone.result()), Err(Failure::Shape));
        let replay = lower(pair_input(&syntax, &sources)).expect("pristine source recovery");
        assert_eq!(program.verified_ir().identity(), replay.verified_ir().identity());
    }
}

fn assert_occurrences(events: &[model::Acquisition], populated: bool) {
    let expected = if populated {
        vec![
            vec![],
            vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(0)],
            vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(1)],
            vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(2)],
            vec![PathPart::Index(2), PathPart::Variant(1), PathPart::Field(0)],
            vec![PathPart::Index(2), PathPart::Variant(1), PathPart::Field(1)],
            vec![PathPart::Index(2), PathPart::Variant(1), PathPart::Field(2)],
        ]
    } else {
        vec![vec![]]
    };
    assert_eq!(events.iter().map(|event| event.path.clone()).collect::<Vec<_>>(), expected);
    let operations = if populated {
        vec![
            LogicalOperation::VecAllocate,
            LogicalOperation::StrongClone,
            LogicalOperation::WeakClone,
            LogicalOperation::StringClone,
            LogicalOperation::StrongClone,
            LogicalOperation::WeakClone,
            LogicalOperation::StringClone,
        ]
    } else {
        vec![LogicalOperation::VecAllocate]
    };
    assert_eq!(events.iter().map(|event| event.operation).collect::<Vec<_>>(), operations);
    if populated {
        assert_ne!(events[1].source, events[4].source, "separate concrete Vec elements");
    }
}

fn assert_replacement(
    instructions: &[FaultVerifiedInstruction<'_>],
    instruction: FaultVerifiedInstruction<'_>,
) -> Vec<FaultPlaceIdentity> {
    let clone = instruction.handle_aware_clone().expect("sealed clone");
    let survivors = instruction.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>();
    let VerifiedHandleAwareCloneSourceAuthority::Root(source_root) = clone.source_authority()
    else {
        panic!("local source root");
    };
    assert!(survivors.contains(&source_root));
    let replacement = instructions
        .iter()
        .find(|i| i.kind() == VerifiedInstructionKind::ReplacePlace)
        .expect("replacement commit");
    let old = replacement.derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(old.len(), 1);
    assert_ne!(old[0].root(), source_root);
    assert!(survivors.contains(&old[0].root()));
    assert_eq!(
        survivors.iter().map(|root| root.index()).collect::<Vec<_>>(),
        [source_root.index(), 3, 2, 1, 5]
    );
    assert_eq!(old[0].root().index(), 5);
    assert_eq!(old[0].moved_projections().len(), 0);
    assert_eq!(old[0].active_variant(), None);
    assert_eq!(old[0].active_variants().len(), 0);
    assert_eq!(replacement.value_operands().collect::<Vec<_>>(), [clone.result()]);
    let prefix = instruction.handle_aware_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(prefix[0].kind(), VerifiedDropActionKind::GenericCloneInitializedPrefix);
    assert_eq!(prefix[0].root(), clone.destination());
    assert_eq!(prefix[1..].iter().map(VerifiedDropAction::root).collect::<Vec<_>>(), survivors);
    assert!(!survivors.contains(&clone.destination()));
    assert!(instruction.derived_drop_actions().all(|drop| drop.moved_projections().len() == 0));
    assert_eq!(instructions.iter().filter(|i| i.handle_aware_clone().is_some()).count(), 1);
    let clone_position = instructions
        .iter()
        .position(|i| i.result() == Some(clone.result()))
        .expect("clone position");
    assert_eq!(instructions[clone_position + 1].kind(), VerifiedInstructionKind::ReplacePlace);
    assert_eq!(instructions[clone_position + 2].kind(), VerifiedInstructionKind::MoveFromPlace);
    assert_eq!(
        instructions[clone_position + 2].place_operands().collect::<Vec<_>>(),
        [old[0].root()]
    );
    survivors
}

fn assert_faults(
    abi: &VerifiedOwnershipRuntimeAbi,
    events: &[model::Acquisition],
    destination: FaultPlaceIdentity,
    survivors: &[FaultPlaceIdentity],
    source_root: FaultPlaceIdentity,
) {
    for (ordinal, event) in events.iter().enumerate() {
        let status = match event.operation {
            LogicalOperation::StrongClone | LogicalOperation::WeakClone => RuntimeStatus::Refcount,
            _ => RuntimeStatus::Allocation,
        };
        let claim = Claim {
            ordinal,
            operation: event.operation,
            status,
            destination,
            reverse_prefix: events[..ordinal].iter().rev().cloned().collect(),
            survivors: survivors.to_vec(),
        };
        if ordinal == 6 {
            assert_eq!(
                claim
                    .reverse_prefix
                    .iter()
                    .map(model::Acquisition::cleanup_operation)
                    .collect::<Vec<_>>(),
                [
                    LogicalOperation::WeakRelease,
                    LogicalOperation::StrongReleaseBegin,
                    LogicalOperation::StringRelease,
                    LogicalOperation::WeakRelease,
                    LogicalOperation::StrongReleaseBegin,
                    LogicalOperation::VecReleaseStorage
                ]
            );
            assert_eq!(
                claim.reverse_prefix.iter().map(|entry| entry.path.clone()).collect::<Vec<_>>(),
                [
                    vec![PathPart::Index(2), PathPart::Variant(1), PathPart::Field(1)],
                    vec![PathPart::Index(2), PathPart::Variant(1), PathPart::Field(0)],
                    vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(2)],
                    vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(1)],
                    vec![PathPart::Index(0), PathPart::Variant(1), PathPart::Field(0)],
                    vec![]
                ]
            );
        }
        let declaration = abi
            .status_declarations()
            .find(|declaration| declaration.status() == status)
            .expect("sealed ABI status");
        assert_eq!(
            declaration.trap_identity(),
            Some(if status == RuntimeStatus::Refcount {
                VerifiedStatusTrapIdentity::RefcountV1
            } else {
                VerifiedStatusTrapIdentity::AllocationV1
            })
        );
        assert_eq!(model::validate(events, destination, survivors, &claim), Ok(()));
        assert_eq!(model::validate(events, destination, survivors, &claim), Ok(()));
        let mut bad = claim.clone();
        bad.reverse_prefix.push(event.clone());
        assert_eq!(model::validate(events, destination, survivors, &bad), Err(Failure::Prefix));
        assert_hostiles(events, &claim, source_root);
    }
}
