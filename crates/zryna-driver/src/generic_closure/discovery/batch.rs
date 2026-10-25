//! One prepaid real analysis and exact imports from that unchanged batch.
use super::super::{
    BTreeMap, BTreeSet, CommandFailure, DiscoveredSource, Discovery, DiscoveryFailure, Duration,
    Import, Instant, ModuleClosureError, ModuleSourceSession, NormalizedSourcePath, SourceMap,
    VerifiedFrontendProviderV5, imports, limits, support,
};
use crate::{MAX_MODULE_FILES, workspace_source::StableSource};
use support::{
    add, budget, checkpoint, invariant, portable, rejected, remaining, snapshot, target,
};
use zryna_source::SourceFileInput;
type SourceBatch = (Vec<SourceFileInput>, BTreeMap<NormalizedSourcePath, StableSource>, usize);
pub(super) fn step(
    session: &mut impl ModuleSourceSession,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    clock: (Instant, &mut impl FnMut() -> Instant, Duration),
    state: &mut Discovery,
    outer: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<(), DiscoveryFailure> {
    let (started, now, limit) = clock;
    let paths = std::mem::take(&mut state.pending);
    let (inputs, stable, bytes) = read(session, paths, &mut state.ledger)?;
    state.ledger.provider(bytes, frontend.analysis_work_units(), false)?;
    let sources = SourceMap::build(inputs).map_err(|_| invariant())?;
    checkpoint(session, &sources, outer)?;
    let syntax = frontend
        .analyze_verified_v5_with_timeout(
            &sources,
            remaining(started, now(), frontend.minimum_analysis_timeout(), limit)?,
        )
        .map_err(ModuleClosureError::Frontend)?;
    remaining(started, now(), Duration::ZERO, limit)?;
    snapshot(&syntax, &sources)?;
    checkpoint(session, &sources, outer)?;
    register(state, stable, &imports::collect(&syntax)?)?;
    Ok(())
}
fn read(
    session: &mut impl ModuleSourceSession,
    paths: BTreeSet<NormalizedSourcePath>,
    ledger: &mut limits::Ledger,
) -> Result<SourceBatch, ModuleClosureError> {
    let mut stable = BTreeMap::new();
    let mut inputs = Vec::with_capacity(paths.len());
    let mut bytes = 0;
    for path in paths {
        let source = session.read_source(&path).map_err(rejected)?;
        bytes = add(bytes, source.text.len())?;
        ledger.source(source.text.len())?;
        inputs.push(SourceFileInput { path: path.as_str().into(), text: source.text.clone() });
        stable.insert(path, source);
    }
    Ok((inputs, stable, bytes))
}
fn register(
    state: &mut Discovery,
    stable: BTreeMap<NormalizedSourcePath, StableSource>,
    imports: &BTreeMap<NormalizedSourcePath, Vec<Import>>,
) -> Result<(), ModuleClosureError> {
    let known = add(state.files.len(), imports.len())?;
    for (path, source) in stable {
        let rows = imports.get(&path).cloned().ok_or_else(invariant)?;
        for import in &rows {
            let target = target(&path, &import.specifier)?;
            portable(&mut state.portable, &target)?;
            state.ledger.import(&path, import, &target, &mut state.edges)?;
            if !state.files.contains_key(&target) && !imports.contains_key(&target) {
                if !state.pending.contains(&target)
                    && add(known, state.pending.len())? >= MAX_MODULE_FILES
                {
                    return Err(budget(
                        "generic module file limit exceeded before pending insertion",
                    ));
                }
                state.pending.insert(target);
            }
        }
        state.files.insert(
            path,
            DiscoveredSource { text: source.text, sha256: source.sha256, imports: rows },
        );
    }
    Ok(())
}
