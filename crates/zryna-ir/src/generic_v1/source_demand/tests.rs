//! A root token is tied to the exact borrowed inventory, including identical cloned claims.
use super::*;
fn program() -> raw::Program {
    raw::Program {
        modules: vec![],
        declarations: vec![],
        type_keys: vec![vec![0], vec![1], vec![2]],
        universe: [0; 32],
        linear32: [0; 32],
        linux_x86_64: [0; 32],
        functions: vec![],
    }
}
#[test]
fn original_root_marks_cannot_be_replayed_against_another_claim_inventory() {
    let original = program();
    // Constructor privacy is tested independently of original replay in this unit control.
    let roots = StoredRoots { program: &original, indices: vec![0, 2] };
    assert_eq!(roots.indices_for(&original).expect("exact borrowed inventory"), [0, 2]);
    let mut foreign = original.clone();
    assert!(
        roots.indices_for(&foreign).is_err(),
        "even identical raw claims have foreign identity"
    );
    foreign.type_keys.swap(0, 2);
    assert!(roots.indices_for(&foreign).is_err(), "reordering cannot reinterpret root indices");
    assert_eq!(roots.indices_for(&original).expect("pristine token after attacks"), [0, 2]);
}
