//! Genuine-source activation/erasure and exact owning graph admission costs.
#[path = "../../../../zryna-backend-native/tests/recursive_arena/source.rs"]
mod source;
use super::{derive, graph::Graph};
use crate::generic_execution_v1::{MAX_CODEGEN_UNITS, lower, shapes};
use zryna_layout::{StorageTarget, TypeCategory};

#[test]
fn actual_source_need_is_opaque_and_erased_weak_array_paths_have_no_graph_edge() {
    for (case, expected) in [
        ("drop-only", true),
        ("clone", true),
        ("controlled-cleanup", true),
        ("erased", false),
        ("counts", false),
    ] {
        source::with_program(case, |program| {
            let mir = lower(program).expect("actual same-owner MIR");
            assert!(std::ptr::eq(mir.program(), program));
            assert_eq!(mir.requires_recursive_arena(), expected, "{case}");
            let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut 0)
                .expect("independent genuine physical shapes");
            let graph = Graph::derive(&types, &mut 0).expect("actual graph");
            let mut erased = 0;
            for (id, shape) in types.iter().enumerate() {
                if shape.view.category() == TypeCategory::Weak
                    || shape.view.array_length() == Some(0)
                {
                    assert!(graph.children(id).expect("actual row").is_empty());
                    erased += 1;
                }
            }
            if !expected {
                assert!(erased > 0, "each negative witnesses a real erased edge");
            }
        });
    }
}
#[test]
fn genuine_recursive_graph_exact_extra_overflow_and_pristine_retry() {
    for case in ["drop-only", "clone", "controlled-cleanup", "erased", "counts"] {
        source::with_program(case, |program| {
            let types = shapes::derive(program.layouts(StorageTarget::LinuxX8664V1), &mut 0)
                .expect("genuine same-seal layouts");
            let mut used = 0;
            let expected = derive(program, &types, &mut used).expect("actual graph baseline");
            assert!(used > 0);
            let mut exact = MAX_CODEGEN_UNITS - used;
            assert_eq!(derive(program, &types, &mut exact).expect("exact"), expected);
            assert_eq!(exact, MAX_CODEGEN_UNITS);
            let mut extra = MAX_CODEGEN_UNITS - used + 1;
            assert_eq!(
                derive(program, &types, &mut extra).expect_err("first extra").code,
                "ZRYNA-N7001"
            );
            assert!(extra <= MAX_CODEGEN_UNITS);
            let mut overflow = u64::MAX;
            assert_eq!(
                derive(program, &types, &mut overflow).expect_err("overflow").code,
                "ZRYNA-N7001"
            );
            assert_eq!(overflow, u64::MAX);
            assert_eq!(derive(program, &types, &mut 0).expect("pristine retry"), expected);
        });
    }
}

/// Reuse the single loaded genuine source issuer in sibling resource controls.
pub(super) fn with_program(
    case: &str,
    inspect: impl FnOnce(&zryna_ir::generic_v1::execution_v1::VerifiedExecution<'_>),
) {
    source::with_program(case, inspect);
}
