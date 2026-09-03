use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory, raw as raw_layout};
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::diagnostics::Errors;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Ty {
    pub(super) layout: layout::TypeId,
    pub(super) ir: raw::TypeId,
    pub(super) category: TypeCategory,
    pub(super) drop_kind: u32,
    pub(super) runtime_kind: u32,
    pub(super) cloneable: bool,
}

impl Ty {
    pub(super) const fn is_copy(self) -> bool {
        self.drop_kind == 0
    }

    pub(super) const fn is_clone(self) -> bool {
        self.cloneable
    }
}

pub(super) fn map_node_types(
    graph: &raw_layout::Graph,
    layouts: &layout::VerifiedLayouts,
    errors: &mut Errors<'_>,
) -> Vec<Option<Ty>> {
    let mut result: Vec<Option<Ty>> = vec![None; graph.types.len()];
    for node in &graph.types {
        let found = match &node.kind {
            raw_layout::TypeKind::Bool => {
                layouts.types().find(|t| t.category() == TypeCategory::Bool)
            }
            raw_layout::TypeKind::I32 => {
                layouts.types().find(|t| t.category() == TypeCategory::I32)
            }
            raw_layout::TypeKind::String => {
                layouts.types().find(|t| t.category() == TypeCategory::String)
            }
            raw_layout::TypeKind::Struct { module, declaration, .. }
            | raw_layout::TypeKind::Enum { module, declaration, .. } => {
                layouts.types().find(|t| t.nominal_identity() == Some((module.0, *declaration)))
            }
            raw_layout::TypeKind::FixedArray { element, length } => {
                let element_index = usize::try_from(element.0).ok();
                let element_id =
                    element_index.and_then(|i| result.get(i)).and_then(|v| *v).map(|v| v.layout);
                layouts.types().find(|t| {
                    t.category() == TypeCategory::FixedArray
                        && t.array_length() == Some(*length)
                        && t.referenced_type() == element_id
                })
            }
            raw_layout::TypeKind::Vec { element } => {
                let element_index = usize::try_from(element.0).ok();
                let element_id =
                    element_index.and_then(|i| result.get(i)).and_then(|v| *v).map(|v| v.layout);
                layouts.types().find(|ty| {
                    ty.category() == TypeCategory::Vec && ty.referenced_type() == element_id
                })
            }
            _ => None,
        };
        if let Some(found) = found {
            let index = usize::try_from(node.id.0).expect("bounded node");
            result[index] = Some(Ty {
                layout: found.id(),
                ir: raw::TypeId(found.id().index()),
                category: found.category(),
                drop_kind: found.drop_kind(),
                runtime_kind: found.runtime_kind(),
                cloneable: false,
            });
        } else {
            errors.global(
                "ZRYNA-M3004",
                format!("derived layout type node #{} has no sealed identity", node.id.0),
                "reduce the aggregate graph and report this deterministic compiler failure",
            );
        }
    }
    let clone_capabilities = derive_clone_capabilities(graph);
    for (index, cloneable) in clone_capabilities.into_iter().enumerate() {
        if let Some(ty) = result[index].as_mut() {
            ty.cloneable = cloneable;
        }
    }
    result
}

