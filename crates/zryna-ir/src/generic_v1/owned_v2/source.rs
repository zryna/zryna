//! Complete original affine source replay, independent of the raw cleanup plan.
//!
//! Unsupported source operations fail closed, including in unused original templates.
//! The symbolic pass retains opaque parameter keys; a closed i32 instance cannot legalize T + T.

use crate::generic_v1::{
    Failure, keys, raw, reject, reserve,
    source::Originals,
    source_types::{Closed, Resolver, type_id},
};
use zryna_syntax::{v4::RawStatementKind, v5::RawFunctionSyntax};

mod branches;
mod clones;
mod copy;
mod enums;
mod expressions;
mod loops;
mod normalize;
mod original_cycles;
mod owners;
mod phi;
mod resources;
mod statements;
mod structs;
mod type_facts;
use super::{
    Admission,
    raw::{Extension, Operation as Owned},
};
use resources::Budget;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Value {
    id: u32,
    ty: Closed,
}

#[derive(Clone)]
struct Binding<'a> {
    name: &'a str,
    value: Value,
    mutable: bool,
}

struct Builder<'a, 'b> {
    program: &'a raw::Program,
    sources: &'a zryna_source::SourceMap,
    resolver: Resolver<'a, 'b>,
    original: &'b RawFunctionSyntax,
    symbolic: bool,
    blocks: Vec<raw::Block>,
    block: usize,
    next: u32,
    scope_start: usize,
    locals: Vec<Binding<'b>>,
    result: Closed,
    extensions: Vec<Extension>,
    affine: Vec<bool>,
    alive: Vec<bool>,
    loans: BTreeMap<u32, (u32, bool)>,
    loan_parents: BTreeMap<u32, u32>,
    budget: Budget,
    type_facts: type_facts::SourceTypeFacts,
}

pub(super) fn check(
    claim: &super::raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
) -> Result<(), Failure> {
    check_for(claim, originals, sources, Admission::LegacyOwned)
}

pub(super) fn check_for(
    claim: &super::raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
    admission: Admission,
) -> Result<(), Failure> {
    let program = &claim.graph;
    if claim.extensions.len() != program.functions.len() {
        return Err(reject("incomplete owned extensions"));
    }
    check_originals_for(program, originals, sources, admission)?;
    let mut budget = Budget::default();
    for (function_index, function) in program.functions.iter().enumerate() {
        let domain = if function.key[0] == 0x40 {
            keys::Domain::FunctionInstance
        } else {
            keys::Domain::SourceRoot
        };
        let key = keys::decode(&function.key, domain)?;
        let (module, index) = key.declaration().ok_or(Failure::InternalFailure)?;
        let original = &originals.units[module as usize].functions[index as usize];
        let mut arguments = reserve(key.arguments().len())?;
        arguments.extend(key.arguments());
        let (blocks, extensions) = build(
            program,
            originals,
            sources,
            BuildFrame { module, original, arguments, symbolic: false, admission },
            &mut budget,
        )?;
        if function.blocks != blocks || claim.extensions[function_index] != extensions {
            return Err(reject(
                "claimed body differs from complete source operation/order/binding replay",
            ));
        }
    }
    crate::generic_v1::source_body::demand(program)
}

pub(super) fn check_originals(
    program: &raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
) -> Result<(), Failure> {
    check_originals_for(program, originals, sources, Admission::LegacyOwned)
}

pub(super) fn check_originals_for(
    program: &raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
    admission: Admission,
) -> Result<(), Failure> {
    if admission == Admission::StructV4 {
        original_cycles::check(originals, sources)?;
    }
    let mut nominal_credit = type_facts::check_declarations_for(originals, admission)?;
    let mut aggregate = Budget::default();
    // Every original is checked before checking instances, including unused templates.
    for unit in originals.units {
        for original in &unit.functions {
            let arity = original.type_parameters.as_ref().map_or(0, |list| list.parameters.len());
            let mut parameters = reserve(arity)?;
            for slot in 0..arity {
                // Each original body has its own isolated slot context. These one-byte opaque
                // tags cannot enter a closed key; they do not inflate the smallest substitution.
                let key = vec![0x30 + u8::try_from(slot).map_err(|_| Failure::InternalFailure)?];
                parameters.push(key);
            }
            let mut arguments = reserve(arity)?;
            arguments.extend(parameters.iter().map(Vec::as_slice));
            let mut original_budget = Budget::original(nominal_credit);
            if admission == Admission::StructV4 {
                original_budget.inherit_operands(&aggregate);
            }
            build(
                program,
                originals,
                sources,
                BuildFrame { module: unit.id, original, arguments, symbolic: true, admission },
                &mut original_budget,
            )?;
            nominal_credit = original_budget.nominal_used();
            if admission == Admission::StructV4 {
                aggregate.inherit_operands(&original_budget);
            }
        }
    }
    Ok(())
}

pub(super) fn produce(
    program: &mut raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
) -> Result<Vec<Vec<Extension>>, Failure> {
    produce_for(program, originals, sources, Admission::LegacyOwned)
}

pub(super) fn produce_for(
    program: &mut raw::Program,
    originals: &Originals<'_>,
    sources: &zryna_source::SourceMap,
    admission: Admission,
) -> Result<Vec<Vec<Extension>>, Failure> {
    check_originals_for(program, originals, sources, admission)?;
    let mut all = reserve(program.functions.len())?;
    let mut budget = Budget::default();
    for function in &program.functions {
        let domain = if function.key[0] == 0x40 {
            keys::Domain::FunctionInstance
        } else {
            keys::Domain::SourceRoot
        };
        let key = keys::decode(&function.key, domain)?;
        let (module, index) = key.declaration().ok_or(Failure::InternalFailure)?;
        let original = &originals.units[module as usize].functions[index as usize];
        let mut arguments = reserve(key.arguments().len())?;
        arguments.extend(key.arguments());
        all.push(build(
            program,
            originals,
            sources,
            BuildFrame { module, original, arguments, symbolic: false, admission },
            &mut budget,
        )?);
    }
    let mut extensions = reserve(all.len())?;
    for (function, (blocks, ext)) in program.functions.iter_mut().zip(all) {
        function.blocks = blocks;
        extensions.push(ext);
    }
    Ok(extensions)
}

mod build;
mod builder;
use build::{Frame as BuildFrame, build};
