use std::collections::{BTreeMap, BTreeSet};

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, raw as raw_layout};
use zryna_syntax::v4 as syntax;

use super::{Binding, Decl, Errors, OwnerState, SemanticInput, Ty};

mod assignment_planning;
mod assignments;
mod clone;
mod constructors;
mod partial_transfers;
mod projected_reads;
mod projections;
mod shape;
mod state;
mod statements;

pub(super) use statements::StatementOutcome;

pub(super) use shape::{
    aggregate_graph_is_supported, complete_owned_projection_shape, owned_enum_graph_is_supported,
};

pub(super) struct PrivateOwnedAggregateLowerer<'a, 'f, 'e> {
    pub(super) input: SemanticInput<'a>,
    pub(super) file: &'a syntax::SourceUnit,
    pub(super) function: &'f syntax::RawFunctionSyntax,
    pub(super) module: usize,
    pub(super) declarations: &'a [Decl],
    pub(super) graph: &'a raw_layout::Graph,
    pub(super) node_types: &'a [Option<Ty>],
    pub(super) layouts: &'a layout::VerifiedLayouts,
    pub(super) errors: &'e mut Errors<'a>,
    pub(super) bindings: BTreeMap<String, Binding>,
    pub(super) projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    pub(super) moved_projections: BTreeSet<raw::PlaceId>,
    pub(super) partial_roots: BTreeSet<raw::PlaceId>,
    pub(super) places: Vec<raw::Place>,
    pub(super) instructions: Vec<raw::Instruction>,
    pub(super) cleanup_plans: Vec<raw::CleanupPlan>,
    pub(super) cleanup_actions: usize,
    pub(super) aggregate_operands: usize,
    pub(super) aggregate_subobject_moves: usize,
    pub(super) projected_aggregate_clones: usize,
    pub(super) projected_aggregate_assignments: usize,
    pub(super) reserved_transitions: usize,
    pub(super) owners: OwnerState,
    pub(super) next_value: u32,
    pub(super) next_local: u32,
}
