//! Genuine authenticated source reaches scope storage and the independent ownership kernel.
use crate::generic_v1::execution_v1::{budget, plan, raw, typed};
mod fixture;

#[test]
fn genuine_sparse_match_table_prepays_both_scans_before_any_row_visit() {
    fixture::program("native", |p, source, _, _| {
        let (function, _) = fixture::match_site(p);
        let sites = &source.matches.0[function];
        assert_eq!(sites.iter().flatten().count(), 1);
        assert!(sites.len() > 2, "source-created sparse CFG contains non-match rows");
        let scans = 2 * sites.len();
        super::SCANNED.with(|count| count.set(0));
        let mut short = budget::MAX_OWNER_UNITS - scans;
        assert!(source.match_scopes(function, &p.functions[function], &mut short).is_err());
        assert_eq!(
            super::SCANNED.with(std::cell::Cell::get),
            0,
            "one unit short of both scans must reject before inspecting any sparse row"
        );
        let scopes = source
            .match_scopes(function, &p.functions[function], &mut 0)
            .expect("pristine source scope retry");
        assert_eq!(scopes.len(), 2, "two genuine original arms");
        assert_eq!(
            super::SCANNED.with(std::cell::Cell::get),
            scans,
            "both real traversals count holes, rather than only matched arms"
        );
    });
}

#[test]
fn raw_cfg_and_loan_metadata_cannot_replace_original_match_scope_authority() {
    fixture::program("native", |p, source, linear, linux| {
        let mut checked = typed::check(p, source, linear, linux, &mut 0)
            .expect("genuine original scope authority");
        plan::derive(p, &checked, &mut 0).expect("own join with its original scope");
        let (function, _) = fixture::match_site(p);
        assert!(!checked.functions[function].match_scopes.is_empty());
        checked.functions[function].match_scopes.clear();
        assert!(
            plan::derive(p, &checked, &mut 0).is_err(),
            "valid raw graph and origin metadata alone cannot grant lexical edge permission"
        );
    });
}

#[test]
fn source_scope_and_state_exact_extra_overflow_and_retry_are_atomic() {
    for provider in ["native", "typescript"] {
        fixture::program(provider, |p, source, linear, linux| {
            let mut cost = 0;
            let checked = typed::check(p, source, linear, linux, &mut cost)
                .expect("actual source-authenticated match ranges");
            let limit = budget::MAX_OWNER_UNITS;
            let mut exact = limit - cost;
            typed::check(p, source, linear, linux, &mut exact).expect("exact typed credit");
            assert_eq!(exact, limit);
            for before in [limit - cost + 1, usize::MAX] {
                let mut spent = before;
                assert!(typed::check(p, source, linear, linux, &mut spent).is_err());
                assert_eq!(spent, before, "failed typed scope cannot publish partial credit");
            }
            let mut replay_cost = 0;
            plan::derive(p, &checked, &mut replay_cost).expect("genuine own-join state");
            let mut exact = limit - replay_cost;
            plan::derive(p, &checked, &mut exact).expect("exact indexed ancestry credit");
            assert_eq!(exact, limit);
            for before in [limit - replay_cost + 1, usize::MAX] {
                let mut spent = before;
                assert!(plan::derive(p, &checked, &mut spent).is_err());
                assert_eq!(spent, before, "failed owner scope cannot publish partial state");
            }
            let mut retry = 0;
            typed::check(p, source, linear, linux, &mut retry).expect("pristine typed retry");
            assert_eq!(retry, cost);
            let mut retry = 0;
            plan::derive(p, &checked, &mut retry).expect("pristine ownership retry");
            assert_eq!(retry, replay_cost);
        });
    }
}

#[test]
fn changed_selected_targets_cannot_reuse_original_arm_scope_permissions() {
    fixture::program("native", |p, source, linear, linux| {
        let mut wrong = p.clone();
        let (f, b) = fixture::match_site(&wrong);
        let raw::Terminator::ClosedEnumMatch { arms, .. } =
            &mut wrong.functions[f].blocks[b].terminator
        else {
            panic!("actual borrowed match")
        };
        let left = arms[0].edge.target;
        arms[0].edge.target = arms[1].edge.target;
        arms[1].edge.target = left;
        assert!(
            typed::check(&wrong, source, linear, linux, &mut 0).is_err(),
            "even a dense reachable CFG cannot install another source arm's scope"
        );
        typed::check(p, source, linear, linux, &mut 0).expect("original source retry");
    });
}

#[test]
fn selected_child_cannot_leak_to_result_join_or_cross_into_sibling_arm() {
    fixture::program("native", |p, source, linear, linux| {
        let checked = typed::check(p, source, linear, linux, &mut 0).expect("actual typed source");
        let (f, b) = fixture::match_site(p);
        let raw::Terminator::ClosedEnumMatch { arms, .. } = &p.functions[f].blocks[b].terminator
        else {
            panic!("actual match")
        };
        let bound = arms.iter().find(|a| a.binding.is_some()).expect("actual selected child");
        let raw::MatchBinding::Loan(child) = bound.binding.as_ref().expect("binding") else {
            panic!("borrowed payload")
        };
        let arm = bound.edge.target as usize;
        let mut leaked = p.clone();
        leaked.functions[f].blocks[arm].instructions.retain(
            |i| !matches!(i.operation, raw::Operation::EndLoan { loan } if loan == child.id),
        );
        assert!(
            plan::derive(&leaked, &checked, &mut 0).is_err(),
            "selected child must end before its own result join"
        );
        let mut sibling = p.clone();
        let raw::Terminator::Jump(edge) = &mut sibling.functions[f].blocks[arm].terminator else {
            panic!("direct original arm ends at result join")
        };
        edge.target =
            arms.iter().find(|a| a.ordinal != bound.ordinal).expect("other actual arm").edge.target;
        assert!(
            plan::derive(&sibling, &checked, &mut 0).is_err(),
            "same parent cannot authorize a direct sibling-arm jump"
        );
        plan::derive(p, &checked, &mut 0).expect("original state retry after both hostile claims");
    });
}
