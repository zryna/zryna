use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4 as syntax;

use super::SemanticInput;
use super::diagnostics::Errors;
use super::function_catalog::FunctionCatalog;
use super::layout_graph::Decl;
use super::owned_cfg_state::OwnedCfgState;
use super::owner_state::OwnerState;
use super::type_model::{Binding, Ty};

mod assignments;
mod branches;
mod expressions_and_calls;
mod preparation;

#[derive(Clone, Copy)]
pub(in crate::data_ownership_v1) struct StringBranchTypes<'a> {
    pub(in crate::data_ownership_v1) file: &'a syntax::SourceUnit,
    pub(in crate::data_ownership_v1) declarations: &'a [Decl],
    pub(in crate::data_ownership_v1) graph: &'a zryna_layout::raw::Graph,
    pub(in crate::data_ownership_v1) node_types: &'a [Option<Ty>],
}

pub(in crate::data_ownership_v1) struct PrivateStringLowerer<'a, 'f, 'e> {
    pub(in crate::data_ownership_v1) input: SemanticInput<'a>,
    pub(in crate::data_ownership_v1) function: &'f syntax::RawFunctionSyntax,
    pub(in crate::data_ownership_v1) module: usize,
    pub(in crate::data_ownership_v1) ty: Ty,
    pub(in crate::data_ownership_v1) catalog: &'a FunctionCatalog,
    pub(in crate::data_ownership_v1) errors: &'e mut Errors<'a>,
    pub(in crate::data_ownership_v1) bindings: BTreeMap<String, Binding>,
    pub(in crate::data_ownership_v1) places: Vec<raw::Place>,
    pub(in crate::data_ownership_v1) reserved_places: usize,
    pub(in crate::data_ownership_v1) cfg: OwnedCfgState,
    pub(in crate::data_ownership_v1) cleanup_plans: Vec<raw::CleanupPlan>,
    pub(in crate::data_ownership_v1) cleanup_actions: usize,
    pub(in crate::data_ownership_v1) reserved_cleanup_plans: usize,
    pub(in crate::data_ownership_v1) reserved_cleanup_actions: usize,
    pub(in crate::data_ownership_v1) owners: OwnerState,
    pub(in crate::data_ownership_v1) known_bytes: BTreeMap<raw::PlaceId, Option<u64>>,
    pub(in crate::data_ownership_v1) next_value: u32,
    pub(in crate::data_ownership_v1) next_local: u32,
}
