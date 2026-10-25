//! Private V5 fixed-point source authority, retained through the caller's last checkpoint.
use crate::{
    CommandFailure, MAX_MODULE_DISCOVERY_WALL_TIME, ModuleClosureError, ModuleEdge, ModuleRecord,
    source_session::{ModuleSourceRoot, ModuleSourceSession},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
    time::{Duration, Instant},
};
use zryna_frontend::{VerifiedFrontendProviderV5, syntax_v5};
use zryna_source::{NormalizedSourcePath, SourceMap, UntrustedSpan};
mod discovery;
mod graph;
mod imports;
mod limits;
mod support;
use support::{failure, invariant};

enum DiscoveryFailure {
    Source(ModuleClosureError),
    Checkpoint(CommandFailure),
}
impl From<ModuleClosureError> for DiscoveryFailure {
    fn from(error: ModuleClosureError) -> Self {
        Self::Source(error)
    }
}
impl DiscoveryFailure {
    fn command(self) -> CommandFailure {
        match self {
            Self::Source(error) => failure(&error),
            Self::Checkpoint(error) => error,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportBinding {
    imported: String,
    local: String,
    imported_span: UntrustedSpan,
    local_span: UntrustedSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Import {
    span: UntrustedSpan,
    specifier: String,
    specifier_span: UntrustedSpan,
    bindings: Vec<ImportBinding>,
}
struct DiscoveredSource {
    text: String,
    sha256: [u8; 32],
    imports: Vec<Import>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct EdgeIdentity {
    importer: NormalizedSourcePath,
    specifier: String,
    imported: String,
    local: String,
}
struct Discovery {
    files: BTreeMap<NormalizedSourcePath, DiscoveredSource>,
    portable: BTreeMap<String, NormalizedSourcePath>,
    pending: BTreeSet<NormalizedSourcePath>,
    edges: HashSet<EdgeIdentity>,
    ledger: limits::Ledger,
}

#[derive(Debug)]
pub(crate) struct VerifiedGenericClosure {
    entrypoint: NormalizedSourcePath,
    sources: SourceMap,
    syntax: syntax_v5::VerifiedProjectSyntaxV5,
    modules: Vec<ModuleRecord>,
    edges: Vec<ModuleEdge>,
    graph_sha256: [u8; 32],
}
impl VerifiedGenericClosure {
    pub(crate) fn entrypoint(&self) -> &NormalizedSourcePath {
        &self.entrypoint
    }
    pub(crate) fn sources(&self) -> &SourceMap {
        &self.sources
    }
    pub(crate) fn syntax(&self) -> &syntax_v5::VerifiedProjectSyntaxV5 {
        &self.syntax
    }
    pub(crate) fn modules(&self) -> &[ModuleRecord] {
        &self.modules
    }
    pub(crate) fn edges(&self) -> &[ModuleEdge] {
        &self.edges
    }
    pub(crate) const fn graph_sha256(&self) -> &[u8; 32] {
        &self.graph_sha256
    }
}

#[cfg(test)]
pub(crate) fn with_closure<R>(
    root: &impl ModuleSourceRoot,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    outer_checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
    inspect: impl FnOnce(
        &VerifiedGenericClosure,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    with_clock(
        root,
        entry,
        frontend,
        outer_checkpoint,
        Instant::now,
        MAX_MODULE_DISCOVERY_WALL_TIME,
        inspect,
    )
}

// Preparation only: the owned result is revalidated after the callback.
#[cfg(test)]
fn with_clock<R>(
    root: &impl ModuleSourceRoot,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    outer_checkpoint: &dyn Fn() -> Result<(), CommandFailure>,
    now: impl FnMut() -> Instant,
    limit: Duration,
    inspect: impl FnOnce(
        &VerifiedGenericClosure,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    with_clock_terminal(
        root,
        entry,
        frontend,
        outer_checkpoint,
        (now, limit),
        inspect,
        |owned, _| Ok(owned),
    )
}

// Only finish may publish. Its success is terminal: no fallible post-commit check is added.
pub(crate) fn with_closure_terminal<P, R>(
    root: &impl ModuleSourceRoot,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    outer: &dyn Fn() -> Result<(), CommandFailure>,
    prepare: impl FnOnce(
        &VerifiedGenericClosure,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<P, CommandFailure>,
    finish: impl FnOnce(P, &dyn Fn() -> Result<(), CommandFailure>) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    with_clock_terminal(
        root,
        entry,
        frontend,
        outer,
        (Instant::now, MAX_MODULE_DISCOVERY_WALL_TIME),
        prepare,
        finish,
    )
}

fn with_clock_terminal<P, R>(
    root: &impl ModuleSourceRoot,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    outer: &dyn Fn() -> Result<(), CommandFailure>,
    clock: (impl FnMut() -> Instant, Duration),
    prepare: impl FnOnce(
        &VerifiedGenericClosure,
        &dyn Fn() -> Result<(), CommandFailure>,
    ) -> Result<P, CommandFailure>,
    finish: impl FnOnce(P, &dyn Fn() -> Result<(), CommandFailure>) -> Result<R, CommandFailure>,
) -> Result<R, CommandFailure> {
    let (mut now, limit) = clock;
    if limit > MAX_MODULE_DISCOVERY_WALL_TIME {
        return Err(failure(&invariant()));
    }
    outer()?;
    let session = RefCell::new(root.begin().map_err(support::source_failure)?);
    let closure =
        discovery::discover(&mut *session.borrow_mut(), entry, frontend, &mut now, limit, outer)
            .map_err(DiscoveryFailure::command)?;
    let checkpoint = || {
        outer()?;
        session
            .borrow_mut()
            .validate_provider_batch(&closure.sources)
            .map_err(support::source_failure)?;
        session.borrow_mut().revalidate_all().map_err(support::source_failure)
    };
    checkpoint()?;
    let prepared = prepare(&closure, &checkpoint)?;
    checkpoint()?;
    // finish must checkpoint immediately before its atomic commit. All owning handles stay live.
    finish(prepared, &checkpoint)
}

#[cfg(test)]
pub(crate) mod test_source;
#[cfg(test)]
mod tests;
