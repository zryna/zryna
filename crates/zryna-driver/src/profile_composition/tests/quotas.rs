use super::*;

fn requirement(metric: usize, row: Row) -> Requirement {
    let (capability, interface) = match metric {
        0 | 1 => (Capability::Clock, "wasi:clocks/monotonic-clock@0.2.12"),
        2 | 3 => (Capability::Environment, "wasi:cli/environment@0.2.12"),
        4 | 5 => (Capability::Filesystem, "wasi:filesystem/types@0.2.12"),
        6 | 7 if row == Row::WitServer => {
            (Capability::Network, "wasi:http/outgoing-handler@0.2.12")
        }
        6 | 7 => (Capability::Network, "wasi:sockets/tcp@0.2.12"),
        8 | 9 => (Capability::Randomness, "wasi:random/random@0.2.12"),
        _ => panic!("unknown metric"),
    };
    Requirement { capability, interface: interface.to_owned() }
}

fn reserve(value: &mut Reservation, metric: usize, amount: u64, prefix: &str) {
    match metric {
        0 => value.subscriptions = amount,
        1 => value.timers = amount,
        2 => {
            value.environment =
                (0..amount).map(|n| (format!("{prefix}{n:03}"), String::new())).collect()
        }
        3 => {
            value.environment.insert(
                prefix.to_owned(),
                "x".repeat(usize::try_from(amount).expect("bounded fixture") - prefix.len()),
            );
        }
        4 => value.preopens = (0..amount).map(|n| format!("{prefix}{n:03}")).collect(),
        5 => value.descriptors = amount,
        6 => value.endpoints = (0..amount).map(|n| format!("{prefix}{n:03}.example:443")).collect(),
        7 => value.operations = amount,
        8 => {
            value.random_per_call = amount;
            value.random_total = amount;
        }
        9 => value.random_total = amount,
        _ => panic!("unknown metric"),
    }
}

fn split(metric: usize, row: Row, total: u64) -> (Input, Claim) {
    let mut input = input(&["A", "B", "C"], &[("A", "B"), ("A", "C")]);
    input.selections = vec![selection(row)];
    let requirement = requirement(metric, row);
    input.selections[0].approved.insert(requirement.clone());
    for (index, amount) in [(1, total / 2), (2, total - total / 2)] {
        input.instances[index].requirements.insert(requirement.clone());
        reserve(
            &mut input.instances[index].reservation,
            metric,
            amount,
            if index == 1 { "b" } else { "c" },
        );
    }
    let mut claim = pure_claim(&input);
    for (id, amount) in [("A", total), ("B", total / 2), ("C", total - total / 2)] {
        let summary = claim.summaries.get_mut(id).expect("summary");
        summary.requirements.insert(requirement.clone());
        summary.quota[metric] = amount;
        match metric {
            2 => summary.quota[3] = amount * 4,
            3 => summary.quota[2] = if id == "A" { 2 } else { 1 },
            _ => (),
        }
    }
    claim.witnesses.insert(requirement, vec!["A".to_owned(), "B".to_owned()]);
    (input, claim)
}

#[test]
fn every_applicable_wit_quota_accepts_exact_and_rejects_first_extra() {
    // Independently fixed #167 expectations; parity checks ensure the loaded authority agrees.
    for (row, limits) in [
        (Row::WitCommand, [64, 64, 128, 65536, 16, 256, 64, 128, 65536, 8388608]),
        (Row::WitServer, [1024, 1024, 0, 0, 0, 0, 128, 1024, 65536, 8388608]),
    ] {
        assert_eq!(policy::Policy::load().expect("registry").limits(row), limits);
        for (metric, maximum) in
            limits.into_iter().enumerate().filter(|(metric, max)| *metric != 8 && *max != 0)
        {
            let (input, claim) = split(metric, row, maximum);
            assert!(
                verify(&input, &claim).is_ok(),
                "row {row:?} metric {metric}: {:?}",
                verify(&input, &claim)
            );
            let (extra, _) = split(metric, row, maximum + 1);
            rejected(&extra, &claim, RESOURCE);
        }
        let mut input = input(&["A", "B"], &[("A", "B")]);
        input.selections = vec![selection(row)];
        let requirement = requirement(8, row);
        input.selections[0].approved.insert(requirement.clone());
        for node in &mut input.instances {
            node.requirements.insert(requirement.clone());
            reserve(&mut node.reservation, 8, 65536, "r");
        }
        let mut claim = pure_claim(&input);
        for (id, total) in [("A", 131072), ("B", 65536)] {
            let summary = claim.summaries.get_mut(id).expect("summary");
            summary.requirements.insert(requirement.clone());
            summary.quota[8] = 65536;
            summary.quota[9] = total;
        }
        claim.witnesses.insert(requirement, vec!["A".to_owned()]);
        assert!(verify(&input, &claim).is_ok());
        input.instances[1].reservation.random_per_call += 1;
        input.instances[1].reservation.random_total += 1;
        rejected(&input, &claim, RESOURCE);
    }
}

