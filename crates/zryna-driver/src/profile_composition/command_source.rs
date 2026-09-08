//! One authenticated I32V1 source instance with an independently verified empty request.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use zryna_backend_webassembly::{WitSource, WitWorldAudit, audit_pinned_wit_worlds};
use zryna_diagnostics::Diagnostic;
use zryna_frontend::VerifiedFrontendProvider;
use zryna_ir::VerifiedProgram;
use zryna_source::SourceMap;

use crate::{SourceToIrError, SourceToIrSuccess, compile_to_verified_ir};

use super::{
    INVALID, ValidatedComposition,
    authority::{Authorities, InstanceAuthority, VerifiedLanguage},
    error, graph,
    model::{
        Claim, Input, Instance, Language, POLICY, Reservation, Row, Selection, Summary, VERSION,
    },
    verify,
};

const SOURCE_DOMAIN: &[u8] = b"zryna.command-self-check.source.v1\0";
const COMMAND_WORLD: &str = "zryna:capability-profiles/command@0.1.0";
const ROOT_INSTANCE: &str = "command-source";

/// Retains the exact source object used by the authenticated compiler invocation.
pub(crate) struct PureCommandSource<'source> {
    sources: &'source SourceMap,
    compiled: SourceToIrSuccess,
    source_identity: [u8; 32],
    composition: ValidatedComposition,
    composition_identity: [u8; 32],
    wit: WitWorldAudit,
}

impl PureCommandSource<'_> {
    pub(crate) fn program(&self) -> &VerifiedProgram {
        self.compiled.program()
    }

    pub(crate) fn source_identity(&self) -> &[u8; 32] {
        &self.source_identity
    }

    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        self.compiled.diagnostics()
    }

    pub(crate) fn composition_identity(&self) -> &[u8; 32] {
        &self.composition_identity
    }

    pub(crate) fn revalidate(&self, wit: &WitWorldAudit) -> Result<(), Vec<Diagnostic>> {
        if wit != &self.wit {
            return Err(vec![error(INVALID, "command composition WIT authority changed")]);
        }
        let identity = source_identity(self.sources)?;
        if identity != self.source_identity {
            return Err(vec![error(INVALID, "authenticated command source identity changed")]);
        }
        let input = pure_input();
        let authorities = command_authorities(self.program(), self.sources, wit);
        self.composition.revalidate(&input, &authorities)?;
        let graph = graph::validate(&input)?;
        if graph.binding(&authorities.binding(&graph.ids())?)? != self.composition_identity {
            return Err(vec![error(INVALID, "command composition identity changed")]);
        }
        if !self.composition.requirements().is_empty() {
            return Err(vec![error(INVALID, "pure command composition acquired a host request")]);
        }
        Ok(())
    }
}

/// The matching source and verified program can only enter together through real compilation.
pub(crate) fn compile_pure_command<'source, Provider: VerifiedFrontendProvider + ?Sized>(
    frontend: &Provider,
    sources: &'source SourceMap,
    wit_sources: &[WitSource],
) -> Result<PureCommandSource<'source>, SourceToIrError> {
    let compiled = compile_to_verified_ir(frontend, sources)?;
    let wit = audit_pinned_wit_worlds(wit_sources)
        .map_err(|error| SourceToIrError::Rejected(vec![error]))?;
    let source_identity = source_identity(sources).map_err(SourceToIrError::Rejected)?;
    let input = pure_input();
    let authorities = command_authorities(compiled.program(), sources, &wit);
    let binding = graph::validate(&input)
        .and_then(|graph| graph.binding(&authorities.binding(&graph.ids())?))
        .map_err(SourceToIrError::Rejected)?;
    // The producer claims only the one-node empty result; verification derives it independently.
    let claim = Claim {
        binding,
        summaries: BTreeMap::from([(ROOT_INSTANCE.to_owned(), Summary::default())]),
        witnesses: BTreeMap::new(),
    };
    let composition = verify(&input, &authorities, &claim).map_err(SourceToIrError::Rejected)?;
    Ok(PureCommandSource {
        sources,
        compiled,
        source_identity,
        composition,
        composition_identity: binding,
        wit,
    })
}

fn source_identity(sources: &SourceMap) -> Result<[u8; 32], Vec<Diagnostic>> {
    let mut digest = Sha256::new();
    digest.update(SOURCE_DOMAIN);
    digest.update((sources.len() as u64).to_le_bytes());
    for index in 0..sources.len() {
        let file = u32::try_from(index)
            .ok()
            .and_then(|index| sources.verify_file_id(index).ok())
            .and_then(|id| sources.source(id))
            .ok_or_else(|| vec![error(INVALID, "command source map identity is unavailable")])?;
        for bytes in [file.path().as_str().as_bytes(), file.text().as_bytes()] {
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
    }
    Ok(digest.finalize().into())
}

fn command_authorities(
    program: &VerifiedProgram,
    sources: &SourceMap,
    wit: &WitWorldAudit,
) -> Authorities {
    Authorities {
        instances: BTreeMap::from([(
            ROOT_INSTANCE.to_owned(),
            InstanceAuthority {
                programs: vec![VerifiedLanguage::I32V1 {
                    program: program.clone(),
                    sources: sources.clone(),
                }],
            },
        )]),
        wit: Some(wit.clone()),
    }
}

fn pure_input() -> Input {
    Input {
        version: VERSION.to_owned(),
        root: ROOT_INSTANCE.to_owned(),
        language: Language::I32V1,
        selections: vec![Selection {
            row: Row::WitCommand,
            policy_version: POLICY.to_owned(),
            world: Some(COMMAND_WORLD.to_owned()),
            approved: BTreeSet::new(),
            ceilings: [0; 10],
        }],
        instances: vec![Instance {
            id: ROOT_INSTANCE.to_owned(),
            rows: BTreeSet::from([Row::WitCommand]),
            requirements: BTreeSet::new(),
            restrictions: BTreeSet::new(),
            reservation: Reservation::default(),
        }],
        edges: Vec::new(),
    }
}
