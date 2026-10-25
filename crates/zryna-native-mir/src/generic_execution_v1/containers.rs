//! Finite helpers retain exact new Clone views and matching successor runtime metadata.
use super::{
    Diagnostic, budget, invariant,
    planning::{reserve, take},
    shapes::Shape,
};
use zryna_ir::generic_v1::execution_v1::{VerifiedCloneNode, VerifiedExecution};
use zryna_layout::{StorageTarget, TypeCategory};
use zryna_ownership_runtime_abi::{
    LogicalOperation,
    generic_v1::{ControlLayout, ElementLayout},
};
mod frontier;
#[derive(Clone, Copy, Default)]
pub(super) struct Extensions {
    pub push: bool,
    pub upgrade: bool,
    pub concat: bool,
}
/// Exact per-program container representation; no public constructor or older ABI identity.
pub struct ContainerPlan<'p, 'a> {
    elements: Vec<Option<ElementLayout>>,
    elements_by_child: Vec<Option<ElementLayout>>,
    controls: Vec<Option<ControlLayout>>,
    clones: Vec<Option<VerifiedCloneNode<'p, 'a>>>,
    clone_symbols: Vec<Option<String>>,
    drop_symbols: Vec<Option<String>>,
    operations: Vec<LogicalOperation>,
}
impl<'p, 'a> ContainerPlan<'p, 'a> {
    /// Exact Vec element allocation metadata for this stored container.
    #[must_use]
    pub fn element(&self, ty: u32) -> Option<ElementLayout> {
        self.elements.get(ty as usize).copied().flatten()
    }
    /// Exact retained native element metadata by canonical element identity.
    /// Metadata alone grants no source, Clone, drop or invocation effect permission.
    #[must_use]
    pub fn element_by_identity(&self, element: u32) -> Option<ElementLayout> {
        self.elements_by_child.get(element as usize).copied().flatten()
    }
    /// Exact Shared/Weak control metadata for this stored container.
    #[must_use]
    pub fn control(&self, ty: u32) -> Option<ControlLayout> {
        self.controls.get(ty as usize).copied().flatten()
    }
    /// Whether a native control geometry is retained in this exact layout world.
    /// Metadata matching grants neither source permission nor an invocation effect.
    #[must_use]
    pub fn matches_control_geometry(&self, size: u64, alignment: u64) -> bool {
        self.controls.iter().flatten().any(|row| row.size() == size && row.alignment() == alignment)
    }
    /// Source-authorized finite Clone node retained from this same program's operations.
    #[must_use]
    pub fn clone_node(&self, ty: u32) -> Option<VerifiedCloneNode<'p, 'a>> {
        self.clones.get(ty as usize).copied().flatten()
    }
    /// Exact checked helper symbol inventories, including absent resource-free drops.
    pub fn clone_symbols(&self) -> impl ExactSizeIterator<Item = Option<&str>> {
        self.clone_symbols.iter().map(Option::as_deref)
    }
    /// Exact resource drop helper symbols in logical type order.
    pub fn drop_symbols(&self) -> impl ExactSizeIterator<Item = Option<&str>> {
        self.drop_symbols.iter().map(Option::as_deref)
    }
    /// Required existing runtime logical operations; these do not mint invocation proof.
    #[must_use]
    pub fn operations(&self) -> &[LogicalOperation] {
        &self.operations
    }
}
pub(super) fn derive<'p, 'a>(
    program: &'p VerifiedExecution<'a>,
    types: &[Shape<'_>],
    required: Extensions,
    work: &mut u64,
) -> Result<Option<ContainerPlan<'p, 'a>>, Diagnostic> {
    take(work, (types.len() as u64).checked_mul(16).ok_or_else(budget)?)?;
    if !types.iter().any(|s| {
        matches!(s.view.category(), TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak)
    }) {
        return Ok(None);
    }
    let metadata = program
        .runtime()
        .element_layouts()
        .len()
        .checked_add(program.runtime().control_layouts().len())
        .ok_or_else(budget)?;
    take(
        work,
        (metadata as u64)
            .checked_mul(128)
            .and_then(|n| n.checked_add((types.len() as u64).checked_mul(1024)?))
            .ok_or_else(budget)?,
    )?;
    let mut elements_by_child = reserve(types.len())?;
    elements_by_child.resize(types.len(), None);
    let mut controls_by_child = reserve(types.len())?;
    controls_by_child.resize(types.len(), None);
    for record in program.runtime().element_layouts() {
        if record.target() == StorageTarget::LinuxX8664V1 {
            let index = record.element().index() as usize;
            if types.get(index).ok_or_else(invariant)?.view.id() != record.element() {
                return Err(invariant());
            }
            elements_by_child[index] = Some(*record);
        }
    }
    for record in program.runtime().control_layouts() {
        if record.target() == StorageTarget::LinuxX8664V1 {
            let index = record.payload().index() as usize;
            if types.get(index).ok_or_else(invariant)?.view.id() != record.payload() {
                return Err(invariant());
            }
            controls_by_child[index] = Some(*record);
        }
    }
    let mut elements = reserve(types.len())?;
    let mut controls = reserve(types.len())?;
    for shape in types {
        let child = shape.view.referenced_type().map(|id| id.index() as usize);
        elements.push(if shape.view.category() == TypeCategory::Vec {
            Some(elements_by_child[child.ok_or_else(invariant)?].ok_or_else(invariant)?)
        } else {
            None
        });
        controls.push(
            if matches!(shape.view.category(), TypeCategory::Shared | TypeCategory::Weak) {
                Some(controls_by_child[child.ok_or_else(invariant)?].ok_or_else(invariant)?)
            } else {
                None
            },
        );
    }
    let clones = frontier::derive(program, types.len(), work)?;
    let mut clone_symbols = reserve(types.len())?;
    let mut drop_symbols = reserve(types.len())?;
    for (index, shape) in types.iter().enumerate() {
        clone_symbols.push(clones[index].map(|_| format!("zryna_gexecution_clone_{index}")));
        drop_symbols
            .push((shape.view.drop_kind() != 0).then(|| format!("zryna_gexecution_drop_{index}")));
        take(work, shape.view.size().div_ceil(8).checked_mul(128).ok_or_else(budget)?)?;
        take(
            work,
            (shape.view.fields().len() as u64)
                .checked_add(shape.view.variants().len() as u64)
                .and_then(|n| n.checked_mul(512))
                .ok_or_else(budget)?,
        )?;
    }
    // Full helper-reference table fanout is paid before any backend context/allocation.
    let helpers = clone_symbols.iter().chain(&drop_symbols).filter(|s| s.is_some()).count();
    take(
        work,
        (program.functions().len() as u64)
            .checked_add(helpers as u64)
            .and_then(|n| n.checked_mul(types.len() as u64))
            .and_then(|n| n.checked_mul(256))
            .ok_or_else(budget)?,
    )?;
    let operations = operations(required)?;

    Ok(Some(ContainerPlan {
        elements,
        elements_by_child,
        controls,
        clones,
        clone_symbols,
        drop_symbols,
        operations,
    }))
}

fn operations(required: Extensions) -> Result<Vec<LogicalOperation>, Diagnostic> {
    use LogicalOperation as O;
    let mut operations = reserve(11)?;
    operations.extend([
        O::Allocate,
        O::VecAllocate,
        O::VecReleaseStorage,
        O::StrongClone,
        O::WeakDowngrade,
        O::WeakClone,
        O::StrongReleaseBegin,
        O::StrongReleaseFinish,
        O::WeakRelease,
    ]);
    if required.push {
        operations.push(O::VecReserve);
    }
    if required.upgrade {
        operations.push(O::WeakUpgrade);
    }
    Ok(operations)
}
