use super::graph_contains_handle;

#[test]
fn handle_reachability_is_linear_for_deep_diamonds_and_cycles() {
    const DEPTH: usize = 65_536;
    let mut visits = 0usize;
    let found = graph_contains_handle(0usize, |node| {
        visits += 1;
        let children = (node + 1 < DEPTH).then(|| vec![node + 1, node + 1]).unwrap_or_default();
        Some((false, children))
    });
    assert!(!found);
    assert_eq!(visits, DEPTH, "shared diamond nodes are described once");

    visits = 0;
    let found = graph_contains_handle(0usize, |node| {
        visits += 1;
        Some((node == 2, vec![(node + 1) % 3]))
    });
    assert!(found);
    assert_eq!(visits, 3, "cycle terminates at the reachable handle");
}
