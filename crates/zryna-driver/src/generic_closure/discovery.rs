//! Bounded fixed-point reads and a fresh whole-map V5 verification before source issuance.
use super::{
    BTreeMap, BTreeSet, CommandFailure, Discovery, DiscoveryFailure, Duration, EdgeIdentity,
    HashSet, Instant, ModuleClosureError, ModuleRecord, ModuleSourceSession, NormalizedSourcePath,
    SourceMap, VerifiedFrontendProviderV5, VerifiedGenericClosure, graph, imports, limits, support,
};
use support::{add, budget, checkpoint, diagnostic, invariant, rejected, remaining, snapshot};
use zryna_source::SourceFileInput;
mod batch;
use crate::{MAX_MODULE_DISCOVERY_ROUNDS, MAX_MODULE_FILES};

pub(super) fn discover(
    session: &mut impl ModuleSourceSession,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    now: &mut impl FnMut() -> Instant,
    limit: Duration,
    outer: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<VerifiedGenericClosure, DiscoveryFailure> {
    let started = now();
    if std::path::Path::new(entry.as_str()).extension().and_then(|s| s.to_str()) != Some("zry") {
        return Err(rejected(diagnostic(
            "ZRYNA-D3001",
            Some(&entry),
            "entry requires exact lowercase .zry extension",
        ))
        .into());
    }
    let mut state = Discovery {
        files: BTreeMap::new(),
        portable: BTreeMap::from([(entry.portable_identity(), entry.clone())]),
        pending: BTreeSet::from([entry.clone()]),
        edges: HashSet::new(),
        ledger: limits::Ledger::default(),
    };
    let mut rounds = 0;
    while !state.pending.is_empty() {
        rounds = add(rounds, 1)?;
        if rounds > MAX_MODULE_DISCOVERY_ROUNDS
            || add(state.files.len(), state.pending.len())? > MAX_MODULE_FILES
        {
            return Err(budget("generic module graph limit exceeded").into());
        }
        batch::step(session, frontend, (started, now, limit), &mut state, outer)?;
    }
    finish(session, entry, frontend, (started, now, limit), state, outer)
}

fn finish(
    session: &mut impl ModuleSourceSession,
    entry: NormalizedSourcePath,
    frontend: &(impl VerifiedFrontendProviderV5 + ?Sized),
    clock: (Instant, &mut impl FnMut() -> Instant, Duration),
    mut state: Discovery,
    outer: &dyn Fn() -> Result<(), CommandFailure>,
) -> Result<VerifiedGenericClosure, DiscoveryFailure> {
    let (started, now, limit) = clock;
    session.revalidate_all().map_err(rejected)?;
    state.ledger.provider(state.ledger.bytes, frontend.analysis_work_units(), true)?;
    let inputs = state
        .files
        .iter()
        .map(|(path, file)| SourceFileInput { path: path.as_str().into(), text: file.text.clone() })
        .collect();
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
    if !imports::matches(&imports::collect(&syntax)?, &state.files) {
        return Err(invariant().into());
    }
    let modules = state
        .files
        .iter()
        .enumerate()
        .map(|(id, (path, file))| {
            Ok(ModuleRecord {
                id: u32::try_from(id).map_err(|_| invariant())?,
                path: path.clone(),
                source_sha256: file.sha256,
            })
        })
        .collect::<Result<Vec<_>, ModuleClosureError>>()?;
    let edges = imports::final_edges(&syntax, &sources)?;
    let actual = edges
        .iter()
        .map(|edge| EdgeIdentity {
            importer: edge.importer.clone(),
            specifier: edge.specifier.clone(),
            imported: edge.imported.clone(),
            local: edge.local.clone(),
        })
        .collect::<HashSet<_>>();
    if actual != state.edges {
        return Err(invariant().into());
    }
    graph::validate(&modules, &edges)?;
    let graph_sha256 = graph::identity(&entry, &modules, &edges)?;
    Ok(VerifiedGenericClosure { entrypoint: entry, sources, syntax, modules, edges, graph_sha256 })
}
