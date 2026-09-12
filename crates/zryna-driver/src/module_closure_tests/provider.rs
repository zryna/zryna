//! Shared in-process provider fixture for source-closure tests.

use std::sync::{Arc, Mutex};
use zryna_diagnostics::Severity;
use zryna_frontend::{VerifiedFrontendProviderV3, WorkerError, syntax_v3};
use zryna_source::SourceMap;

use super::raw_snapshot;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ProviderCall {
    pub(super) paths: Vec<String>,
    pub(super) source_bytes: usize,
}

type CallHook = Arc<dyn Fn(usize) + Send + Sync>;

pub(super) struct FixtureProvider {
    calls: Mutex<Vec<ProviderCall>>,
    omit_imports_on_call: Option<usize>,
    error_on_call: Option<usize>,
    hook: Option<CallHook>,
}

impl FixtureProvider {
    pub(super) fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: None,
            hook: None,
        }
    }

    pub(super) fn omitting_imports_on(call: usize) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: Some(call),
            error_on_call: None,
            hook: None,
        }
    }

    pub(super) fn reporting_error_on(call: usize) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: Some(call),
            hook: None,
        }
    }

    #[cfg(unix)]
    pub(super) fn with_hook(call: usize, hook: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: None,
            hook: Some(Arc::new(move |current| {
                if current == call {
                    hook();
                }
            })),
        }
    }

    #[cfg(unix)]
    pub(super) fn reporting_error_with_hook(
        call: usize,
        hook: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            omit_imports_on_call: None,
            error_on_call: Some(call),
            hook: Some(Arc::new(move |current| {
                if current == call {
                    hook();
                }
            })),
        }
    }

    pub(super) fn calls(&self) -> Vec<ProviderCall> {
        self.calls.lock().expect("provider call log must remain available").clone()
    }
}

impl VerifiedFrontendProviderV3 for FixtureProvider {
    fn analyze_verified_v3(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v3::ProjectSyntaxSnapshot, WorkerError> {
        let mut paths = Vec::with_capacity(sources.len());
        let mut source_bytes = 0_usize;
        for index in 0..sources.len() {
            let raw = u32::try_from(index).expect("bounded fixture file id");
            let id = sources.verify_file_id(raw).expect("fixture file id must be valid");
            let source = sources.source(id).expect("fixture source must exist");
            paths.push(source.path().as_str().to_owned());
            source_bytes = source_bytes
                .checked_add(source.text().len())
                .expect("fixture source accounting must fit");
        }
        let mut calls = self.calls.lock().expect("provider call log must remain available");
        calls.push(ProviderCall { paths, source_bytes });
        let call = calls.len();
        drop(calls);
        if let Some(hook) = &self.hook {
            hook(call);
        }
        let omit_imports = self.omit_imports_on_call == Some(call);
        let mut raw = raw_snapshot(sources, omit_imports);
        if self.error_on_call == Some(call) {
            raw.diagnostics.push(syntax_v3::RawProviderDiagnostic {
                code: "TS1000".to_owned(),
                severity: Severity::Error,
                location: syntax_v3::RawDiagnosticLocation::Global,
                message: "fixture provider error".to_owned(),
                guidance: "fix the fixture source".to_owned(),
            });
        }
        let verified = syntax_v3::verify_snapshot(raw, sources)
            .expect("fixture provider must construct valid exact-v3 syntax");
        Ok(verified)
    }
}
