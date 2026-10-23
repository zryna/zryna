//! Proposed hostile witness controls; integrate only after actual semantic RED.
use super::rejects_direct_self_member;
use super::{Argument, DirectMember, NominalIdentity, ParameterIdentity};
use crate::generic_v1::raw::NominalKind;

fn owner(module: u32, declaration: u32, kind: NominalKind) -> NominalIdentity {
    NominalIdentity { module, declaration, kind }
}

fn parameter(owner: &NominalIdentity, slot: u32) -> Argument {
    Argument::Parameter(ParameterIdentity { owner: owner.clone(), slot })
}

#[test]
fn direct_self_requires_complete_original_identity_and_exact_ordered_own_binders() {
    let original = owner(7, 11, NominalKind::Struct);
    let one = [parameter(&original, 0)];
    let two = [parameter(&original, 0), parameter(&original, 1)];
    for (parameters, arguments) in [(1, one.as_slice()), (2, two.as_slice())] {
        assert!(rejects_direct_self_member(&DirectMember {
            owner: original.clone(),
            parameters,
            target: original.clone(),
            arguments,
        }));
    }
    let foreign_targets = [
        owner(8, 11, NominalKind::Struct),
        owner(7, 12, NominalKind::Struct),
        owner(7, 11, NominalKind::Enum),
    ];
    for target in foreign_targets {
        assert!(
            !rejects_direct_self_member(&DirectMember {
                owner: original.clone(),
                parameters: 2,
                target,
                arguments: &two,
            }),
            "module, declaration and kind each belong to identity"
        );
    }
    let foreign = owner(8, 11, NominalKind::Struct);
    let invalid = [
        [parameter(&original, 1), parameter(&original, 0)],
        [parameter(&original, 0), parameter(&original, 0)],
        [parameter(&original, 0), parameter(&original, 2)],
        [parameter(&foreign, 0), parameter(&original, 1)],
        [parameter(&original, 0), parameter(&foreign, 1)],
        [Argument::Other, parameter(&original, 1)],
        [parameter(&original, 0), Argument::Other],
    ];
    for arguments in invalid {
        assert!(
            !rejects_direct_self_member(&DirectMember {
                owner: original.clone(),
                parameters: 2,
                target: original.clone(),
                arguments: &arguments,
            }),
            "only direct exact own ordered parameters prove this cycle"
        );
    }
}

#[test]
fn zero_arity_oversized_arity_and_inexact_argument_vectors_prove_nothing() {
    let original = owner(0, 0, NominalKind::Struct);
    let arguments = [parameter(&original, 0), parameter(&original, 1), Argument::Other];
    for (parameters, count) in [(0, 0), (1, 0), (1, 2), (2, 1), (2, 3), (3, 3)] {
        assert!(!rejects_direct_self_member(&DirectMember {
            owner: original.clone(),
            parameters,
            target: original.clone(),
            arguments: &arguments[..count],
        }));
    }
    let empty: [Argument; 0] = [];
    assert!(
        !rejects_direct_self_member(&DirectMember {
            owner: original.clone(),
            parameters: u32::MAX,
            target: original,
            arguments: &empty,
        }),
        "the arity guard must precede argument indexing or traversal"
    );
}
