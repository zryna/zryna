use super::{Command, Input};

#[test]
fn command_vector_clone_append_preserves_adjacent_owner() {
    let mut command = Command::new("clone-vector-growth", Input::Denied);
    assert_eq!(
        command.run.call(&mut command.store, ()).expect("ordinary cloned vector append"),
        0,
        "appending to the clone must preserve the separately owned neighboring vector"
    );
    assert_eq!(command.store.data().calls, 0, "pure command grants no host authority");
    assert_eq!(command.canonical_state(1), 0, "canonical allocations are drained");
    assert_eq!(command.canonical_state(3), 0, "cleanup succeeds");
}
