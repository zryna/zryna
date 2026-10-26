//! Independent full-profile source ownership; old issuers and symbolic IDs are not reused.
use super::budget::charge;
use super::{
    raw,
    type_world::{Builder as WorldBuilder, Frame, Kind, TypeWorld},
};
use crate::generic_v1::{
    Failure, budget as exhausted, reject, reserve,
    source::{Originals, Target},
    source_types::{Closed, Resolver},
};
use zryna_layout::generic_v1::VerifiedLayouts;
use zryna_source::{FileId, SourceMap, UntrustedSpan};
use zryna_syntax::v5::{
    self, RawDataDeclarationKind, RawExpressionKind as Expr, VerifiedProjectSyntaxV5,
};
mod originals;
mod proof;
mod replay;
mod world;
pub(super) use proof::SourceProof;
pub(super) use proof::matches::Scope as MatchScope;
pub(super) fn check_original_bodies(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
) -> Result<(), Failure> {
    let mut spent = 0;
    let inventory = originals::inventory(syntax, sources, &mut spent)?;
    let original = originals::authenticate(&inventory, syntax, &mut spent)?;
    originals::check(&original, sources, &mut spent)
}
pub(super) fn produce_claim(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    generic_keys: &[&[u8]],
) -> Result<raw::Program, Failure> {
    let mut spent = 0;
    produce(syntax, sources, entry, linear, linux, generic_keys, &mut spent)
        .map(|(program, _)| program)
}
pub(super) fn verify_source(
    claim: &raw::Program,
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    spent: &mut usize,
) -> Result<SourceProof, Failure> {
    charge(spent, claim.functions.len())?;
    let mut keys = reserve(claim.functions.len())?;
    for function in &claim.functions {
        if function.key.first() == Some(&0x40) {
            keys.push(function.key.as_slice());
        }
    }
    let (expected, proof) = produce(syntax, sources, entry, linear, linux, &keys, spent)?;
    proof::compare(&expected, claim, spent)?;
    Ok(proof)
}
fn produce(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    generic_keys: &[&[u8]],
    spent: &mut usize,
) -> Result<(raw::Program, SourceProof), Failure> {
    let mut work = *spent;
    let result = produce_paid(syntax, sources, entry, linear, linux, generic_keys, &mut work)?;
    *spent = work;
    Ok(result)
}
fn produce_paid(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    entry: FileId,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    generic_keys: &[&[u8]],
    spent: &mut usize,
) -> Result<(raw::Program, SourceProof), Failure> {
    let inventory = originals::inventory(syntax, sources, spent)?;
    let original = originals::authenticate(&inventory, syntax, spent)?;
    originals::check(&original, sources, spent)?;
    let mut program =
        originals::program(inventory, sources, entry, linear, linux, generic_keys, spent)?;
    let world = world::closed(&original, &program.type_keys, spent)?;
    let matches = replay::produce(&mut program, &original, sources, &world, spent)?;
    let proof = SourceProof::new(world, &program, matches, spent)?;
    Ok((program, proof))
}
fn diagnostic(
    sources: &SourceMap,
    span: UntrustedSpan,
    code: &'static str,
    message: &str,
) -> Failure {
    match sources.verify_span(span) {
        Ok(span) => Failure::Diagnostics(vec![zryna_diagnostics::Diagnostic::error_at(
            code,
            span,
            message,
            "use the exact accepted bounded-generics operation and ownership",
        )]),
        Err(_) => Failure::InternalFailure,
    }
}
fn copy_key(key: &[u8], spent: &mut usize) -> Result<Vec<u8>, Failure> {
    charge(spent, key.len())?;
    let mut retained = reserve(key.len())?;
    retained.extend_from_slice(key);
    Ok(retained)
}
fn lookup(keys: &[Vec<u8>], key: &[u8], spent: &mut usize) -> Result<u32, Failure> {
    let mut lo = 0;
    let mut hi = keys.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        charge(spent, keys[mid].len().min(key.len()) + 1)?;
        match keys[mid].as_slice().cmp(key) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => {
                return u32::try_from(mid).map_err(|_| Failure::InternalFailure);
            }
        }
    }
    Err(reject("complete source type is absent from the closed universe"))
}

#[cfg(test)]
mod container_tests;

#[cfg(test)]
mod partial_replay_tests;

#[cfg(test)]
mod borrowed_replay_tests;

#[cfg(test)]
mod indexed_replay_tests;
