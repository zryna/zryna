use std::cmp::Ordering;

use zryna_diagnostics::Diagnostic;
use zryna_source::{SourceMap, Span};

use super::MAX_SEMANTIC_DIAGNOSTICS;

pub(super) struct Errors<'a> {
    pub(super) sources: &'a SourceMap,
    diagnostics: Vec<Diagnostic>,
    exhausted: bool,
}
impl<'a> Errors<'a> {
    pub(super) fn new(sources: &'a SourceMap) -> Self {
        Self { sources, diagnostics: Vec::new(), exhausted: false }
    }
    pub(super) fn at(
        &mut self,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
        guidance: &'static str,
    ) {
        self.push(Diagnostic::error_at(code, span, message, guidance));
    }
    pub(super) fn global(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        guidance: &'static str,
    ) {
        self.push(Diagnostic::error(code, None, message, guidance));
    }
    fn push(&mut self, diagnostic: Diagnostic) {
        if self.exhausted {
            return;
        }
        if self.diagnostics.len() < MAX_SEMANTIC_DIAGNOSTICS - 1 {
            self.diagnostics.push(diagnostic);
        } else {
            let message = format!(
                "semantic analysis reached its diagnostic limit of {MAX_SEMANTIC_DIAGNOSTICS}"
            );
            let guidance = "fix the retained diagnostics before compiling again";
            self.diagnostics.push(if let Some(at) = diagnostic.primary_span() {
                Diagnostic::error_at("ZRYNA-M3202", at, message, guidance)
            } else {
                Diagnostic::error("ZRYNA-M3202", None, message, guidance)
            });
            self.exhausted = true;
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
    pub(super) fn len(&self) -> usize {
        self.diagnostics.len()
    }
    pub(super) fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort_by(compare_diagnostics);
        self.diagnostics
    }
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    match (left.primary_span(), right.primary_span()) {
        (Some(l), Some(r)) => {
            (l.file().index(), l.start(), l.end(), left.code(), left.message(), left.guidance())
                .cmp(&(
                    r.file().index(),
                    r.start(),
                    r.end(),
                    right.code(),
                    right.message(),
                    right.guidance(),
                ))
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => (left.code(), left.message(), left.guidance()).cmp(&(
            right.code(),
            right.message(),
            right.guidance(),
        )),
    }
}
