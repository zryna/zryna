//! Count the actual complete functions against the prepaid source-derived instruction law.
use super::fixture;
use crate::generic_execution_v1::recursive::calls::Kind;
use zryna_native_mir::generic_execution_v1::TypeCategory;
#[test]
fn complete_actual_protocol_instruction_count_fits_source_prepaid_bound() {
    for case in ["drop-only", "clone", "controlled-cleanup", "shared-weak-recursive", "mixed"] {
        let mut total = 0u64;
        let mut expected = None;
        for kind in [Kind::Arena, Kind::Find, Kind::Drop, Kind::Release] {
            fixture::with_graph(case, kind, |p, f, _, _| {
                let mut members = 0u64;
                let mut cases = 0u64;
                let mut covered = [false; 9];
                for id in 0..p.program().type_count() {
                    let ty = p
                        .stored_type(u32::try_from(id).expect("bounded id"))
                        .expect("same-owner layout");
                    let (index, cost) = category(ty.category());
                    covered[index] = true;
                    cases += cost;
                    members += u64::try_from(ty.fields().len() + ty.variants().len())
                        .expect("bounded members");
                }
                if case == "mixed" {
                    assert!(
                        covered.into_iter().all(|present| present),
                        "the existing genuine source must cover all nine physical categories"
                    );
                }
                let limit = 192 + cases + members * 64;
                assert!(expected.is_none_or(|old| old == limit));
                expected = Some(limit);
                total += f
                    .layout
                    .blocks()
                    .map(|b| {
                        u64::try_from(f.layout.block_insts(b).count())
                            .expect("bounded instructions")
                    })
                    .sum::<u64>();
            });
        }
        assert!(total <= expected.expect("genuine instruction bound"), "{case}: {total}");
    }
}

// Independent test model: the actual SSA count above is the observation.
fn category(kind: TypeCategory) -> (usize, u64) {
    match kind {
        TypeCategory::Bool => (0, 5),
        TypeCategory::I32 => (1, 5),
        TypeCategory::String => (2, 58),
        TypeCategory::Struct => (3, 10),
        TypeCategory::Enum => (4, 5),
        TypeCategory::FixedArray => (5, 116),
        TypeCategory::Vec => (6, 133),
        TypeCategory::Shared => (7, 144),
        TypeCategory::Weak => (8, 71),
    }
}
