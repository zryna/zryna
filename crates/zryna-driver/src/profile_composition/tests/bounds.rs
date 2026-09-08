use super::*;

fn star(count: usize) -> Input {
    let mut input = input(&["A"], &[]);
    for index in 1..count {
        let id = format!("n{index:03}");
        input.instances.push(node(&id));
        input.edges.push(("A".to_owned(), id));
    }
    input
}

#[test]
fn instances_and_edges_accept_exact_then_reject_first_extra() {
    let exact = star(256);
    let claim = pure_claim(&exact);
    assert!(verify(&exact, &claim).is_ok());
    rejected(&star(257), &claim, INVALID);
    let mut layered = input(&["A"], &[]);
    for index in 0..128 {
        layered.instances.push(node(&format!("n{index:03}")));
    }
    for left in 0..64 {
        layered.edges.push(("A".to_owned(), format!("n{left:03}")));
        for right in 64..128 {
            if layered.edges.len() < 4096 {
                layered.edges.push((format!("n{left:03}"), format!("n{right:03}")));
            }
        }
    }
    // Keep all root edges while selecting exactly the required cross-layer edge count.
    layered.edges.retain(|(from, _)| from != "A");
    for left in 0..64 {
        layered.edges.push(("A".to_owned(), format!("n{left:03}")));
    }
    layered.edges.truncate(4096);
    assert_eq!(layered.edges.len(), 4096);
    let claim = pure_claim(&layered);
    assert!(verify(&layered, &claim).is_ok());
    layered.edges.push(("A".to_owned(), "n127".to_owned()));
    rejected(&layered, &claim, INVALID);
}

#[test]
fn longest_depth_is_bounded_even_when_every_node_has_a_shortcut() {
    let mut input = input(&["A"], &[]);
    let mut previous = "A".to_owned();
    for index in 1..=32 {
        let id = format!("n{index:02}");
        input.instances.push(node(&id));
        input.edges.push((previous, id.clone()));
        previous = id;
    }
    let claim = pure_claim(&input);
    assert!(verify(&input, &claim).is_ok());
    input.instances.push(node("extra"));
    input.edges.push((previous, "extra".to_owned()));
    input.edges.push(("A".to_owned(), "extra".to_owned()));
    rejected(&input, &claim, INVALID);
}

#[test]
fn identity_accounting_uses_utf8_bytes_and_rejects_first_extra() {
    let mut input = input(&["A"], &[]);
    // The single-instance input contains these identity occurrences, independent of JSON framing.
    let fixed = input.version.len() + input.selections[0].policy_version.len();
    let id = "é".repeat((65_536 - fixed) / 4);
    input.root = id.clone();
    input.instances[0].id = id;
    let claim = pure_claim(&input);
    assert!(verify(&input, &claim).is_ok());
    input.root.push('x');
    input.instances[0].id.push('x');
    rejected(&input, &claim, INVALID);
}

#[test]
fn diagnostics_reserve_exactly_one_terminal_slot_then_recover() {
    let mut input = star(256);
    let claim = pure_claim(&input);
    for node in input.instances.iter_mut().skip(1) {
        node.rows = BTreeSet::from([Row::UniversalNative]);
    }
    let exact = rejected(&input, &claim, UNSUPPORTED);
    assert_eq!(exact.len(), 255);
    assert!(exact.iter().all(|diagnostic| diagnostic.code() == UNSUPPORTED));
    input.instances[0].rows = BTreeSet::from([Row::UniversalNative]);
    let extra = rejected(&input, &claim, UNSUPPORTED);
    assert_eq!(extra.len(), 256);
    assert_eq!(extra[255].code(), EXHAUSTED);
    input.instances.reverse();
    input.edges.reverse();
    assert_eq!(extra, verify(&input, &claim).expect_err("permuted errors"));
}