fn derive_clone_capabilities(graph: &raw_layout::Graph) -> Vec<bool> {
    let mut capabilities = graph
        .types
        .iter()
        .map(|node| !matches!(node.kind, raw_layout::TypeKind::Borrow { .. }))
        .collect::<Vec<_>>();
    loop {
        let previous = capabilities.clone();
        for node in &graph.types {
            let child = |id: raw_layout::NodeId| {
                usize::try_from(id.0)
                    .ok()
                    .and_then(|index| previous.get(index))
                    .copied()
                    .unwrap_or(false)
            };
            capabilities[node.id.0 as usize] = match &node.kind {
                raw_layout::TypeKind::Bool
                | raw_layout::TypeKind::I32
                | raw_layout::TypeKind::String
                | raw_layout::TypeKind::Shared { .. }
                | raw_layout::TypeKind::Weak { .. } => true,
                raw_layout::TypeKind::Struct { fields, .. } => {
                    fields.iter().all(|field| child(field.ty))
                }
                raw_layout::TypeKind::Enum { variants, .. } => {
                    variants.iter().all(|variant| variant.payload.is_none_or(child))
                }
                raw_layout::TypeKind::FixedArray { element, .. }
                | raw_layout::TypeKind::Vec { element } => child(*element),
                raw_layout::TypeKind::Borrow { .. } => false,
            };
        }
        if capabilities == previous {
            return capabilities;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Binding {
    pub(super) ty: Ty,
    pub(super) place: raw::PlaceId,
    pub(super) mutable: bool,
}

pub(super) enum ProjectedAggregateAssignmentSource {
    MoveRoot { name: syntax::RawIdentifierSyntax, at: Span },
    MoveProjection { expression: u32, missing_path_places: usize, missing_descendant_places: usize },
    CloneRoot { binding: Binding, at: Span },
    CloneProjection { expression: u32, at: Span, missing_path_places: usize },
}

#[derive(Clone, Copy)]
pub(super) enum RootBorrowLiteral {
    Bool(bool),
    I32(i32),
}

#[derive(Clone)]
pub(super) enum RootBorrowInitializer {
    Literal { literal: RootBorrowLiteral, ty: Ty, at: Span },
    Struct { ty: Ty, fields: Vec<Self>, at: Span },
    FixedArray { ty: Ty, elements: Vec<Self>, at: Span },
}

impl RootBorrowInitializer {
    pub(super) fn checked_value_count(&self) -> Option<usize> {
        match self {
            Self::Literal { .. } => Some(1),
            Self::Struct { fields, .. } => fields
                .iter()
                .try_fold(1_usize, |count, field| count.checked_add(field.checked_value_count()?)),
            Self::FixedArray { elements, .. } => {
                elements.iter().try_fold(1_usize, |count, element| {
                    count.checked_add(element.checked_value_count()?)
                })
            }
        }
    }

    pub(super) fn value_count(&self) -> usize {
        match self {
            Self::Literal { .. } => 1,
            Self::Struct { fields, .. } => fields
                .iter()
                .fold(1_usize, |count, field| count.saturating_add(field.value_count())),
            Self::FixedArray { elements, .. } => elements
                .iter()
                .fold(1_usize, |count, element| count.saturating_add(element.value_count())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum RootBorrowProjectionKey {
    StructField(u32),
    FixedArrayConstant(u32),
}

#[derive(Clone)]
pub(super) struct RootBorrowProjection {
    pub(super) key: RootBorrowProjectionKey,
    pub(super) ty: Ty,
    pub(super) at: Span,
}

#[derive(Clone)]
pub(super) struct RootBorrowPlacePlan {
    pub(super) ty: Ty,
    pub(super) projections: Vec<RootBorrowProjection>,
}

impl RootBorrowPlacePlan {
    pub(super) fn key(&self) -> Vec<RootBorrowProjectionKey> {
        self.projections.iter().map(|projection| projection.key).collect()
    }
}

#[derive(Clone)]
pub(super) enum RootBorrowStep {
    Begin { id: raw::BorrowId, place: RootBorrowPlacePlan, access: raw::BorrowAccess, at: Span },
    Read { id: raw::BorrowId, ty: Ty, at: Span },
    Write { id: raw::BorrowId, value: RootBorrowInitializer, at: Span },
    OwnerRead { place: RootBorrowPlacePlan, at: Span },
    Call(RootBorrowCallPlan),
}

#[derive(Clone)]
pub(super) enum RootBorrowCallArgumentPlan {
    Value { index: u32, value: RootBorrowInitializer },
    Borrow { index: u32, id: raw::BorrowId },
}

#[derive(Clone)]
pub(super) struct RootBorrowCallPlan {
    pub(super) callee: raw::FunctionId,
    pub(super) arguments: Vec<RootBorrowCallArgumentPlan>,
    pub(super) value_parameters: usize,
    pub(super) borrow_parameters: usize,
    pub(super) result: Ty,
    pub(super) at: Span,
}

impl RootBorrowCallPlan {
    pub(super) fn checked_argument_value_count(&self) -> Option<usize> {
        self.arguments.iter().try_fold(0_usize, |count, argument| match argument {
            RootBorrowCallArgumentPlan::Value { value, .. } => {
                count.checked_add(value.checked_value_count()?)
            }
            RootBorrowCallArgumentPlan::Borrow { .. } => Some(count),
        })
    }
}

#[derive(Clone)]
pub(super) struct RootBorrowAlias {
    pub(super) id: raw::BorrowId,
    pub(super) ty: Ty,
    pub(super) place: RootBorrowPlacePlan,
    pub(super) access: raw::BorrowAccess,
    pub(super) used: bool,
}

pub(super) struct RootBorrowPlan {
    pub(super) root_ty: Ty,
    pub(super) root_initializer: RootBorrowInitializer,
    pub(super) root_at: Span,
    pub(super) shape: RootBorrowShape,
    pub(super) aliases: usize,
    pub(super) reads: usize,
    pub(super) writes: usize,
    pub(super) calls: usize,
    pub(super) call_values: usize,
    pub(super) return_at: Span,
}

pub(super) struct RootBorrowArmPlan {
    pub(super) steps: Vec<RootBorrowStep>,
    pub(super) aliases: usize,
    pub(super) reads: usize,
    pub(super) writes: usize,
    pub(super) calls: usize,
    pub(super) call_values: usize,
    pub(super) block_exit: Span,
}

pub(super) enum RootBorrowShape {
    Straight(RootBorrowArmPlan),
    Loop {
        condition_at: Span,
        loop_at: Span,
        body: RootBorrowArmPlan,
    },
    Conditional {
        condition_at: Span,
        branch_at: Span,
        then_arm: RootBorrowArmPlan,
        else_arm: RootBorrowArmPlan,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootBorrowBudgetLimit {
    Values,
    Places,
    Transitions,
    Blocks,
    Edges,
    ActiveBorrows,
    CleanupPlans,
}

#[derive(Clone, Copy, Default)]
pub(super) struct RootBorrowResources {
    pub(super) values: usize,
    pub(super) places: usize,
    pub(super) transitions: usize,
    pub(super) blocks: usize,
    pub(super) edges: usize,
    pub(super) active_peak: usize,
    pub(super) cleanup_plans: usize,
}

pub(super) struct OwnedRootBorrowSyntax {
    pub(super) synthetic: syntax::RawFunctionSyntax,
    pub(super) borrow_at: Span,
    pub(super) end_at: Span,
}

#[derive(Clone, Copy)]
pub(super) struct TerminalOwnedIf {
    pub(super) condition: u32,
    pub(super) then_value: u32,
    pub(super) then_span: Span,
    pub(super) else_value: u32,
    pub(super) else_span: Span,
    pub(super) span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProjectedAggregateMoveContext {
    DirectLocal,
    FinalReturn,
    ProjectedReplacement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OwnedAggregatePlace {
    pub(super) ty: Ty,
    pub(super) place: raw::PlaceId,
    pub(super) root: raw::PlaceId,
    pub(super) mutable: bool,
    pub(super) is_root: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedAggregatePlacePreflight {
    pub(super) place: OwnedAggregatePlace,
    pub(super) missing: usize,
    pub(super) lineage: Vec<raw::PlaceId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OwnedStaticProjectionKind {
    StructField { ordinal: u32 },
    FixedArrayConstant { index: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OwnedProjectionShapeEntry {
    pub(super) parent: Option<usize>,
    pub(super) ty: Ty,
    pub(super) kind: OwnedStaticProjectionKind,
}