#[test]
fn shared_diamond_and_distinct_instances_have_different_quotas() {
    let (mut input, mut claim) = diamond();
    input.instances.push(node("E"));
    input.instances[4].requirements.insert(clock());
    input.instances[4].reservation.subscriptions = 1;
    input.edges = vec![
        ("A".to_owned(), "B".to_owned()),
        ("A".to_owned(), "C".to_owned()),
        ("B".to_owned(), "D".to_owned()),
        ("C".to_owned(), "E".to_owned()),
    ];
    claim.binding = pure_claim(&input).binding;
    rejected(&input, &claim, INVALID);
    claim.summaries.insert("E".to_owned(), claim.summaries["D"].clone());
    claim.summaries.get_mut("A").expect("root").quota[0] = 2;
    assert!(verify(&input, &claim).is_ok());
}

#[test]
fn deduplication_conflicts_overflow_and_secret_safe_recovery() {
    let (mut input, mut claim) = split(3, Row::WitCommand, 8);
    input.instances[1].reservation.environment =
        BTreeMap::from([("key".to_owned(), "é".to_owned())]);
    input.instances[2].reservation.environment = input.instances[1].reservation.environment.clone();
    claim.binding = pure_claim(&input).binding;
    for summary in claim.summaries.values_mut() {
        summary.quota[2] = 1;
        summary.quota[3] = 5;
    }
    assert!(verify(&input, &claim).is_ok());
    input.instances[2]
        .reservation
        .environment
        .insert("key".to_owned(), "private-secret".to_owned());
    let diagnostics = rejected(&input, &claim, RESOURCE);
    assert!(!format!("{diagnostics:?}").contains("private-secret"));
    let (mut input, claim) = split(0, Row::WitCommand, 64);
    input.instances[1].reservation.subscriptions = u64::MAX;
    rejected(&input, &claim, RESOURCE);
    input.instances[1].reservation.subscriptions = 32;
    input.instances[1].requirements.clear();
    rejected(&input, &claim, RESOURCE);
}

#[test]
fn canonical_policy_entries_deduplicate_and_aliases_reject() {
    for metric in [4, 6] {
        let (mut input, mut claim) = split(metric, Row::WitCommand, 2);
        input.instances[2].reservation = input.instances[1].reservation.clone();
        claim.binding = pure_claim(&input).binding;
        claim.summaries.get_mut("A").expect("root").quota[metric] = 1;
        assert!(verify(&input, &claim).is_ok());
    }
    let (valid, claim) = split(6, Row::WitCommand, 2);
    for alias in ["EXAMPLE.com:443", "example.com:0443", "example.com.:443", "example.com:0"] {
        let mut input = valid.clone();
        input.instances[1].reservation.endpoints = BTreeSet::from([alias.to_owned()]);
        rejected(&input, &claim, INVALID);
    }
    let (input, claim) = split(0, Row::WitCommand, 64);
    let result = verify(&input, &claim).expect("initial policy");
    let mut narrowed = input.clone();
    narrowed.selections[0].ceilings[0] = 63;
    rejected(&narrowed, &claim, RESOURCE);
    assert!(result.revalidate(&narrowed, &authorities(&narrowed)).is_err());
    assert!(result.revalidate(&input, &authorities(&input)).is_ok());
}
