//! Independent same-domain operation/type/place/CFG verification after source authentication.
use super::type_world::{Fact, Kind, TypeWorld};
use super::{
    Failure, budget, frontier, places, raw, reject, reserve, resource_failure, source::SourceProof,
};
use zryna_layout::generic_v1::VerifiedLayouts;
mod calls;
#[cfg(test)]
mod census_tests;
pub(super) mod cfg;
#[cfg(test)]
pub(super) mod container_test_support;
pub(super) mod loan_sites;
mod operations;
mod terminators;
#[cfg(test)]
mod tests;
mod world;

#[derive(Debug)]
pub(super) struct Function {
    pub(super) graph: cfg::Graph,
    pub(super) values: Vec<raw::ValueType>,
    pub(super) places: places::Table,
    pub(super) callees: Vec<Vec<Option<usize>>>,
    pub(super) sites: Vec<Vec<usize>>,
    pub(super) may_fail: bool,
    pub(super) upgrades: Vec<Option<zryna_source::UntrustedSpan>>,
    pub(super) match_scopes: Vec<super::source::MatchScope>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Site {
    pub(super) function: usize,
    pub(super) block: usize,
    pub(super) instruction: usize,
}
#[derive(Debug)]
pub(super) struct Typed {
    pub(super) functions: Vec<Function>,
    pub(super) frontier: frontier::Table,
    pub(super) sites: Vec<Site>,
    pub(super) facts: Vec<Fact>,
}

pub(super) fn check(
    p: &raw::Program,
    source: &SourceProof,
    linear: &VerifiedLayouts,
    linux: &VerifiedLayouts,
    spent: &mut usize,
) -> Result<Typed, Failure> {
    let mut work = *spent;
    world::check(p, source, linear, linux, &mut work)?;
    census(p, &mut work)?;
    let world = source.world();
    let frontier = frontier::Table::build(world, &mut work)?;
    let mut functions = reserve(p.functions.len())?;
    let mut sites = Vec::new();
    let mut facts = reserve(world.len())?;
    for id in 0..world.len() {
        facts.push(
            world
                .fact(u32::try_from(id).map_err(|_| Failure::InternalFailure)?)
                .ok_or_else(|| reject("execution source fact absent"))?,
        );
    }
    for (fi, f) in p.functions.iter().enumerate() {
        let graph = cfg::Graph::build(f, world.len(), &mut work)?;
        let mut values = reserve(graph.types().len())?;
        values.extend(graph.types());
        let table = places::check(f, &values, world, &mut work)?;
        budget::charge(&mut work, f.blocks.len())?;
        let mut checked = Function {
            graph,
            values,
            places: table,
            callees: reserve(f.blocks.len())?,
            sites: reserve(f.blocks.len())?,
            may_fail: false,
            upgrades: reserve(f.blocks.len())?,
            match_scopes: source.match_scopes(fi, f, &mut work)?,
        };
        for (bi, block) in f.blocks.iter().enumerate() {
            let mut targets = reserve(block.instructions.len())?;
            let mut site_ids = reserve(block.instructions.len())?;
            for (position, instruction) in block.instructions.iter().enumerate() {
                if !places::within(f.original_span, instruction.site_span) {
                    return Err(reject("execution instruction span is outside its original"));
                }
                let site = Site { function: fi, block: bi, instruction: position };
                let target = calls::target(p, &instruction.operation, &mut work)?;
                loan_sites::instruction(f, instruction, &mut work)?;
                checked.may_fail |= operations::check(
                    operations::Inputs {
                        program: p,
                        source,
                        frontier: &frontier,
                        function: &checked,
                        site,
                        callee: target,
                    },
                    instruction,
                    &mut work,
                )?;
                site_ids.push(sites.len());
                sites.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
                sites.push(site);
                targets.push(target);
            }
            let upgrade = if let raw::Terminator::WeakUpgradeBranch { weak, failure, .. } =
                block.terminator
            {
                let span = source.upgrade_site(fi, bi, weak, failure, &mut work)?;
                loan_sites::upgrade_at(f, &block.terminator, span, &mut work)?;
                Some(span)
            } else {
                None
            };
            if matches!(
                block.terminator,
                raw::Terminator::ClosedEnumMatch {
                    mode: raw::MatchMode::SharedLoan | raw::MatchMode::ExclusiveLoan,
                    ..
                }
            ) {
                let span = source.match_site(fi, bi, f, &block.terminator, &mut work)?;
                loan_sites::match_at(f, &block.terminator, span, &mut work)?;
            }
            terminators::check(f, &checked, world, bi, &mut work)?;
            checked.may_fail |= matches!(
                block.terminator,
                raw::Terminator::Trap { .. } | raw::Terminator::WeakUpgradeBranch { .. }
            );
            checked.upgrades.push(upgrade);
            checked.callees.push(targets);
            checked.sites.push(site_ids);
        }
        functions.push(checked);
    }
    calls::fallibility(p, &mut functions, &mut work)?;
    *spent = work;
    Ok(Typed { functions, frontier, sites, facts })
}

fn census(p: &raw::Program, spent: &mut usize) -> Result<(), Failure> {
    let mut work = *spent;
    census_scan(p, &mut work)?;
    *spent = work;
    Ok(())
}
fn census_scan(p: &raw::Program, work: &mut usize) -> Result<(), Failure> {
    use crate::data_ownership_v1 as limits;
    if p.functions.len() > limits::MAX_FUNCTIONS_PER_PROGRAM
        || p.type_keys.len() > limits::MAX_INSTANTIATED_TYPES
    {
        return Err(resource_failure("execution function/type inventory exceeds inherited limit"));
    }
    budget::charge(
        work,
        p.functions
            .len()
            .checked_mul(4)
            .ok_or_else(|| resource_failure("execution function census overflow"))?,
    )?;
    let (mut values, mut blocks, mut operands, mut literals, mut parameters) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let (mut edges, mut calls) = (0usize, 0usize);
    for (fi, f) in p.functions.iter().enumerate() {
        signature(p, fi, work)?;
        parameters = add(parameters, f.parameters.len())?;
        blocks = add(blocks, f.blocks.len())?;
        budget::charge(work, f.parameters.len())?;
        values = add(
            values,
            f.parameters.iter().filter(|p| matches!(p, raw::Parameter::Value(_))).count(),
        )?;
        budget::charge(work, f.blocks.len())?;
        for block in &f.blocks {
            edges = add(edges, cfg::edge_count_of(&block.terminator))?;
            if block.value_parameters.len() > limits::MAX_BLOCK_PARAMETERS {
                return Err(resource_failure(
                    "execution block parameter count exceeds inherited limit",
                ));
            }
            budget::charge(
                work,
                block
                    .instructions
                    .len()
                    .checked_mul(8)
                    .and_then(|n| n.checked_add(block.value_parameters.len().checked_mul(4)?))
                    .ok_or_else(|| resource_failure("execution operation census overflow"))?,
            )?;
            values = add(values, block.value_parameters.len())?;
            for i in &block.instructions {
                calls = add(
                    calls,
                    usize::from(matches!(
                        i.operation,
                        raw::Operation::SourceCall { .. }
                            | raw::Operation::ClosedGenericCall { .. }
                    )),
                )?;
                values = add(values, usize::from(i.result.is_some()))?;
                let n = match &i.operation {
                    raw::Operation::StructConstruct { fields, .. } => fields.len(),
                    raw::Operation::FixedArrayConstruct { elements, .. }
                    | raw::Operation::VecConstruct { elements, .. } => elements.len(),
                    raw::Operation::ClosedGenericCall { arguments, .. }
                    | raw::Operation::SourceCall { arguments, .. } => arguments.len(),
                    _ => 0,
                };
                operands = add(operands, n)?;
                if let raw::Operation::StringFromUtf8 { bytes, .. } = &i.operation {
                    if bytes.len() > 65_536 {
                        return Err(resource_failure(
                            "execution String literal exceeds inherited per-literal limit",
                        ));
                    }
                    literals = add(literals, bytes.len())?;
                }
            }
        }
    }
    if edges > limits::MAX_CFG_EDGES_PER_PROGRAM || calls > limits::MAX_CALL_EDGES {
        return Err(resource_failure("execution global control/call edge limits exceeded"));
    }
    if values > limits::MAX_VALUES_PER_PROGRAM
        || blocks > limits::MAX_BLOCKS_PER_PROGRAM
        || operands > limits::MAX_AGGREGATE_OPERANDS
        || literals > limits::MAX_STRING_LITERAL_BYTES
        || parameters > limits::MAX_PARAMETERS_PER_PROGRAM
    {
        return Err(resource_failure("execution aggregate inventory exceeds inherited limits"));
    }
    budget::charge(work, add(add(operands, literals)?, values)?)?;
    Ok(())
}
fn add(a: usize, b: usize) -> Result<usize, Failure> {
    a.checked_add(b).ok_or_else(|| resource_failure("execution aggregate census overflow"))
}

fn signature(p: &raw::Program, fi: usize, work: &mut usize) -> Result<(), Failure> {
    use crate::data_ownership_v1 as limits;
    let f = &p.functions[fi];
    if f.parameters.len() > limits::MAX_PARAMETERS_PER_FUNCTION
        || f.cleanup_plans.len() > limits::MAX_CLEANUP_PLANS_PER_FUNCTION
    {
        return Err(resource_failure("execution signature/plan count exceeds inherited limit"));
    }
    if fi != 0 {
        budget::charge(work, p.functions[fi - 1].key.len().min(f.key.len()) + 1)?;
        if p.functions[fi - 1].key >= f.key {
            return Err(reject("execution function keys are not canonical unique order"));
        }
    }
    budget::charge(work, f.parameters.len())?;
    if matches!(f.result_type, raw::ValueType::Stored(id) if id as usize >= p.type_keys.len())
        || f.parameters.iter().any(|p| {
            matches!(
                p,
                raw::Parameter::Value(raw::ValueDefinition { ty: raw::ValueType::Unit, .. })
            )
        })
    {
        return Err(reject("execution signature has an invalid stored/unit type"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod continuation_test_support;

#[cfg(test)]
pub(super) mod borrowed_match_test_support;
