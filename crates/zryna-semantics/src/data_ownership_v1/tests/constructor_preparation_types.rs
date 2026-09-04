use super::*;
use zryna_source::{SourceFileInput, SourceMap, Span};

fn at() -> Span {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: "x".to_owned(),
    }])
    .expect("source map");
    sources.span(sources.verify_file_id(0).expect("file"), 0, 1).expect("span")
}

// These are isolated dense-cache probes, not independently verified IR programs.
fn instruction(index: u32) -> raw::Instruction {
    let span = at();
    raw::Instruction {
        result: Some(raw::ValueDefinition {
            id: raw::ValueId(index),
            ty: raw::TypeId(index),
            span,
        }),
        span,
        kind: raw::InstructionKind::I32Literal(0),
    }
}

fn effect() -> raw::Instruction {
    raw::Instruction {
        result: None,
        span: at(),
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    }
}

fn copy(cache: &ConstructorValueTypes) -> ConstructorValueTypes {
    ConstructorValueTypes {
        types: cache.types.clone(),
        scanned_instructions: cache.scanned_instructions,
    }
}

#[test]
fn constructor_preparation_types_snapshot_observes_only_copy_and_keeps_distinct_cursors() {
    let mut live = ConstructorValueTypes::default();
    live.record_parameter(instruction(0).result.as_ref().expect("parameter"))
        .expect("dense parameter");
    let instructions = [instruction(1), effect(), instruction(2), effect()];
    assert_eq!(live.observe(&instructions[..1]), Ok(1));
    assert_eq!(live.checkpoint(), (2, 1));
    let before = copy(&live);
    let mut snapshot = live.observed_snapshot(&instructions).expect("pending suffix");
    assert_eq!(live, before);
    assert_eq!(snapshot.checkpoint(), (3, 4));
    assert_eq!(snapshot.get(raw::ValueId(2)), Some(raw::TypeId(2)));
    assert_eq!(snapshot.observe(&instructions), Ok(0), "suffix already observed");
    assert_eq!(live.observed_snapshot(&instructions), Ok(copy(&snapshot)));
    assert_eq!(live, before);
}

#[test]
fn constructor_preparation_types_failed_snapshot_keeps_live_cache_and_pending_cursor() {
    let mut live = ConstructorValueTypes::default();
    assert_eq!(live.observe(&[instruction(0)]), Ok(1));
    let before = copy(&live);
    for bad in [0, 1, 3] {
        let instructions = [instruction(0), instruction(1), instruction(bad)];
        assert_eq!(live.observed_snapshot(&instructions), Err(ConstructorPlanError::WrongShape));
        assert_eq!(live, before, "even a valid pending prefix cannot enter the live cache");
    }
    assert_eq!(live.observed_snapshot(&[]), Err(ConstructorPlanError::WrongShape));
    assert_eq!(live, before);
    let recovered = live.observed_snapshot(&[instruction(0), instruction(1)]).expect("recovery");
    assert_eq!(recovered.checkpoint(), (2, 2));
    assert_eq!(live, before);
}

#[test]
fn constructor_preparation_types_predicted_append_checks_value_and_instruction_identity() {
    let mut live = ConstructorValueTypes::default();
    let instructions = [instruction(0), effect(), effect()];
    live.observe(&instructions).expect("real prefix");
    let before = copy(&live);
    let mut snapshot = live.observed_snapshot(&instructions).expect("snapshot");
    assert_eq!(snapshot.checkpoint(), (1, 3));
    for (value, cursor) in [(0, 3), (2, 3), (1, 1), (1, 2), (1, 4)] {
        let unchanged = copy(&snapshot);
        assert_eq!(
            snapshot.append_predicted(raw::ValueId(value), raw::TypeId(7), cursor),
            Err(ConstructorPlanError::WrongShape)
        );
        assert_eq!(snapshot, unchanged);
    }
    snapshot.append_predicted(raw::ValueId(1), raw::TypeId(7), 3).expect("first predicted value");
    snapshot.append_predicted(raw::ValueId(2), raw::TypeId(9), 4).expect("next predicted value");
    assert_eq!(snapshot.checkpoint(), (3, 5));
    assert_eq!(snapshot.get(raw::ValueId(1)), Some(raw::TypeId(7)));
    assert_eq!(snapshot.get(raw::ValueId(2)), Some(raw::TypeId(9)));
    assert_eq!(snapshot.get(raw::ValueId(3)), None);
    assert_eq!(live, before, "predicted definitions do not update the emission cache");
}

#[test]
fn constructor_preparation_types_cursor_overflow_is_atomic_before_type_append() {
    let mut cache = ConstructorValueTypes { types: Vec::new(), scanned_instructions: usize::MAX };
    let before = copy(&cache);
    assert_eq!(
        cache.append_predicted(raw::ValueId(0), raw::TypeId(7), usize::MAX),
        Err(ConstructorPlanError::WrongShape)
    );
    assert_eq!(cache, before);
}

#[test]
fn constructor_preparation_types_empty_and_effect_only_snapshots_keep_dense_zero() {
    let live = ConstructorValueTypes::default();
    assert_eq!(live.observed_snapshot(&[]), Ok(ConstructorValueTypes::default()));
    let mut snapshot = live.observed_snapshot(&[effect(), effect()]).expect("effect prefix");
    assert_eq!(snapshot.checkpoint(), (0, 2));
    snapshot.append_predicted(raw::ValueId(0), raw::TypeId(7), 2).expect("first definition");
    assert_eq!(snapshot.checkpoint(), (1, 3));
    assert_eq!(live, ConstructorValueTypes::default());
}
