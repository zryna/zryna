//! Provider accounting failures must precede every analysis.

use super::{
    SourceMap, TemporaryWorkspace, discover_module_closure, entry, rejection_code, syntax_v3,
};
use crate::{MAX_MODULE_EDGE_MANIFEST_BYTES, module_closure::account_edge_manifest_budget};

struct UnaffordableProviderWork {
    units: usize,
    analyses: std::sync::atomic::AtomicUsize,
}

impl zryna_frontend::VerifiedFrontendProviderV3 for UnaffordableProviderWork {
    fn analysis_work_units(&self) -> usize {
        self.units
    }

    fn analyze_verified_v3(
        &self,
        _sources: &SourceMap,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
        self.analyses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        panic!("unaffordable provider work must be rejected before analysis")
    }
}

impl zryna_frontend::VerifiedFrontendProviderV4 for UnaffordableProviderWork {
    fn analysis_work_units(&self) -> usize {
        self.units
    }

    fn analyze_verified_v4(
        &self,
        _sources: &SourceMap,
    ) -> Result<zryna_frontend::syntax_v4::ProjectSyntaxSnapshot, zryna_frontend::WorkerError> {
        self.analyses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        panic!("unaffordable ownership provider work must be rejected before analysis")
    }
}

#[test]
fn invalid_or_unaffordable_provider_work_never_starts_m2_analysis() {
    for units in [0, crate::MAX_MODULE_PROVIDER_CALLS, usize::MAX] {
        let workspace = TemporaryWorkspace::new("unaffordable-m2-provider");
        workspace.write("main.zry", "");
        let provider =
            UnaffordableProviderWork { units, analyses: std::sync::atomic::AtomicUsize::new(0) };
        let error = discover_module_closure(&workspace.root(), entry(), &provider)
            .expect_err("invalid or unaffordable work must fail before provider invocation");
        assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3201");
        assert_eq!(provider.analyses.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}

#[test]
fn invalid_or_unaffordable_provider_work_never_starts_m3_analysis() {
    for units in [0, crate::MAX_MODULE_PROVIDER_CALLS, usize::MAX] {
        let workspace = TemporaryWorkspace::new("unaffordable-m3-provider");
        workspace.write("main.zry", "");
        let provider =
            UnaffordableProviderWork { units, analyses: std::sync::atomic::AtomicUsize::new(0) };
        let error = crate::ownership_closure::discover_ownership_module_closure(
            &workspace.root(),
            entry(),
            &provider,
        )
        .expect_err("invalid or unaffordable ownership work must fail before provider invocation");
        assert_eq!(error.diagnostics()[0].code(), "ZRYNA-D3303");
        assert_eq!(provider.analyses.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}
#[test]
fn canonical_edge_manifest_budget_accepts_exact_limit_and_rejects_plus_one() {
    let mut exact = 0;
    account_edge_manifest_budget(&mut exact, MAX_MODULE_EDGE_MANIFEST_BYTES)
        .expect("the exact edge-manifest budget must be accepted");
    assert_eq!(exact, MAX_MODULE_EDGE_MANIFEST_BYTES);

    let mut plus_one = 0;
    let error = account_edge_manifest_budget(&mut plus_one, MAX_MODULE_EDGE_MANIFEST_BYTES + 1)
        .expect_err("one byte beyond the edge-manifest budget must be rejected");
    assert_eq!(rejection_code(error), "ZRYNA-D3201");
}
