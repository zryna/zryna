//! Source-bound storage need, hostile raw claims and physical/arithmetic resource frontiers.
use super::super::{derive, derive_with_work, verify, verify_with_work};
use super::*;
use zryna_layout::{StorageTarget, TypeCategory};
mod budget;
mod mixed;
mod source;
mod traversal;

#[test]
fn physical_partition_covers_exact_small_remainders_and_minimal_owner_bound() {
    for remaining in
        (0..=4096).chain([MEMORY_BYTES - 24, MEMORY_BYTES - 16, MEMORY_BYTES - 8, MEMORY_BYTES])
    {
        let (capacity, bytes) = split(remaining).expect("physical nonwrapping extent");
        let heap = remaining - bytes;
        assert_eq!(bytes, capacity * RECURSIVE_FRAME_BYTES);
        assert!(heap / MIN_POSITIVE_ALLOCATION_BYTES <= capacity);
        if capacity > 0 {
            let smaller = capacity - 1;
            assert!(
                (remaining - smaller * RECURSIVE_FRAME_BYTES) / MIN_POSITIVE_ALLOCATION_BYTES
                    > smaller,
                "one fewer frame cannot cover all positive allocations"
            );
        }
    }
    for remaining in [0, 8, 16] {
        assert_eq!(split(remaining).expect("small remainder"), (0, 0));
    }
    for remaining in [MEMORY_BYTES + 1, u32::MAX] {
        assert!(split(remaining).is_err());
    }
}

#[test]
fn owned_recursive_source_reserves_disjoint_storage_and_verifies_exact_binding() {
    source::with_program(true, |program| {
        assert!(required(program, &mut 0).expect("actual owned reachability"));
        for observed in [false, true] {
            let claim = derive(program, observed).expect("actual source memory");
            let arena = claim.recursive.as_ref().expect("owned Node/Vec cycle");
            assert_eq!(
                arena.metadata_start,
                align(add(claim.frames_start, claim.frames_bytes).expect("source end"), 8)
                    .expect("alignment")
            );
            assert_eq!(arena.start, arena.metadata_start + arena.metadata_bytes);
            let layout = program.layouts(StorageTarget::Linear32V1);
            let type_count = layout.types().len();
            let members = layout.types().map(|t| t.fields().len() + t.variants().len()).sum();
            assert_eq!(
                arena.metadata_bytes,
                metadata_bytes(type_count, members).expect("full immutable metadata schema")
            );
            assert_eq!(claim.heap_start, arena.start + arena.bytes);
            assert_eq!(arena.bytes, arena.capacity * RECURSIVE_FRAME_BYTES);
            assert!(
                (MEMORY_BYTES - claim.heap_start) / MIN_POSITIVE_ALLOCATION_BYTES <= arena.capacity
            );
            let verified =
                verify(claim.clone(), program).expect("same-source independent plan owner");
            assert!(verified.is_bound_to(program));
            assert_eq!(verified.static_data_bytes(), arena.start);
            assert_eq!(verified.recursive_arena(), claim.recursive.as_ref());
        }
    });
}

#[test]
fn erased_recursive_argument_does_not_create_an_owned_traversal_or_arena() {
    source::with_program(false, |program| {
        assert!(
            graph::all_layouts_have_cycle(program, &mut 0).expect("real unused closed cycle"),
            "control must include the Node/Vec cycle behind the erased argument"
        );
        let array = program
            .layouts(StorageTarget::Linear32V1)
            .types()
            .find(|t| t.category() == TypeCategory::FixedArray && t.array_length() == Some(0))
            .expect("actual erased stored root");
        assert_ne!(array.drop_kind(), 0, "zero length preserves the declared child constraints");
        assert!(!required(program, &mut 0).expect("zero-element edge is terminal"));
        let claim = derive(program, false).expect("erased source memory");
        assert!(claim.recursive.is_none());
        assert_eq!(
            claim.heap_start,
            align(add(claim.frames_start, claim.frames_bytes).expect("source end"), 8)
                .expect("source alignment")
        );
        let plan = verify(claim, program).expect("unchanged acyclic memory authority");
        assert_eq!(plan.static_data_bytes(), plan.claims().heap_start);
    });
}

