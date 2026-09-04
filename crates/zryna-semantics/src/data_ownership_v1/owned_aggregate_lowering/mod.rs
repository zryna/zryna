use std::collections::{BTreeMap, BTreeSet};

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, raw as raw_layout};
use zryna_syntax::v4 as syntax;

use super::owned_constructor_plan::ConstructorValueTypes;
use super::{Binding, Decl, Errors, OwnerState, SemanticInput, Ty};

mod assignment_planning;
mod assignments;
mod clone;
mod constructor_resources;
mod constructors;
mod driver;
mod partial_transfers;
mod projected_reads;
mod projection_resolution;
#[cfg(test)]
#[path = "../tests/projection_resolution_checks.rs"]
pub(in crate::data_ownership_v1) mod projection_resolution_checks;
mod projection_topology;
mod projections;
mod shape;
mod state;
mod statements;

pub(super) use driver::{
    is_private_owned_aggregate_candidate, lower_private_owned_aggregate_function,
};
use statements::StatementOutcome;

use shape::owned_enum_graph_is_supported;
pub(super) use shape::{aggregate_graph_is_supported, complete_owned_projection_shape};

struct PrivateOwnedAggregateLowerer<'a, 'f, 'e> {
    input: SemanticInput<'a>,
    file: &'a syntax::SourceUnit,
    function: &'f syntax::RawFunctionSyntax,
    module: usize,
    declarations: &'a [Decl],
    graph: &'a raw_layout::Graph,
    node_types: &'a [Option<Ty>],
    layouts: &'a layout::VerifiedLayouts,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved_projections: BTreeSet<raw::PlaceId>,
    partial_roots: BTreeSet<raw::PlaceId>,
    places: Vec<raw::Place>,
    instructions: Vec<raw::Instruction>,
    constructor_types: ConstructorValueTypes,
    constructor_storage: constructor_resources::ConstructorStorage,
    cleanup_plans: Vec<raw::CleanupPlan>,
    cleanup_actions: usize,
    aggregate_operands: usize,
    aggregate_subobject_moves: usize,
    projected_aggregate_clones: usize,
    projected_aggregate_assignments: usize,
    reserved_transitions: usize,
    owners: OwnerState,
    next_value: u32,
    next_local: u32,
}
