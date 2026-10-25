//! Test-only inputs for the independent state phase; no source/execution authority is issued.
use super::{Function, Site, Typed, cfg, frontier, places, raw};
use crate::generic_v1::execution_v1::type_world::TypeWorld;

pub(in crate::generic_v1::execution_v1) fn phase_inputs(
    p: &raw::Program,
    world: &TypeWorld,
) -> Typed {
    let mut functions = Vec::new();
    let mut sites = Vec::new();
    for (function, f) in p.functions.iter().enumerate() {
        let graph =
            cfg::Graph::build(f, world.len(), &mut 0).expect("actual private dense SSA/CFG kernel");
        let values = graph.types().collect::<Vec<_>>();
        let places = places::check(f, &values, world, &mut 0)
            .expect("actual private place/type/owner kernel");
        let mut rows = Vec::new();
        let mut callees = Vec::new();
        for (block, b) in f.blocks.iter().enumerate() {
            let mut row = Vec::new();
            for instruction in 0..b.instructions.len() {
                row.push(sites.len());
                sites.push(Site { function, block, instruction });
            }
            rows.push(row);
            callees.push(vec![None; b.instructions.len()]);
        }
        let upgrades = f
            .blocks
            .iter()
            .map(|b| match b.terminator {
                raw::Terminator::WeakUpgradeBranch { failure, .. } => {
                    Some(f.cleanup_plans[failure as usize].span)
                }
                _ => None,
            })
            .collect();
        functions.push(Function {
            graph,
            values,
            places,
            callees,
            sites: rows,
            may_fail: true,
            upgrades,
            match_scopes: Vec::new(),
        });
    }
    let facts = (0..world.len())
        .map(|id| {
            world
                .fact(u32::try_from(id).expect("small fixture ID"))
                .expect("checked source-world fact")
        })
        .collect();
    Typed {
        functions,
        sites,
        facts,
        frontier: frontier::Table::build(world, &mut 0).expect("actual finite frontier kernel"),
    }
}
