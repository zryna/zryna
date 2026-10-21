//! Resolver, edge, and physical provider accounting boundaries.

use super::provider_budget::{
    ProviderCallPhase, account_provider_bytes, account_provider_call, account_provider_call_units,
    account_provider_work,
};
use std::collections::HashSet;

use zryna_source::{NormalizedSourcePath, resolve_explicit_zry_import};

use super::{
    EdgeIdentity, MAX_MODULE_IMPORT_EDGES, MAX_MODULE_PROVIDER_CALLS,
    MAX_MODULE_PROVIDER_SOURCE_BYTES, ResolvedEdge, register_edge,
};

fn path(value: &str) -> NormalizedSourcePath {
    NormalizedSourcePath::new(value).expect("test path must be normalized")
}

#[test]
fn resolver_accepts_only_explicit_normalized_in_root_zry_paths() {
    let importer = path("src/nested/main.zry");
    for (specifier, expected) in [
        ("./dep.zry", "src/nested/dep.zry"),
        ("./child/../dep.zry", "src/nested/dep.zry"),
        ("../shared.zry", "src/shared.zry"),
        ("../../root.zry", "root.zry"),
    ] {
        assert_eq!(
            resolve_explicit_zry_import(&importer, specifier)
                .expect("specifier must resolve")
                .as_str(),
            expected
        );
    }

    for rejected in [
        "",
        "dep.zry",
        "/dep.zry",
        "C:/dep.zry",
        "//server/dep.zry",
        "https://example.invalid/dep.zry",
        "./dep",
        "./dep.ZRY",
        "./dep.zry?query",
        "./dep.zry#fragment",
        ".\\dep.zry",
        "../../../escape.zry",
    ] {
        assert!(
            resolve_explicit_zry_import(&importer, rejected).is_err(),
            "specifier must be rejected: {rejected}"
        );
    }
}

#[test]
fn edge_registry_accepts_exact_limit_and_rejects_duplicate_or_first_extra_edge() {
    let importer = path("main.zry");
    let target = path("dep.zry");
    let mut identities = HashSet::new();
    let mut edges = Vec::<ResolvedEdge>::new();
    for index in 0..MAX_MODULE_IMPORT_EDGES {
        register_edge(
            &mut identities,
            &mut edges,
            EdgeIdentity {
                importer: importer.clone(),
                specifier: "./dep.zry".to_owned(),
                imported: "value".to_owned(),
                local: format!("local{index}"),
            },
            target.clone(),
        )
        .expect("every edge through the exact limit must succeed");
    }
    assert_eq!(identities.len(), MAX_MODULE_IMPORT_EDGES);
    assert_eq!(edges.len(), MAX_MODULE_IMPORT_EDGES);

    let duplicate = register_edge(
        &mut identities,
        &mut edges,
        EdgeIdentity {
            importer: importer.clone(),
            specifier: "./dep.zry".to_owned(),
            imported: "value".to_owned(),
            local: "local0".to_owned(),
        },
        target.clone(),
    )
    .expect_err("a duplicate remains a duplicate at the exact limit");
    assert_eq!(duplicate.diagnostics()[0].code(), "ZRYNA-D3006");

    let extra = register_edge(
        &mut identities,
        &mut edges,
        EdgeIdentity {
            importer,
            specifier: "./dep.zry".to_owned(),
            imported: "value".to_owned(),
            local: "firstExtra".to_owned(),
        },
        target,
    )
    .expect_err("the first extra edge must fail");
    assert_eq!(extra.diagnostics()[0].code(), "ZRYNA-D3201");
}

#[test]
fn provider_accounting_accepts_exact_call_and_byte_limits_and_rejects_first_extra() {
    let mut calls = 0;
    for _ in 0..MAX_MODULE_PROVIDER_CALLS - 1 {
        account_provider_call(&mut calls, ProviderCallPhase::Discovery)
            .expect("every discovery call that reserves the final call must succeed");
    }
    account_provider_call(&mut calls, ProviderCallPhase::Final)
        .expect("the exact final provider call must succeed");
    assert_eq!(calls, MAX_MODULE_PROVIDER_CALLS);
    let extra_call = account_provider_call(&mut calls, ProviderCallPhase::Final)
        .expect_err("the first extra provider call must fail");
    assert_eq!(extra_call.diagnostics()[0].code(), "ZRYNA-D3201");

    let mut no_final_call = MAX_MODULE_PROVIDER_CALLS - 1;
    let reserved_call = account_provider_call(&mut no_final_call, ProviderCallPhase::Discovery)
        .expect_err("discovery must reserve exactly one final provider call");
    assert_eq!(reserved_call.diagnostics()[0].code(), "ZRYNA-D3201");

    let mut bytes = 0;
    account_provider_bytes(&mut bytes, MAX_MODULE_PROVIDER_SOURCE_BYTES)
        .expect("the exact cumulative provider-byte limit must succeed");
    assert_eq!(bytes, MAX_MODULE_PROVIDER_SOURCE_BYTES);
    let extra_byte = account_provider_bytes(&mut bytes, 1)
        .expect_err("the first extra cumulative provider byte must fail");
    assert_eq!(extra_byte.diagnostics()[0].code(), "ZRYNA-D3201");
}

