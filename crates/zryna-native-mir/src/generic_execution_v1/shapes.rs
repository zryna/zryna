//! Memoized all-variant expansion census; compact DAGs cannot trigger unpaid expansion.
use super::{
    Diagnostic, MAX_TYPE_LANES, budget, invariant,
    planning::{reserve, take},
};
use zryna_layout::{
    TypeCategory,
    generic_v1::{TypeView, VerifiedLayouts},
};
#[derive(Clone, Copy)]
pub(super) struct Shape<'a> {
    pub view: TypeView<'a>,
    pub lanes: u64,
    pub expanded: u64,
    pub strings: u64,
}
pub(super) fn derive<'a>(
    layouts: &'a VerifiedLayouts,
    work: &mut u64,
) -> Result<Vec<Shape<'a>>, Diagnostic> {
    take(work, (layouts.types().len() as u64).checked_mul(128).ok_or_else(budget)?)?;
    let mut views = reserve(layouts.types().len())?;
    views.extend(layouts.types());
    let mut cache = reserve(views.len())?;
    cache.resize(views.len(), None);
    let mut marks = reserve(views.len())?;
    marks.resize(views.len(), false);
    for i in 0..views.len() {
        shape(i, &views, &mut cache, &mut marks, 0, work)?;
    }
    cache.into_iter().map(|s| s.ok_or_else(invariant)).collect()
}
fn shape<'a>(
    id: usize,
    views: &[TypeView<'a>],
    cache: &mut [Option<Shape<'a>>],
    marks: &mut [bool],
    depth: usize,
    work: &mut u64,
) -> Result<Shape<'a>, Diagnostic> {
    take(work, 1)?;
    if depth > zryna_layout::MAX_TRAVERSAL_DEPTH {
        return Err(budget());
    }
    if let Some(value) = cache.get(id).copied().flatten() {
        return Ok(value);
    }
    if *marks.get(id).ok_or_else(invariant)? {
        return Err(invariant());
    }
    marks[id] = true;
    let view = *views.get(id).ok_or_else(invariant)?;
    let count = view.fields().len().checked_add(view.variants().len()).ok_or_else(budget)?;
    take(work, (count as u64).checked_mul(16).ok_or_else(budget)?)?;
    let mut lanes = 0u64;
    let mut expanded = 1u64;
    let mut strings = 0u64;
    let mut child = |id: u32| shape(id as usize, views, cache, marks, depth + 1, work);
    match view.category() {
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::Shared | TypeCategory::Weak => {
            lanes = 1;
        }
        TypeCategory::String => {
            lanes = 3;
            strings = 1;
        }
        TypeCategory::Struct => {
            for (_, id, _) in view.fields() {
                let c = child(id.index())?;
                lanes = lanes.checked_add(c.lanes).ok_or_else(budget)?;
                expanded = expanded.checked_add(c.expanded).ok_or_else(budget)?;
                strings = strings.checked_add(c.strings).ok_or_else(budget)?;
            }
        }
        TypeCategory::Enum => {
            for (_, id) in view.variants() {
                if let Some(id) = id {
                    let c = child(id.index())?;
                    lanes = lanes.max(c.lanes);
                    expanded = expanded.checked_add(c.expanded).ok_or_else(budget)?;
                    strings = strings.checked_add(c.strings).ok_or_else(budget)?;
                }
            }
            lanes = lanes.checked_add(1).ok_or_else(budget)?;
        }
        TypeCategory::FixedArray => {
            let c = child(view.referenced_type().ok_or_else(invariant)?.index())?;
            let length = view.array_length().ok_or_else(invariant)?;
            lanes = length.checked_mul(c.lanes).ok_or_else(budget)?;
            expanded = expanded
                .checked_add(length.checked_mul(c.expanded).ok_or_else(budget)?)
                .ok_or_else(budget)?;
            strings = length.checked_mul(c.strings).ok_or_else(budget)?;
        }
        TypeCategory::Vec => lanes = 3,
    }
    if lanes > u64::from(MAX_TYPE_LANES)
        || expanded > super::MAX_CODEGEN_UNITS
        || view.size() > i32::MAX as u64
    {
        return Err(budget());
    }
    let value = Shape { view, lanes, expanded, strings };
    cache[id] = Some(value);
    marks[id] = false;
    Ok(value)
}
