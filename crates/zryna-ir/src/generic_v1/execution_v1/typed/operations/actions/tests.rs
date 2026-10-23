//! Pure post-source Clone contract; these private graphs never manufacture `SourceProof`.
use super::{clone_shape, frontier, raw};
use crate::generic_v1::execution_v1::type_world::{Builder, Kind};
fn instruction(ty: u32) -> raw::Instruction {
    raw::Instruction {
        site_span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        result: Some(raw::ValueDefinition { id: 0, ty: raw::ValueType::Stored(ty) }),
        operation: raw::Operation::Clone {
            source: raw::PlaceOrLoan::Place(0),
            outer_failure: None,
            prefix_failure: None,
        },
    }
}
#[test]
fn scalar_clone_exact_results_reject_absent_wrong_type_and_either_illicit_failure() {
    let mut builder = Builder::new();
    let boolean = builder.declare(Kind::Bool, &[], &mut 0).expect("valid checked test fixture");
    let integer = builder.declare(Kind::I32, &[], &mut 0).expect("valid checked test fixture");
    let world = builder.seal(&mut 0).expect("private capability graph only");
    let frontier = frontier::Table::build(&world, &mut 0).expect("valid checked test fixture");
    for (ty, wrong) in [(boolean, integer), (integer, boolean)] {
        let original = instruction(ty);
        assert!(!clone_shape(&frontier, ty, &original).expect("valid checked test fixture"));
        let mut absent = original.clone();
        absent.result = None;
        assert!(clone_shape(&frontier, ty, &absent).is_err());
        let mut other = instruction(wrong);
        assert!(clone_shape(&frontier, ty, &other).is_err());
        other.result.as_mut().expect("valid checked test fixture").ty = raw::ValueType::Unit;
        assert!(clone_shape(&frontier, ty, &other).is_err());
        for (outer_failure, prefix_failure) in
            [(Some(0), None), (None, Some(0)), (Some(0), Some(1))]
        {
            let mut hostile = original.clone();
            hostile.operation = raw::Operation::Clone {
                source: raw::PlaceOrLoan::Place(0),
                outer_failure,
                prefix_failure,
            };
            assert!(clone_shape(&frontier, ty, &hostile).is_err());
        }
        assert!(
            !clone_shape(&frontier, ty, &original).expect("valid checked test fixture"),
            "pristine retry"
        );
    }
}
#[test]
fn enum_and_option_clone_retain_exact_result_identity_and_all_variant_failure_frontier() {
    let mut builder = Builder::new();
    let string = builder.declare(Kind::String, &[], &mut 0).expect("valid checked test fixture");
    let first = builder
        .declare(Kind::Enum { module: 0, declaration: 0, arity: 0 }, &[], &mut 0)
        .expect("valid checked test fixture");
    let other = builder
        .declare(Kind::Enum { module: 0, declaration: 1, arity: 0 }, &[], &mut 0)
        .expect("valid checked test fixture");
    builder.set_members(first, &[None, Some(string)], &mut 0).expect("valid checked test fixture");
    builder.set_members(other, &[None, Some(string)], &mut 0).expect("valid checked test fixture");
    let option =
        builder.declare(Kind::Option, &[first], &mut 0).expect("valid checked test fixture");
    let world = builder.seal(&mut 0).expect("valid checked test fixture");
    let frontier = frontier::Table::build(&world, &mut 0).expect("valid checked test fixture");
    for ty in [first, option] {
        let mut original = instruction(ty);
        original.operation = raw::Operation::Clone {
            source: raw::PlaceOrLoan::Place(0),
            outer_failure: Some(0),
            prefix_failure: Some(1),
        };
        assert!(clone_shape(&frontier, ty, &original).expect("valid checked test fixture"));
        let mut wrong = original.clone();
        wrong.result.as_mut().expect("valid checked test fixture").ty =
            raw::ValueType::Stored(other);
        assert!(
            clone_shape(&frontier, ty, &wrong).is_err(),
            "equal-shaped nominal is a different identity"
        );
        for (outer_failure, prefix_failure) in [(None, None), (Some(0), None), (None, Some(1))] {
            let mut incomplete = original.clone();
            incomplete.operation = raw::Operation::Clone {
                source: raw::PlaceOrLoan::Place(0),
                outer_failure,
                prefix_failure,
            };
            assert!(
                clone_shape(&frontier, ty, &incomplete).is_err(),
                "inactive empty variant cannot erase String failure"
            );
        }
        assert!(clone_shape(&frontier, ty, &original).expect("valid checked test fixture"));
    }
}