#[test]
fn comparison_provider_accounting_charges_both_analyses_and_reserves_final_pair() {
    let (mut calls, mut bytes) = (0, 0);
    account_provider_work(&mut calls, &mut bytes, 10, 10, 2, ProviderCallPhase::Discovery)
        .expect("both physical discovery analyses and the full final pair fit");
    assert_eq!((calls, bytes), (2, 20));
    account_provider_work(&mut calls, &mut bytes, 10, 10, 2, ProviderCallPhase::Final)
        .expect("both final analyses fit");
    assert_eq!((calls, bytes), (4, 40));
}

#[test]
fn comparison_provider_call_budget_rejects_first_extra_pair_and_preserves_final_reserve() {
    let mut calls = MAX_MODULE_PROVIDER_CALLS - 2;
    account_provider_call_units(&mut calls, ProviderCallPhase::Final, 2)
        .expect("the exact final physical pair fits");
    assert_eq!(calls, MAX_MODULE_PROVIDER_CALLS);
    let extra = account_provider_call_units(&mut calls, ProviderCallPhase::Final, 2)
        .expect_err("the first extra physical pair must fail");
    assert_eq!(extra.diagnostics()[0].code(), "ZRYNA-D3201");

    let mut reserved = MAX_MODULE_PROVIDER_CALLS - 4;
    account_provider_call_units(&mut reserved, ProviderCallPhase::Discovery, 2)
        .expect("discovery can consume two calls while reserving the final two");
    assert_eq!(reserved, MAX_MODULE_PROVIDER_CALLS - 2);
    let mut insufficient = MAX_MODULE_PROVIDER_CALLS - 3;
    let error = account_provider_call_units(&mut insufficient, ProviderCallPhase::Discovery, 2)
        .expect_err("one remaining slot cannot authenticate the final pair");
    assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3201");
}

#[test]
fn comparison_provider_source_bytes_accept_exact_budget_and_reject_first_extra_pair() {
    let (mut calls, mut bytes) = (0, 0);
    let input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 4;
    account_provider_work(&mut calls, &mut bytes, input, input, 2, ProviderCallPhase::Discovery)
        .expect("discovery and reserved final pair exactly fit the byte budget");
    assert_eq!((calls, bytes), (2, MAX_MODULE_PROVIDER_SOURCE_BYTES / 2));
    account_provider_work(&mut calls, &mut bytes, input, input, 2, ProviderCallPhase::Final)
        .expect("the complete final pair consumes exactly the remaining bytes");
    assert_eq!((calls, bytes), (4, MAX_MODULE_PROVIDER_SOURCE_BYTES));
    let error = account_provider_work(&mut calls, &mut bytes, 1, 1, 2, ProviderCallPhase::Final)
        .expect_err("the first extra pair of source bytes must fail");
    assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3201");
    assert_eq!((calls, bytes), (4, MAX_MODULE_PROVIDER_SOURCE_BYTES));
}

#[test]
fn provider_work_units_zero_and_arithmetic_overflow_fail_without_charging() {
    for (initial_calls, initial_bytes, input, aggregate, units) in [
        (0, 0, 1, 1, 0),
        (0, 0, usize::MAX, usize::MAX, 2),
        (0, 0, 1, usize::MAX, 2),
        (usize::MAX, 0, 1, 1, 1),
        (0, usize::MAX, 1, 1, 1),
    ] {
        let (mut calls, mut bytes) = (initial_calls, initial_bytes);
        let error = account_provider_work(
            &mut calls,
            &mut bytes,
            input,
            aggregate,
            units,
            ProviderCallPhase::Discovery,
        )
        .expect_err("invalid physical work accounting must fail closed");
        assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3201");
        assert_eq!((calls, bytes), (initial_calls, initial_bytes));
    }
}

#[test]
fn comparison_discovery_rejects_unaffordable_final_pair_and_native_unit_one_is_unchanged() {
    let input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 4 + 1;
    let (mut calls, mut bytes) = (0, 0);
    account_provider_work(&mut calls, &mut bytes, input, input, 2, ProviderCallPhase::Discovery)
        .expect_err("discovery must not run when its complete final pair cannot fit");
    assert_eq!((calls, bytes), (0, 0));

    let native_input = MAX_MODULE_PROVIDER_SOURCE_BYTES / 2;
    account_provider_work(
        &mut calls,
        &mut bytes,
        native_input,
        native_input,
        1,
        ProviderCallPhase::Discovery,
    )
    .expect("ordinary one-analysis discovery retains its exact original source limit");
    account_provider_work(
        &mut calls,
        &mut bytes,
        native_input,
        native_input,
        1,
        ProviderCallPhase::Final,
    )
    .expect("ordinary final analysis retains the original exact cumulative limit");
    assert_eq!((calls, bytes), (2, MAX_MODULE_PROVIDER_SOURCE_BYTES));
}
