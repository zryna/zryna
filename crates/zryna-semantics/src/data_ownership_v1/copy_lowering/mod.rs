use std::collections::BTreeMap;

use super::{
    Binding, Decl, Errors, FunctionCatalog, SemanticInput, Ty, layout, raw, raw_layout, syntax,
};

mod resolvers;
mod state;

pub(super) struct FunctionLowerer<'a, 'f, 'e> {
    pub(super) input: SemanticInput<'a>,
    pub(super) file: &'a syntax::SourceUnit,
    pub(super) function: &'f syntax::RawFunctionSyntax,
    pub(super) module: usize,
    pub(super) declarations: &'a [Decl],
    pub(super) graph: &'a raw_layout::Graph,
    pub(super) node_types: &'a [Option<Ty>],
    pub(super) layouts: &'a layout::VerifiedLayouts,
    pub(super) catalog: &'a FunctionCatalog,
    pub(super) errors: &'e mut Errors<'a>,
    pub(super) bindings: BTreeMap<String, Binding>,
    pub(super) borrow_bindings: BTreeMap<String, BorrowBinding>,
    pub(super) places: Vec<raw::Place>,
    pub(super) projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    pub(super) instructions: Vec<raw::Instruction>,
    pub(super) cleanup_plans: Vec<raw::CleanupPlan>,
    pub(super) values: u32,
}

#[derive(Clone, Copy)]
pub(super) struct BorrowBinding {
    pub(super) ty: Ty,
    pub(super) borrow: raw::BorrowId,
    pub(super) access: raw::BorrowAccess,
}
