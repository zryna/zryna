//! Test-only retained session and actual current native syntax; no fabricated positive source seal.
use super::*;
use crate::workspace_source::StableSource;
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::{WorkerError, native_lexer::lex, native_parser::v5::parse_v5_candidate};
pub(crate) struct MemoryRoot {
    files: BTreeMap<NormalizedSourcePath, String>,
    pub(crate) drift: Arc<AtomicBool>,
    pub(crate) active: AtomicBool,
    pub(crate) checks: AtomicUsize,
}
pub(crate) struct Session<'a>(&'a MemoryRoot);
impl Drop for Session<'_> {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::SeqCst);
    }
}
impl MemoryRoot {
    pub(crate) fn new(rows: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            files: rows
                .into_iter()
                .map(|(path, text)| (NormalizedSourcePath::new(path).expect("test path"), text))
                .collect(),
            drift: Arc::new(AtomicBool::new(false)),
            active: AtomicBool::new(false),
            checks: AtomicUsize::new(0),
        }
    }
    fn unchanged(&self) -> Result<(), Diagnostic> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        if self.drift.load(Ordering::SeqCst) {
            return Err(Diagnostic::error(
                "ZRYNA-S1001",
                None,
                "test source changed",
                "retain exact source",
            ));
        }
        Ok(())
    }
}
impl ModuleSourceRoot for MemoryRoot {
    type Session<'a> = Session<'a>;
    fn begin(&self) -> Result<Session<'_>, Diagnostic> {
        assert!(!self.active.swap(true, Ordering::SeqCst));
        Ok(Session(self))
    }
}
impl ModuleSourceSession for Session<'_> {
    fn read_source(&mut self, path: &NormalizedSourcePath) -> Result<StableSource, Diagnostic> {
        self.0.unchanged()?;
        let text = self
            .0
            .files
            .get(path)
            .ok_or_else(|| {
                Diagnostic::error("ZRYNA-S1001", None, "unknown test source", "retain source")
            })?
            .clone();
        Ok(StableSource { sha256: Sha256::digest(text.as_bytes()).into(), text })
    }
    fn validate_provider_batch(&mut self, sources: &SourceMap) -> Result<(), Diagnostic> {
        self.0.unchanged()?;
        for id in 0..sources.len() {
            let file = sources
                .source(sources.verify_file_id(u32::try_from(id).expect("bounded")).expect("file"))
                .expect("source");
            assert_eq!(self.0.files.get(file.path()).map(String::as_str), Some(file.text()));
        }
        Ok(())
    }
    fn revalidate_all(&mut self) -> Result<(), Diagnostic> {
        self.0.unchanged()
    }
}
#[derive(Default)]
pub(crate) struct CurrentSyntax {
    pub(crate) calls: AtomicUsize,
    pub(crate) units: usize,
    pub(crate) foreign: bool,
    pub(crate) drift: Option<Arc<AtomicBool>>,
    pub(crate) advisory_error: bool,
    pub(crate) minimum: Duration,
}
impl CurrentSyntax {
    pub(crate) fn ordinary() -> Self {
        Self { units: 1, ..Self::default() }
    }
}
impl VerifiedFrontendProviderV5 for CurrentSyntax {
    fn analysis_work_units(&self) -> usize {
        self.units
    }
    fn minimum_analysis_timeout(&self) -> Duration {
        self.minimum
    }
    fn analyze_verified_v5(
        &self,
        sources: &SourceMap,
    ) -> Result<syntax_v5::VerifiedProjectSyntaxV5, WorkerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let foreign;
        let source = if self.foreign {
            let rows = (0..sources.len())
                .map(|id| {
                    let file = sources
                        .source(
                            sources
                                .verify_file_id(u32::try_from(id).expect("bounded"))
                                .expect("file"),
                        )
                        .expect("source");
                    zryna_source::SourceFileInput {
                        path: file.path().as_str().into(),
                        text: file.text().into(),
                    }
                })
                .collect();
            foreign = SourceMap::build(rows).expect("independent same-byte source map");
            &foreign
        } else {
            sources
        };
        let mut raw =
            parse_v5_candidate(source, &lex(source).expect("actual current native lexer"))
                .expect("actual current parser on complete test source");
        if self.advisory_error {
            raw.diagnostics.push(zryna_syntax::v4::RawProviderDiagnostic {
                code: "fixture-provider".into(),
                severity: zryna_diagnostics::Severity::Error,
                location: zryna_syntax::v4::RawDiagnosticLocation::Global,
                message: "provider rejected".into(),
                guidance: "fix source".into(),
            });
        }
        let syntax =
            syntax_v5::verify_snapshot(raw, source).expect("actual complete source authentication");
        if let Some(drift) = &self.drift {
            drift.store(true, Ordering::SeqCst);
        }
        Ok(syntax)
    }
}
