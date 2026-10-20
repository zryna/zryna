use serde_json::Value;
use std::fmt::Debug;
use zryna_diagnostics::{Diagnostic, Severity};
use zryna_driver::ModuleClosureError;
use zryna_frontend::{WorkerError, syntax_v2, syntax_v3, syntax_v4};
use zryna_source::{NormalizedSourcePath, SourceMap};

use super::brands;

#[derive(Debug, Eq, PartialEq)]
pub struct Observation {
    pub phase: &'static str,
    pub ir: Option<String>,
    diagnostics: Value,
    worker: Option<String>,
}
impl Observation {
    fn rejected(phase: &'static str, diagnostics: &[Diagnostic]) -> Self {
        Self {
            phase,
            ir: None,
            diagnostics: serde_json::to_value(diagnostics).unwrap(),
            worker: None,
        }
    }
    fn worker(error: WorkerError) -> Self {
        Self {
            phase: "worker",
            ir: None,
            diagnostics: serde_json::to_value(error.diagnostics()).unwrap(),
            worker: Some(format!("{}:{:?}", error.code(), error.failure())),
        }
    }
    pub fn closure(error: ModuleClosureError) -> Self {
        match error {
            ModuleClosureError::Frontend(error) => Self::worker(error),
            ModuleClosureError::Rejected(diagnostics) => Self::rejected("graph", &diagnostics),
        }
    }
    pub fn lowered<T: Debug>(
        result: Result<T, Vec<Diagnostic>>,
        provider_diagnostics: &[Diagnostic],
        sources: &SourceMap,
    ) -> Self {
        match result {
            Ok(ir) => Self {
                phase: "verified-ir",
                ir: Some(brands::debug(&ir, sources)),
                diagnostics: serde_json::to_value(provider_diagnostics).unwrap(),
                worker: None,
            },
            Err(diagnostics) => Self::rejected("semantic-or-ir", &diagnostics),
        }
    }
    pub fn codes(&self) -> Vec<String> {
        let diagnostics = self.diagnostics.as_array().expect("ordered diagnostic list");
        if diagnostics.is_empty() {
            self.worker
                .as_ref()
                .map(|worker| worker.split(':').next().unwrap().to_owned())
                .into_iter()
                .collect()
        } else {
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic["code"].as_str().unwrap().to_owned())
                .collect()
        }
    }
}

pub fn pair<T: Debug + Eq>(
    native: Result<T, WorkerError>,
    bootstrap: Result<T, WorkerError>,
    id: &str,
    lower: impl Fn(&T) -> Observation,
) -> Observation {
    match (native, bootstrap) {
        (Ok(left), Ok(right)) => {
            assert_eq!(left, right, "{id} complete exact syntax on the SAME SourceMap");
            let left = lower(&left);
            assert_eq!(left, lower(&right), "{id} complete verified IR and ordered diagnostics");
            left
        }
        (Err(left), Err(right)) => {
            let left = Observation::worker(left);
            assert_eq!(left, Observation::worker(right), "{id} original worker rejection");
            left
        }
        values => panic!("{id} syntax acceptance differs: {values:?}"),
    }
}

fn errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.severity() == Severity::Error)
}
fn entry(sources: &SourceMap, path: &str) -> zryna_source::FileId {
    sources
        .file_id(&NormalizedSourcePath::new(path.to_owned()).unwrap())
        .expect("exact authoritative entry")
}
pub fn lower_v2(syntax: &syntax_v2::ProjectSyntaxSnapshot, sources: &SourceMap) -> Observation {
    match zryna_driver::lower_verified_syntax(syntax, sources) {
        Ok(result) => Observation::lowered(Ok(result.program()), result.diagnostics(), sources),
        Err(diagnostics) => Observation::rejected("semantic-or-ir", &diagnostics),
    }
}
pub fn lower_v3(
    syntax: &syntax_v3::ProjectSyntaxSnapshot,
    sources: &SourceMap,
    path: &str,
) -> Observation {
    if errors(syntax.diagnostics()) {
        return Observation::rejected("semantic-or-ir", syntax.diagnostics());
    }
    let input = zryna_semantics::control_flow_v1::SemanticInput::try_new(
        syntax,
        sources,
        entry(sources, path),
    )
    .expect("original v3 source-bound semantic admission");
    Observation::lowered(
        zryna_semantics::control_flow_v1::lower(input),
        syntax.diagnostics(),
        sources,
    )
}
pub fn lower_v4(
    syntax: &syntax_v4::ProjectSyntaxSnapshot,
    sources: &SourceMap,
    path: &str,
) -> Observation {
    if errors(syntax.diagnostics()) {
        return Observation::rejected("semantic-or-ir", syntax.diagnostics());
    }
    let input = zryna_semantics::data_ownership_v1::SemanticInput::try_new(
        syntax,
        sources,
        entry(sources, path),
    )
    .expect("original v4 source-bound semantic admission");
    Observation::lowered(
        zryna_semantics::data_ownership_v1::lower(input),
        syntax.diagnostics(),
        sources,
    )
}
