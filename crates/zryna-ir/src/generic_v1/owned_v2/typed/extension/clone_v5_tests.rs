//! Exact extension typing is checked independently of source Clone authority.
use super::super::super::plan::struct_v4::layouts;
use super::*;
#[test]
fn clone_struct_requires_v5_exact_stored_struct_operand_and_preserves_nominal_identity() {
    let layouts = layouts();
    let views = layouts.types().collect::<Vec<_>>();
    let operation = raw::Operation::CloneStruct { value: 17 };
    let operand = |id| {
        assert_eq!(id, 17);
        Ok(graph::Type::Stored(3))
    };
    assert_eq!(
        check(&operation, &views, &operand, Admission::StructCloneV5).expect("same Struct type"),
        graph::Type::Stored(3)
    );
    for admission in [Admission::LegacyOwned, Admission::StructV4] {
        assert!(check(&operation, &views, &operand, admission).is_err());
    }
    for ty in [
        graph::Type::Unit,
        graph::Type::Stored(2),
        graph::Type::Stored(u32::MAX),
        graph::Type::Borrow { referent: 3, exclusive: false },
    ] {
        assert!(check(&operation, &views, &|_| Ok(ty), Admission::StructCloneV5).is_err());
    }
}
