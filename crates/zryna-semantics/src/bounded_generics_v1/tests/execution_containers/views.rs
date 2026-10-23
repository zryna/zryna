//! Inspect only opaque same-program getters after the complete independent issuer succeeds.
use zryna_ir::generic_v1::execution_v1::{
    CloneNodeKind, VerifiedCleanupAction, VerifiedCleanupPlan, VerifiedExecution,
    VerifiedOperation, raw,
};
use zryna_layout::StorageTarget;
pub(super) fn check(case: &str, program: &VerifiedExecution<'_>) {
    let mut counts = [0; 5];
    let mut clones = Vec::new();
    for operation in program.operations() {
        if let Some(vector) = operation.vec_construct() {
            counts[0] += 1;
            assert_eq!(vector.result().ty(), raw::ValueType::Stored(vector.ty()));
            assert_eq!(vector.destination_place().ty(), vector.ty());
            assert_eq!(
                vector.destination_place().kind(),
                raw::PlaceKind::Temporary(vector.result().id())
            );
            assert_child(program, vector.ty(), vector.element_type());
            assert_excludes(vector.outer_failure(), vector.destination_place().id());
            assert_eq!(
                operation.failure().expect("Vec allocation failure").id(),
                vector.outer_failure().id()
            );
            for value in vector.elements() {
                assert_eq!(value.ty(), raw::ValueType::Stored(vector.element_type()));
            }
            if case == "vec-owned-prepare" {
                prepared_strings(program, operation);
            }
        }
        if let Some(shared) = operation.shared_construct() {
            counts[1] += 1;
            assert_eq!(shared.payload().ty(), raw::ValueType::Stored(shared.referent_type()));
            assert_eq!(shared.result().ty(), raw::ValueType::Stored(shared.ty()));
            assert_eq!(
                shared.destination_place().kind(),
                raw::PlaceKind::Temporary(shared.result().id())
            );
            assert_child(program, shared.ty(), shared.referent_type());
            assert_excludes(shared.outer_failure(), shared.destination_place().id());
            let function =
                program.functions().nth(operation.site().function).expect("same function");
            let payload = function
                .places()
                .find(|place| place.kind() == raw::PlaceKind::Temporary(shared.payload().id()))
                .expect("pending String owner");
            assert_eq!(drops(shared.outer_failure()), vec![payload.id()]);
        }
        if let Some(weak) = operation.weak_downgrade() {
            counts[2] += 1;
            assert_eq!(weak.result().ty(), raw::ValueType::Stored(weak.ty()));
            assert_child(program, weak.ty(), weak.referent_type());
            let source = weak.source_place().expect("literal uses actual local Shared");
            assert!(weak.source_loan().is_none());
            assert_child(program, source.ty(), weak.referent_type());
            assert!(
                drops(weak.outer_failure()).contains(&source.id()),
                "failed count retains original Shared"
            );
            assert_excludes(weak.outer_failure(), weak.destination_place().id());
        }
        if let Some(index) = operation.index_copy() {
            counts[3] += 1;
            assert_eq!(index.result().ty(), raw::ValueType::Stored(index.element_type()));
            assert_child(program, index.container_type(), index.element_type());
            let source = index.source_place().expect("literal Vec source");
            assert!(index.source_loan().is_none());
            assert_eq!(source.ty(), index.container_type());
            assert!(
                drops(index.outer_failure()).contains(&source.id()),
                "failed bounds retains whole Vec"
            );
            assert_eq!(index.index().ty(), raw::ValueType::Stored(1));
        }
        if let Some(kind) = clone_kind(case, operation) {
            counts[4] += 1;
            clones.push(kind);
        }
    }
    let (expected, kinds, functions) = match case {
        "vec-copy" => ([1, 0, 0, 2, 1], vec![CloneNodeKind::VecEach], 2),
        "vec-owned-prepare" => ([1, 0, 0, 0, 1], vec![CloneNodeKind::VecEach], 1),
        "shared-weak-count" => {
            ([0, 1, 1, 0, 2], vec![CloneNodeKind::SharedCount, CloneNodeKind::WeakCount], 1)
        }
        "opaque-count-unused" => ([0; 5], vec![], 1),
        _ => panic!("only settled exact originals"),
    };
    assert_eq!(counts, expected);
    assert_eq!(clones, kinds);
    assert_eq!(program.functions().len(), functions);
}
fn assert_child(program: &VerifiedExecution<'_>, container: u32, child: u32) {
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let layouts = program.layouts(target);
        let shape = layouts
            .types()
            .find(|ty| ty.id().index() == container)
            .expect("same-seal exact container");
        assert_eq!(shape.referenced_type().expect("actual element/referent").index(), child);
    }
}
fn drops(plan: VerifiedCleanupPlan<'_, '_>) -> Vec<u32> {
    plan.actions()
        .filter_map(|action| {
            if let VerifiedCleanupAction::DropPlace(place) = action {
                Some(place.id())
            } else {
                None
            }
        })
        .collect()
}
fn assert_excludes(plan: VerifiedCleanupPlan<'_, '_>, destination: u32) {
    assert!(!drops(plan).contains(&destination), "failure never owns unwritten result");
}
fn prepared_strings(program: &VerifiedExecution<'_>, operation: VerifiedOperation<'_, '_>) {
    let vector = operation.vec_construct().expect("Vec<String> constructor");
    let function = program.functions().nth(operation.site().function).expect("same function");
    let values = vector
        .elements()
        .map(zryna_ir::generic_v1::execution_v1::VerifiedValue::id)
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 2);
    let expected = values
        .iter()
        .rev()
        .map(|id| {
            function
                .places()
                .find(|place| place.kind() == raw::PlaceKind::Temporary(*id))
                .expect("completed prepared String owner")
                .id()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        drops(vector.outer_failure()),
        expected,
        "both prepared strings remain owned before buffer commit"
    );
}

fn clone_kind(case: &str, operation: VerifiedOperation<'_, '_>) -> Option<CloneNodeKind> {
    let clone = operation.clone_frontier()?;
    let frontier = clone.frontier();
    assert!(frontier.may_fail());
    let destination = clone.destination_place().expect("affine container destination");
    let mut actions = clone.prefix_failure().expect("local prefix rollback").actions();
    let Some(VerifiedCleanupAction::DropInitializedPrefix { destination: actual, preparation }) =
        actions.next()
    else {
        panic!("prefix rollback first")
    };
    assert_eq!(actual.id(), destination.id());
    assert_eq!(preparation.site(), operation.site());
    assert_eq!(frontier.fields().len(), 0);
    assert_eq!(frontier.variants().len(), 0);
    if frontier.kind() == CloneNodeKind::VecEach {
        let element = frontier.element().expect("finite actual runtime-element edge");
        assert_eq!(
            element.kind(),
            if case == "vec-copy" { CloneNodeKind::Copy } else { CloneNodeKind::String }
        );
    } else {
        assert!(matches!(frontier.kind(), CloneNodeKind::SharedCount | CloneNodeKind::WeakCount));
        assert!(frontier.element().is_none(), "count acquisition never clones payload");
    }
    Some(frontier.kind())
}
