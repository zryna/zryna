use super::*;
use zryna_diagnostics::Diagnostic;

#[path = "mixed_root_replacement_fixture.rs"]
mod fixture;
pub(in crate::data_ownership_v1) use fixture::{
    ReplacementCase, ReplacementRoot, replacement_fixture,
};

#[test]
fn mixed_root_replacement_constructors_moves_and_repeated_commits_reach_verified_ir() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for case in [ReplacementCase::Constructor, ReplacementCase::Move, ReplacementCase::Repeated]
        {
            let (source, raw) = replacement_fixture(root, case);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated replacement source");
            let mut previous = None;
            for _ in 0..2 {
                let program = lower(pair_input(&syntax, &sources))
                    .unwrap_or_else(|errors| panic!("{root:?} {case:?}: {errors:?}"));
                let function =
                    program.modules().next().expect("module").functions().next().expect("function");
                let block = function.blocks().next().expect("block");
                let instructions = block.instructions().collect::<Vec<_>>();
                let replacements = instructions
                    .iter()
                    .filter(|i| i.kind() == VerifiedInstructionKind::ReplacePlace)
                    .collect::<Vec<_>>();
                assert_eq!(
                    replacements.len(),
                    if matches!(case, ReplacementCase::Repeated) { 2 } else { 1 }
                );
                let target = replacements[0].place_operands().next().expect("root target");
                let mut observed = Vec::new();
                for (ordinal, replacement) in replacements.iter().enumerate() {
                    assert_eq!(replacement.place_operands().collect::<Vec<_>>(), vec![target]);
                    let drops = replacement.derived_drop_actions().collect::<Vec<_>>();
                    assert_eq!(drops.len(), 1, "exactly one old root cleanup");
                    assert_eq!(drops[0].root(), target);
                    if matches!(root, ReplacementRoot::Enum) {
                        assert_eq!(drops[0].active_variant(), Some(u32::from(ordinal == 0)));
                    }
                    observed.push((target.index(), drops[0].active_variant()));
                }
                assert_eq!(
                    block.terminator().derived_drop_actions().count(),
                    0,
                    "returned owner excluded from cleanup"
                );
                assert_eq!(block.terminator().value_operands().count(), 1);
                if let Some(previous) = &previous {
                    assert_eq!(&observed, previous);
                }
                previous = Some(observed);
            }
        }
    }
}

#[test]
fn mixed_root_replacement_rejects_unavailable_wrong_type_and_self_consumption_exactly() {
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for case in [
            ReplacementCase::Immutable,
            ReplacementCase::Moved,
            ReplacementCase::WrongType,
            ReplacementCase::SelfDirect,
            ReplacementCase::InvalidLater,
        ] {
            let (source, raw) = replacement_fixture(root, case);
            let sources = sources_for(&source);
            let body = &raw.files[0].functions[0].body;
            let (target, value) = body
                .statements
                .iter()
                .find_map(|statement| {
                    if let RawStatementKind::Assignment { target, value, .. } = statement.kind {
                        Some((target, value))
                    } else {
                        None
                    }
                })
                .expect("assignment");
            let target = body.expressions[target as usize].span;
            let value = body.expressions[value as usize].span;
            let (code, at, message, help) = match case {
                ReplacementCase::Immutable | ReplacementCase::Moved => (
                    "ZRYNA-M3014",
                    target,
                    "owned aggregate assignment target is immutable, moved, or only partially available",
                    "assign only to an initialized mutable aggregate root before moving any projection",
                ),
                ReplacementCase::SelfDirect => (
                    "ZRYNA-M3014",
                    value,
                    "owned aggregate assignment cannot consume its destination while preparing its replacement",
                    "clone the destination or prepare a distinct aggregate value before replacement",
                ),
                ReplacementCase::WrongType if matches!(root, ReplacementRoot::Vec) => (
                    "ZRYNA-M3013",
                    value,
                    "Vec construction type differs from its contextual type",
                    "construct the exact annotated Vec type",
                ),
                ReplacementCase::WrongType => (
                    "ZRYNA-M3016",
                    value,
                    "expression is outside private owned Struct/Enum/FixedArray lowering",
                    "use literals, whole-value moves, and exact Struct/Enum/FixedArray constructors",
                ),
                ReplacementCase::InvalidLater => (
                    "ZRYNA-M3002",
                    nth_untrusted_span(&source, "lost", 0),
                    "aggregate value 'lost' is not declared",
                    "reference one exact preceding local using its declared spelling",
                ),
                _ => unreachable!("selected negative cases"),
            };
            let expected = vec![Diagnostic::error_at(code, span(&sources, at), message, help)];
            let syntax =
                verify_snapshot(raw, &sources).expect("authenticated rejected replacement source");
            let first =
                lower(pair_input(&syntax, &sources)).expect_err("invalid replacement must fail");
            assert_eq!(first, expected, "{root:?} {case:?}");
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("deterministic rejection"),
                first,
                "{root:?} {case:?}"
            );
        }
    }
}

#[test]
fn mixed_root_replacement_keeps_nested_self_and_aggregate_identity_shapes_excluded() {
    // The nested Vec changes the exact RHS type; aggregate identity calls are outside
    // the admitted call subset. These controls do not prove destination retention.
    for root in [
        ReplacementRoot::Struct,
        ReplacementRoot::Enum,
        ReplacementRoot::Array,
        ReplacementRoot::Vec,
    ] {
        for case in [ReplacementCase::SelfNested, ReplacementCase::SelfCall] {
            let (source, raw) = replacement_fixture(root, case);
            let sources = sources_for(&source);
            let syntax = verify_snapshot(raw, &sources).expect("authenticated excluded shape");
            let first =
                lower(pair_input(&syntax, &sources)).expect_err("excluded replacement shape");
            assert!(!first.is_empty());
            assert_eq!(
                lower(pair_input(&syntax, &sources)).expect_err("deterministic exclusion"),
                first
            );
        }
    }
}