#[test]
fn opaque_weak_reference_has_no_owned_child_edge() {
    crate::generic_execution_v1::tests::container_source::with_program(
        "shared-weak-count",
        "native",
        |program| {
            let types = program.layouts(StorageTarget::Linear32V1).types().collect::<Vec<_>>();
            let at = types
                .iter()
                .position(|t| t.category() == TypeCategory::Weak)
                .expect("actual Weak source type");
            let mut edges = Vec::new();
            graph::children(&types, at, &mut 0, |id| {
                edges.push(id);
                Ok(())
            })
            .expect("terminal Weak traversal");
            assert!(edges.is_empty());
            assert!(!required(program, &mut 0).expect("acyclic Shared/String/Weak graph"));
            assert!(derive(program, false).expect("acyclic source memory").recursive.is_none());
        },
    );
}

#[test]
fn forged_omitted_underpaid_or_overlapping_arenas_never_issue_memory_authority() {
    source::with_program(true, |program| {
        let original = derive(program, false).expect("actual source memory");
        for change in 0..8 {
            let mut claim = original.clone();
            let arena = claim.recursive.as_mut().expect("actual recursive arena");
            match change {
                0 => {
                    claim.heap_start = arena.metadata_start;
                    claim.recursive = None;
                }
                1 => {
                    arena.capacity -= 1;
                    arena.bytes -= RECURSIVE_FRAME_BYTES;
                    claim.heap_start -= RECURSIVE_FRAME_BYTES;
                }
                2 => {
                    arena.capacity += 1;
                    arena.bytes += RECURSIVE_FRAME_BYTES;
                    claim.heap_start += RECURSIVE_FRAME_BYTES;
                }
                3 => {
                    arena.metadata_start = claim.frames_start;
                    arena.start = arena.metadata_start + arena.metadata_bytes;
                    claim.heap_start = arena.start + arena.bytes;
                }
                4 => {
                    arena.metadata_start = 40;
                    arena.start = arena.metadata_start + arena.metadata_bytes;
                    claim.heap_start = arena.start + arena.bytes;
                }
                5 => {
                    arena.capacity = u32::MAX;
                }
                6 => {
                    arena.metadata_bytes -= 8;
                    arena.start -= 8;
                    claim.heap_start -= 8;
                }
                _ => {
                    arena.metadata_bytes += 8;
                    arena.start += 8;
                    claim.heap_start += 8;
                }
            }
            assert!(verify(claim, program).is_err(), "hostile arena {change}");
        }
        verify(original.clone(), program).expect("pristine retry after hostile claims");
        source::with_program(true, |foreign| {
            assert!(
                verify(original, foreign).is_err(),
                "a second issuer from the same source bytes is not the retained owner"
            );
        });
    });
}

#[test]
fn immutable_metadata_schema_checks_padding_and_arithmetic_before_storage() {
    assert_eq!(metadata_bytes(0, 0).expect("empty metadata header"), 16);
    assert_eq!(metadata_bytes(1, 1).expect("type/member padding"), 64);
    for (types, members) in
        [(usize::MAX, 0), (0, usize::MAX), (usize::MAX, usize::MAX), (MEMORY_BYTES as usize, 0)]
    {
        assert!(metadata_bytes(types, members).is_err());
    }
}

#[test]
fn an_erased_root_cannot_opt_into_an_unrequired_interpreter_arena() {
    source::with_program(false, |program| {
        let mut claim = derive(program, false).expect("actual erased source memory");
        let source_end = claim.heap_start;
        let (arena, heap) = partition(program, source_end, true, &mut 0)
            .expect("raw storage can be described without granting authority");
        claim.recursive = arena;
        claim.heap_start = heap;
        assert!(verify(claim, program).is_err(), "actual owned-root graph remains mandatory");
    });
}
