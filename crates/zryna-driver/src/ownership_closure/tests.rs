use std::collections::BTreeMap;

use zryna_source::{NormalizedSourcePath, UntrustedSpan};

use super::{
    DiscoveredSource, Import, ImportBinding, MAX_MODULE_PROVIDER_CALLS,
    MAX_MODULE_PROVIDER_SOURCE_BYTES,
    support::{account_provider, imports_match},
};

fn path(value: &str) -> NormalizedSourcePath {
    NormalizedSourcePath::new(value.to_owned()).expect("test path must be normalized")
}

const fn span(file: u32, start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file, start, end }
}

fn import(file: u32) -> Import {
    Import {
        span: span(file, 0, 52),
        specifier: "./dependency.zry".to_owned(),
        specifier_span: span(file, 31, 49),
        bindings: vec![ImportBinding {
            imported: "value".to_owned(),
            local: "local_value".to_owned(),
            imported_span: span(file, 9, 14),
            local_span: span(file, 18, 29),
        }],
    }
}

fn final_imports(file: u32) -> BTreeMap<NormalizedSourcePath, Vec<Import>> {
    BTreeMap::from([(path("main.zry"), vec![import(file)])])
}

fn discovered(file: u32) -> BTreeMap<NormalizedSourcePath, DiscoveredSource> {
    BTreeMap::from([(
        path("main.zry"),
        DiscoveredSource {
            text: "source".to_owned(),
            sha256: [0; 32],
            imports: vec![import(file)],
        },
    )])
}

#[test]
fn final_import_matching_ignores_only_map_local_file_ids() {
    assert!(imports_match(&final_imports(41), &discovered(7)));
}

#[test]
fn final_import_matching_rejects_every_malformed_identity_or_offset() {
    let discovered = discovered(7);
    let mut malformed = Vec::new();

    malformed.push(BTreeMap::new());
    let mut extra_path = final_imports(41);
    extra_path.insert(path("other.zry"), Vec::new());
    malformed.push(extra_path);

    let mut missing_import = final_imports(41);
    missing_import.get_mut(&path("main.zry")).expect("fixture").clear();
    malformed.push(missing_import);

    for mutate in [
        |item: &mut Import| item.span.start += 1,
        |item: &mut Import| item.specifier.push('x'),
        |item: &mut Import| item.specifier_span.end -= 1,
        |item: &mut Import| item.bindings[0].imported.push('x'),
        |item: &mut Import| item.bindings[0].local.push('x'),
        |item: &mut Import| item.bindings[0].imported_span.start += 1,
        |item: &mut Import| item.bindings[0].local_span.end -= 1,
    ] {
        let mut imports = final_imports(41);
        mutate(&mut imports.get_mut(&path("main.zry")).expect("fixture")[0]);
        malformed.push(imports);
    }

    let mut missing_binding = final_imports(41);
    missing_binding.get_mut(&path("main.zry")).expect("fixture")[0].bindings.clear();
    malformed.push(missing_binding);

    for imports in malformed {
        assert!(!imports_match(&imports, &discovered));
    }
}

#[test]
fn ownership_comparison_accounts_physical_pair_calls_and_bytes_at_exact_limit() {
    let (mut calls, mut bytes) = (0, 0);
    let input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 4;
    account_provider(&mut calls, &mut bytes, input, input, 2, false)
        .expect("discovery pair fits with the whole final source pair reserved");
    assert_eq!((calls, bytes), (2, MAX_MODULE_PROVIDER_SOURCE_BYTES / 2));
    account_provider(&mut calls, &mut bytes, input, input, 2, true)
        .expect("final pair fits exactly");
    assert_eq!((calls, bytes), (4, MAX_MODULE_PROVIDER_SOURCE_BYTES));
    let extra = account_provider(&mut calls, &mut bytes, 1, 1, 2, true)
        .expect_err("first extra pair must fail");
    assert_eq!(extra.diagnostics()[0].code(), "ZRYNA-D3303");
    assert_eq!((calls, bytes), (4, MAX_MODULE_PROVIDER_SOURCE_BYTES));
}

#[test]
fn ownership_discovery_reserves_two_final_calls_and_complete_final_source_pair() {
    let (mut calls, mut bytes) = (MAX_MODULE_PROVIDER_CALLS - 4, 0);
    account_provider(&mut calls, &mut bytes, 1, 1, 2, false)
        .expect("exact whole final pair reserve fits");
    assert_eq!((calls, bytes), (MAX_MODULE_PROVIDER_CALLS - 2, 2));
    account_provider(&mut calls, &mut bytes, 1, 1, 2, true)
        .expect("exact final physical calls fit");
    assert_eq!(calls, MAX_MODULE_PROVIDER_CALLS);

    let (mut calls, mut bytes) = (MAX_MODULE_PROVIDER_CALLS - 3, 0);
    account_provider(&mut calls, &mut bytes, 1, 1, 2, false)
        .expect_err("a lone final call cannot authenticate a pair");
    assert_eq!((calls, bytes), (MAX_MODULE_PROVIDER_CALLS - 3, 0));

    let (mut calls, mut bytes) = (0, 0);
    let input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 4 + 1;
    account_provider(&mut calls, &mut bytes, input, input, 2, false)
        .expect_err("unaffordable final source pair rejects before discovery work");
    assert_eq!((calls, bytes), (0, 0));
}

#[test]
fn ownership_provider_zero_units_and_checked_arithmetic_overflow_fail_closed() {
    for (initial_calls, initial_bytes, input, aggregate, units) in [
        (0, 0, 1, 1, 0),
        (0, 0, usize::MAX, usize::MAX, 2),
        (0, 0, 1, usize::MAX, 2),
        (usize::MAX, 0, 1, 1, 1),
        (0, usize::MAX, 1, 1, 1),
    ] {
        let (mut calls, mut bytes) = (initial_calls, initial_bytes);
        let error = account_provider(&mut calls, &mut bytes, input, aggregate, units, false)
            .expect_err("invalid provider work arithmetic must fail closed");
        assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3303");
        assert_eq!((calls, bytes), (initial_calls, initial_bytes));
    }
}

#[test]
fn ownership_native_one_analysis_preserves_original_exact_source_budget() {
    let (mut calls, mut bytes) = (0, 0);
    let input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 2;
    account_provider(&mut calls, &mut bytes, input, input, 1, false)
        .expect("original single-analysis discovery source limit fits");
    assert_eq!((calls, bytes), (1, input));
    account_provider(&mut calls, &mut bytes, input, input, 1, true)
        .expect("original single-analysis final source limit fits");
    assert_eq!((calls, bytes), (2, MAX_MODULE_PROVIDER_SOURCE_BYTES));
}
