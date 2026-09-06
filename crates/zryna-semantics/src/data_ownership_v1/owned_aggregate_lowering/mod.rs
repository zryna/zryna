use std::collections::{BTreeMap, BTreeSet};

use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, raw as raw_layout};
use zryna_syntax::v4 as syntax;

use super::owned_constructor_plan::ConstructorValueTypes;
use super::{Binding, Decl, Errors, OwnerState, SemanticInput, Ty};

mod assignment_planning;
mod assignments;
mod availability;
mod chained_indexed_preparation;
mod clone;
mod clone_decisions;
mod constructor_preparation;
mod constructor_resources;
mod constructors;
mod copy_statements;
mod driver;
mod expression_decisions;
mod fresh_indexed_preparation;
mod generic_clone_preparation;
mod generic_function_shape;
mod generic_projection_preparation;
mod handle_preparation;
mod indexed_vec_preparation;
mod lexical_chained_preparation;
mod lexical_indexed_preparation;
mod lexical_indexed_scope;
mod lexical_indexed_statements;
mod mixed_shape;
mod nonindexed_borrow_shape;
pub(in crate::data_ownership_v1) use nonindexed_borrow_shape::has_nonindexed_owned_borrow;
mod operand_decisions;
mod ordinary_indexed_array_preparation;
mod partial_transfers;
mod preparation_operations;
mod preparation_plan;
mod preparation_state;
mod projected_reads;
mod projection_resolution;
#[cfg(test)]
#[path = "../tests/projection_resolution_checks.rs"]
pub(in crate::data_ownership_v1) mod projection_resolution_checks;
mod projection_topology;
mod projections;
mod resource_decisions;
mod shape;
mod state;
mod statements;
mod structured_call;
mod structured_call_arguments;
mod structured_cfg;
mod structured_checkpoint;
mod structured_constructor;
mod structured_graph;
mod structured_handoff;
mod structured_indexed;
mod structured_indexed_preparation;
mod structured_match;
mod structured_match_local;
mod structured_scratch;
mod structured_shape;
mod structured_state;
mod structured_string;
mod structured_upgrade;
mod vec_push_preparation;

pub(super) use driver::{
    is_private_mixed_constructor_candidate, is_private_owned_aggregate_candidate,
    lower_private_owned_aggregate_function,
};
pub(super) use generic_function_shape::requires_generic_function;
pub(super) use lexical_indexed_statements::has_indexed_borrow;
use statements::StatementOutcome;
pub(super) use structured_shape::requires_structured_cfg;

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
    catalog: &'a super::function_catalog::FunctionCatalog,
    mixed_function: bool,
    errors: &'e mut Errors<'a>,
    bindings: BTreeMap<String, Binding>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved_projections: BTreeSet<raw::PlaceId>,
    partial_roots: BTreeSet<raw::PlaceId>,
    places: Vec<raw::Place>,
    instructions: Vec<raw::Instruction>,
    constructor_types: ConstructorValueTypes,
    constructor_storage: constructor_resources::ConstructorStorage,
    preparation_facts: preparation_plan::PreparationFacts,
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
