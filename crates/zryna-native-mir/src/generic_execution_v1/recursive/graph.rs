//! Paid CSR ownership graph; Weak and zero-length arrays have no traversal edge.
use super::{Diagnostic, Shape, budget, invariant, reserve, take};
use zryna_layout::TypeCategory;
pub(super) struct Graph {
    starts: Vec<usize>,
    edges: Vec<u32>,
}
impl Graph {
    pub fn derive(types: &[Shape<'_>], work: &mut u64) -> Result<Self, Diagnostic> {
        let n = u64::try_from(types.len()).map_err(|_| budget())?;
        // Pays all type/header visits and CSR/queue/indegree/reachability scratch.
        take(work, n.checked_mul(64).and_then(|v| v.checked_add(256)).ok_or_else(budget)?)?;
        let mut bound = 0usize;
        for shape in types {
            bound = bound
                .checked_add(shape.view.fields().len())
                .and_then(|v| v.checked_add(shape.view.variants().len()))
                .and_then(|v| v.checked_add(1))
                .ok_or_else(budget)?;
        }
        // 4 bytes per edge plus build/reachability/indegree/Kahn visits and guards.
        // Bound includes nullary rows and unused reference slots; no hidden scan.
        take(
            work,
            u64::try_from(bound).map_err(|_| budget())?.checked_mul(16).ok_or_else(budget)?,
        )?;
        let mut starts = reserve(types.len().checked_add(1).ok_or_else(budget)?)?;
        let mut edges = reserve(bound)?;
        for shape in types {
            starts.push(edges.len());
            let view = shape.view;
            match view.category() {
                TypeCategory::Struct => {
                    edges.extend(view.fields().map(|(_, id, _)| id.index()));
                }
                TypeCategory::Enum => {
                    edges.extend(
                        view.variants()
                            .filter_map(|(_, id)| id.map(zryna_layout::generic_v1::TypeId::index)),
                    );
                }
                TypeCategory::FixedArray if view.array_length().ok_or_else(invariant)? != 0 => {
                    edges.push(view.referenced_type().ok_or_else(invariant)?.index());
                }
                TypeCategory::Vec | TypeCategory::Shared => {
                    edges.push(view.referenced_type().ok_or_else(invariant)?.index());
                }
                _ => {}
            }
        }
        starts.push(edges.len());
        Ok(Self { starts, edges })
    }
    pub fn children(&self, id: usize) -> Result<&[u32], Diagnostic> {
        let begin = *self.starts.get(id).ok_or_else(invariant)?;
        let end =
            *self.starts.get(id.checked_add(1).ok_or_else(invariant)?).ok_or_else(invariant)?;
        self.edges.get(begin..end).ok_or_else(invariant)
    }
}
