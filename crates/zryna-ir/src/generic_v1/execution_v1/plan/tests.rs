//! Exact raw-plan comparison controls; no source/owner/executable proof is manufactured.
use super::{compare, raw};

fn plan(actions: Vec<raw::CleanupAction>) -> raw::CleanupPlan {
    raw::CleanupPlan {
        id: 0,
        span: zryna_source::UntrustedSpan { file: 0, start: 0, end: 0 },
        actions,
    }
}

#[test]
fn cleanup_comparison_rejects_omitted_reordered_or_fabricated_prefix_actions() {
    let expected = [raw::CleanupAction::DropPlace(2), raw::CleanupAction::DropPlace(0)];
    compare(&plan(expected.to_vec()), &expected, &mut 0).expect("exact independent actions");
    assert!(compare(&plan(vec![raw::CleanupAction::DropPlace(0)]), &expected, &mut 0).is_err());
    let reversed = [raw::CleanupAction::DropPlace(0), raw::CleanupAction::DropPlace(2)];
    assert!(compare(&plan(reversed.to_vec()), &expected, &mut 0).is_err());
    let forged = raw::CleanupAction::DropInitializedPrefix {
        destination: 7,
        preparation_site: raw::PreparationSite { function: 0, block: 0, instruction: 1 },
    };
    assert!(compare(&plan(vec![forged]), &expected, &mut 0).is_err());
}

#[test]
fn cleanup_comparison_prepays_and_rolls_back_failed_budget() {
    let expected = [raw::CleanupAction::EndLoan(0)];
    let mut spent = usize::MAX;
    assert!(compare(&plan(expected.to_vec()), &expected, &mut spent).is_err());
    assert_eq!(spent, usize::MAX);
    compare(&plan(expected.to_vec()), &expected, &mut 0).expect("pristine retry");
}
