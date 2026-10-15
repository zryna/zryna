//! Exact resolved-closure limits with genuine independent declaration/source issuers.
use super::*;
mod capture;

fn graph(issuers: Vec<NativeAuthority>) -> (Input, Authorities, NativeAuthorities) {
    let (mut input, mut pure, _) = fixture();
    input.instances.retain(|instance| instance.id == "A");
    input.edges.clear();
    pure.instances.retain(|id, _| id == "A");
    let mut native = NativeAuthorities::new();
    for (index, issuer) in issuers.into_iter().enumerate() {
        let id = format!("C{index:03}");
        let mut node = input.instances[0].clone();
        node.id = id.clone();
        input.instances.push(node);
        input.edges.push(("A".into(), id.clone()));
        native.insert(id, issuer);
    }
    (input, pure, native)
}
fn rejected_before_emission(
    exact: &(Input, Authorities, NativeAuthorities),
    changed: &(Input, Authorities, NativeAuthorities),
    code: &str,
) {
    let seal = verify(&exact.0, &exact.1, &exact.2).expect("exact aggregate admitted");
    rejects_emit(&seal, &changed.0, &changed.1, &changed.2, code);
    verify(&exact.0, &exact.1, &exact.2).expect("pristine aggregate recovers");
    seal.revalidate(&exact.0, &exact.1, &exact.2).expect("retained original issuer recovers");
}

#[test]
fn resolved_native_closure_accepts_sixteen_libraries_and_rejects_seventeenth_before_emission() {
    let exact = graph((0..16).map(|index| capture::imports(index, 1, index)).collect());
    let extra = graph((0..17).map(|index| capture::imports(index, 1, index)).collect());
    assert_eq!(
        exact.2.values().map(|issuer| issuer.declarations.library_ids().len()).sum::<usize>(),
        16
    );
    rejected_before_emission(&exact, &extra, "ZRYNA-C4107");
    let joint = graph((0..16).map(|index| capture::imports(index, 16, index)).collect());
    verify(&joint.0, &joint.1, &joint.2)
        .expect("sixteen libraries and256 unique operations together");
}

#[test]
fn resolved_native_closure_counts_imports_and_exports_at256_and_rejects257_before_emission() {
    let authorities = |extra| {
        (0..16)
            .map(|index| {
                if index < 8 {
                    capture::imports(index, 16, index)
                } else {
                    capture::exports(index, 16 + usize::from(extra && index == 15), index)
                }
            })
            .collect()
    };
    let exact = graph(authorities(false));
    let extra = graph(authorities(true));
    assert_eq!(
        exact.2.values().map(|issuer| issuer.declarations.operation_count()).sum::<usize>(),
        256
    );
    assert_eq!(
        extra.2.values().map(|issuer| issuer.declarations.operation_count()).sum::<usize>(),
        257
    );
    rejected_before_emission(&exact, &extra, "ZRYNA-C4107");
}

#[test]
fn resolved_native_closure_deduplicates_matching_imports_from_different_source_bindings() {
    let data = graph((0..17).map(|tag| capture::imports(0, 16, tag)).collect());
    assert_eq!(
        data.2.values().map(|issuer| issuer.declarations.operation_count()).sum::<usize>(),
        272
    );
    let first = &data.2["C000"];
    let other = &data.2["C001"];
    assert_ne!(first.sources.identity(), other.sources.identity());
    assert_ne!(first.declarations.declaration_bytes(), other.declarations.declaration_bytes());
    assert_eq!(
        first.declarations.policy_bytes("library000@0"),
        other.declarations.policy_bytes("library000@0")
    );
    verify(&data.0, &data.1, &data.2).expect("one library and sixteen unique import contracts");
}

#[test]
fn resolved_native_closure_deduplicates_same_export_issuer_but_rejects_rebuilt_implementation() {
    let original = capture::exports(0, 16, 0);
    let exact = graph(vec![original.clone(); 17]);
    verify(&exact.0, &exact.1, &exact.2).expect("same original export issuer counted once");
    let changed = graph(vec![original, capture::exports(0, 16, 0)]);
    assert_ne!(changed.2["C000"].sources.identity(), changed.2["C001"].sources.identity());
    rejected_before_emission(&exact, &changed, "ZRYNA-C4102");
}

#[test]
fn resolved_native_closure_rejects_conflicting_library_material_and_symbol_aliases() {
    let names = vec!["shared".into()];
    let first = capture::make(0, names.clone(), 0, false, "");
    let exact = graph(vec![first.clone()]);
    let conflict = graph(vec![
        first.clone(),
        capture::make(0, names.clone(), 1, false, "/* changed header */\n"),
    ]);
    rejected_before_emission(&exact, &conflict, "ZRYNA-C4102");
    for name in ["shared", "Shared"] {
        let collision =
            graph(vec![first.clone(), capture::make(1, vec![name.into()], 1, false, "")]);
        rejected_before_emission(&exact, &collision, "ZRYNA-C4102");
    }
}

#[test]
fn resolved_native_closure_rejects_export_symbols_aliased_across_distinct_modules() {
    let first = capture::exports(0, 1, 0);
    let exact = graph(vec![first.clone()]);
    for name in ["export_000_000", "Export_000_000"] {
        let changed = graph(vec![first.clone(), capture::make(1, vec![name.into()], 1, true, "")]);
        rejected_before_emission(&exact, &changed, "ZRYNA-C4102");
    }
}
